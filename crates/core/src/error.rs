use alloy::{contract, providers::MulticallError, transports::TransportError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Hook(#[from] HookError),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to fetch config: {0}")]
    Transport(#[from] TransportError),

    #[error("contract call failed: {0}")]
    Contract(#[from] contract::Error),

    #[error("multicall failed: {0}")]
    Multicall(#[from] MulticallError),
}

#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook rejected bid: {reason}")]
    Rejected { reason: String },

    #[error("hook preparation failed: {0}")]
    PreparationFailed(String),

    #[error("hook validation failed: {0}")]
    ValidationFailed(String),
}
