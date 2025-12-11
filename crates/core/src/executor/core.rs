use alloy::providers::Provider;

use crate::{
    client::AuctionClient,
    error::{Error, StateError},
    types::{
        action::{ClaimParams, ExitBidParams, SubmitBidInput},
        bid::BidStatus,
        primitives::{BidId, BlockNumber, CurrencyAmount, Price},
        state::AuctionState,
    },
    validation,
};

use super::{EvaluationContext, ExecutorCache, Intent, IntentOutcome, IntentResult};

pub struct IntentExecutor<P>
where
    P: Provider + Clone,
{
    client: AuctionClient<P>,
    cache: ExecutorCache,
}

impl<P> IntentExecutor<P>
where
    P: Provider + Clone,
{
    pub fn new(client: AuctionClient<P>) -> Self {
        Self {
            client,
            cache: ExecutorCache::new(),
        }
    }

    pub async fn execute(&mut self, intent: Intent, block: BlockNumber) -> IntentOutcome {
        match self.execute_inner(intent.clone(), block).await {
            Ok(result) => IntentOutcome::Success(result),
            Err(error) => IntentOutcome::Failed { intent, error },
        }
    }

    pub fn context(&self, block: BlockNumber) -> EvaluationContext<'_> {
        let phase =
            AuctionState::compute_phase(self.client.config(), block, self.cache.tokens_received);

        let tracked_bids: Vec<BidId> = self
            .client
            .tracked_bids()
            .map(|tracked| tracked.id)
            .collect();

        EvaluationContext {
            block,
            phase,
            cache: &self.cache,
            tracked_bids,
            config: self.client.config(),
        }
    }

    pub fn client(&self) -> &AuctionClient<P> {
        &self.client
    }

    pub fn client_mut(&mut self) -> &mut AuctionClient<P> {
        &mut self.client
    }

    pub fn cache(&self) -> &ExecutorCache {
        &self.cache
    }

    async fn execute_inner(
        &mut self,
        intent: Intent,
        block: BlockNumber,
    ) -> Result<IntentResult, Error> {
        match intent {
            Intent::SubmitBid { max_price, amount } => {
                self.execute_submit_bid(max_price, amount, block).await
            }
            Intent::Exit { bid_id } => self.execute_exit(bid_id, block).await,
            Intent::Claim { bid_ids } => self.execute_claim(bid_ids, block).await,
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

    fn is_past_end(&self, block: BlockNumber) -> bool {
        block >= self.client.config().end_block
    }
}
