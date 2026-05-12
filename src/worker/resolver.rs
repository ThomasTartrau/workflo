use std::future::Future;

use crate::error::EngineError;
use crate::execution::{Execution, NewExecution};
use crate::operation::Operation;

/// Bridge between stored execution state and concrete operations.
///
/// Each project implements this to deserialize its workflow definition
/// and produce the next batch of operations.
///
/// Must update `execution.definition` to reflect the new state after returning.
/// Return an empty Vec when the workflow is complete.
pub trait WorkflowResolver<Ctx: Send + Sync + 'static>: Send + Sync + 'static {
    type Op: Operation<Ctx>;

    /// Prerequisite workflows that must complete before this one proceeds.
    ///
    /// Called once before the operation loop. If non-empty, the returned
    /// workflows are created and registered as dependencies. The current
    /// execution is rescheduled until all dependencies complete.
    ///
    /// Must update `execution.definition` so the next call returns empty.
    fn needed_workflows(
        &self,
        _execution: &mut Execution,
        _ctx: &Ctx,
    ) -> impl Future<Output = Result<Vec<NewExecution>, EngineError>> + Send {
        async { Ok(vec![]) }
    }

    /// Next batch of operations to execute.
    ///
    /// Must update `execution.definition` to reflect the new state.
    /// Return an empty Vec when the workflow is complete.
    fn resolve(
        &self,
        execution: &mut Execution,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<Vec<Self::Op>, EngineError>> + Send;

    /// Workflows to schedule after this one completes successfully.
    fn next_workflows(
        &self,
        _execution: &Execution,
        _ctx: &Ctx,
    ) -> impl Future<Output = Result<Vec<NewExecution>, EngineError>> + Send {
        async { Ok(vec![]) }
    }
}
