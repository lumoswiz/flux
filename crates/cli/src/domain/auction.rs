// src/domain/auction.rs

use crate::abi::AuctionState;
use alloy::primitives::{Address, U256};

/// High-level view of an auction's state (for CLI/TUI).
#[derive(Debug, Clone)]
pub struct AuctionInfo {
    pub address: Address,

    // Global metrics
    pub clearing_price_q96: U256,
    pub currency_raised: U256,
    pub total_cleared: U256,
    pub is_graduated: bool,

    // Time bounds
    pub start_block: u64,
    pub end_block: u64,
    pub claim_block: u64,

    // Assets
    pub token: Address,
    pub currency: Address,
}

/// Coarse-grained lifecycle of the auction itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionPhase {
    BeforeStart,
    Running,
    Ended,
}

impl AuctionInfo {
    /// Construct from lens `AuctionState` + additional info looked up separately.
    pub fn from_lens_state(address: Address, state: AuctionState, extra: ExtraAuctionInfo) -> Self {
        Self {
            address,
            clearing_price_q96: state.checkpoint.clearingPrice,
            currency_raised: state.currencyRaised,
            total_cleared: state.totalCleared,
            is_graduated: state.isGraduated,
            start_block: extra.start_block,
            end_block: extra.end_block,
            claim_block: extra.claim_block,
            token: extra.token,
            currency: extra.currency,
        }
    }

    /// Determine which phase the auction is in given the current block.
    pub fn phase(&self, current_block: u64) -> AuctionPhase {
        if current_block < self.start_block {
            AuctionPhase::BeforeStart
        } else if current_block < self.end_block {
            AuctionPhase::Running
        } else {
            AuctionPhase::Ended
        }
    }
}

/// Extra info not provided by the lens contract (`AuctionStateLens`).
///
/// Your commands layer populates this by calling:
/// - startBlock() / endBlock() / claimBlock()
/// - token() / currency()
#[derive(Debug, Clone)]
pub struct ExtraAuctionInfo {
    pub start_block: u64,
    pub end_block: u64,
    pub claim_block: u64,
    pub token: Address,
    pub currency: Address,
}
