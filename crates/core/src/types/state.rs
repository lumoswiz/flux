use super::{
    checkpoint::Checkpoint,
    config::AuctionConfig,
    primitives::{BlockNumber, CurrencyAmount},
};

pub enum GraduationStatus {
    NotGraduated,
    Graduated,
}

pub enum TokenDepositStatus {
    Unknown,
    NotReceived,
    Received,
}

pub enum AuctionPhase {
    PreStart { blocks_until_start: u64 },
    PreTokens,
    Active { blocks_remaining: u64 },
    Ended { blocks_until_claim: u64 },
    Claimable,
}

pub struct AuctionState {
    pub current_block: BlockNumber,
    pub phase: AuctionPhase,
    pub checkpoint: Checkpoint,
    pub graduation: GraduationStatus,
    pub tokens_received: TokenDepositStatus,
    pub currency_raised: CurrencyAmount,
}

impl AuctionState {
    pub fn compute_phase(
        config: &AuctionConfig,
        current_block: BlockNumber,
        tokens_received: TokenDepositStatus,
    ) -> AuctionPhase {
        let current = current_block.as_u64();
        let start = config.start_block.as_u64();
        let end = config.end_block.as_u64();
        let claim = config.claim_block.as_u64();
        let tokens_ready = match tokens_received {
            TokenDepositStatus::Received => true,
            TokenDepositStatus::Unknown | TokenDepositStatus::NotReceived => false,
        };

        if current < start {
            AuctionPhase::PreStart {
                blocks_until_start: start - current,
            }
        } else if !tokens_ready {
            AuctionPhase::PreTokens
        } else if current < end {
            AuctionPhase::Active {
                blocks_remaining: end - current,
            }
        } else if current < claim {
            AuctionPhase::Ended {
                blocks_until_claim: claim - current,
            }
        } else {
            AuctionPhase::Claimable
        }
    }

    pub fn can_submit_bid(&self) -> bool {
        let active = match self.phase {
            AuctionPhase::Active { .. } => true,
            _ => false,
        };
        active && !self.checkpoint.is_sold_out()
    }

    pub fn can_early_exit(&self) -> bool {
        let graduated = match self.graduation {
            GraduationStatus::Graduated => true,
            GraduationStatus::NotGraduated => false,
        };
        let active = match self.phase {
            AuctionPhase::Active { .. } => true,
            _ => false,
        };
        graduated && active
    }
}
