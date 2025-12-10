use futures::StreamExt;

use crate::{
    blocks::BlockStream,
    client::AuctionClient,
    error::Error,
    orchestrator::{
        BlockResult, CompletionReason, EvaluationContext, Intent, OrchestratorCache,
        OrchestratorResult, Strategy,
    },
    types::{
        primitives::{BidId, BlockNumber, TokenAmount},
        state::AuctionState,
    },
};

pub struct Orchestrator<P, S>
where
    P: alloy::providers::Provider + Clone + Send + Sync + 'static,
    S: Strategy,
{
    client: AuctionClient<P>,
    strategy: S,
    cache: OrchestratorCache,
    cached_state: Option<AuctionState>,
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
            cached_state: None,
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
        let state = self.refresh_state().await?;

        let tracked_ids: Vec<BidId> = self
            .client
            .tracked_bids()
            .map(|tracked| tracked.id)
            .collect();

        let ctx = EvaluationContext {
            block,
            phase: state.phase.clone(),
            cache: &self.cache,
            tracked_bids: tracked_ids,
            config: self.client.config(),
        };

        let _intents: Vec<Intent> = self.strategy.evaluate(&ctx);

        Ok(BlockResult::Continue)
    }

    pub async fn refresh_state(&mut self) -> Result<AuctionState, Error> {
        let state = self.client.fetch_state().await?;
        self.cached_state = Some(state.clone());
        Ok(state)
    }

    fn finalize(&self, reason: CompletionReason) -> OrchestratorResult {
        OrchestratorResult {
            bids_submitted: self.bids_submitted,
            bids_exited: self.bids_exited,
            tokens_claimed: self.tokens_claimed,
            reason,
        }
    }
}
