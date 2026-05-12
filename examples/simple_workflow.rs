//! Minimal workflow example: a single-step notification workflow.
//!
//! Shows how to:
//! - Implement `Operation` and `OperationError`
//! - Implement `WorkflowResolver`
//! - Wire up the `Worker` and enqueue a workflow
//!
//! Requires a running PostgreSQL with migrations applied:
//!   cargo sqlx migrate run --source migrations

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::watch;
use tracing::info;

use workflo::{
    EngineError, Execution, ExecutionStore, NewExecution, Operation, OperationError,
    PgExecutionStore, Worker, WorkerConfig, WorkflowInitiator, WorkflowResolver,
};

// -- Context: shared resources injected into operations --

struct AppContext {
    notification_endpoint: String,
}

// -- Operation error --

#[derive(Debug, thiserror::Error, OperationError)]
#[allow(dead_code)]
enum NotifyError {
    #[error("transient: {0}")]
    Transient(String),
    #[error("invalid recipient: {0}")]
    #[invariant]
    InvalidRecipient(String),
}

// -- Operation --

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SendNotification {
    recipient: String,
    message: String,
}

impl Operation<AppContext> for SendNotification {
    type Error = NotifyError;

    fn name(&self) -> &str {
        "send_notification"
    }

    async fn execute(self, ctx: &AppContext) -> Result<Self, Self::Error> {
        if self.recipient.is_empty() {
            return Err(NotifyError::InvalidRecipient(
                "recipient cannot be empty".to_owned(),
            ));
        }

        info!(
            endpoint = %ctx.notification_endpoint,
            recipient = %self.recipient,
            "sending notification"
        );

        // Simulate HTTP call to notification service
        // In production: reqwest::Client::post(&ctx.notification_endpoint)...

        Ok(self)
    }

    // No rollback needed for fire-and-forget notifications.
    // No commit needed either -- default impls return Ok(()).
}

// -- Workflow definition stored as JSON --

#[derive(Debug, Serialize, Deserialize)]
struct NotifyDefinition {
    recipient: String,
    message: String,
    sent: bool,
}

// -- Resolver: maps stored state to operations --

struct NotifyResolver;

impl WorkflowResolver<AppContext> for NotifyResolver {
    type Op = SendNotification;

    async fn resolve(
        &self,
        execution: &mut Execution,
        _ctx: &AppContext,
    ) -> Result<Vec<SendNotification>, EngineError> {
        let def: NotifyDefinition = serde_json::from_value(execution.definition.clone())
            .map_err(|e| EngineError::Resolution(e.to_string()))?;

        if def.sent {
            return Ok(vec![]);
        }

        let op = SendNotification {
            recipient: def.recipient.clone(),
            message: def.message.clone(),
        };

        execution.definition = serde_json::to_value(NotifyDefinition { sent: true, ..def })
            .map_err(|e| EngineError::Resolution(e.to_string()))?;

        Ok(vec![op])
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let store = PgExecutionStore::new(pool);

    // 1. Enqueue a workflow execution
    let new_execution = NewExecution {
        workflow_name: "notify".to_owned(),
        definition: json!({
            "recipient": "user@example.com",
            "message": "Your order has shipped!",
            "sent": false
        }),
        max_try_count: 3,
        schedule_at: None,
        initiated_by: WorkflowInitiator::System,
    };

    let execution = store
        .create(new_execution)
        .await
        .expect("failed to enqueue execution");

    info!(id = %execution.id, "enqueued notification workflow");

    // 2. Start the worker
    let ctx = AppContext {
        notification_endpoint: "https://api.example.com/notify".to_owned(),
    };

    let config = WorkerConfig {
        concurrency: 2,
        poll_interval: Duration::from_secs(1),
    };

    let worker = Worker::new(store, NotifyResolver, ctx, config);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Shut down after 5 seconds for demo purposes
    let shutdown_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let _ = shutdown_tx.send(true);
        info!("shutdown signal sent");
    });

    worker.run(shutdown_rx).await;
    shutdown_handle.await.unwrap();

    info!("worker stopped");
}
