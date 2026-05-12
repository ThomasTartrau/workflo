use futures::future::join_all;
use tracing::error;

use crate::error::BatchError;
use crate::operation::{Operation, OperationError};

pub async fn run_batch<Ctx, Op>(ctx: &Ctx, operations: Vec<Op>) -> Result<(), BatchError>
where
    Ctx: Send + Sync,
    Op: Operation<Ctx>,
{
    if operations.is_empty() {
        return Ok(());
    }

    let results: Vec<Result<Op, Op::Error>> =
        join_all(operations.into_iter().map(|op| op.execute(ctx))).await;

    let mut succeeded: Vec<Op> = Vec::new();
    let mut first_error: Option<Op::Error> = None;

    for result in results {
        match result {
            Ok(op) => succeeded.push(op),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                } else {
                    error!(error = %e, "additional operation failure");
                }
            }
        }
    }

    if let Some(error) = first_error {
        let consume_retry = error.consume_retry();
        let is_violated_invariant = error.is_violated_invariant();
        let message = error.to_string();

        let mut rollback_errors = Vec::new();
        for op in succeeded.into_iter().rev() {
            let op_name = op.name().to_owned();
            if let Err(e) = op.rollback(ctx).await {
                error!(operation = %op_name, error = %e, "rollback failed");
                rollback_errors.push(format!("{op_name}: {e}"));
            }
        }

        return Err(BatchError {
            message,
            consume_retry,
            is_violated_invariant,
            rollback_errors,
        });
    }

    for i in 0..succeeded.len() {
        let op = succeeded[i].clone();
        let op_name = op.name().to_owned();
        if let Err(e) = op.commit(ctx).await {
            error!(operation = %op_name, error = %e, "commit failed");
            let mut rollback_errors = Vec::new();
            for rb_op in succeeded.drain(i..).rev() {
                let rb_name = rb_op.name().to_owned();
                if let Err(re) = rb_op.rollback(ctx).await {
                    error!(operation = %rb_name, error = %re, "rollback failed");
                    rollback_errors.push(format!("{rb_name}: {re}"));
                }
            }
            return Err(BatchError {
                message: format!("commit failed for {op_name}: {e}"),
                consume_retry: true,
                is_violated_invariant: false,
                rollback_errors,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct TestOp {
        label: String,
        should_fail: bool,
        should_fail_commit: bool,
        invariant_violation: bool,
        execute_count: Arc<AtomicU32>,
        rollback_count: Arc<AtomicU32>,
        commit_count: Arc<AtomicU32>,
    }

    struct Counters {
        execute: Arc<AtomicU32>,
        rollback: Arc<AtomicU32>,
        commit: Arc<AtomicU32>,
    }

    impl Counters {
        fn new() -> Self {
            Self {
                execute: Arc::new(AtomicU32::new(0)),
                rollback: Arc::new(AtomicU32::new(0)),
                commit: Arc::new(AtomicU32::new(0)),
            }
        }
    }

    impl TestOp {
        fn ok(label: &str, c: &Counters) -> Self {
            Self {
                label: label.to_owned(),
                should_fail: false,
                should_fail_commit: false,
                invariant_violation: false,
                execute_count: c.execute.clone(),
                rollback_count: c.rollback.clone(),
                commit_count: c.commit.clone(),
            }
        }

        fn failing(label: &str, c: &Counters) -> Self {
            Self {
                label: label.to_owned(),
                should_fail: true,
                should_fail_commit: false,
                invariant_violation: false,
                execute_count: c.execute.clone(),
                rollback_count: c.rollback.clone(),
                commit_count: c.commit.clone(),
            }
        }

        fn invariant_violation(label: &str, c: &Counters) -> Self {
            Self {
                label: label.to_owned(),
                should_fail: true,
                should_fail_commit: false,
                invariant_violation: true,
                execute_count: c.execute.clone(),
                rollback_count: c.rollback.clone(),
                commit_count: c.commit.clone(),
            }
        }

        fn failing_commit(label: &str, c: &Counters) -> Self {
            Self {
                label: label.to_owned(),
                should_fail: false,
                should_fail_commit: true,
                invariant_violation: false,
                execute_count: c.execute.clone(),
                rollback_count: c.rollback.clone(),
                commit_count: c.commit.clone(),
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("test error")]
    struct TestError {
        invariant: bool,
    }

    impl OperationError for TestError {
        fn consume_retry(&self) -> bool {
            true
        }
        fn is_violated_invariant(&self) -> bool {
            self.invariant
        }
    }

    impl Operation<()> for TestOp {
        type Error = TestError;

        fn name(&self) -> &str {
            &self.label
        }

        async fn execute(self, _ctx: &()) -> Result<Self, Self::Error> {
            self.execute_count.fetch_add(1, Ordering::Relaxed);
            if self.should_fail {
                Err(TestError {
                    invariant: self.invariant_violation,
                })
            } else {
                Ok(self)
            }
        }

        async fn rollback(self, _ctx: &()) -> Result<(), Self::Error> {
            self.rollback_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn commit(self, _ctx: &()) -> Result<(), Self::Error> {
            self.commit_count.fetch_add(1, Ordering::Relaxed);
            if self.should_fail_commit {
                Err(TestError { invariant: false })
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn empty_batch_succeeds() {
        let ops: Vec<TestOp> = vec![];
        assert!(run_batch(&(), ops).await.is_ok());
    }

    #[tokio::test]
    async fn all_succeed_commits_all() {
        let c = Counters::new();
        let ops = vec![TestOp::ok("a", &c), TestOp::ok("b", &c)];

        assert!(run_batch(&(), ops).await.is_ok());
        assert_eq!(c.execute.load(Ordering::Relaxed), 2);
        assert_eq!(c.commit.load(Ordering::Relaxed), 2);
        assert_eq!(c.rollback.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn failure_rolls_back_succeeded() {
        let c = Counters::new();
        let ops = vec![TestOp::ok("a", &c), TestOp::failing("b", &c)];

        let err = run_batch(&(), ops).await.unwrap_err();
        assert!(err.consume_retry);
        assert!(!err.is_violated_invariant);
        assert_eq!(c.commit.load(Ordering::Relaxed), 0);
        assert!(c.rollback.load(Ordering::Relaxed) >= 1);
    }

    #[tokio::test]
    async fn invariant_violation_flagged() {
        let c = Counters::new();
        let ops = vec![TestOp::invariant_violation("bad", &c)];

        let err = run_batch(&(), ops).await.unwrap_err();
        assert!(err.is_violated_invariant);
    }

    #[tokio::test]
    async fn commit_failure_rolls_back_uncommitted() {
        let c = Counters::new();
        let ops = vec![
            TestOp::ok("a", &c),
            TestOp::failing_commit("b", &c),
            TestOp::ok("c", &c),
        ];

        let err = run_batch(&(), ops).await.unwrap_err();
        assert!(err.message.contains("commit failed"));
        assert_eq!(c.execute.load(Ordering::Relaxed), 3);
        assert_eq!(c.commit.load(Ordering::Relaxed), 2);
        assert_eq!(c.rollback.load(Ordering::Relaxed), 2);
    }
}
