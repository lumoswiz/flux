use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook rejected bid: {reason}")]
    Rejected { reason: String },

    #[error("hook preparation failed: {0}")]
    PreparationFailed(String),

    #[error("hook validation failed: {0}")]
    ValidationFailed(String),
}
