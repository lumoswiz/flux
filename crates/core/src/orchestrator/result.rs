use crate::types::{
    action::{ClaimResult, ExitResult, SubmitBidResult},
    primitives::TokenAmount,
};

pub enum BlockResult {
    Continue,
    Finished(OrchestratorResult),
}

pub struct OrchestratorResult {
    pub bids_submitted: u32,
    pub bids_exited: u32,
    pub tokens_claimed: TokenAmount,
    pub reason: CompletionReason,
}

pub enum CompletionReason {
    AllBidsProcessed,
    AuctionEndedWithPending,
    BlockStreamEnded,
    Error(String),
}

pub enum IntentResult {
    BidSubmitted(SubmitBidResult),
    BidExited(ExitResult),
    TokensClaimed(ClaimResult),
    Skipped,
}
