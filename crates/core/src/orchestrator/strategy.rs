use crate::types::{
    action::SubmitBidInput,
    config::AuctionConfig,
    primitives::{BidId, BlockNumber},
    state::AuctionPhase,
};

use super::OrchestratorCache;

pub struct EvaluationContext<'a> {
    pub block: BlockNumber,
    pub phase: AuctionPhase,
    pub cache: &'a OrchestratorCache,
    pub tracked_bids: Vec<BidId>,
    pub config: &'a AuctionConfig,
}

pub trait Strategy: Send + Sync {
    fn evaluate(&self, ctx: &EvaluationContext) -> Vec<Intent>;
}

#[derive(Clone, Debug)]
pub enum Intent {
    SubmitBid(SubmitBidInput),
    Exit { bid_id: BidId },
    Claim(Vec<BidId>),
    Skip,
}
