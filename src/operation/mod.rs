pub mod runner;

use std::error::Error;
use std::future::Future;

/// Classifies operation errors for retry decisions.
pub trait OperationError: Error + Send + Sync + 'static {
    /// Hard failure that counts toward max retries.
    /// Return false for soft failures (rate limit, lock contention) that retry without counting.
    fn consume_retry(&self) -> bool;

    /// Unrecoverable error - skip retries, go to dead letter queue.
    fn is_violated_invariant(&self) -> bool;
}

/// Atomic unit of work with saga-style execute/rollback/commit lifecycle.
pub trait Operation<Ctx: Send + Sync>: Clone + Send + 'static {
    type Error: OperationError;

    fn name(&self) -> &str;

    fn execute(self, ctx: &Ctx) -> impl Future<Output = Result<Self, Self::Error>> + Send;

    fn rollback(self, _ctx: &Ctx) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }

    fn commit(self, _ctx: &Ctx) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
}
