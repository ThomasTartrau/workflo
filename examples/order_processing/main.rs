mod context;
mod definition;
mod error;
mod operation;
mod resolver;

use std::time::Duration;

use serde_json::json;
use sqlx::PgPool;
use tokio::sync::watch;
use tracing::info;

use workflo::{
    ExecutionStore, NewExecution, PgExecutionStore, Worker, WorkerConfig, WorkflowInitiator,
};

use crate::context::OrderContext;
use crate::resolver::OrderResolver;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let store = PgExecutionStore::new(pool);

    let new_execution = NewExecution {
        workflow_name: "order_processing".to_owned(),
        definition: json!({
            "order_id": "ORD-2026-0042",
            "item_id": "SKU-WIDGET-001",
            "quantity": 3,
            "amount_cents": 8997,
            "currency": "EUR",
            "shipping_address": "12 rue de Rivoli, 75001 Paris",
            "current_step": "reserve_stock"
        }),
        max_try_count: 5,
        schedule_at: None,
        initiated_by: WorkflowInitiator::System,
    };

    let execution = store
        .create(new_execution)
        .await
        .expect("failed to enqueue order workflow");

    info!(id = %execution.id, "enqueued order processing workflow");

    let ctx = OrderContext {
        inventory_api: "https://api.example.com/inventory".to_owned(),
        payment_api: "https://api.example.com/payments".to_owned(),
        shipping_api: "https://api.example.com/shipping".to_owned(),
    };

    let config = WorkerConfig {
        concurrency: 4,
        poll_interval: Duration::from_secs(1),
    };

    let worker = Worker::new(store, OrderResolver, ctx, config);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let shutdown_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let _ = shutdown_tx.send(true);
        info!("shutdown signal sent");
    });

    worker.run(shutdown_rx).await;
    shutdown_handle.await.unwrap();

    info!("worker stopped");
}
