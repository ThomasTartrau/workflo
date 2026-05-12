use sqlx::{Error as SqlxError, PgConnection};
use uuid::Uuid;

use crate::error::StoreError;

fn db_err(e: SqlxError) -> StoreError {
    StoreError::Database(e.to_string())
}

#[derive(Debug, Clone)]
struct FsmTransition {
    state_from: String,
    state_to: String,
    event: String,
}

pub struct FsmRepository;

impl FsmRepository {
    async fn fetch_available_transitions(
        conn: &mut PgConnection,
        state_machine_id: &Uuid,
    ) -> Result<Vec<FsmTransition>, StoreError> {
        sqlx::query_as!(
            FsmTransition,
            r#"WITH available AS (
                    SELECT abt.to_abstract_state__id, abt.event, abs.name AS state_from
                    FROM lib_fsm.state_machine sm
                    JOIN lib_fsm.abstract_state abs ON abs.abstract_state__id = sm.abstract_state__id
                    JOIN lib_fsm.abstract_transition abt ON abt.from_abstract_state__id = abs.abstract_state__id
                    WHERE sm.state_machine__id = $1
                )
                SELECT a.state_from as "state_from!",
                       abs.name AS "state_to!",
                       a.event as "event!"
                FROM lib_fsm.abstract_state abs
                JOIN available a ON a.to_abstract_state__id = abs.abstract_state__id"#,
            state_machine_id
        )
        .fetch_all(conn)
        .await
        .map_err(db_err)
    }

    pub async fn transition(
        conn: &mut PgConnection,
        state_machine_id: &Uuid,
        target_state: &str,
    ) -> Result<(), StoreError> {
        let transitions = Self::fetch_available_transitions(conn, state_machine_id).await?;

        let matched = transitions.iter().find(|t| t.state_to == target_state);

        if let Some(t) = matched {
            sqlx::query_scalar!(
                r#"SELECT (lib_fsm.state_machine_transition($1, $2::lib_fsm.event_identifier)).abstract_state__id as "id!: Uuid""#,
                state_machine_id,
                &t.event as &str
            )
            .fetch_one(&mut *conn)
            .await
            .map_err(db_err)?;

            return Ok(());
        }

        let current_state = transitions
            .first()
            .map(|t| t.state_from.clone())
            .unwrap_or_else(|| "unknown".to_owned());

        let available = transitions
            .iter()
            .map(|t| format!("{} -> {} (event '{}')", t.state_from, t.state_to, t.event))
            .collect::<Vec<_>>()
            .join(", ");

        Err(StoreError::InvalidTransition {
            from: current_state,
            to: target_state.to_owned(),
            available,
        })
    }
}
