use crate::{
    error::HookError,
    types::{action::SubmitBidParams, state::AuctionState},
};
use alloy::primitives::Bytes;
use async_trait::async_trait;

#[allow(unused_variables)]
#[async_trait]
pub trait ValidationHook: Send + Sync {
    async fn prepare_hook_data(
        &self,
        params: &SubmitBidParams,
        state: &AuctionState,
    ) -> Result<Bytes, HookError> {
        Ok(Bytes::new())
    }

    async fn validate(
        &self,
        params: &SubmitBidParams,
        state: &AuctionState,
    ) -> Result<(), HookError> {
        Ok(())
    }
}
