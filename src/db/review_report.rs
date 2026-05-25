//! Cognitive review report database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CognitiveReviewReportDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub lens_id: Uuid,
    pub report_type: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub report: Value,
    pub source_memory_ids: Vec<Uuid>,
    pub source_lens_run_ids: Vec<Uuid>,
    pub summary_provider: String,
    pub summary_source: String,
    pub summary_model: Option<String>,
    pub summary_fallback_reason: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCognitiveReviewReport {
    pub space_id: Uuid,
    pub lens_id: Uuid,
    pub report_type: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub report: Value,
    pub source_memory_ids: Vec<Uuid>,
    pub source_lens_run_ids: Vec<Uuid>,
    pub summary_provider: String,
    pub summary_source: String,
    pub summary_model: Option<String>,
    pub summary_fallback_reason: Option<String>,
    pub created_by: Uuid,
}

#[derive(Debug, Clone)]
pub struct ReviewReportListFilter {
    pub space_id: Uuid,
    pub lens_id: Option<Uuid>,
    pub limit: i64,
}

#[async_trait::async_trait]
pub trait CognitiveReviewReportRepository: Send + Sync {
    async fn create(
        &self,
        report: CreateCognitiveReviewReport,
    ) -> Result<CognitiveReviewReportDb, Error>;
    async fn find_for_user(
        &self,
        report_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveReviewReportDb>, Error>;
    async fn list_for_user(
        &self,
        filter: ReviewReportListFilter,
        user_id: Uuid,
    ) -> Result<Vec<CognitiveReviewReportDb>, Error>;
}

pub struct PostgresCognitiveReviewReportRepository {
    pool: PgPool,
}

impl PostgresCognitiveReviewReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CognitiveReviewReportRepository for PostgresCognitiveReviewReportRepository {
    async fn create(
        &self,
        report: CreateCognitiveReviewReport,
    ) -> Result<CognitiveReviewReportDb, Error> {
        sqlx::query_as::<_, CognitiveReviewReportDb>(
            r#"
            INSERT INTO cognitive_review_reports (
                space_id,
                lens_id,
                report_type,
                window_start,
                window_end,
                report,
                source_memory_ids,
                source_lens_run_ids,
                summary_provider,
                summary_source,
                summary_model,
                summary_fallback_reason,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(report.space_id)
        .bind(report.lens_id)
        .bind(&report.report_type)
        .bind(report.window_start)
        .bind(report.window_end)
        .bind(&report.report)
        .bind(&report.source_memory_ids)
        .bind(&report.source_lens_run_ids)
        .bind(&report.summary_provider)
        .bind(&report.summary_source)
        .bind(&report.summary_model)
        .bind(&report.summary_fallback_reason)
        .bind(report.created_by)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_for_user(
        &self,
        report_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveReviewReportDb>, Error> {
        sqlx::query_as::<_, CognitiveReviewReportDb>(
            r#"
            SELECT r.*
            FROM cognitive_review_reports r
            INNER JOIN cognitive_space_members m ON m.space_id = r.space_id
            WHERE r.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(report_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list_for_user(
        &self,
        filter: ReviewReportListFilter,
        user_id: Uuid,
    ) -> Result<Vec<CognitiveReviewReportDb>, Error> {
        sqlx::query_as::<_, CognitiveReviewReportDb>(
            r#"
            SELECT r.*
            FROM cognitive_review_reports r
            INNER JOIN cognitive_space_members m ON m.space_id = r.space_id
            WHERE m.user_id = $1
              AND r.space_id = $2
              AND ($3::uuid IS NULL OR r.lens_id = $3)
            ORDER BY r.created_at DESC
            LIMIT $4
            "#,
        )
        .bind(user_id)
        .bind(filter.space_id)
        .bind(filter.lens_id)
        .bind(filter.limit)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_review_report_keeps_sources_and_provider() {
        let memory_id = Uuid::new_v4();
        let report = CreateCognitiveReviewReport {
            space_id: Uuid::new_v4(),
            lens_id: Uuid::new_v4(),
            report_type: "weekly_review".to_string(),
            window_start: Utc::now(),
            window_end: Utc::now(),
            report: json!({"summary": "review"}),
            source_memory_ids: vec![memory_id],
            source_lens_run_ids: vec![],
            summary_provider: "deterministic".to_string(),
            summary_source: "deterministic".to_string(),
            summary_model: None,
            summary_fallback_reason: Some("summary provider not configured".to_string()),
            created_by: Uuid::new_v4(),
        };

        assert_eq!(report.source_memory_ids, vec![memory_id]);
        assert_eq!(report.summary_provider, "deterministic");
        assert_eq!(report.report["summary"], "review");
    }
}
