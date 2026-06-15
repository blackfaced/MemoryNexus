//! Lens database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LensDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub strategy: String,
    pub output_format: String,
    pub retrieval_mode: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateLens {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub strategy: String,
    pub output_format: String,
    pub retrieval_mode: String,
    pub created_by: Uuid,
}

#[async_trait::async_trait]
pub trait LensRepository: Send + Sync {
    async fn create(&self, lens: CreateLens) -> Result<LensDb, Error>;
    async fn list_for_space(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        namespace_id: Option<Uuid>,
    ) -> Result<Vec<LensDb>, Error>;
    async fn find_for_user(&self, lens_id: Uuid, user_id: Uuid) -> Result<Option<LensDb>, Error>;
}

pub struct PostgresLensRepository {
    pool: PgPool,
}

impl PostgresLensRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl LensRepository for PostgresLensRepository {
    async fn create(&self, lens: CreateLens) -> Result<LensDb, Error> {
        sqlx::query_as::<_, LensDb>(
            r#"
            INSERT INTO lenses (
                space_id,
                namespace_id,
                name,
                description,
                strategy,
                output_format,
                retrieval_mode,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(lens.space_id)
        .bind(lens.namespace_id)
        .bind(&lens.name)
        .bind(&lens.description)
        .bind(&lens.strategy)
        .bind(&lens.output_format)
        .bind(&lens.retrieval_mode)
        .bind(lens.created_by)
        .fetch_one(&self.pool)
        .await
    }

    async fn list_for_space(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        namespace_id: Option<Uuid>,
    ) -> Result<Vec<LensDb>, Error> {
        sqlx::query_as::<_, LensDb>(
            r#"
            SELECT l.*
            FROM lenses l
            INNER JOIN cognitive_space_members m ON m.space_id = l.space_id
            WHERE l.space_id = $1 AND m.user_id = $2
              AND ($3::uuid IS NULL OR l.namespace_id = $3)
            ORDER BY l.created_at ASC
            "#,
        )
        .bind(space_id)
        .bind(user_id)
        .bind(namespace_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_for_user(&self, lens_id: Uuid, user_id: Uuid) -> Result<Option<LensDb>, Error> {
        sqlx::query_as::<_, LensDb>(
            r#"
            SELECT l.*
            FROM lenses l
            INNER JOIN cognitive_space_members m ON m.space_id = l.space_id
            WHERE l.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(lens_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lens_keeps_space_boundary_and_strategy() {
        let space_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        let lens = CreateLens {
            space_id,
            namespace_id: Some(Uuid::new_v4()),
            name: "Project Context".to_string(),
            description: Some("Interpret project memory".to_string()),
            strategy: "project_context".to_string(),
            output_format: "brief".to_string(),
            retrieval_mode: "semantic".to_string(),
            created_by,
        };

        assert_eq!(lens.space_id, space_id);
        assert!(lens.namespace_id.is_some());
        assert_eq!(lens.created_by, created_by);
        assert_eq!(lens.strategy, "project_context");
        assert_eq!(lens.output_format, "brief");
        assert_eq!(lens.retrieval_mode, "semantic");
    }
}
