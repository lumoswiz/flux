use alloy::primitives::Address;

use crate::{
    error::ValidationError,
    types::{
        action::SubmitBidInput,
        bid::{Bid, BidStatus},
        config::AuctionConfig,
        state::{AuctionPhase, AuctionState, GraduationStatus, TokenDepositStatus},
    },
};

pub fn validate_submit_bid(
    input: &SubmitBidInput,
    state: &AuctionState,
    config: &AuctionConfig,
) -> Result<(), ValidationError> {
    let current_block = state.current_block.as_u64();
    let start_block = config.start_block.as_u64();
    let end_block = config.end_block.as_u64();

    if current_block < start_block {
        return Err(ValidationError::AuctionNotStarted);
    }

    if current_block >= end_block {
        return Err(ValidationError::AuctionIsOver);
    }

    if !matches!(state.phase, AuctionPhase::Active { .. }) {
        return Err(ValidationError::AuctionNotActive);
    }

    if !matches!(state.tokens_received, TokenDepositStatus::Received) {
        return Err(ValidationError::TokensNotReceived);
    }

    if input.amount.is_zero() {
        return Err(ValidationError::AmountTooSmall);
    }

    if input.owner == Address::ZERO {
        return Err(ValidationError::OwnerIsZeroAddress);
    }

    if !config.is_valid_price(input.max_price) {
        return Err(ValidationError::InvalidPrice);
    }

    if state.checkpoint.is_sold_out() {
        return Err(ValidationError::AuctionSoldOut);
    }

    if input.max_price <= state.checkpoint.clearing_price {
        return Err(ValidationError::BidBelowClearingPrice);
    }

    Ok(())
}

pub fn validate_exit_bid(
    bid: &Bid,
    state: &AuctionState,
    config: &AuctionConfig,
) -> Result<(), ValidationError> {
    let current_block = state.current_block.as_u64();
    let end_block = config.end_block.as_u64();

    if current_block < end_block {
        return Err(ValidationError::AuctionNotOver);
    }

    if bid.exited_block.is_some() {
        return Err(ValidationError::BidAlreadyExited);
    }

    if matches!(state.graduation, GraduationStatus::Graduated) {
        let status = bid.status(state.checkpoint.clearing_price);
        if !matches!(status, BidStatus::ITM) {
            return Err(ValidationError::BidNotITM);
        }
    }

    Ok(())
}

pub fn validate_exit_partially_filled(
    bid: &Bid,
    state: &AuctionState,
    config: &AuctionConfig,
) -> Result<(), ValidationError> {
    if bid.exited_block.is_some() {
        return Err(ValidationError::BidAlreadyExited);
    }

    let is_graduated = matches!(state.graduation, GraduationStatus::Graduated);
    let is_ended = state.current_block.as_u64() >= config.end_block.as_u64();
    let status = bid.status(state.checkpoint.clearing_price);

    match (is_graduated, is_ended) {
        (true, false) => {
            if !matches!(status, BidStatus::OTM) {
                return Err(ValidationError::BidNotOutbid);
            }
        }

        (true, true) => {
            if matches!(status, BidStatus::ITM) {
                return Err(ValidationError::BidIsITM);
            }
        }

        (false, true) => {
            return Err(ValidationError::UseExitBidForRefund);
        }

        (false, false) => {
            return Err(ValidationError::CannotPartiallyExitBeforeGraduation);
        }
    }

    Ok(())
}

pub fn validate_claim(
    bids: &[Bid],
    expected_owner: Address,
    state: &AuctionState,
    config: &AuctionConfig,
) -> Result<(), ValidationError> {
    let current_block = state.current_block.as_u64();
    let claim_block = config.claim_block.as_u64();

    if current_block < claim_block {
        return Err(ValidationError::ClaimBlockNotReached);
    }

    if !matches!(state.graduation, GraduationStatus::Graduated) {
        return Err(ValidationError::NotGraduated);
    }

    for bid in bids {
        if bid.exited_block.is_none() {
            return Err(ValidationError::BidNotExited);
        }

        if bid.tokens_filled.is_zero() {
            return Err(ValidationError::NoTokensToClaim);
        }

        if bid.owner != expected_owner {
            return Err(ValidationError::OwnerMismatch);
        }
    }

    Ok(())
}
