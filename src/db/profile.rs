//! Cognitive profile snapshot database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CognitiveProfileSnapshotDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
    pub lens_id: Option<Uuid>,
    pub target: String,
    pub profile: Value,
    pub source_memory_ids: Vec<Uuid>,
    pub source_lens_run_ids: Vec<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCognitiveProfileSnapshot {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
    pub lens_id: Option<Uuid>,
    pub target: String,
    pub profile: Value,
    pub source_memory_ids: Vec<Uuid>,
    pub source_lens_run_ids: Vec<Uuid>,
    pub created_by: Uuid,
}

#[async_trait::async_trait]
pub trait CognitiveProfileRepository: Send + Sync {
    async fn create(
        &self,
        snapshot: CreateCognitiveProfileSnapshot,
    ) -> Result<CognitiveProfileSnapshotDb, Error>;
    async fn find_for_user(
        &self,
        snapshot_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveProfileSnapshotDb>, Error>;
}

pub struct PostgresCognitiveProfileRepository {
    pool: PgPool,
}

impl PostgresCognitiveProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CognitiveProfileRepository for PostgresCognitiveProfileRepository {
    async fn create(
        &self,
        snapshot: CreateCognitiveProfileSnapshot,
    ) -> Result<CognitiveProfileSnapshotDb, Error> {
        sqlx::query_as::<_, CognitiveProfileSnapshotDb>(
            r#"
            INSERT INTO cognitive_profile_snapshots (
                space_id,
                namespace_id,
                feedback_loop_id,
                lens_id,
                target,
                profile,
                source_memory_ids,
                source_lens_run_ids,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(snapshot.space_id)
        .bind(snapshot.namespace_id)
        .bind(snapshot.feedback_loop_id)
        .bind(snapshot.lens_id)
        .bind(&snapshot.target)
        .bind(&snapshot.profile)
        .bind(&snapshot.source_memory_ids)
        .bind(&snapshot.source_lens_run_ids)
        .bind(snapshot.created_by)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_for_user(
        &self,
        snapshot_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveProfileSnapshotDb>, Error> {
        sqlx::query_as::<_, CognitiveProfileSnapshotDb>(
            r#"
            SELECT p.*
            FROM cognitive_profile_snapshots p
            INNER JOIN cognitive_space_members m ON m.space_id = p.space_id
            WHERE p.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(snapshot_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_profile_snapshot_keeps_space_and_sources() {
        let space_id = Uuid::new_v4();
        let memory_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let snapshot = CreateCognitiveProfileSnapshot {
            space_id,
            namespace_id: Some(Uuid::new_v4()),
            feedback_loop_id: Some(Uuid::new_v4()),
            lens_id: None,
            target: "llm_context".to_string(),
            profile: json!({"summary": "compact context"}),
            source_memory_ids: vec![memory_id],
            source_lens_run_ids: vec![run_id],
            created_by: Uuid::new_v4(),
        };

        assert_eq!(snapshot.space_id, space_id);
        assert!(snapshot.namespace_id.is_some());
        assert!(snapshot.feedback_loop_id.is_some());
        assert_eq!(snapshot.source_memory_ids, vec![memory_id]);
        assert_eq!(snapshot.source_lens_run_ids, vec![run_id]);
        assert_eq!(snapshot.profile["summary"], "compact context");
    }
}
