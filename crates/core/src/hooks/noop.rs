use crate::{
    error::HookError,
    hooks::traits::ValidationHook,
    types::{action::SubmitBidParams, state::AuctionState},
};
use alloy::primitives::Bytes;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpHook;

#[async_trait]
impl ValidationHook for NoOpHook {
    async fn prepare_hook_data(
        &self,
        _params: &SubmitBidParams,
        _state: &AuctionState,
    ) -> Result<Bytes, HookError> {
        Ok(Bytes::new())
    }

    async fn validate(
        &self,
        _params: &SubmitBidParams,
        _state: &AuctionState,
    ) -> Result<(), HookError> {
        Ok(())
    }
}
