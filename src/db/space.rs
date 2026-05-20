//! Cognitive Space database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CognitiveSpaceDb {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: Uuid,
    pub default_lens_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCognitiveSpace {
    pub name: String,
    pub description: Option<String>,
    pub owner_user_id: Uuid,
}

#[async_trait::async_trait]
pub trait CognitiveSpaceRepository: Send + Sync {
    async fn create(&self, space: CreateCognitiveSpace) -> Result<CognitiveSpaceDb, Error>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<CognitiveSpaceDb>, Error>;
    async fn find_for_user(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceDb>, Error>;
    async fn default_for_user(&self, user_id: Uuid) -> Result<Option<CognitiveSpaceDb>, Error>;
    async fn ensure_default_for_user(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> Result<CognitiveSpaceDb, Error>;
}

pub struct PostgresCognitiveSpaceRepository {
    pool: PgPool,
}

impl PostgresCognitiveSpaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CognitiveSpaceRepository for PostgresCognitiveSpaceRepository {
    async fn create(&self, space: CreateCognitiveSpace) -> Result<CognitiveSpaceDb, Error> {
        let mut tx = self.pool.begin().await?;

        let created = sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            INSERT INTO cognitive_spaces (name, description, owner_user_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(&space.name)
        .bind(&space.description)
        .bind(space.owner_user_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO cognitive_space_members (space_id, user_id, role)
            VALUES ($1, $2, 'owner')
            ON CONFLICT (space_id, user_id) DO NOTHING
            "#,
        )
        .bind(created.id)
        .bind(space.owner_user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(created)
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<CognitiveSpaceDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            SELECT s.*
            FROM cognitive_spaces s
            INNER JOIN cognitive_space_members m ON m.space_id = s.id
            WHERE m.user_id = $1
            ORDER BY s.created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_for_user(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CognitiveSpaceDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            SELECT s.*
            FROM cognitive_spaces s
            INNER JOIN cognitive_space_members m ON m.space_id = s.id
            WHERE s.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn default_for_user(&self, user_id: Uuid) -> Result<Option<CognitiveSpaceDb>, Error> {
        sqlx::query_as::<_, CognitiveSpaceDb>(
            r#"
            SELECT s.*
            FROM cognitive_spaces s
            INNER JOIN cognitive_space_members m ON m.space_id = s.id
            WHERE m.user_id = $1
            ORDER BY
                CASE WHEN s.owner_user_id = $1 THEN 0 ELSE 1 END,
                s.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn ensure_default_for_user(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> Result<CognitiveSpaceDb, Error> {
        if let Some(space) = self.default_for_user(user_id).await? {
            return Ok(space);
        }

        self.create(CreateCognitiveSpace {
            name: format!("{username} Personal Space"),
            description: Some("Default personal cognitive space".to_string()),
            owner_user_id: user_id,
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_cognitive_space_keeps_owner_boundary() {
        let owner_user_id = Uuid::new_v4();
        let space = CreateCognitiveSpace {
            name: "Personal Space".to_string(),
            description: Some("Private cognitive space".to_string()),
            owner_user_id,
        };

        assert_eq!(space.owner_user_id, owner_user_id);
        assert_eq!(space.name, "Personal Space");
    }
}
