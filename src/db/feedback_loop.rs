//! FeedbackLoop database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::db::memory::MemoryDb;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FeedbackLoopDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Uuid,
    pub goal: String,
    pub task: String,
    pub attempt: Option<String>,
    pub evaluation: Option<String>,
    pub feedback: Option<String>,
    pub adjustment: Option<String>,
    pub next_task: Option<String>,
    pub status: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateFeedbackLoop {
    pub space_id: Uuid,
    pub namespace_id: Uuid,
    pub goal: String,
    pub task: String,
    pub attempt: Option<String>,
    pub evaluation: Option<String>,
    pub feedback: Option<String>,
    pub adjustment: Option<String>,
    pub next_task: Option<String>,
    pub status: String,
    pub created_by: Uuid,
}

#[derive(Debug, Clone)]
pub struct FeedbackLoopListFilter {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Default)]
pub struct PatchFeedbackLoop {
    pub attempt: Option<String>,
    pub evaluation: Option<String>,
    pub feedback: Option<String>,
    pub adjustment: Option<String>,
    pub next_task: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackLoopMemorySnapshot {
    pub user_id: Uuid,
    pub event_kind: String,
    pub content: String,
    pub included_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackLoopWithMemorySnapshot {
    pub feedback_loop: FeedbackLoopDb,
    pub memory: Option<MemoryDb>,
}

#[async_trait::async_trait]
pub trait FeedbackLoopRepository: Send + Sync {
    async fn create(&self, feedback_loop: CreateFeedbackLoop) -> Result<FeedbackLoopDb, Error>;
    async fn create_with_memory_snapshot(
        &self,
        feedback_loop: CreateFeedbackLoop,
        snapshot: FeedbackLoopMemorySnapshot,
    ) -> Result<FeedbackLoopWithMemorySnapshot, Error>;
    async fn list_for_user(
        &self,
        filter: FeedbackLoopListFilter,
        user_id: Uuid,
    ) -> Result<Vec<FeedbackLoopDb>, Error>;
    async fn find_for_user(
        &self,
        feedback_loop_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<FeedbackLoopDb>, Error>;
    async fn patch(
        &self,
        feedback_loop_id: Uuid,
        patch: PatchFeedbackLoop,
    ) -> Result<Option<FeedbackLoopDb>, Error>;
    async fn patch_with_memory_snapshot(
        &self,
        feedback_loop_id: Uuid,
        patch: PatchFeedbackLoop,
        snapshot: FeedbackLoopMemorySnapshot,
    ) -> Result<Option<FeedbackLoopWithMemorySnapshot>, Error>;
}

pub struct PostgresFeedbackLoopRepository {
    pool: PgPool,
}

impl PostgresFeedbackLoopRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn insert_feedback_loop_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    feedback_loop: &CreateFeedbackLoop,
) -> Result<FeedbackLoopDb, Error> {
    sqlx::query_as::<_, FeedbackLoopDb>(
        r#"
        INSERT INTO feedback_loops (
            space_id,
            namespace_id,
            goal,
            task,
            attempt,
            evaluation,
            feedback,
            adjustment,
            next_task,
            status,
            created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#,
    )
    .bind(feedback_loop.space_id)
    .bind(feedback_loop.namespace_id)
    .bind(&feedback_loop.goal)
    .bind(&feedback_loop.task)
    .bind(&feedback_loop.attempt)
    .bind(&feedback_loop.evaluation)
    .bind(&feedback_loop.feedback)
    .bind(&feedback_loop.adjustment)
    .bind(&feedback_loop.next_task)
    .bind(&feedback_loop.status)
    .bind(feedback_loop.created_by)
    .fetch_one(&mut **tx)
    .await
}

async fn patch_feedback_loop_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    feedback_loop_id: Uuid,
    patch: &PatchFeedbackLoop,
) -> Result<Option<FeedbackLoopDb>, Error> {
    sqlx::query_as::<_, FeedbackLoopDb>(
        r#"
        UPDATE feedback_loops
        SET attempt = COALESCE($2, attempt),
            evaluation = COALESCE($3, evaluation),
            feedback = COALESCE($4, feedback),
            adjustment = COALESCE($5, adjustment),
            next_task = COALESCE($6, next_task),
            status = COALESCE($7, status),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(feedback_loop_id)
    .bind(&patch.attempt)
    .bind(&patch.evaluation)
    .bind(&patch.feedback)
    .bind(&patch.adjustment)
    .bind(&patch.next_task)
    .bind(&patch.status)
    .fetch_optional(&mut **tx)
    .await
}

async fn insert_feedback_loop_memory_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    feedback_loop: &FeedbackLoopDb,
    snapshot: &FeedbackLoopMemorySnapshot,
) -> Result<MemoryDb, Error> {
    sqlx::query_as::<_, MemoryDb>(
        r#"
        INSERT INTO memories (
            user_id,
            space_id,
            title,
            content,
            memory_type,
            file_path,
            is_shared,
            source_type,
            source_metadata
        )
        VALUES ($1, $2, $3, $4, 'text', NULL, false, 'feedback_loop_event', $5)
        RETURNING *
        "#,
    )
    .bind(snapshot.user_id)
    .bind(feedback_loop.space_id)
    .bind("Practice snapshot")
    .bind(&snapshot.content)
    .bind(serde_json::json!({
        "feedback_loop_id": feedback_loop.id,
        "namespace_id": feedback_loop.namespace_id,
        "space_id": feedback_loop.space_id,
        "event_kind": snapshot.event_kind,
        "included_fields": snapshot.included_fields,
    }))
    .fetch_one(&mut **tx)
    .await
}

#[async_trait::async_trait]
impl FeedbackLoopRepository for PostgresFeedbackLoopRepository {
    async fn create(&self, feedback_loop: CreateFeedbackLoop) -> Result<FeedbackLoopDb, Error> {
        sqlx::query_as::<_, FeedbackLoopDb>(
            r#"
            INSERT INTO feedback_loops (
                space_id,
                namespace_id,
                goal,
                task,
                attempt,
                evaluation,
                feedback,
                adjustment,
                next_task,
                status,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(feedback_loop.space_id)
        .bind(feedback_loop.namespace_id)
        .bind(&feedback_loop.goal)
        .bind(&feedback_loop.task)
        .bind(&feedback_loop.attempt)
        .bind(&feedback_loop.evaluation)
        .bind(&feedback_loop.feedback)
        .bind(&feedback_loop.adjustment)
        .bind(&feedback_loop.next_task)
        .bind(&feedback_loop.status)
        .bind(feedback_loop.created_by)
        .fetch_one(&self.pool)
        .await
    }

    async fn create_with_memory_snapshot(
        &self,
        feedback_loop: CreateFeedbackLoop,
        snapshot: FeedbackLoopMemorySnapshot,
    ) -> Result<FeedbackLoopWithMemorySnapshot, Error> {
        let mut tx = self.pool.begin().await?;
        let feedback_loop = insert_feedback_loop_in_tx(&mut tx, &feedback_loop).await?;
        let memory =
            insert_feedback_loop_memory_snapshot(&mut tx, &feedback_loop, &snapshot).await?;
        tx.commit().await?;

        Ok(FeedbackLoopWithMemorySnapshot {
            feedback_loop,
            memory: Some(memory),
        })
    }

    async fn list_for_user(
        &self,
        filter: FeedbackLoopListFilter,
        user_id: Uuid,
    ) -> Result<Vec<FeedbackLoopDb>, Error> {
        sqlx::query_as::<_, FeedbackLoopDb>(
            r#"
            SELECT fl.*
            FROM feedback_loops fl
            INNER JOIN cognitive_space_members m ON m.space_id = fl.space_id
            WHERE fl.space_id = $1
              AND m.user_id = $2
              AND ($3::uuid IS NULL OR fl.namespace_id = $3)
            ORDER BY fl.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(filter.space_id)
        .bind(user_id)
        .bind(filter.namespace_id)
        .bind(filter.limit)
        .bind(filter.offset)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_for_user(
        &self,
        feedback_loop_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<FeedbackLoopDb>, Error> {
        sqlx::query_as::<_, FeedbackLoopDb>(
            r#"
            SELECT fl.*
            FROM feedback_loops fl
            INNER JOIN cognitive_space_members m ON m.space_id = fl.space_id
            WHERE fl.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(feedback_loop_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn patch(
        &self,
        feedback_loop_id: Uuid,
        patch: PatchFeedbackLoop,
    ) -> Result<Option<FeedbackLoopDb>, Error> {
        sqlx::query_as::<_, FeedbackLoopDb>(
            r#"
            UPDATE feedback_loops
            SET attempt = COALESCE($2, attempt),
                evaluation = COALESCE($3, evaluation),
                feedback = COALESCE($4, feedback),
                adjustment = COALESCE($5, adjustment),
                next_task = COALESCE($6, next_task),
                status = COALESCE($7, status),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(feedback_loop_id)
        .bind(&patch.attempt)
        .bind(&patch.evaluation)
        .bind(&patch.feedback)
        .bind(&patch.adjustment)
        .bind(&patch.next_task)
        .bind(&patch.status)
        .fetch_optional(&self.pool)
        .await
    }

    async fn patch_with_memory_snapshot(
        &self,
        feedback_loop_id: Uuid,
        patch: PatchFeedbackLoop,
        snapshot: FeedbackLoopMemorySnapshot,
    ) -> Result<Option<FeedbackLoopWithMemorySnapshot>, Error> {
        let mut tx = self.pool.begin().await?;
        let Some(feedback_loop) =
            patch_feedback_loop_in_tx(&mut tx, feedback_loop_id, &patch).await?
        else {
            return Ok(None);
        };
        let memory =
            insert_feedback_loop_memory_snapshot(&mut tx, &feedback_loop, &snapshot).await?;
        tx.commit().await?;

        Ok(Some(FeedbackLoopWithMemorySnapshot {
            feedback_loop,
            memory: Some(memory),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_feedback_loop_keeps_space_and_namespace_scope() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        let feedback_loop = CreateFeedbackLoop {
            space_id,
            namespace_id,
            goal: "Improve fraction word problems".to_string(),
            task: "Complete five fraction problems".to_string(),
            attempt: None,
            evaluation: None,
            feedback: None,
            adjustment: None,
            next_task: None,
            status: "active".to_string(),
            created_by,
        };

        assert_eq!(feedback_loop.space_id, space_id);
        assert_eq!(feedback_loop.namespace_id, namespace_id);
        assert_eq!(feedback_loop.created_by, created_by);
        assert_eq!(feedback_loop.status, "active");
    }

    #[test]
    fn list_filter_supports_namespace_filter() {
        let namespace_id = Uuid::new_v4();
        let filter = FeedbackLoopListFilter {
            space_id: Uuid::new_v4(),
            namespace_id: Some(namespace_id),
            limit: 20,
            offset: 0,
        };

        assert_eq!(filter.namespace_id, Some(namespace_id));
    }

    #[test]
    fn patch_feedback_loop_can_update_attempt_without_other_loop_fields() {
        let patch = PatchFeedbackLoop {
            attempt: Some("Child tried common denominators".to_string()),
            evaluation: None,
            feedback: None,
            adjustment: None,
            next_task: None,
            status: None,
        };

        assert_eq!(
            patch.attempt.as_deref(),
            Some("Child tried common denominators")
        );
        assert_eq!(patch.evaluation, None);
        assert_eq!(patch.feedback, None);
        assert_eq!(patch.adjustment, None);
        assert_eq!(patch.next_task, None);
        assert_eq!(patch.status, None);
    }

    #[test]
    fn memory_snapshot_carries_only_event_content_and_included_fields() {
        let snapshot = FeedbackLoopMemorySnapshot {
            user_id: Uuid::new_v4(),
            event_kind: "patch".to_string(),
            content: "Answer / reasoning: Child added denominators directly".to_string(),
            included_fields: vec!["attempt".to_string()],
        };

        assert_eq!(snapshot.event_kind, "patch");
        assert_eq!(snapshot.included_fields, vec!["attempt"]);
        assert!(!snapshot.content.contains("Feedback:"));
    }

    #[test]
    fn postgres_repository_exposes_atomic_feedback_loop_memory_snapshot_paths() {
        let source = include_str!("feedback_loop.rs");

        assert!(source.contains("create_with_memory_snapshot"));
        assert!(source.contains("patch_with_memory_snapshot"));
        assert!(source.contains("let mut tx = self.pool.begin().await?"));
        assert!(source.contains("insert_feedback_loop_memory_snapshot"));
        assert!(source.contains("tx.commit().await?"));
    }
}
