//! Trace database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TraceDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub source_type: String,
    pub task_type: String,
    pub mode: String,
    pub runtime: String,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub latency_ms: Option<i64>,
    pub status: String,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub token_usage: Option<Value>,
    pub estimated_cost_usd: Option<f64>,
    pub local_processing_ratio: Option<f64>,
    pub related_memory_ids: Vec<Uuid>,
    pub generated_memory_ids: Vec<Uuid>,
    pub generated_lens_run_ids: Vec<Uuid>,
    pub generated_review_report_ids: Vec<Uuid>,
    pub generated_feedback_loop_ids: Vec<Uuid>,
    pub user_feedback: Option<Value>,
    pub error: Option<Value>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCompletedTrace {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub source_type: TraceSourceType,
    pub task_type: TraceTaskType,
    pub mode: TraceMode,
    pub runtime: TraceRuntime,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub latency_ms: Option<i64>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub token_usage: Option<Value>,
    pub estimated_cost_usd: Option<f64>,
    pub local_processing_ratio: Option<f64>,
    pub related_memory_ids: Vec<Uuid>,
    pub generated_memory_ids: Vec<Uuid>,
    pub generated_lens_run_ids: Vec<Uuid>,
    pub generated_review_report_ids: Vec<Uuid>,
    pub generated_feedback_loop_ids: Vec<Uuid>,
    pub user_feedback: Option<Value>,
    pub error: Option<Value>,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
pub struct TraceListFilter {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub task_type: Option<TraceTaskType>,
    pub mode: Option<TraceMode>,
    pub runtime: Option<TraceRuntime>,
    pub status: Option<TraceStatus>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceSourceType {
    Http,
    Cli,
    Mcp,
    Ui,
    Background,
    TestFixture,
}

impl TraceSourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Cli => "cli",
            Self::Mcp => "mcp",
            Self::Ui => "ui",
            Self::Background => "background",
            Self::TestFixture => "test_fixture",
        }
    }
}

impl std::fmt::Display for TraceSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceTaskType {
    Chat,
    Capture,
    Search,
    LensRun,
    Review,
    Practice,
    Feedback,
    Planning,
    Observation,
    Install,
    Profile,
    Routing,
    Consolidation,
    Dreaming,
}

impl TraceTaskType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Capture => "capture",
            Self::Search => "search",
            Self::LensRun => "lens_run",
            Self::Review => "review",
            Self::Practice => "practice",
            Self::Feedback => "feedback",
            Self::Planning => "planning",
            Self::Observation => "observation",
            Self::Install => "install",
            Self::Profile => "profile",
            Self::Routing => "routing",
            Self::Consolidation => "consolidation",
            Self::Dreaming => "dreaming",
        }
    }
}

impl std::fmt::Display for TraceTaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMode {
    Fast,
    Focused,
    Deep,
    None,
}

impl TraceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Focused => "focused",
            Self::Deep => "deep",
            Self::None => "none",
        }
    }
}

impl std::fmt::Display for TraceMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceRuntime {
    Local,
    Cloud,
    Hybrid,
    Deterministic,
    Unknown,
}

impl TraceRuntime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Cloud => "cloud",
            Self::Hybrid => "hybrid",
            Self::Deterministic => "deterministic",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for TraceRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Started,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

impl TraceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

impl std::fmt::Display for TraceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[async_trait::async_trait]
pub trait TraceRepository: Send + Sync {
    async fn create_completed(&self, trace: CreateCompletedTrace) -> Result<TraceDb, Error>;
    async fn find_for_user(&self, trace_id: Uuid, user_id: Uuid) -> Result<Option<TraceDb>, Error>;
    async fn list_for_user(
        &self,
        filter: TraceListFilter,
        user_id: Uuid,
    ) -> Result<Vec<TraceDb>, Error>;
}

pub struct PostgresTraceRepository {
    pool: PgPool,
}

impl PostgresTraceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TraceRepository for PostgresTraceRepository {
    async fn create_completed(&self, trace: CreateCompletedTrace) -> Result<TraceDb, Error> {
        sqlx::query_as::<_, TraceDb>(
            r#"
            INSERT INTO traces (
                space_id,
                namespace_id,
                source_type,
                task_type,
                mode,
                runtime,
                input_summary,
                output_summary,
                started_at,
                completed_at,
                latency_ms,
                status,
                model_provider,
                model_name,
                token_usage,
                estimated_cost_usd,
                local_processing_ratio,
                related_memory_ids,
                generated_memory_ids,
                generated_lens_run_ids,
                generated_review_report_ids,
                generated_feedback_loop_ids,
                user_feedback,
                error,
                metadata
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, 'completed', $12, $13, $14, $15, $16, $17, $18, $19,
                $20, $21, $22, $23, $24
            )
            RETURNING *
            "#,
        )
        .bind(trace.space_id)
        .bind(trace.namespace_id)
        .bind(trace.source_type.to_string())
        .bind(trace.task_type.to_string())
        .bind(trace.mode.to_string())
        .bind(trace.runtime.to_string())
        .bind(&trace.input_summary)
        .bind(&trace.output_summary)
        .bind(trace.started_at)
        .bind(trace.completed_at)
        .bind(trace.latency_ms)
        .bind(&trace.model_provider)
        .bind(&trace.model_name)
        .bind(&trace.token_usage)
        .bind(trace.estimated_cost_usd)
        .bind(trace.local_processing_ratio)
        .bind(&trace.related_memory_ids)
        .bind(&trace.generated_memory_ids)
        .bind(&trace.generated_lens_run_ids)
        .bind(&trace.generated_review_report_ids)
        .bind(&trace.generated_feedback_loop_ids)
        .bind(&trace.user_feedback)
        .bind(&trace.error)
        .bind(&trace.metadata)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_for_user(&self, trace_id: Uuid, user_id: Uuid) -> Result<Option<TraceDb>, Error> {
        sqlx::query_as::<_, TraceDb>(
            r#"
            SELECT t.*
            FROM traces t
            INNER JOIN cognitive_space_members m ON m.space_id = t.space_id
            WHERE t.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(trace_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list_for_user(
        &self,
        filter: TraceListFilter,
        user_id: Uuid,
    ) -> Result<Vec<TraceDb>, Error> {
        sqlx::query_as::<_, TraceDb>(
            r#"
            SELECT t.*
            FROM traces t
            INNER JOIN cognitive_space_members m ON m.space_id = t.space_id
            WHERE t.space_id = $1
              AND m.user_id = $2
              AND ($3::uuid IS NULL OR t.namespace_id = $3)
              AND ($4::text IS NULL OR t.task_type = $4)
              AND ($5::text IS NULL OR t.mode = $5)
              AND ($6::text IS NULL OR t.runtime = $6)
              AND ($7::text IS NULL OR t.status = $7)
            ORDER BY t.started_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(filter.space_id)
        .bind(user_id)
        .bind(filter.namespace_id)
        .bind(filter.task_type.map(|task_type| task_type.to_string()))
        .bind(filter.mode.map(|mode| mode.to_string()))
        .bind(filter.runtime.map(|runtime| runtime.to_string()))
        .bind(filter.status.map(|status| status.to_string()))
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn create_completed_trace_keeps_space_scope_and_redacted_summaries() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let memory_id = Uuid::new_v4();
        let lens_run_id = Uuid::new_v4();
        let started_at = Utc::now();
        let completed_at = Utc::now();

        let trace = CreateCompletedTrace {
            space_id,
            namespace_id: Some(namespace_id),
            source_type: TraceSourceType::Mcp,
            task_type: TraceTaskType::LensRun,
            mode: TraceMode::Focused,
            runtime: TraceRuntime::Deterministic,
            input_summary: Some("redacted query summary".to_string()),
            output_summary: Some("redacted lens result summary".to_string()),
            started_at,
            completed_at,
            latency_ms: Some(42),
            model_provider: Some("deterministic".to_string()),
            model_name: None,
            token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
            estimated_cost_usd: Some(0.0),
            local_processing_ratio: Some(1.0),
            related_memory_ids: vec![memory_id],
            generated_memory_ids: vec![],
            generated_lens_run_ids: vec![lens_run_id],
            generated_review_report_ids: vec![],
            generated_feedback_loop_ids: vec![],
            user_feedback: None,
            error: None,
            metadata: json!({"contract": "trace-foundation"}),
        };

        assert_eq!(trace.space_id, space_id);
        assert_eq!(trace.namespace_id, Some(namespace_id));
        assert_eq!(trace.related_memory_ids, vec![memory_id]);
        assert_eq!(trace.generated_lens_run_ids, vec![lens_run_id]);
        assert_eq!(trace.source_type, TraceSourceType::Mcp);
        assert_eq!(trace.task_type, TraceTaskType::LensRun);
        assert_eq!(trace.mode, TraceMode::Focused);
        assert_eq!(trace.runtime, TraceRuntime::Deterministic);
        assert_eq!(
            trace.input_summary.as_deref(),
            Some("redacted query summary")
        );
    }

    #[test]
    fn trace_enums_serialize_to_contract_values() {
        assert_eq!(TraceSourceType::TestFixture.to_string(), "test_fixture");
        assert_eq!(TraceTaskType::LensRun.to_string(), "lens_run");
        assert_eq!(TraceMode::Focused.to_string(), "focused");
        assert_eq!(TraceRuntime::Deterministic.to_string(), "deterministic");
        assert_eq!(TraceStatus::Completed.to_string(), "completed");
    }
}
