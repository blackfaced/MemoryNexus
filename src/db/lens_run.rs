//! Lens Run database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LensRunDb {
    pub id: Uuid,
    pub lens_id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
    pub query: Option<String>,
    pub input_memory_ids: Vec<Uuid>,
    pub output: Option<Value>,
    pub status: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct CreateCompletedLensRun {
    pub lens_id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
    pub query: Option<String>,
    pub input_memory_ids: Vec<Uuid>,
    pub output: Value,
    pub created_by: Uuid,
}

#[derive(Debug, Clone)]
pub struct LensRunListFilter {
    pub lens_id: Option<Uuid>,
    pub space_id: Option<Uuid>,
    pub namespace_id: Option<Uuid>,
    pub limit: i64,
}

#[async_trait::async_trait]
pub trait LensRunRepository: Send + Sync {
    async fn create_completed(&self, run: CreateCompletedLensRun) -> Result<LensRunDb, Error>;
    async fn find_for_user(&self, run_id: Uuid, user_id: Uuid) -> Result<Option<LensRunDb>, Error>;
    async fn list_for_user(
        &self,
        filter: LensRunListFilter,
        user_id: Uuid,
    ) -> Result<Vec<LensRunDb>, Error>;
}

pub struct PostgresLensRunRepository {
    pool: PgPool,
}

impl PostgresLensRunRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl LensRunRepository for PostgresLensRunRepository {
    async fn create_completed(&self, run: CreateCompletedLensRun) -> Result<LensRunDb, Error> {
        sqlx::query_as::<_, LensRunDb>(
            r#"
            INSERT INTO lens_runs (
                lens_id,
                space_id,
                namespace_id,
                feedback_loop_id,
                query,
                input_memory_ids,
                output,
                status,
                created_by,
                completed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'completed', $8, NOW())
            RETURNING *
            "#,
        )
        .bind(run.lens_id)
        .bind(run.space_id)
        .bind(run.namespace_id)
        .bind(run.feedback_loop_id)
        .bind(&run.query)
        .bind(&run.input_memory_ids)
        .bind(&run.output)
        .bind(run.created_by)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_for_user(&self, run_id: Uuid, user_id: Uuid) -> Result<Option<LensRunDb>, Error> {
        sqlx::query_as::<_, LensRunDb>(
            r#"
            SELECT r.*
            FROM lens_runs r
            INNER JOIN cognitive_space_members m ON m.space_id = r.space_id
            WHERE r.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(run_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list_for_user(
        &self,
        filter: LensRunListFilter,
        user_id: Uuid,
    ) -> Result<Vec<LensRunDb>, Error> {
        sqlx::query_as::<_, LensRunDb>(
            r#"
            SELECT r.*
            FROM lens_runs r
            INNER JOIN cognitive_space_members m ON m.space_id = r.space_id
            WHERE m.user_id = $1
              AND ($2::uuid IS NULL OR r.lens_id = $2)
              AND ($3::uuid IS NULL OR r.space_id = $3)
              AND ($5::uuid IS NULL OR r.namespace_id = $5)
            ORDER BY r.created_at DESC
            LIMIT $4
            "#,
        )
        .bind(user_id)
        .bind(filter.lens_id)
        .bind(filter.space_id)
        .bind(filter.limit)
        .bind(filter.namespace_id)
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
    fn create_completed_lens_run_keeps_provenance() {
        let lens_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let memory_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        let run = CreateCompletedLensRun {
            lens_id,
            space_id,
            namespace_id: Some(Uuid::new_v4()),
            feedback_loop_id: Some(Uuid::new_v4()),
            query: Some("Summarize project direction".to_string()),
            input_memory_ids: vec![memory_id],
            output: json!({
                "query": "Summarize project direction",
                "memory_count": 1,
            }),
            created_by,
        };

        assert_eq!(run.lens_id, lens_id);
        assert_eq!(run.space_id, space_id);
        assert!(run.namespace_id.is_some());
        assert!(run.feedback_loop_id.is_some());
        assert_eq!(run.input_memory_ids, vec![memory_id]);
        assert_eq!(run.created_by, created_by);
        assert_eq!(run.output["memory_count"], 1);
    }

    #[test]
    fn lens_run_list_filter_keeps_space_or_lens_scope() {
        let lens_id = Uuid::new_v4();
        let filter = LensRunListFilter {
            lens_id: Some(lens_id),
            space_id: None,
            namespace_id: Some(Uuid::new_v4()),
            limit: 5,
        };

        assert_eq!(filter.lens_id, Some(lens_id));
        assert_eq!(filter.space_id, None);
        assert!(filter.namespace_id.is_some());
        assert_eq!(filter.limit, 5);
    }
}
