//! Namespace database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NamespaceDb {
    pub id: Uuid,
    pub space_id: Uuid,
    pub name: String,
    pub kind: String,
    pub description: Option<String>,
    pub status: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateNamespace {
    pub space_id: Uuid,
    pub name: String,
    pub kind: NamespaceKind,
    pub description: Option<String>,
    pub status: NamespaceStatus,
    pub created_by: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NamespaceKind {
    Reflective,
    Skill,
}

impl NamespaceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reflective => "reflective",
            Self::Skill => "skill",
        }
    }
}

impl std::fmt::Display for NamespaceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NamespaceStatus {
    #[default]
    Active,
    Archived,
}

impl NamespaceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

impl std::fmt::Display for NamespaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[async_trait::async_trait]
pub trait NamespaceRepository: Send + Sync {
    async fn create(&self, namespace: CreateNamespace) -> Result<NamespaceDb, Error>;
    async fn list_for_space(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<NamespaceDb>, Error>;
    async fn find_for_user(
        &self,
        namespace_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<NamespaceDb>, Error>;
}

pub struct PostgresNamespaceRepository {
    pool: PgPool,
}

impl PostgresNamespaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl NamespaceRepository for PostgresNamespaceRepository {
    async fn create(&self, namespace: CreateNamespace) -> Result<NamespaceDb, Error> {
        sqlx::query_as::<_, NamespaceDb>(
            r#"
            INSERT INTO namespaces (
                space_id,
                name,
                kind,
                description,
                status,
                created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(namespace.space_id)
        .bind(&namespace.name)
        .bind(namespace.kind.to_string())
        .bind(&namespace.description)
        .bind(namespace.status.to_string())
        .bind(namespace.created_by)
        .fetch_one(&self.pool)
        .await
    }

    async fn list_for_space(
        &self,
        space_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<NamespaceDb>, Error> {
        sqlx::query_as::<_, NamespaceDb>(
            r#"
            SELECT n.*
            FROM namespaces n
            INNER JOIN cognitive_space_members m ON m.space_id = n.space_id
            WHERE n.space_id = $1 AND m.user_id = $2
            ORDER BY n.created_at ASC
            "#,
        )
        .bind(space_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_for_user(
        &self,
        namespace_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<NamespaceDb>, Error> {
        sqlx::query_as::<_, NamespaceDb>(
            r#"
            SELECT n.*
            FROM namespaces n
            INNER JOIN cognitive_space_members m ON m.space_id = n.space_id
            WHERE n.id = $1 AND m.user_id = $2
            "#,
        )
        .bind(namespace_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_namespace_keeps_space_boundary_and_creator() {
        let space_id = Uuid::new_v4();
        let created_by = Uuid::new_v4();
        let namespace = CreateNamespace {
            space_id,
            name: "personal.thoughts".to_string(),
            kind: NamespaceKind::Reflective,
            description: Some("Reflective Thought Review domain".to_string()),
            status: NamespaceStatus::Active,
            created_by,
        };

        assert_eq!(namespace.space_id, space_id);
        assert_eq!(namespace.created_by, created_by);
        assert_eq!(namespace.kind, NamespaceKind::Reflective);
        assert_eq!(namespace.status, NamespaceStatus::Active);
    }

    #[test]
    fn namespace_kind_serializes_to_storage_value() {
        assert_eq!(NamespaceKind::Reflective.to_string(), "reflective");
        assert_eq!(NamespaceKind::Skill.to_string(), "skill");
    }
}
