use thiserror::Error as ThisError;

use crate::execution::ExecutionId;

#[derive(Debug, ThisError)]
#[error("batch failed: {message}")]
pub struct BatchError {
    pub message: String,
    pub consume_retry: bool,
    pub is_violated_invariant: bool,
    pub rollback_errors: Vec<String>,
}

#[derive(Debug, ThisError)]
pub enum StoreError {
    #[error("invalid transition from {from} to {to}, available: {available}")]
    InvalidTransition {
        from: String,
        to: String,
        available: String,
    },

    #[error("execution not found: {0}")]
    NotFound(ExecutionId),

    #[error("database error: {0}")]
    Database(String),
}

#[derive(Debug, ThisError)]
pub enum EngineError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),

    #[error("batch failed: {0}")]
    Batch(#[from] BatchError),

    #[error("workflow resolution failed: {0}")]
    Resolution(String),

    #[error("invariant violated: {0}")]
    InvariantViolation(String),

    #[error("max retries exceeded for execution {0}")]
    MaxRetriesExceeded(ExecutionId),

    #[error("dependency failed: {0}")]
    DependencyFailed(ExecutionId),
}
