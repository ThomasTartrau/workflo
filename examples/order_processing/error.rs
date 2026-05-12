use workflo::OperationError;

#[derive(Debug, thiserror::Error, OperationError)]
#[allow(dead_code)]
pub enum OrderOperationError {
    #[error("transient: {0}")]
    Transient(String),
    #[error("insufficient stock for {item_id} (requested {requested})")]
    #[invariant]
    InsufficientStock { item_id: String, requested: u32 },
    #[error("payment declined: {0}")]
    #[invariant]
    PaymentDeclined(String),
}
