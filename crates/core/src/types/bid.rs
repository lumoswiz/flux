use alloy::primitives::Address;

use super::primitives::{BidId, BlockNumber, CurrencyAmount, Mps, Price, TokenAmount};

pub enum BidStatus {
    ITM,
    ATM,
    OTM,
}

pub enum BidLifecycle {
    Active,
    Exited { block: BlockNumber },
    Claimed,
}

pub struct Bid {
    pub id: BidId,
    pub owner: Address,
    pub max_price: Price,
    pub amount: CurrencyAmount,
    pub start_block: BlockNumber,
    pub start_cumulative_mps: Mps,
    pub exited_block: Option<BlockNumber>,
    pub tokens_filled: TokenAmount,
}

impl Bid {
    pub fn status(&self, clearing_price: Price) -> BidStatus {
        if self.max_price > clearing_price {
            BidStatus::ITM
        } else if self.max_price == clearing_price {
            BidStatus::ATM
        } else {
            BidStatus::OTM
        }
    }

    pub fn lifecycle(&self) -> BidLifecycle {
        match self.exited_block {
            None => BidLifecycle::Active,
            Some(_) if self.tokens_filled.is_zero() => BidLifecycle::Claimed,
            Some(block) => BidLifecycle::Exited { block },
        }
    }

    pub fn needs_exit(&self) -> bool {
        self.exited_block.is_none()
    }

    pub fn needs_claim(&self) -> bool {
        self.exited_block.is_some() && !self.tokens_filled.is_zero()
    }
}
