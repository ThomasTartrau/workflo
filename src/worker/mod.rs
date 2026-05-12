pub mod resolver;
pub mod retry;

use std::sync::Arc;
use std::time::Duration;

use time::OffsetDateTime;
use tokio::spawn;
use tokio::sync::watch::Receiver;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::error::EngineError;
use crate::execution::status::ExecutionStatus;
use crate::execution::{Execution, WorkflowInitiator};
use crate::operation::runner::run_batch;
use crate::store::ExecutionStore;
use crate::worker::resolver::WorkflowResolver;
use crate::worker::retry::retry_delay;

pub struct WorkerConfig {
    pub concurrency: usize,
    pub poll_interval: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: 2,
            poll_interval: Duration::from_secs(5),
        }
    }
}

pub struct Worker<S, R, Ctx> {
    store: Arc<S>,
    resolver: Arc<R>,
    ctx: Arc<Ctx>,
    config: WorkerConfig,
}

impl<S, R, Ctx> Worker<S, R, Ctx>
where
    S: ExecutionStore,
    R: WorkflowResolver<Ctx>,
    Ctx: Send + Sync + 'static,
{
    pub fn new(store: S, resolver: R, ctx: Ctx, config: WorkerConfig) -> Self {
        Self {
            store: Arc::new(store),
            resolver: Arc::new(resolver),
            ctx: Arc::new(ctx),
            config,
        }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    pub async fn run(&self, mut shutdown: Receiver<bool>) {
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        loop {
            if *shutdown.borrow() {
                info!("worker shutting down");
                break;
            }

            let permit = tokio::select! {
                permit = semaphore.clone().acquire_owned() => match permit {
                    Ok(p) => p,
                    Err(_) => break,
                },
                _ = shutdown.changed() => continue,
            };

            match self.store.fetch_and_lock_next().await {
                Ok(Some(execution)) => {
                    let store = self.store.clone();
                    let resolver = self.resolver.clone();
                    let ctx = self.ctx.clone();
                    let shutdown_rx = shutdown.clone();

                    spawn(async move {
                        let _permit = permit;
                        let id = execution.id;
                        let name = execution.workflow_name.clone();
                        if let Err(e) =
                            process_execution(store, resolver, ctx, execution, shutdown_rx).await
                        {
                            error!(
                                execution_id = %id,
                                workflow = %name,
                                error = %e,
                                "execution failed"
                            );
                        }
                    });
                }
                Ok(None) => {
                    drop(permit);
                    tokio::select! {
                        _ = sleep(self.config.poll_interval) => {}
                        _ = shutdown.changed() => {}
                    }
                }
                Err(e) => {
                    drop(permit);
                    error!(error = %e, "failed to fetch next execution");
                    sleep(self.config.poll_interval).await;
                }
            }
        }
    }
}

async fn process_execution<S, R, Ctx>(
    store: Arc<S>,
    resolver: Arc<R>,
    ctx: Arc<Ctx>,
    mut execution: Execution,
    shutdown: Receiver<bool>,
) -> Result<(), EngineError>
where
    S: ExecutionStore,
    R: WorkflowResolver<Ctx>,
    Ctx: Send + Sync + 'static,
{
    // Phase 1: Check existing dependencies
    let deps = store.get_dependencies(execution.id).await?;
    if !deps.is_empty() {
        for (dep_id, dep_status) in &deps {
            match dep_status {
                ExecutionStatus::Failed => {
                    let msg = format!("dependency {dep_id} failed");
                    if let Err(e) = store.mark_failed(execution.id, &msg).await {
                        error!(execution_id = %execution.id, error = %e, "failed to mark as failed");
                    }
                    return Err(EngineError::DependencyFailed(*dep_id));
                }
                ExecutionStatus::Completed => continue,
                _ => {
                    let next = OffsetDateTime::now_utc() + time::Duration::seconds(5);
                    store
                        .schedule_retry(
                            execution.id,
                            execution.soft_try_count,
                            execution.hard_try_count,
                            next,
                        )
                        .await?;
                    info!(
                        execution_id = %execution.id,
                        dependency = %dep_id,
                        "waiting for dependency"
                    );
                    return Ok(());
                }
            }
        }
    }

    // Phase 2: Resolve prerequisite workflows
    let needed = resolver.needed_workflows(&mut execution, &ctx).await?;
    if !needed.is_empty() {
        let mut dep_ids = Vec::with_capacity(needed.len());
        for mut new_exec in needed {
            new_exec.initiated_by = WorkflowInitiator::Workflow(execution.id);
            let created = store.create(new_exec).await?;
            dep_ids.push(created.id);
        }
        store.add_dependencies(execution.id, &dep_ids).await?;
        store
            .save_definition(execution.id, execution.definition.clone())
            .await?;
        let next = OffsetDateTime::now_utc() + time::Duration::seconds(2);
        store
            .schedule_retry(
                execution.id,
                execution.soft_try_count,
                execution.hard_try_count,
                next,
            )
            .await?;
        info!(
            execution_id = %execution.id,
            workflow = %execution.workflow_name,
            count = dep_ids.len(),
            "scheduled prerequisites, waiting"
        );
        return Ok(());
    }

    // Phase 3: Operation loop
    loop {
        if *shutdown.borrow() {
            warn!(
                execution_id = %execution.id,
                workflow = %execution.workflow_name,
                "shutdown requested, rescheduling"
            );
            let next = OffsetDateTime::now_utc();
            if let Err(e) = store
                .schedule_retry(
                    execution.id,
                    execution.soft_try_count,
                    execution.hard_try_count,
                    next,
                )
                .await
            {
                error!(execution_id = %execution.id, error = %e, "failed to reschedule on shutdown");
            }
            return Ok(());
        }

        let operations = match resolver.resolve(&mut execution, &ctx).await {
            Ok(ops) => ops,
            Err(e) => {
                if let Err(store_err) = store.mark_failed(execution.id, &e.to_string()).await {
                    error!(execution_id = %execution.id, error = %store_err, "failed to mark as failed");
                }
                return Err(e);
            }
        };

        if operations.is_empty() {
            break;
        }

        match run_batch(&*ctx, operations).await {
            Ok(()) => {
                store
                    .save_definition(execution.id, execution.definition.clone())
                    .await?;
            }
            Err(batch_error) => {
                if batch_error.is_violated_invariant {
                    let msg = batch_error.message.clone();
                    if let Err(store_err) = store.mark_failed(execution.id, &msg).await {
                        error!(execution_id = %execution.id, error = %store_err, "failed to mark as failed");
                    }
                    return Err(EngineError::InvariantViolation(msg));
                }

                execution.soft_try_count += 1;

                if batch_error.consume_retry {
                    execution.hard_try_count += 1;
                }

                if execution.hard_try_count >= execution.max_try_count {
                    if let Err(store_err) =
                        store.mark_failed(execution.id, &batch_error.message).await
                    {
                        error!(execution_id = %execution.id, error = %store_err, "failed to mark as failed");
                    }
                    return Err(EngineError::MaxRetriesExceeded(execution.id));
                }

                let delay = retry_delay(execution.hard_try_count);
                let next = OffsetDateTime::now_utc() + delay;
                store
                    .schedule_retry(
                        execution.id,
                        execution.soft_try_count,
                        execution.hard_try_count,
                        next,
                    )
                    .await?;

                warn!(
                    execution_id = %execution.id,
                    workflow = %execution.workflow_name,
                    soft_try_count = execution.soft_try_count,
                    hard_try_count = execution.hard_try_count,
                    max_try_count = execution.max_try_count,
                    "scheduling retry"
                );
                return Ok(());
            }
        }
    }

    // Phase 4: Schedule follow-up workflows
    match resolver.next_workflows(&execution, &ctx).await {
        Ok(workflows) => {
            for mut new_exec in workflows {
                new_exec.initiated_by = WorkflowInitiator::Workflow(execution.id);
                if let Err(e) = store.create(new_exec).await {
                    error!(execution_id = %execution.id, error = %e, "failed to schedule follow-up");
                }
            }
        }
        Err(e) => {
            error!(execution_id = %execution.id, error = %e, "failed to resolve next workflows");
        }
    }

    // Phase 5: Mark completed
    store.mark_completed(execution.id).await?;
    info!(
        execution_id = %execution.id,
        workflow = %execution.workflow_name,
        "workflow completed"
    );
    Ok(())
}
