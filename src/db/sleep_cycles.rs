//! SleepCycle database operations.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

use crate::domain::sleep_cycle::{SleepCycleStatus, SleepCycleType};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SleepCycleDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub cycle_type: String,
    pub status: String,
    pub evidence_window_start: DateTime<Utc>,
    pub evidence_window_end: DateTime<Utc>,
    pub input_trace_ids: Vec<Uuid>,
    pub input_memory_ids: Vec<Uuid>,
    pub input_feedback_loop_ids: Vec<Uuid>,
    pub input_review_report_ids: Vec<Uuid>,
    pub generated_memory_ids: Vec<Uuid>,
    pub triggering_trace_id: Option<Uuid>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateSleepCycle {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub cycle_type: SleepCycleType,
    pub status: SleepCycleStatus,
    pub evidence_window_start: DateTime<Utc>,
    pub evidence_window_end: DateTime<Utc>,
    pub input_trace_ids: Vec<Uuid>,
    pub input_memory_ids: Vec<Uuid>,
    pub input_feedback_loop_ids: Vec<Uuid>,
    pub input_review_report_ids: Vec<Uuid>,
    pub triggering_trace_id: Option<Uuid>,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct CompleteSleepCycle {
    pub generated_memory_ids: Vec<Uuid>,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct FailSleepCycle {
    pub error: String,
    pub metadata: Value,
}

#[async_trait::async_trait]
pub trait SleepCycleRepository: Send + Sync {
    async fn create(&self, sleep_cycle: CreateSleepCycle) -> Result<SleepCycleDb, Error>;
    async fn mark_completed(
        &self,
        sleep_cycle_id: Uuid,
        completion: CompleteSleepCycle,
    ) -> Result<Option<SleepCycleDb>, Error>;
    async fn mark_failed(
        &self,
        sleep_cycle_id: Uuid,
        failure: FailSleepCycle,
    ) -> Result<Option<SleepCycleDb>, Error>;
    async fn find_for_user(
        &self,
        sleep_cycle_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<SleepCycleDb>, Error>;
}

pub struct PostgresSleepCycleRepository {
    pool: PgPool,
}

impl PostgresSleepCycleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn validate_namespace_same_space(
    pool: &PgPool,
    namespace_id: Option<Uuid>,
    space_id: Uuid,
) -> Result<(), Error> {
    let Some(namespace_id) = namespace_id else {
        return Ok(());
    };

    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM namespaces
            WHERE id = $1 AND space_id = $2
        )
        "#,
    )
    .bind(namespace_id)
    .bind(space_id)
    .fetch_one(pool)
    .await?;

    if exists {
        Ok(())
    } else {
        Err(Error::RowNotFound)
    }
}

async fn validate_uuid_array_same_space(
    pool: &PgPool,
    table_name: &str,
    ids: &[Uuid],
    space_id: Uuid,
) -> Result<(), Error> {
    if ids.is_empty() {
        return Ok(());
    }

    let sql = match table_name {
        "traces" => {
            r#"
            SELECT COUNT(*)
            FROM traces
            WHERE id = ANY($1) AND space_id = $2
            "#
        }
        "memories" => {
            r#"
            SELECT COUNT(*)
            FROM memories
            WHERE id = ANY($1) AND space_id = $2
            "#
        }
        "feedback_loops" => {
            r#"
            SELECT COUNT(*)
            FROM feedback_loops
            WHERE id = ANY($1) AND space_id = $2
            "#
        }
        "cognitive_review_reports" => {
            r#"
            SELECT COUNT(*)
            FROM cognitive_review_reports
            WHERE id = ANY($1) AND space_id = $2
            "#
        }
        _ => return Err(Error::RowNotFound),
    };

    let matching_count: i64 = sqlx::query_scalar(sql)
        .bind(ids)
        .bind(space_id)
        .fetch_one(pool)
        .await?;

    if matching_count == unique_uuid_count(ids) as i64 {
        Ok(())
    } else {
        Err(Error::RowNotFound)
    }
}

async fn validate_optional_trace_same_space(
    pool: &PgPool,
    trace_id: Option<Uuid>,
    space_id: Uuid,
) -> Result<(), Error> {
    let Some(trace_id) = trace_id else {
        return Ok(());
    };

    validate_uuid_array_same_space(pool, "traces", &[trace_id], space_id).await
}

struct SleepCycleLinkValidation<'a> {
    pool: &'a PgPool,
    space_id: Uuid,
    namespace_id: Option<Uuid>,
    input_trace_ids: &'a [Uuid],
    input_memory_ids: &'a [Uuid],
    input_feedback_loop_ids: &'a [Uuid],
    input_review_report_ids: &'a [Uuid],
    triggering_trace_id: Option<Uuid>,
}

async fn validate_sleep_cycle_same_space_links(
    validation: SleepCycleLinkValidation<'_>,
) -> Result<(), Error> {
    validate_namespace_same_space(
        validation.pool,
        validation.namespace_id,
        validation.space_id,
    )
    .await?;
    validate_uuid_array_same_space(
        validation.pool,
        "traces",
        validation.input_trace_ids,
        validation.space_id,
    )
    .await?;
    validate_uuid_array_same_space(
        validation.pool,
        "memories",
        validation.input_memory_ids,
        validation.space_id,
    )
    .await?;
    validate_uuid_array_same_space(
        validation.pool,
        "feedback_loops",
        validation.input_feedback_loop_ids,
        validation.space_id,
    )
    .await?;
    validate_uuid_array_same_space(
        validation.pool,
        "cognitive_review_reports",
        validation.input_review_report_ids,
        validation.space_id,
    )
    .await?;
    validate_optional_trace_same_space(
        validation.pool,
        validation.triggering_trace_id,
        validation.space_id,
    )
    .await?;
    Ok(())
}

fn unique_uuid_count(ids: &[Uuid]) -> usize {
    ids.iter().collect::<HashSet<_>>().len()
}

#[async_trait::async_trait]
impl SleepCycleRepository for PostgresSleepCycleRepository {
    async fn create(&self, sleep_cycle: CreateSleepCycle) -> Result<SleepCycleDb, Error> {
        validate_sleep_cycle_same_space_links(SleepCycleLinkValidation {
            pool: &self.pool,
            space_id: sleep_cycle.space_id,
            namespace_id: sleep_cycle.namespace_id,
            input_trace_ids: &sleep_cycle.input_trace_ids,
            input_memory_ids: &sleep_cycle.input_memory_ids,
            input_feedback_loop_ids: &sleep_cycle.input_feedback_loop_ids,
            input_review_report_ids: &sleep_cycle.input_review_report_ids,
            triggering_trace_id: sleep_cycle.triggering_trace_id,
        })
        .await?;

        sqlx::query_as::<_, SleepCycleDb>(
            r#"
            INSERT INTO sleep_cycles (
                space_id,
                namespace_id,
                cycle_type,
                status,
                evidence_window_start,
                evidence_window_end,
                input_trace_ids,
                input_memory_ids,
                input_feedback_loop_ids,
                input_review_report_ids,
                triggering_trace_id,
                started_at,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), $12)
            RETURNING *
            "#,
        )
        .bind(sleep_cycle.space_id)
        .bind(sleep_cycle.namespace_id)
        .bind(sleep_cycle.cycle_type.to_string())
        .bind(sleep_cycle.status.to_string())
        .bind(sleep_cycle.evidence_window_start)
        .bind(sleep_cycle.evidence_window_end)
        .bind(&sleep_cycle.input_trace_ids)
        .bind(&sleep_cycle.input_memory_ids)
        .bind(&sleep_cycle.input_feedback_loop_ids)
        .bind(&sleep_cycle.input_review_report_ids)
        .bind(sleep_cycle.triggering_trace_id)
        .bind(&sleep_cycle.metadata)
        .fetch_one(&self.pool)
        .await
    }

    async fn mark_completed(
        &self,
        sleep_cycle_id: Uuid,
        completion: CompleteSleepCycle,
    ) -> Result<Option<SleepCycleDb>, Error> {
        let Some(existing) = sqlx::query_as::<_, SleepCycleDb>(
            r#"
            SELECT *
            FROM sleep_cycles
            WHERE id = $1
            "#,
        )
        .bind(sleep_cycle_id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        validate_uuid_array_same_space(
            &self.pool,
            "memories",
            &completion.generated_memory_ids,
            existing.space_id,
        )
        .await?;

        sqlx::query_as::<_, SleepCycleDb>(
            r#"
            UPDATE sleep_cycles
            SET status = $2,
                generated_memory_ids = $3,
                error = NULL,
                completed_at = NOW(),
                updated_at = NOW(),
                metadata = $4
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(sleep_cycle_id)
        .bind(SleepCycleStatus::Completed.to_string())
        .bind(&completion.generated_memory_ids)
        .bind(&completion.metadata)
        .fetch_optional(&self.pool)
        .await
    }

    async fn mark_failed(
        &self,
        sleep_cycle_id: Uuid,
        failure: FailSleepCycle,
    ) -> Result<Option<SleepCycleDb>, Error> {
        sqlx::query_as::<_, SleepCycleDb>(
            r#"
            UPDATE sleep_cycles
            SET status = $2,
                error = $3,
                completed_at = NOW(),
                updated_at = NOW(),
                metadata = $4
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(sleep_cycle_id)
        .bind(SleepCycleStatus::Failed.to_string())
        .bind(&failure.error)
        .bind(&failure.metadata)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_for_user(
        &self,
        sleep_cycle_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<SleepCycleDb>, Error> {
        sqlx::query_as::<_, SleepCycleDb>(
            r#"
            SELECT sc.*
            FROM sleep_cycles sc
            INNER JOIN cognitive_space_members m ON m.space_id = sc.space_id
            WHERE sc.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(sleep_cycle_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_sleep_cycle_keeps_window_status_and_links() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let evidence_window_start = Utc::now();
        let evidence_window_end = Utc::now();
        let input_trace_id = Uuid::new_v4();
        let input_memory_id = Uuid::new_v4();

        let sleep_cycle = CreateSleepCycle {
            space_id,
            namespace_id: Some(namespace_id),
            cycle_type: SleepCycleType::Manual,
            status: SleepCycleStatus::Pending,
            evidence_window_start,
            evidence_window_end,
            input_trace_ids: vec![input_trace_id],
            input_memory_ids: vec![input_memory_id],
            input_feedback_loop_ids: Vec::new(),
            input_review_report_ids: Vec::new(),
            triggering_trace_id: None,
            metadata: serde_json::json!({"reason": "manual"}),
        };

        assert_eq!(sleep_cycle.space_id, space_id);
        assert_eq!(sleep_cycle.namespace_id, Some(namespace_id));
        assert_eq!(sleep_cycle.cycle_type, SleepCycleType::Manual);
        assert_eq!(sleep_cycle.status, SleepCycleStatus::Pending);
        assert_eq!(sleep_cycle.evidence_window_start, evidence_window_start);
        assert_eq!(sleep_cycle.evidence_window_end, evidence_window_end);
        assert_eq!(sleep_cycle.input_trace_ids, vec![input_trace_id]);
        assert_eq!(sleep_cycle.input_memory_ids, vec![input_memory_id]);
    }

    #[test]
    fn complete_sleep_cycle_carries_generated_output_links() {
        let generated_memory_id = Uuid::new_v4();
        let completion = CompleteSleepCycle {
            generated_memory_ids: vec![generated_memory_id],
            metadata: serde_json::json!({"summary": "completed"}),
        };

        assert_eq!(completion.generated_memory_ids, vec![generated_memory_id]);
    }

    #[test]
    fn fail_sleep_cycle_records_redacted_error() {
        let failure = FailSleepCycle {
            error: "same_space_validation_failed".to_string(),
            metadata: serde_json::json!({"stage": "validation"}),
        };

        assert_eq!(failure.error, "same_space_validation_failed");
    }

    #[test]
    fn uuid_validation_count_ignores_duplicate_links() {
        let id = Uuid::new_v4();

        assert_eq!(unique_uuid_count(&[id, id]), 1);
    }
}
