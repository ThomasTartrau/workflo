extern crate self as workflo;

pub mod datetime;
pub mod error;
pub mod execution;
pub mod operation;
pub mod repository;
pub mod store;
pub mod worker;

pub use error::{BatchError, EngineError, StoreError};
pub use execution::status::ExecutionStatus;
pub use execution::{Execution, ExecutionId, NewExecution, WorkflowInitiator};
pub use operation::runner::run_batch;
pub use operation::{Operation, OperationError};
pub use repository::execution::ExecutionRepository;
pub use store::{ExecutionStore, PgExecutionStore};
pub use worker::resolver::WorkflowResolver;
pub use worker::retry::retry_delay;
pub use worker::{Worker, WorkerConfig};
pub use workflo_macros::{OperationEnum, OperationError};

use sqlx::PgPool;

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
