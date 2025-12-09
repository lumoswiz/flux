use crate::domain::auction::{AuctionInfo, AuctionPhase};
use alloy::primitives::{Address, U256};
use flux_abi::IContinuousClearingAuction::Bid;

/// Canonical representation of a user's bid in an auction,
/// derived from the on-chain `Bid` struct.
#[derive(Debug, Clone)]
pub struct BidInfo {
    pub auction: Address,
    pub bid_id: U256,

    pub owner: Address,
    pub max_price_q96: U256,
    pub amount_q96: U256,
    pub tokens_filled: U256,

    pub start_block: u64,
    pub start_cumulative_mps: u32,
    pub exited_block: u64,
}

/// High-level lifecycle status of a bid, used by CLI/TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidStatus {
    /// Auction not started yet.
    NotStarted,
    /// Auction running, bid is active (maxPrice >= current clearing price).
    ActiveInTheMoney,
    /// Auction running, bid has been outbid (maxPrice < clearing price).
    ActiveOutbid,
    /// Auction ended but not graduated yet (waiting to see if it passes).
    AwaitingGraduation,
    /// Auction ended & graduated, bid holds no tokens.
    FinishedUnfilled,
    /// Auction ended & graduated, bid has tokens and still needs exit.
    FinishedFilledNeedsExit,
    /// Bid already exited (currency + filled tokens accounted for).
    Exited,
    /// Bid exited and auction graduated; tokens can be claimed (after claimBlock).
    Claimable,
}

impl BidInfo {
    /// Infer a coarse-grained status from current chain state + auction info.
    ///
    /// NOTE: This is intentionally conservative; more precise distinctions
    ///       (like exactly partially filled vs fully filled at maxPrice)
    ///       can be added later using checkpoints.
    pub fn derive_status(&self, current_block: u64, auction: &AuctionInfo) -> BidStatus {
        // If exited, either Exited or Claimable
        if self.exited_block > 0 {
            if current_block >= auction.claim_block && auction.is_graduated {
                return BidStatus::Claimable;
            } else {
                return BidStatus::Exited;
            }
        }

        let phase = auction.phase(current_block);

        match phase {
            AuctionPhase::BeforeStart => BidStatus::NotStarted,
            AuctionPhase::Running => {
                if self.max_price_q96 >= auction.clearing_price_q96 {
                    BidStatus::ActiveInTheMoney
                } else {
                    BidStatus::ActiveOutbid
                }
            }
            AuctionPhase::Ended => {
                // Auction ended, but maybe not yet graduated
                if !auction.is_graduated {
                    BidStatus::AwaitingGraduation
                } else {
                    // Auction graduated
                    if self.tokens_filled.is_zero() {
                        BidStatus::FinishedUnfilled
                    } else {
                        BidStatus::FinishedFilledNeedsExit
                    }
                }
            }
        }
    }

    /// Convenience: fraction of the normalized amount that has been filled.
    ///
    /// This is not exact “tokens / intended tokens”, but a simple
    /// `tokensFilled / totalCleared` style ratio can be added if needed.
    pub fn filled_any(&self) -> bool {
        !self.tokens_filled.is_zero()
    }

    /// Returns true if the bid has been fully processed (exited + tokens claimable or not relevant).
    pub fn is_terminal(&self, current_block: u64, auction: &AuctionInfo) -> bool {
        match self.derive_status(current_block, auction) {
            BidStatus::Exited | BidStatus::Claimable | BidStatus::FinishedUnfilled => true,
            _ => false,
        }
    }
}

/// Map from ABI-level `Bid` struct to our domain `BidInfo`.
///
/// The commands layer will typically:
///   let b = auction.bids(bid_id).call().await?;
///   let info: BidInfo = (auction_addr, bid_id, b).into();
impl From<(Address, U256, Bid)> for BidInfo {
    fn from((auction_addr, bid_id, b): (Address, U256, Bid)) -> Self {
        Self {
            auction: auction_addr,
            bid_id,
            owner: b.owner,
            max_price_q96: b.maxPrice,
            amount_q96: b.amountQ96,
            tokens_filled: b.tokensFilled,
            start_block: b.startBlock,
            start_cumulative_mps: b.startCumulativeMps.to::<u32>(),
            exited_block: b.exitedBlock,
        }
    }
}
