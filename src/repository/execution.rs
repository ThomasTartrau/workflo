use serde_json::Value;
use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

use crate::datetime::DateTime;
use crate::error::StoreError;
use crate::execution::status::ExecutionStatus;
use crate::execution::{Execution, ExecutionId, NewExecution, WorkflowInitiator};
use crate::repository::fsm::FsmRepository;

fn db_err(e: SqlxError) -> StoreError {
    StoreError::Database(e.to_string())
}

fn parse_status(s: &str) -> Result<ExecutionStatus, StoreError> {
    s.parse::<ExecutionStatus>().map_err(StoreError::Database)
}

struct ExecutionRow {
    id: Uuid,
    workflow_name: String,
    status: Uuid,
    status_name: String,
    definition: Value,
    initiated_by_user: Option<Uuid>,
    initiated_by_workflow: Option<Uuid>,
    soft_try_count: i32,
    hard_try_count: i32,
    max_try_count: i32,
    error: Option<String>,
    next_retry_at: Option<DateTime>,
    locked_until: Option<DateTime>,
    created_at: DateTime,
    updated_at: DateTime,
    started_at: Option<DateTime>,
    completed_at: Option<DateTime>,
}

impl TryFrom<ExecutionRow> for Execution {
    type Error = StoreError;

    fn try_from(row: ExecutionRow) -> Result<Self, Self::Error> {
        let initiated_by = match (row.initiated_by_user, row.initiated_by_workflow) {
            (Some(user_id), _) => WorkflowInitiator::User(user_id),
            (_, Some(wf_id)) => WorkflowInitiator::Workflow(ExecutionId(wf_id)),
            _ => WorkflowInitiator::System,
        };

        Ok(Self {
            id: ExecutionId(row.id),
            workflow_name: row.workflow_name,
            status: parse_status(&row.status_name)?,
            status_machine_id: row.status,
            definition: row.definition,
            initiated_by,
            soft_try_count: row.soft_try_count,
            hard_try_count: row.hard_try_count,
            max_try_count: row.max_try_count,
            error: row.error,
            next_retry_at: row.next_retry_at,
            locked_until: row.locked_until,
            created_at: row.created_at,
            updated_at: row.updated_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
        })
    }
}

struct LockRow {
    id: Uuid,
    status: Uuid,
}

struct DependencyRow {
    dependency_id: Uuid,
    status_name: String,
}

pub struct ExecutionRepository;

impl ExecutionRepository {
    pub async fn create(pool: &PgPool, new: NewExecution) -> Result<Execution, StoreError> {
        let id = Uuid::now_v7();
        let (initiated_by_user, initiated_by_workflow) = match &new.initiated_by {
            WorkflowInitiator::User(user_id) => (Some(*user_id), None),
            WorkflowInitiator::Workflow(exec_id) => (None, Some(exec_id.0)),
            WorkflowInitiator::System => (None, None),
        };

        // INSERT and SELECT are separate statements because the DEFAULT on `status`
        // calls lib_fsm.state_machine_create(), which inserts into lib_fsm.state_machine.
        // In a CTE, the outer SELECT shares the same snapshot as the INSERT and cannot
        // see rows written to other tables by functions invoked during that INSERT.
        sqlx::query!(
            "INSERT INTO workflow_executions \
                 (id, workflow_name, definition, max_try_count, next_retry_at, \
                  initiated_by_user, initiated_by_workflow, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())",
            id,
            &new.workflow_name,
            &new.definition,
            new.max_try_count,
            new.schedule_at,
            initiated_by_user,
            initiated_by_workflow,
        )
        .execute(pool)
        .await
        .map_err(db_err)?;

        let row = sqlx::query_as!(
            ExecutionRow,
            r#"SELECT e.id, e.workflow_name, e.status,
                      abs.name as "status_name!",
                      e.definition as "definition: Value",
                      e.initiated_by_user, e.initiated_by_workflow,
                      e.soft_try_count, e.hard_try_count, e.max_try_count,
                      e.error, e.next_retry_at, e.locked_until,
                      e.created_at, e.updated_at, e.started_at, e.completed_at
               FROM workflow_executions e
               JOIN lib_fsm.state_machine sm ON sm.state_machine__id = e.status
               JOIN lib_fsm.abstract_state abs ON abs.abstract_state__id = sm.abstract_state__id
               WHERE e.id = $1"#,
            id
        )
        .fetch_one(pool)
        .await
        .map_err(db_err)?;

        row.try_into()
    }

    pub async fn get(pool: &PgPool, id: ExecutionId) -> Result<Execution, StoreError> {
        let row = sqlx::query_as!(
            ExecutionRow,
            r#"SELECT e.id, e.workflow_name, e.status,
                      abs.name as "status_name!",
                      e.definition as "definition: Value",
                      e.initiated_by_user, e.initiated_by_workflow,
                      e.soft_try_count, e.hard_try_count, e.max_try_count,
                      e.error, e.next_retry_at, e.locked_until,
                      e.created_at, e.updated_at, e.started_at, e.completed_at
               FROM workflow_executions e
               JOIN lib_fsm.state_machine sm ON sm.state_machine__id = e.status
               JOIN lib_fsm.abstract_state abs ON abs.abstract_state__id = sm.abstract_state__id
               WHERE e.id = $1"#,
            id.0
        )
        .fetch_optional(pool)
        .await
        .map_err(db_err)?
        .ok_or(StoreError::NotFound(id))?;

        row.try_into()
    }

    pub async fn fetch_and_lock_next(pool: &PgPool) -> Result<Option<Execution>, StoreError> {
        let mut tx = pool.begin().await.map_err(db_err)?;

        let lock_row = sqlx::query_as!(
            LockRow,
            "SELECT e.id, e.status \
             FROM workflow_executions e \
             JOIN lib_fsm.state_machine sm ON sm.state_machine__id = e.status \
             JOIN lib_fsm.abstract_state abs ON abs.abstract_state__id = sm.abstract_state__id \
             WHERE abs.name IN ('pending', 'will_retry', 'running') \
               AND (e.next_retry_at IS NULL OR e.next_retry_at <= now()) \
               AND (e.locked_until IS NULL OR e.locked_until <= now()) \
             ORDER BY e.next_retry_at ASC \
             FOR UPDATE OF e SKIP LOCKED \
             LIMIT 1"
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;

        let Some(lock_row) = lock_row else {
            tx.commit().await.map_err(db_err)?;
            return Ok(None);
        };

        FsmRepository::transition(&mut tx, &lock_row.status, "running").await?;

        sqlx::query!(
            "UPDATE workflow_executions \
             SET started_at = COALESCE(started_at, now()), \
                 locked_until = now() + interval '5 minutes', \
                 updated_at = now() \
             WHERE id = $1",
            lock_row.id
        )
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        let row = sqlx::query_as!(
            ExecutionRow,
            r#"SELECT e.id, e.workflow_name, e.status,
                      abs.name as "status_name!",
                      e.definition as "definition: Value",
                      e.initiated_by_user, e.initiated_by_workflow,
                      e.soft_try_count, e.hard_try_count, e.max_try_count,
                      e.error, e.next_retry_at, e.locked_until,
                      e.created_at, e.updated_at, e.started_at, e.completed_at
               FROM workflow_executions e
               JOIN lib_fsm.state_machine sm ON sm.state_machine__id = e.status
               JOIN lib_fsm.abstract_state abs ON abs.abstract_state__id = sm.abstract_state__id
               WHERE e.id = $1"#,
            lock_row.id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;

        tx.commit().await.map_err(db_err)?;
        Ok(Some(row.try_into()?))
    }

    pub async fn transition(
        pool: &PgPool,
        id: ExecutionId,
        target: ExecutionStatus,
    ) -> Result<(), StoreError> {
        let mut tx = pool.begin().await.map_err(db_err)?;

        let status_uuid: Uuid =
            sqlx::query_scalar!("SELECT status FROM workflow_executions WHERE id = $1", id.0)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?
                .ok_or(StoreError::NotFound(id))?;

        FsmRepository::transition(&mut tx, &status_uuid, &target.to_string()).await?;

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn save_definition(
        pool: &PgPool,
        id: ExecutionId,
        definition: Value,
    ) -> Result<(), StoreError> {
        let result = sqlx::query!(
            "UPDATE workflow_executions SET definition = $2, updated_at = now() WHERE id = $1",
            id.0,
            definition,
        )
        .execute(pool)
        .await
        .map_err(db_err)?;

        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound(id));
        }

        Ok(())
    }

    pub async fn schedule_retry(
        pool: &PgPool,
        id: ExecutionId,
        soft_try_count: i32,
        hard_try_count: i32,
        next_retry_at: DateTime,
    ) -> Result<(), StoreError> {
        let mut tx = pool.begin().await.map_err(db_err)?;

        let status_uuid: Uuid = sqlx::query_scalar!(
            "SELECT status FROM workflow_executions WHERE id = $1 FOR UPDATE",
            id.0
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;

        FsmRepository::transition(&mut tx, &status_uuid, "will_retry").await?;

        sqlx::query!(
            "UPDATE workflow_executions \
             SET soft_try_count = $2, hard_try_count = $3, \
                 next_retry_at = $4, locked_until = NULL, updated_at = now() \
             WHERE id = $1",
            id.0,
            soft_try_count,
            hard_try_count,
            next_retry_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn mark_completed(pool: &PgPool, id: ExecutionId) -> Result<(), StoreError> {
        let mut tx = pool.begin().await.map_err(db_err)?;

        let status_uuid: Uuid = sqlx::query_scalar!(
            "SELECT status FROM workflow_executions WHERE id = $1 FOR UPDATE",
            id.0
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;

        FsmRepository::transition(&mut tx, &status_uuid, "completed").await?;

        sqlx::query!(
            "UPDATE workflow_executions \
             SET completed_at = now(), locked_until = NULL, updated_at = now() \
             WHERE id = $1",
            id.0
        )
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn mark_failed(
        pool: &PgPool,
        id: ExecutionId,
        error: &str,
    ) -> Result<(), StoreError> {
        let mut tx = pool.begin().await.map_err(db_err)?;

        let status_uuid: Uuid = sqlx::query_scalar!(
            "SELECT status FROM workflow_executions WHERE id = $1 FOR UPDATE",
            id.0
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;

        FsmRepository::transition(&mut tx, &status_uuid, "failed").await?;

        sqlx::query!(
            "UPDATE workflow_executions \
             SET error = $2, completed_at = now(), locked_until = NULL, updated_at = now() \
             WHERE id = $1",
            id.0,
            error,
        )
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn add_dependencies(
        pool: &PgPool,
        id: ExecutionId,
        dependencies: &[ExecutionId],
    ) -> Result<(), StoreError> {
        if dependencies.is_empty() {
            return Ok(());
        }

        let dep_uuids: Vec<Uuid> = dependencies.iter().map(|d| d.0).collect();
        sqlx::query!(
            "INSERT INTO workflow_execution_dependencies (execution_id, dependency_id) \
             SELECT $1, unnest($2::uuid[]) \
             ON CONFLICT DO NOTHING",
            id.0,
            &dep_uuids,
        )
        .execute(pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }

    pub async fn get_dependencies(
        pool: &PgPool,
        id: ExecutionId,
    ) -> Result<Vec<(ExecutionId, ExecutionStatus)>, StoreError> {
        let rows = sqlx::query_as!(
            DependencyRow,
            r#"SELECT d.dependency_id, abs.name as "status_name!"
               FROM workflow_execution_dependencies d
               JOIN workflow_executions e ON e.id = d.dependency_id
               JOIN lib_fsm.state_machine sm ON sm.state_machine__id = e.status
               JOIN lib_fsm.abstract_state abs ON abs.abstract_state__id = sm.abstract_state__id
               WHERE d.execution_id = $1"#,
            id.0
        )
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

        rows.into_iter()
            .map(|row| {
                let status = parse_status(&row.status_name)?;
                Ok((ExecutionId(row.dependency_id), status))
            })
            .collect()
    }
}
