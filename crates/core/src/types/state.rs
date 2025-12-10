use super::{
    checkpoint::Checkpoint,
    config::AuctionConfig,
    primitives::{BlockNumber, CurrencyAmount},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GraduationStatus {
    #[default]
    NotGraduated,
    Graduated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TokenDepositStatus {
    #[default]
    Unknown,
    NotReceived,
    Received,
}

#[derive(Clone, Debug)]
pub enum AuctionPhase {
    PreStart { blocks_until_start: u64 },
    PreTokens,
    Active { blocks_remaining: u64 },
    Ended { blocks_until_claim: u64 },
    Claimable,
}

#[derive(Clone, Debug)]
pub struct AuctionState {
    pub current_block: BlockNumber,
    pub phase: AuctionPhase,
    pub checkpoint: Checkpoint,
    pub graduation: GraduationStatus,
    pub tokens_received: TokenDepositStatus,
    pub currency_raised: CurrencyAmount,
}

impl AuctionState {
    pub fn new(
        block: BlockNumber,
        checkpoint: Checkpoint,
        graduation: GraduationStatus,
        tokens_received: TokenDepositStatus,
        config: &AuctionConfig,
    ) -> Self {
        Self {
            current_block: block,
            phase: Self::compute_phase(config, block, tokens_received),
            checkpoint,
            graduation,
            tokens_received,
            currency_raised: CurrencyAmount::ZERO,
        }
    }

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
        let active = matches!(self.phase, AuctionPhase::Active { .. });
        active && !self.checkpoint.is_sold_out()
    }

    pub fn can_early_exit(&self) -> bool {
        let graduated = matches!(self.graduation, GraduationStatus::Graduated);
        let active = matches!(self.phase, AuctionPhase::Active { .. });
        graduated && active
    }
}
