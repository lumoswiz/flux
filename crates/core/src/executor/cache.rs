use crate::types::{
    checkpoint::Checkpoint,
    state::{GraduationStatus, TokenDepositStatus},
};

#[derive(Debug, Default)]
pub struct ExecutorCache {
    pub tokens_received: TokenDepositStatus,
    pub graduated: GraduationStatus,
    pub final_checkpoint: Option<Checkpoint>,
}

impl ExecutorCache {
    pub fn new() -> Self {
        Self {
            tokens_received: TokenDepositStatus::Unknown,
            graduated: GraduationStatus::NotGraduated,
            final_checkpoint: None,
        }
    }

    pub fn update(
        &mut self,
        tokens: Option<TokenDepositStatus>,
        graduation: Option<GraduationStatus>,
        checkpoint: Option<Checkpoint>,
        past_end_block: bool,
    ) {
        if let Some(status) = tokens {
            if matches!(status, TokenDepositStatus::Received) {
                self.tokens_received = status;
            }
        }

        if let Some(status) = graduation {
            if matches!(status, GraduationStatus::Graduated) {
                self.graduated = status;
            }
        }

        if past_end_block && checkpoint.is_some() && self.final_checkpoint.is_none() {
            self.final_checkpoint = checkpoint;
        }
    }

    pub fn needs_token_balance(&self) -> bool {
        !matches!(self.tokens_received, TokenDepositStatus::Received)
    }

    pub fn needs_graduation(&self) -> bool {
        !matches!(self.graduated, GraduationStatus::Graduated)
    }

    pub fn needs_checkpoint(&self, past_end_block: bool) -> bool {
        if past_end_block {
            self.final_checkpoint.is_none()
        } else {
            true
        }
    }
}
