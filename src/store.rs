use std::future::Future;

use serde_json::Value;

use crate::datetime::DateTime;
use crate::error::StoreError;
use crate::execution::status::ExecutionStatus;
use crate::execution::{Execution, ExecutionId, NewExecution};

pub trait ExecutionStore: Send + Sync + 'static {
    fn create(
        &self,
        new: NewExecution,
    ) -> impl Future<Output = Result<Execution, StoreError>> + Send;

    fn get(&self, id: ExecutionId) -> impl Future<Output = Result<Execution, StoreError>> + Send;

    fn fetch_and_lock_next(
        &self,
    ) -> impl Future<Output = Result<Option<Execution>, StoreError>> + Send;

    fn save_definition(
        &self,
        id: ExecutionId,
        definition: Value,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn transition(
        &self,
        id: ExecutionId,
        target: ExecutionStatus,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn schedule_retry(
        &self,
        id: ExecutionId,
        soft_try_count: i32,
        hard_try_count: i32,
        next_retry_at: DateTime,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn mark_completed(
        &self,
        id: ExecutionId,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn mark_failed(
        &self,
        id: ExecutionId,
        error: &str,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn add_dependencies(
        &self,
        id: ExecutionId,
        dependencies: &[ExecutionId],
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn get_dependencies(
        &self,
        id: ExecutionId,
    ) -> impl Future<Output = Result<Vec<(ExecutionId, ExecutionStatus)>, StoreError>> + Send;
}

// -- PostgreSQL implementation --

use sqlx::PgPool;

use crate::repository::execution::ExecutionRepository;

#[derive(Clone)]
pub struct PgExecutionStore {
    pool: PgPool,
}

impl PgExecutionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl ExecutionStore for PgExecutionStore {
    async fn create(&self, new: NewExecution) -> Result<Execution, StoreError> {
        ExecutionRepository::create(&self.pool, new).await
    }

    async fn get(&self, id: ExecutionId) -> Result<Execution, StoreError> {
        ExecutionRepository::get(&self.pool, id).await
    }

    async fn fetch_and_lock_next(&self) -> Result<Option<Execution>, StoreError> {
        ExecutionRepository::fetch_and_lock_next(&self.pool).await
    }

    async fn save_definition(&self, id: ExecutionId, definition: Value) -> Result<(), StoreError> {
        ExecutionRepository::save_definition(&self.pool, id, definition).await
    }

    async fn transition(&self, id: ExecutionId, target: ExecutionStatus) -> Result<(), StoreError> {
        ExecutionRepository::transition(&self.pool, id, target).await
    }

    async fn schedule_retry(
        &self,
        id: ExecutionId,
        soft_try_count: i32,
        hard_try_count: i32,
        next_retry_at: DateTime,
    ) -> Result<(), StoreError> {
        ExecutionRepository::schedule_retry(
            &self.pool,
            id,
            soft_try_count,
            hard_try_count,
            next_retry_at,
        )
        .await
    }

    async fn mark_completed(&self, id: ExecutionId) -> Result<(), StoreError> {
        ExecutionRepository::mark_completed(&self.pool, id).await
    }

    async fn mark_failed(&self, id: ExecutionId, error: &str) -> Result<(), StoreError> {
        ExecutionRepository::mark_failed(&self.pool, id, error).await
    }

    async fn add_dependencies(
        &self,
        id: ExecutionId,
        dependencies: &[ExecutionId],
    ) -> Result<(), StoreError> {
        ExecutionRepository::add_dependencies(&self.pool, id, dependencies).await
    }

    async fn get_dependencies(
        &self,
        id: ExecutionId,
    ) -> Result<Vec<(ExecutionId, ExecutionStatus)>, StoreError> {
        ExecutionRepository::get_dependencies(&self.pool, id).await
    }
}
