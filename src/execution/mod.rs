pub mod status;

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::execution::status::ExecutionStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionId(pub Uuid);

impl ExecutionId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for ExecutionId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl Display for ExecutionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum WorkflowInitiator {
    User(Uuid),
    Workflow(ExecutionId),
    System,
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub id: ExecutionId,
    pub workflow_name: String,
    pub status: ExecutionStatus,
    pub status_machine_id: Uuid,
    pub definition: Value,
    pub initiated_by: WorkflowInitiator,
    pub soft_try_count: i32,
    pub hard_try_count: i32,
    pub max_try_count: i32,
    pub error: Option<String>,
    pub next_retry_at: Option<OffsetDateTime>,
    pub locked_until: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub started_at: Option<OffsetDateTime>,
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
pub struct NewExecution {
    pub workflow_name: String,
    pub definition: Value,
    pub max_try_count: i32,
    pub schedule_at: Option<OffsetDateTime>,
    pub initiated_by: WorkflowInitiator,
}
