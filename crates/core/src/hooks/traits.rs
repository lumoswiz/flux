use crate::{
    error::HookError,
    types::{action::SubmitBidParams, state::AuctionState},
};
use alloy::primitives::Bytes;
use async_trait::async_trait;

#[async_trait]
pub trait ValidationHook: Send + Sync {
    async fn prepare_hook_data(
        &self,
        params: &SubmitBidParams,
        state: &AuctionState,
    ) -> Result<Bytes, HookError>;

    async fn validate(
        &self,
        params: &SubmitBidParams,
        state: &AuctionState,
    ) -> Result<(), HookError>;
}
