use crate::types::{
    config::AuctionConfig,
    primitives::{BidId, BlockNumber},
    state::AuctionPhase,
};

use super::ExecutorCache;

pub struct EvaluationContext<'a> {
    pub block: BlockNumber,
    pub phase: AuctionPhase,
    pub cache: &'a ExecutorCache,
    pub tracked_bids: Vec<BidId>,
    pub config: &'a AuctionConfig,
}
