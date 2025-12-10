use alloy::{
    contract,
    primitives::B256,
    providers::{MulticallError, PendingTransactionError},
    transports::TransportError,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Hook(#[from] HookError),

    #[error(transparent)]
    State(#[from] StateError),

    #[error(transparent)]
    Transaction(#[from] TransactionError),

    #[error(transparent)]
    BlockStream(#[from] BlockStreamError),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to fetch config: {0}")]
    Transport(#[from] TransportError),

    #[error("contract call failed: {0}")]
    Contract(#[from] contract::Error),

    #[error("multicall failed: {0}")]
    Multicall(#[from] MulticallError),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("auction not active")]
    AuctionNotActive,

    #[error("tokens not deposited")]
    TokensNotReceived,

    #[error("auction not started")]
    AuctionNotStarted,

    #[error("auction already over")]
    AuctionIsOver,

    #[error("auction not over yet")]
    AuctionNotOver,

    #[error("bid amount must be greater than zero")]
    AmountTooSmall,

    #[error("bid owner cannot be zero address")]
    OwnerIsZeroAddress,

    #[error("bid price is invalid for this auction")]
    InvalidPrice,

    #[error("bid price must be above current clearing price")]
    BidBelowClearingPrice,

    #[error("auction is sold out")]
    AuctionSoldOut,

    #[error("bid already exited")]
    BidAlreadyExited,

    #[error("cannot partially exit bid before graduation")]
    CannotPartiallyExitBeforeGraduation,

    #[error("bid is not ITM")]
    BidNotITM,

    #[error("bid is ITM, use exitBid instead")]
    BidIsITM,

    #[error("bid is not OTM")]
    BidNotOutbid,

    #[error("claim block not yet reached")]
    ClaimBlockNotReached,

    #[error("auction not graduated")]
    NotGraduated,

    #[error("bid not yet exited")]
    BidNotExited,

    #[error("bid has no tokens to claim")]
    NoTokensToClaim,

    #[error("bid owner does not match expected owner")]
    OwnerMismatch,

    #[error("auction not graduated, use exitBid for full refund")]
    UseExitBidForRefund,
}

#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook rejected bid: {reason}")]
    Rejected { reason: String },

    #[error("hook preparation failed: {0}")]
    PreparationFailed(String),

    #[error("hook validation failed: {0}")]
    ValidationFailed(String),
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("failed to fetch state: {0}")]
    Transport(#[from] TransportError),

    #[error("contract call failed: {0}")]
    Contract(#[from] contract::Error),

    #[error("multicall failed: {0}")]
    Multicall(#[from] MulticallError),

    #[error("bid not found")]
    BidNotFound,

    #[error("final checkpoint not cached when expected")]
    FinalCheckpointNotCached,
}

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("transaction failed: {0}")]
    Contract(#[from] contract::Error),

    #[error("pending transaction error: {0}")]
    Pending(#[from] PendingTransactionError),

    #[error("transaction receipt missing body")]
    MissingReceipt,

    #[error("BidSubmitted event not found in receipt logs")]
    MissingBidSubmittedEvent,

    #[error("BidExited event not found in receipt logs")]
    MissingBidExitedEvent,

    #[error("TokensClaimed event not found in receipt logs")]
    MissingTokensClaimedEvent,

    #[error("transaction reverted: {tx_hash:?}")]
    Reverted { tx_hash: B256 },
}

#[derive(Debug, Error)]
pub enum BlockStreamError {
    #[error("block stream error: {0}")]
    Transport(#[from] TransportError),
}
