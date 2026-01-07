use std::time::Duration;

use alloy::providers::Provider;
use eyre::{Result, eyre};
use flux_core::{
    AuctionPhase, BlockNumber, BlockProducer, Intent, IntentExecutor, IntentOutcome, IntentResult,
    TokenDepositStatus,
};
use futures::StreamExt;
use tokio::signal;
use tracing::{error, info, warn};

use crate::state::{BidState, BidTracker};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

pub struct Runner<P>
where
    P: Provider + Clone,
{
    executor: IntentExecutor<P>,
    tracker: BidTracker,
    provider: P,
}

impl<P> Runner<P>
where
    P: Provider + Clone,
{
    pub fn new(executor: IntentExecutor<P>, tracker: BidTracker, provider: P) -> Self {
        Self {
            executor,
            tracker,
            provider,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let block_producer = BlockProducer::new(self.provider.clone());
        let mut block_stream = block_producer
            .into_stream()
            .await
            .map_err(|e| eyre!("Failed to create block stream: {:?}", e))?;

        info!("Block stream started");

        loop {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("Received shutdown signal");
                    self.log_final_state();
                    return Ok(());
                }

                block_result = block_stream.next() => {
                    match block_result {
                        Some(Ok(block)) => {
                            if let Err(e) = self.process_block(block).await {
                                error!("Error processing block {}: {:?}", block.as_u64(), e);
                            }


                            if self.tracker.all_terminal() {
                                info!("All bids have reached terminal state");
                                self.log_final_state();
                                return Ok(());
                            }
                        }
                        Some(Err(e)) => {
                            error!("Block stream error: {:?}", e);

                        }
                        None => {
                            warn!("Block stream ended unexpectedly");
                            return Err(eyre!("Block stream ended"));
                        }
                    }
                }
            }
        }
    }

    async fn process_block(&mut self, block: BlockNumber) -> Result<()> {
        let ctx = self.executor.context(block);

        match &ctx.phase {
            AuctionPhase::PreStart { blocks_until_start } => {
                info!(
                    "[block {}] Phase: PreStart ({} blocks until start)",
                    block.as_u64(),
                    blocks_until_start
                );
            }

            AuctionPhase::PreTokens => {
                // Actively poll token balance to detect when tokens are deposited
                // The cache doesn't update unless we execute intents, so we must check directly
                info!(
                    "[block {}] Phase: PreTokens (checking token deposit...)",
                    block.as_u64()
                );

                match self.executor.refresh_tokens_received().await {
                    Ok(TokenDepositStatus::Received) => {
                        info!(
                            "[block {}] Tokens received! Proceeding to Active phase",
                            block.as_u64()
                        );
                        // Tokens are now deposited - handle as Active phase
                        // The next block will have correct phase from cache update during execute
                        self.handle_active_phase().await?;
                    }
                    Ok(TokenDepositStatus::NotReceived | TokenDepositStatus::Unknown) => {
                        info!(
                            "[block {}] Tokens not yet deposited, waiting...",
                            block.as_u64()
                        );
                    }
                    Err(e) => {
                        warn!(
                            "[block {}] Failed to check token balance: {:?}",
                            block.as_u64(),
                            e
                        );
                    }
                }
            }

            AuctionPhase::Active { blocks_remaining } => {
                info!(
                    "[block {}] Phase: Active ({} blocks remaining)",
                    block.as_u64(),
                    blocks_remaining
                );
                self.handle_active_phase().await?;
            }

            AuctionPhase::Ended { blocks_until_claim } => {
                info!(
                    "[block {}] Phase: Ended ({} blocks until claim)",
                    block.as_u64(),
                    blocks_until_claim
                );
                self.handle_ended_phase().await?;
            }

            AuctionPhase::Claimable => {
                info!("[block {}] Phase: Claimable", block.as_u64());
                self.handle_claimable_phase().await?;
            }
        }

        Ok(())
    }

    async fn handle_active_phase(&mut self) -> Result<()> {
        // Collect pending bids to submit
        let pending: Vec<(usize, flux_core::Price, flux_core::CurrencyAmount)> = self
            .tracker
            .pending_bids()
            .filter_map(|(idx, state)| match state {
                BidState::Pending { max_price, amount } => Some((idx, *max_price, *amount)),
                _ => None,
            })
            .collect();

        for (idx, max_price, amount) in pending {
            self.submit_bid(idx, max_price, amount).await;
        }

        Ok(())
    }

    async fn handle_ended_phase(&mut self) -> Result<()> {
        let submitted: Vec<(usize, flux_core::BidId)> = self
            .tracker
            .submitted_bids()
            .filter_map(|(idx, state)| match state {
                BidState::Submitted { bid_id, .. } => Some((idx, *bid_id)),
                _ => None,
            })
            .collect();

        for (idx, bid_id) in submitted {
            self.exit_bid(idx, bid_id).await;
        }

        // Mark zero-token exits as terminal immediately after exit
        // No need to wait for Claimable phase - refund already happened
        self.mark_refunded_bids_terminal();

        Ok(())
    }

    async fn handle_claimable_phase(&mut self) -> Result<()> {
        // First, exit any remaining submitted bids
        self.handle_ended_phase().await?;

        // Then claim tokens for all exited bids with tokens
        let claimable_ids = self.tracker.claimable_bid_ids();
        if !claimable_ids.is_empty() {
            self.claim_tokens(claimable_ids).await;
        }

        Ok(())
    }

    /// Mark exited bids with zero tokens as terminal (Claimed with no tokens)
    /// These bids received full refunds and don't need to wait for claim phase
    fn mark_refunded_bids_terminal(&mut self) {
        for bid in &mut self.tracker.bids {
            if let BidState::Exited {
                bid_id,
                tokens_filled,
                ..
            } = bid
            {
                if tokens_filled.is_zero() {
                    info!(
                        "Bid {} exited with no tokens (full refund) - marking terminal",
                        bid_id.as_u256()
                    );
                    *bid = BidState::Claimed {
                        bid_id: *bid_id,
                        tokens_claimed: *tokens_filled,
                        tx_hash: alloy::primitives::B256::ZERO,
                    };
                }
            }
        }
    }

    async fn submit_bid(
        &mut self,
        idx: usize,
        max_price: flux_core::Price,
        amount: flux_core::CurrencyAmount,
    ) {
        info!(
            "Submitting bid {}: price={}, amount={}",
            idx,
            max_price.as_u256(),
            amount.as_u256()
        );

        // Update state to submitting
        self.tracker.bids[idx] = BidState::Submitting {
            max_price,
            amount,
            attempts: 0,
        };

        let intent = Intent::SubmitBid { max_price, amount };

        match self.execute_with_retry(intent.clone(), idx).await {
            Ok(IntentResult::BidSubmitted(result)) => {
                info!(
                    "Bid {} submitted: id={}, tx={}",
                    idx,
                    result.bid_id.as_u256(),
                    result.tx_hash
                );
                self.tracker.bids[idx] = BidState::Submitted {
                    bid_id: result.bid_id,
                    max_price,
                    amount,
                    tx_hash: result.tx_hash,
                };
            }
            Ok(_) => {
                error!("Unexpected result for submit bid");
                self.tracker.bids[idx] = BidState::Failed {
                    reason: "Unexpected result type".to_string(),
                };
            }
            Err(reason) => {
                error!("Bid {} failed: {}", idx, reason);
                self.tracker.bids[idx] = BidState::Failed { reason };
            }
        }
    }

    async fn exit_bid(&mut self, idx: usize, bid_id: flux_core::BidId) {
        info!("Exiting bid {} (id={})", idx, bid_id.as_u256());

        // Update state to exiting
        self.tracker.bids[idx] = BidState::Exiting {
            bid_id,
            attempts: 0,
        };

        let intent = Intent::Exit { bid_id };

        match self.execute_with_retry(intent.clone(), idx).await {
            Ok(IntentResult::BidExited(result)) => {
                info!(
                    "Bid {} exited: tokens={}, refund={}, tx={}",
                    idx,
                    result.tokens_filled.as_u256(),
                    result.currency_refunded.as_u256(),
                    result.tx_hash
                );
                self.tracker.bids[idx] = BidState::Exited {
                    bid_id,
                    tokens_filled: result.tokens_filled,
                    currency_refunded: result.currency_refunded,
                    tx_hash: result.tx_hash,
                };
            }
            Ok(_) => {
                error!("Unexpected result for exit bid");
                self.tracker.bids[idx] = BidState::Failed {
                    reason: "Unexpected result type".to_string(),
                };
            }
            Err(reason) => {
                error!("Exit bid {} failed: {}", idx, reason);
                self.tracker.bids[idx] = BidState::Failed { reason };
            }
        }
    }

    async fn claim_tokens(&mut self, bid_ids: Vec<flux_core::BidId>) {
        info!("Claiming tokens for {} bids", bid_ids.len());

        // Update states to claiming
        for bid in &mut self.tracker.bids {
            if let BidState::Exited {
                bid_id,
                tokens_filled,
                ..
            } = bid
            {
                if bid_ids.contains(bid_id) && !tokens_filled.is_zero() {
                    *bid = BidState::Claiming {
                        bid_id: *bid_id,
                        tokens_filled: *tokens_filled,
                        attempts: 0,
                    };
                }
            }
        }

        // Try batch claim first
        if bid_ids.len() > 1 {
            if self.try_batch_claim(&bid_ids).await {
                return;
            }
            // Batch failed, fall back to per-bid claims
            warn!("Batch claim failed, falling back to individual claims");
        }

        // Per-bid claims (either single bid or fallback from batch failure)
        for bid_id in &bid_ids {
            self.claim_single_bid(*bid_id).await;
        }
    }

    /// Attempt batch claim, returns true if successful
    async fn try_batch_claim(&mut self, bid_ids: &[flux_core::BidId]) -> bool {
        let intent = Intent::Claim {
            bid_ids: bid_ids.to_vec(),
        };

        match self.execute_with_retry(intent, usize::MAX).await {
            Ok(IntentResult::TokensClaimed(result)) => {
                info!(
                    "Batch claimed {} tokens total, tx={}",
                    result.total_tokens.as_u256(),
                    result.tx_hash
                );

                // Mark all claimed bids
                for bid in &mut self.tracker.bids {
                    if let BidState::Claiming {
                        bid_id,
                        tokens_filled,
                        ..
                    } = bid
                    {
                        if bid_ids.contains(bid_id) {
                            *bid = BidState::Claimed {
                                bid_id: *bid_id,
                                tokens_claimed: *tokens_filled,
                                tx_hash: result.tx_hash,
                            };
                        }
                    }
                }
                true
            }
            Ok(_) => {
                error!("Unexpected result for batch claim");
                false
            }
            Err(reason) => {
                warn!("Batch claim failed: {}", reason);
                false
            }
        }
    }

    /// Claim a single bid
    async fn claim_single_bid(&mut self, bid_id: flux_core::BidId) {
        let intent = Intent::Claim {
            bid_ids: vec![bid_id],
        };

        // Find the bid index
        let bid_idx = self
            .tracker
            .bids
            .iter()
            .position(|b| matches!(b, BidState::Claiming { bid_id: id, .. } if *id == bid_id));

        let Some(idx) = bid_idx else {
            warn!("Bid {} not found in claiming state", bid_id.as_u256());
            return;
        };

        match self.execute_with_retry(intent, idx).await {
            Ok(IntentResult::TokensClaimed(result)) => {
                info!(
                    "Claimed {} tokens for bid {}, tx={}",
                    result.total_tokens.as_u256(),
                    bid_id.as_u256(),
                    result.tx_hash
                );

                if let BidState::Claiming { tokens_filled, .. } = &self.tracker.bids[idx] {
                    self.tracker.bids[idx] = BidState::Claimed {
                        bid_id,
                        tokens_claimed: *tokens_filled,
                        tx_hash: result.tx_hash,
                    };
                }
            }
            Ok(_) => {
                error!("Unexpected result for claim bid {}", bid_id.as_u256());
                self.tracker.bids[idx] = BidState::Failed {
                    reason: "Unexpected result type".to_string(),
                };
            }
            Err(reason) => {
                error!("Claim bid {} failed: {}", bid_id.as_u256(), reason);
                self.tracker.bids[idx] = BidState::Failed { reason };
            }
        }
    }

    /// Get fresh block number for each retry attempt
    async fn get_current_block(&self) -> Result<BlockNumber, String> {
        self.provider
            .get_block_number()
            .await
            .map(BlockNumber::new)
            .map_err(|e| format!("Failed to get current block: {:?}", e))
    }

    async fn execute_with_retry(
        &mut self,
        intent: Intent,
        bid_idx: usize,
    ) -> std::result::Result<IntentResult, String> {
        let mut attempts = 0;

        loop {
            // Update attempt count in state (if valid index)
            if bid_idx < self.tracker.bids.len() {
                match &mut self.tracker.bids[bid_idx] {
                    BidState::Submitting { attempts: a, .. } => *a = attempts,
                    BidState::Exiting { attempts: a, .. } => *a = attempts,
                    BidState::Claiming { attempts: a, .. } => *a = attempts,
                    _ => {}
                }
            }

            // Get fresh block number for each attempt
            let block = match self.get_current_block().await {
                Ok(b) => b,
                Err(e) => {
                    attempts += 1;
                    if attempts >= MAX_RETRIES {
                        return Err(e);
                    }
                    let backoff = Duration::from_millis(INITIAL_BACKOFF_MS * (1 << (attempts - 1)));
                    warn!("Failed to get block, retrying in {:?}...", backoff);
                    tokio::time::sleep(backoff).await;
                    continue;
                }
            };

            let outcome = self.executor.execute(intent.clone(), block).await;

            match outcome {
                IntentOutcome::Success(result) => return Ok(result),
                IntentOutcome::Failed { error, .. } => {
                    attempts += 1;

                    if attempts >= MAX_RETRIES || !Self::is_transient_error(&error) {
                        return Err(format!("{:?}", error));
                    }

                    let backoff = Duration::from_millis(INITIAL_BACKOFF_MS * (1 << (attempts - 1)));
                    warn!("Attempt {} failed, retrying in {:?}...", attempts, backoff);
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    fn is_transient_error(error: &flux_core::Error) -> bool {
        // Check if error is transient (network/RPC issues)
        matches!(
            error,
            flux_core::Error::State(flux_core::StateError::Transport(_))
                | flux_core::Error::Transaction(flux_core::TransactionError::Pending(_))
                | flux_core::Error::Config(flux_core::ConfigError::Transport(_))
        )
    }

    fn log_final_state(&self) {
        info!("Final bid states:");
        for (idx, bid) in self.tracker.bids.iter().enumerate() {
            match bid {
                BidState::Claimed {
                    bid_id,
                    tokens_claimed,
                    ..
                } => {
                    info!(
                        "  Bid {}: Claimed {} tokens (id={})",
                        idx,
                        tokens_claimed.as_u256(),
                        bid_id.as_u256()
                    );
                }
                BidState::Failed { reason } => {
                    info!("  Bid {}: Failed - {}", idx, reason);
                }
                other => {
                    info!("  Bid {}: {} (incomplete)", idx, other.status_str());
                }
            }
        }
    }
}
