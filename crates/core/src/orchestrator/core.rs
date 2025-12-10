use futures::StreamExt;

use crate::{
    blocks::BlockStream,
    client::AuctionClient,
    error::{Error, StateError},
    orchestrator::{
        BlockResult, CompletionReason, EvaluationContext, Intent, OrchestratorCache,
        OrchestratorResult, Strategy, result::IntentResult,
    },
    types::{
        action::{ClaimParams, ExitBidParams, SubmitBidInput},
        bid::BidStatus,
        primitives::{BidId, BlockNumber, CurrencyAmount, Price, TokenAmount},
        state::{AuctionPhase, AuctionState, GraduationStatus},
    },
    validation,
};

pub struct Orchestrator<P, S>
where
    P: alloy::providers::Provider + Clone + Send + Sync + 'static,
    S: Strategy,
{
    client: AuctionClient<P>,
    strategy: S,
    cache: OrchestratorCache,
    bids_submitted: u32,
    bids_exited: u32,
    tokens_claimed: TokenAmount,
}

impl<P, S> Orchestrator<P, S>
where
    P: alloy::providers::Provider + Clone + Send + Sync + 'static,
    S: Strategy,
{
    pub fn new(client: AuctionClient<P>, strategy: S) -> Self {
        Self {
            client,
            strategy,
            cache: OrchestratorCache::new(),
            bids_submitted: 0,
            bids_exited: 0,
            tokens_claimed: TokenAmount::ZERO,
        }
    }

    pub async fn run<B>(&mut self, mut blocks: B) -> Result<OrchestratorResult, Error>
    where
        B: BlockStream,
    {
        while let Some(block) = blocks.next().await {
            let block = block?;
            match self.handle_block(block).await? {
                BlockResult::Continue => continue,
                BlockResult::Finished(result) => return Ok(result),
            }
        }

        Ok(self.finalize(CompletionReason::BlockStreamEnded))
    }

    pub async fn handle_block(&mut self, block: BlockNumber) -> Result<BlockResult, Error> {
        let phase =
            AuctionState::compute_phase(self.client.config(), block, self.cache.tokens_received);

        if self.is_complete(&phase) {
            return Ok(BlockResult::Finished(
                self.finalize(CompletionReason::AllBidsProcessed),
            ));
        }

        let tracked_ids: Vec<BidId> = self
            .client
            .tracked_bids()
            .map(|tracked| tracked.id)
            .collect();

        let ctx = EvaluationContext {
            block,
            phase,
            cache: &self.cache,
            tracked_bids: tracked_ids,
            config: self.client.config(),
        };

        let intents: Vec<Intent> = self.strategy.evaluate(&ctx);

        if intents.is_empty() || intents.iter().all(|i| matches!(i, Intent::Skip)) {
            return Ok(BlockResult::Continue);
        }

        for intent in intents {
            let result = self.resolve_and_execute(intent, block).await?;
            self.record_result(&result);
        }

        Ok(BlockResult::Continue)
    }

    fn finalize(&self, reason: CompletionReason) -> OrchestratorResult {
        OrchestratorResult {
            bids_submitted: self.bids_submitted,
            bids_exited: self.bids_exited,
            tokens_claimed: self.tokens_claimed,
            reason,
        }
    }

    async fn resolve_and_execute(
        &mut self,
        intent: Intent,
        block: BlockNumber,
    ) -> Result<IntentResult, Error> {
        match intent {
            Intent::SubmitBid { max_price, amount } => {
                self.execute_submit_bid(max_price, amount, block).await
            }
            Intent::Exit { bid_id } => self.execute_exit(bid_id, block).await,
            Intent::Claim(bid_ids) => self.execute_claim(bid_ids, block).await,
            Intent::Skip => Ok(IntentResult::Skipped),
        }
    }

    async fn execute_submit_bid(
        &mut self,
        max_price: Price,
        amount: CurrencyAmount,
        block: BlockNumber,
    ) -> Result<IntentResult, Error> {
        let checkpoint = self.client.fetch_checkpoint().await?;

        let tokens_received = if self.cache.needs_token_balance() {
            self.client.fetch_token_balance().await?
        } else {
            self.cache.tokens_received
        };

        let past_end_block = self.is_past_end(block);
        self.cache.update(
            Some(tokens_received),
            None,
            Some(checkpoint),
            past_end_block,
        );

        let state = AuctionState::new(
            block,
            checkpoint,
            self.cache.graduated,
            tokens_received,
            self.client.config(),
        );

        let input = SubmitBidInput {
            max_price,
            amount,
            owner: self.client.owner(),
        };

        validation::validate_submit_bid(&input, &state, self.client.config())?;

        let params = self.client.prepare_bid(input, &state).await?;
        self.client.hook().validate(&params, &state).await?;
        let result = self.client.submit_bid(params).await?;

        Ok(IntentResult::BidSubmitted(result))
    }

    async fn execute_exit(
        &mut self,
        bid_id: BidId,
        block: BlockNumber,
    ) -> Result<IntentResult, Error> {
        let past_end_block = self.is_past_end(block);

        let checkpoint = if self.cache.needs_checkpoint(past_end_block) {
            let cp = self.client.fetch_checkpoint().await?;
            self.cache.update(None, None, Some(cp), past_end_block);
            cp
        } else {
            self.cache
                .final_checkpoint
                .ok_or(StateError::FinalCheckpointNotCached)?
        };

        let graduation = if self.cache.needs_graduation() {
            let g = self.client.fetch_graduation().await?;
            self.cache.update(None, Some(g), None, past_end_block);
            g
        } else {
            self.cache.graduated
        };

        let bids = self.client.fetch_bids(&[bid_id]).await?;
        let bid = bids.first().ok_or(StateError::BidNotFound)?;

        let state = AuctionState::new(
            block,
            checkpoint,
            graduation,
            self.cache.tokens_received,
            self.client.config(),
        );

        let status = bid.status(checkpoint.clearing_price);

        let exit_result = match status {
            BidStatus::ITM => {
                validation::validate_exit_bid(bid, &state, self.client.config())?;
                let params = ExitBidParams { bid_id };
                self.client.exit_bid(params).await?
            }
            BidStatus::ATM | BidStatus::OTM => {
                validation::validate_exit_partially_filled(bid, &state, self.client.config())?;
                let params = self.client.prepare_exit_partially_filled(bid_id).await?;
                self.client.exit_partially_filled(params).await?
            }
        };

        Ok(IntentResult::BidExited(exit_result))
    }

    async fn execute_claim(
        &mut self,
        bid_ids: Vec<BidId>,
        block: BlockNumber,
    ) -> Result<IntentResult, Error> {
        let past_end_block = self.is_past_end(block);

        let graduation = if self.cache.needs_graduation() {
            let g = self.client.fetch_graduation().await?;
            self.cache.update(None, Some(g), None, past_end_block);
            g
        } else {
            self.cache.graduated
        };

        let bids = self.client.fetch_bids(&bid_ids).await?;

        let checkpoint = self
            .cache
            .final_checkpoint
            .ok_or(StateError::FinalCheckpointNotCached)?;

        let state = AuctionState::new(
            block,
            checkpoint,
            graduation,
            self.cache.tokens_received,
            self.client.config(),
        );

        validation::validate_claim(&bids, self.client.owner(), &state, self.client.config())?;

        let params = ClaimParams {
            owner: self.client.owner(),
            bid_ids,
        };
        let result = self.client.claim(params).await?;

        Ok(IntentResult::TokensClaimed(result))
    }

    fn record_result(&mut self, result: &IntentResult) {
        match result {
            IntentResult::BidSubmitted(_) => {
                self.bids_submitted += 1;
            }
            IntentResult::BidExited(_) => {
                self.bids_exited += 1;
            }
            IntentResult::TokensClaimed(res) => {
                self.tokens_claimed += res.total_tokens;
            }
            IntentResult::Skipped => {}
        }
    }

    fn is_past_end(&self, block: BlockNumber) -> bool {
        block >= self.client.config().end_block
    }

    fn is_complete(&self, phase: &AuctionPhase) -> bool {
        let no_tracked_bids = self.client.tracked_bids().next().is_none();

        match phase {
            AuctionPhase::Claimable => no_tracked_bids,
            AuctionPhase::Ended { .. } => {
                no_tracked_bids && matches!(self.cache.graduated, GraduationStatus::NotGraduated)
            }
            _ => false,
        }
    }
}
