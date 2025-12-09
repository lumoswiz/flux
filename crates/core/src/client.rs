use std::sync::Arc;

use alloy::providers::Provider;
use alloy::{
    consensus::TxReceipt,
    primitives::{Address, Bytes, U256},
};
use flux_abi::{IContinuousClearingAuction, IERC20Minimal};

use crate::{
    error::{ConfigError, Error, StateError, TransactionError, ValidationError},
    hooks::ValidationHook,
    types::{
        action::{
            ExitBidParams, ExitHints, ExitPartiallyFilledParams, ExitResult, SubmitBidInput,
            SubmitBidParams, SubmitBidResult,
        },
        bid::{Bid, TrackedBid},
        checkpoint::Checkpoint,
        config::AuctionConfig,
        primitives::{
            BidId, BlockNumber, CurrencyAddr, CurrencyAmount, HookAddr, Mps, Price, TickSpacing,
            TokenAddr, TokenAmount,
        },
        state::{AuctionPhase, AuctionState, GraduationStatus, TokenDepositStatus},
    },
};

pub struct AuctionClient<P>
where
    P: Provider + Clone,
{
    provider: P,
    auction: Address,
    owner: Address,
    hook: Arc<dyn ValidationHook>,
    tracked_bids: Vec<TrackedBid>,
    config: AuctionConfig,
}

impl<P> AuctionClient<P>
where
    P: Provider + Clone,
{
    pub async fn new(
        provider: P,
        auction: Address,
        owner: Address,
        hook: impl Into<Arc<dyn ValidationHook>>,
        tracked_bids: Vec<TrackedBid>,
    ) -> Result<Self, Error> {
        let config = Self::fetch_config(&provider, auction).await?;
        Ok(Self {
            provider,
            auction,
            owner,
            hook: hook.into(),
            tracked_bids,
            config,
        })
    }

    pub fn config(&self) -> &AuctionConfig {
        &self.config
    }

    pub fn address(&self) -> Address {
        self.auction
    }

    pub async fn fetch_state(&self) -> Result<AuctionState, Error> {
        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);
        let token = IERC20Minimal::new(self.config.token.as_address(), &self.provider);

        let (
            checkpoint_raw,
            is_graduated,
            currency_raised_raw,
            last_checkpoint_block,
            token_balance,
        ) = self
            .provider
            .multicall()
            .add(cca.latestCheckpoint())
            .add(cca.isGraduated())
            .add(cca.currencyRaised())
            .add(cca.lastCheckpointedBlock())
            .add(token.balanceOf(self.auction))
            .aggregate()
            .await
            .map_err(StateError::from)?;

        let current_block = BlockNumber::new(
            self.provider
                .get_block_number()
                .await
                .map_err(StateError::from)?,
        );

        let checkpoint = Checkpoint {
            block: BlockNumber::new(last_checkpoint_block),
            clearing_price: Price::new(checkpoint_raw.clearingPrice),
            cumulative_mps: Mps::new(checkpoint_raw.cumulativeMps),
            prev_block: BlockNumber::new(checkpoint_raw.prev),
            next_block: BlockNumber::new(checkpoint_raw.next),
        };

        let tokens_received = if U256::from(token_balance) >= self.config.total_supply.as_u256() {
            TokenDepositStatus::Received
        } else {
            TokenDepositStatus::NotReceived
        };

        let phase = AuctionState::compute_phase(&self.config, current_block, tokens_received);

        let graduation = if is_graduated {
            GraduationStatus::Graduated
        } else {
            GraduationStatus::NotGraduated
        };

        let currency_raised = CurrencyAmount::new(currency_raised_raw);

        Ok(AuctionState {
            current_block,
            phase,
            checkpoint,
            graduation,
            tokens_received,
            currency_raised,
        })
    }

    pub async fn fetch_bids(&self, bid_ids: &[BidId]) -> Result<Vec<Bid>, Error> {
        // Might we want to throw here?
        if bid_ids.is_empty() {
            return Ok(Vec::new());
        }

        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);

        if bid_ids.len() == 1 {
            let bid_id = bid_ids[0];
            let bid_return = cca
                .bids(bid_id.as_u256())
                .call()
                .await
                .map_err(StateError::from)?;
            return Ok(vec![Self::decode_bid(bid_id, bid_return)]);
        }

        let mut multicall = self.provider.multicall().dynamic();

        for bid_id in bid_ids {
            multicall = multicall.add_dynamic(cca.bids(bid_id.as_u256()));
        }

        let bid_returns = multicall.aggregate().await.map_err(StateError::from)?;

        let bids = bid_ids
            .iter()
            .zip(bid_returns.into_iter())
            .map(|(bid_id, bid_return)| Self::decode_bid(*bid_id, bid_return))
            .collect();

        Ok(bids)
    }

    fn decode_bid(bid_id: BidId, bid_return: IContinuousClearingAuction::Bid) -> Bid {
        let exited_block = if bid_return.exitedBlock == 0 {
            None
        } else {
            Some(BlockNumber::new(bid_return.exitedBlock))
        };

        Bid {
            id: bid_id,
            owner: bid_return.owner,
            max_price: Price::new(bid_return.maxPrice),
            amount: CurrencyAmount::new(bid_return.amountQ96),
            start_block: BlockNumber::new(bid_return.startBlock),
            start_cumulative_mps: Mps::new(bid_return.startCumulativeMps),
            exited_block,
            tokens_filled: TokenAmount::new(bid_return.tokensFilled),
        }
    }

    pub async fn fetch_config(provider: &P, auction: Address) -> Result<AuctionConfig, Error> {
        let cca = IContinuousClearingAuction::new(auction, provider);

        let (
            start_block,
            end_block,
            claim_block,
            total_supply,
            tick_spacing,
            floor_price,
            max_bid_price,
            currency,
            token,
            validation_hook,
        ) = provider
            .multicall()
            .add(cca.startBlock())
            .add(cca.endBlock())
            .add(cca.claimBlock())
            .add(cca.totalSupply())
            .add(cca.tickSpacing())
            .add(cca.floorPrice())
            .add(cca.MAX_BID_PRICE())
            .add(cca.currency())
            .add(cca.token())
            .add(cca.validationHook())
            .aggregate()
            .await
            .map_err(ConfigError::from)?;

        Ok(AuctionConfig {
            address: auction,
            start_block: BlockNumber::new(start_block),
            end_block: BlockNumber::new(end_block),
            claim_block: BlockNumber::new(claim_block),
            total_supply: TokenAmount::new(U256::from(total_supply)),
            tick_spacing: TickSpacing::new(tick_spacing),
            floor_price: Price::new(floor_price),
            max_bid_price: Price::new(max_bid_price),
            currency: CurrencyAddr::new(currency),
            token: TokenAddr::new(token),
            validation_hook: HookAddr::new(validation_hook),
        })
    }

    pub async fn validate_bid_params(
        &self,
        input: &SubmitBidInput,
        state: &AuctionState,
    ) -> Result<(), Error> {
        let current_block = state.current_block.as_u64();
        let start_block = self.config.start_block.as_u64();
        let end_block = self.config.end_block.as_u64();

        if current_block < start_block {
            return Err(ValidationError::AuctionNotStarted.into());
        }

        if current_block >= end_block {
            return Err(ValidationError::AuctionIsOver.into());
        }

        if !matches!(state.phase, AuctionPhase::Active { .. }) {
            return Err(ValidationError::AuctionNotActive.into());
        }

        if !matches!(state.tokens_received, TokenDepositStatus::Received) {
            return Err(ValidationError::TokensNotReceived.into());
        }

        if input.amount.is_zero() {
            return Err(ValidationError::AmountTooSmall.into());
        }

        if input.owner == Address::ZERO {
            return Err(ValidationError::OwnerIsZeroAddress.into());
        }

        if !self.config.is_valid_price(input.max_price) {
            return Err(ValidationError::InvalidPrice.into());
        }

        if state.checkpoint.is_sold_out() {
            return Err(ValidationError::AuctionSoldOut.into());
        }

        if input.max_price <= state.checkpoint.clearing_price {
            return Err(ValidationError::BidBelowClearingPrice.into());
        }

        Ok(())
    }

    pub async fn prepare_bid(&self, input: SubmitBidInput) -> Result<SubmitBidParams, Error> {
        let state = self.fetch_state().await?;
        self.validate_bid_params(&input, &state).await?;

        let prev_tick_price = self.compute_prev_tick_price(input.max_price).await?;
        let amount = input.amount;

        let mut params = SubmitBidParams {
            max_price: input.max_price,
            amount,
            owner: input.owner,
            prev_tick_price,
            hook_data: Bytes::new(),
            value: CurrencyAmount::new(U256::ZERO),
        };

        if self.config.is_native_currency() {
            params.value = amount;
        }

        let hook_data = self.hook.prepare_hook_data(&params, &state).await?;
        params.hook_data = hook_data;

        self.hook.validate(&params, &state).await?;

        Ok(params)
    }

    pub async fn submit_bid(&mut self, params: SubmitBidParams) -> Result<SubmitBidResult, Error> {
        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);

        let call = cca
            .submitBid_1(
                params.max_price.as_u256(),
                params.amount.as_u128(),
                params.owner,
                params.prev_tick_price.as_u256(),
                params.hook_data,
            )
            .value(params.value.as_u256());

        let pending = call.send().await.map_err(TransactionError::from)?;
        let receipt = pending
            .with_required_confirmations(3)
            .get_receipt()
            .await
            .map_err(TransactionError::from)?;

        let receipt_body = receipt
            .inner
            .as_receipt()
            .ok_or(TransactionError::MissingReceipt)?;

        if !receipt_body.status() {
            return Err(TransactionError::Reverted {
                tx_hash: receipt.transaction_hash,
            }
            .into());
        }

        let bid_id = receipt_body
            .logs()
            .iter()
            .find_map(|log| {
                log.log_decode::<IContinuousClearingAuction::BidSubmitted>()
                    .ok()
            })
            .map(|decoded| BidId::new(decoded.inner.data.id))
            .ok_or(TransactionError::MissingBidSubmittedEvent)?;

        self.tracked_bids.push(TrackedBid {
            id: bid_id,
            tx_hash: receipt.transaction_hash,
        });

        Ok(SubmitBidResult {
            bid_id,
            tx_hash: receipt.transaction_hash,
        })
    }

    pub async fn exit_bid(&mut self, params: ExitBidParams) -> Result<ExitResult, Error> {
        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);

        let pending = cca
            .exitBid(params.bid_id.as_u256())
            .send()
            .await
            .map_err(TransactionError::from)?;

        let receipt = pending
            .with_required_confirmations(3)
            .get_receipt()
            .await
            .map_err(TransactionError::from)?;

        let receipt_body = receipt
            .inner
            .as_receipt()
            .ok_or(TransactionError::MissingReceipt)?;

        if !receipt_body.status() {
            return Err(TransactionError::Reverted {
                tx_hash: receipt.transaction_hash,
            }
            .into());
        }

        let exit_event = receipt_body
            .logs()
            .iter()
            .find_map(|log| {
                log.log_decode::<IContinuousClearingAuction::BidExited>()
                    .ok()
            })
            .ok_or(TransactionError::MissingBidExitedEvent)?;

        let data = exit_event.inner.data;
        let tokens_filled = TokenAmount::new(data.tokensFilled);
        let currency_refunded = CurrencyAmount::new(data.currencyRefunded);

        Ok(ExitResult {
            bid_id: params.bid_id,
            tokens_filled,
            currency_refunded,
            tx_hash: receipt.transaction_hash,
        })
    }

    pub async fn exit_partially_filled(
        &mut self,
        params: ExitPartiallyFilledParams,
    ) -> Result<ExitResult, Error> {
        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);

        let outbid_block = params.outbid_block.map_or(0u64, |block| block.as_u64());

        let pending = cca
            .exitPartiallyFilledBid(
                params.bid_id.as_u256(),
                params.last_fully_filled_checkpoint_block.as_u64(),
                outbid_block,
            )
            .send()
            .await
            .map_err(TransactionError::from)?;

        let receipt = pending
            .with_required_confirmations(3)
            .get_receipt()
            .await
            .map_err(TransactionError::from)?;

        let receipt_body = receipt
            .inner
            .as_receipt()
            .ok_or(TransactionError::MissingReceipt)?;

        if !receipt_body.status() {
            return Err(TransactionError::Reverted {
                tx_hash: receipt.transaction_hash,
            }
            .into());
        }

        let exit_event = receipt_body
            .logs()
            .iter()
            .find_map(|log| {
                log.log_decode::<IContinuousClearingAuction::BidExited>()
                    .ok()
            })
            .ok_or(TransactionError::MissingBidExitedEvent)?;

        let data = exit_event.inner.data;
        let tokens_filled = TokenAmount::new(data.tokensFilled);
        let currency_refunded = CurrencyAmount::new(data.currencyRefunded);

        Ok(ExitResult {
            bid_id: params.bid_id,
            tokens_filled,
            currency_refunded,
            tx_hash: receipt.transaction_hash,
        })
    }

    // orchestration layer should validate: is_ended, is_graduated, bids.exited_block.is_none()
    pub async fn prepare_exit_partially_filled(
        &self,
        bid_id: BidId,
    ) -> Result<ExitPartiallyFilledParams, Error> {
        let bids = self.fetch_bids(&[bid_id]).await?;
        let bid = bids.first().ok_or(StateError::BidNotFound)?;

        let hints = self.compute_exit_hints(bid).await?;

        Ok(ExitPartiallyFilledParams {
            bid_id,
            last_fully_filled_checkpoint_block: hints.last_fully_filled_checkpoint_block,
            outbid_block: hints.outbid_block,
        })
    }

    pub async fn compute_prev_tick_price(&self, max_price: Price) -> Result<Price, Error> {
        if !self.config.is_valid_price(max_price) {
            return Err(ValidationError::InvalidPrice.into());
        }

        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);
        let mut prev = self.config.floor_price;

        let next_active = Price::new(
            cca.nextActiveTickPrice()
                .call()
                .await
                .map_err(StateError::from)?,
        );
        if next_active < max_price && next_active >= prev {
            prev = next_active;
        }

        loop {
            let tick_return = cca
                .ticks(prev.as_u256())
                .call()
                .await
                .map_err(StateError::from)?;
            let next_price = Price::new(tick_return.next);

            if next_price >= max_price {
                break Ok(prev);
            }

            if next_price == prev {
                break Ok(prev);
            }

            prev = next_price;
        }
    }

    pub async fn compute_exit_hints(&self, bid: &Bid) -> Result<ExitHints, Error> {
        let cca = IContinuousClearingAuction::new(self.auction, &self.provider);
        let tail = cca
            .MAX_BLOCK_NUMBER()
            .call()
            .await
            .map_err(StateError::from)?;

        let mut last_fully_filled = bid.start_block;
        let mut current_cp = cca
            .checkpoints(bid.start_block.as_u64())
            .call()
            .await
            .map_err(StateError::from)?;

        while current_cp.next != tail {
            let next_block = BlockNumber::new(current_cp.next);
            let next_cp = cca
                .checkpoints(next_block.as_u64())
                .call()
                .await
                .map_err(StateError::from)?;

            if next_cp.clearingPrice >= bid.max_price.as_u256() {
                break;
            }

            last_fully_filled = next_block;
            current_cp = next_cp;
        }

        let mut outbid_block = None;

        while current_cp.next != tail {
            let next_block = BlockNumber::new(current_cp.next);
            let next_cp = cca
                .checkpoints(next_block.as_u64())
                .call()
                .await
                .map_err(StateError::from)?;

            if next_cp.clearingPrice > bid.max_price.as_u256() {
                outbid_block = Some(next_block);
                break;
            }

            current_cp = next_cp;
        }

        Ok(ExitHints {
            last_fully_filled_checkpoint_block: last_fully_filled,
            outbid_block,
        })
    }
}
