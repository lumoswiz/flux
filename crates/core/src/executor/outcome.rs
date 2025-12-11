use crate::{
    error::Error,
    types::action::{ClaimResult, ExitResult, SubmitBidResult},
};

use super::Intent;

#[derive(Debug)]
pub enum IntentOutcome {
    Success(IntentResult),
    Failed { intent: Intent, error: Error },
}

#[derive(Debug)]
pub enum IntentResult {
    BidSubmitted(SubmitBidResult),
    BidExited(ExitResult),
    TokensClaimed(ClaimResult),
}
