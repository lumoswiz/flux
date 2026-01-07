use alloy::primitives::B256;
use flux_core::{BidId, CurrencyAmount, Price, TokenAmount};

/// Tracks the lifecycle state of a single bid
#[derive(Debug, Clone)]
pub enum BidState {
    /// Bid has not been submitted yet
    Pending {
        max_price: Price,
        amount: CurrencyAmount,
    },

    /// Bid submission is in progress
    Submitting {
        max_price: Price,
        amount: CurrencyAmount,
        attempts: u32,
    },

    /// Bid has been submitted and is active in the auction
    Submitted {
        bid_id: BidId,
        max_price: Price,
        amount: CurrencyAmount,
        tx_hash: B256,
    },

    /// Bid exit is in progress
    Exiting { bid_id: BidId, attempts: u32 },

    /// Bid has been exited, waiting for claim
    Exited {
        bid_id: BidId,
        tokens_filled: TokenAmount,
        currency_refunded: CurrencyAmount,
        tx_hash: B256,
    },

    /// Token claim is in progress
    Claiming {
        bid_id: BidId,
        tokens_filled: TokenAmount,
        attempts: u32,
    },

    /// Tokens have been claimed - terminal state
    Claimed {
        bid_id: BidId,
        tokens_claimed: TokenAmount,
        tx_hash: B256,
    },

    /// Bid failed irrecoverably - terminal state
    Failed { reason: String },
}

impl BidState {
    /// Check if this bid is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, BidState::Claimed { .. } | BidState::Failed { .. })
    }

    /// Get the bid ID if available
    pub fn bid_id(&self) -> Option<BidId> {
        match self {
            BidState::Submitted { bid_id, .. }
            | BidState::Exiting { bid_id, .. }
            | BidState::Exited { bid_id, .. }
            | BidState::Claiming { bid_id, .. }
            | BidState::Claimed { bid_id, .. } => Some(*bid_id),
            _ => None,
        }
    }

    /// Get a human-readable status string
    pub fn status_str(&self) -> &'static str {
        match self {
            BidState::Pending { .. } => "Pending",
            BidState::Submitting { .. } => "Submitting",
            BidState::Submitted { .. } => "Submitted",
            BidState::Exiting { .. } => "Exiting",
            BidState::Exited { .. } => "Exited",
            BidState::Claiming { .. } => "Claiming",
            BidState::Claimed { .. } => "Claimed",
            BidState::Failed { .. } => "Failed",
        }
    }
}

/// Collection of tracked bids with their states
#[derive(Debug)]
pub struct BidTracker {
    pub bids: Vec<BidState>,
}

impl BidTracker {
    pub fn new() -> Self {
        Self { bids: Vec::new() }
    }

    pub fn add_pending(&mut self, max_price: Price, amount: CurrencyAmount) {
        self.bids.push(BidState::Pending { max_price, amount });
    }

    pub fn all_terminal(&self) -> bool {
        !self.bids.is_empty() && self.bids.iter().all(|b| b.is_terminal())
    }

    pub fn pending_bids(&self) -> impl Iterator<Item = (usize, &BidState)> {
        self.bids
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, BidState::Pending { .. }))
    }

    pub fn submitted_bids(&self) -> impl Iterator<Item = (usize, &BidState)> {
        self.bids
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, BidState::Submitted { .. }))
    }

    pub fn exited_bids(&self) -> impl Iterator<Item = (usize, &BidState)> {
        self.bids
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, BidState::Exited { .. }))
    }

    /// Get IDs of all exited bids that have tokens to claim
    pub fn claimable_bid_ids(&self) -> Vec<BidId> {
        self.bids
            .iter()
            .filter_map(|b| match b {
                BidState::Exited {
                    bid_id,
                    tokens_filled,
                    ..
                } if !tokens_filled.is_zero() => Some(*bid_id),
                _ => None,
            })
            .collect()
    }
}

impl Default for BidTracker {
    fn default() -> Self {
        Self::new()
    }
}
