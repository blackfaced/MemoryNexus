//! 标签数据库操作

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

/// 标签数据库模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TagDb {
    pub id: Uuid,
    pub name: String,
    pub user_id: Option<Uuid>, // None 表示系统标签
    pub created_at: DateTime<Utc>,
}

/// 创建标签参数
#[derive(Debug, Clone)]
pub struct CreateTag {
    pub name: String,
    pub user_id: Uuid,
}

/// 标签仓储 trait
#[async_trait::async_trait]
pub trait TagRepository: Send + Sync {
    async fn create(&self, name: &str, user_id: Uuid) -> Result<TagDb, Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TagDb>, Error>;
    async fn find_by_name(&self, name: &str, user_id: Uuid) -> Result<Option<TagDb>, Error>;
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TagDb>, Error>;
    async fn update(&self, id: Uuid, name: &str) -> Result<TagDb, Error>;
    async fn delete(&self, id: Uuid) -> Result<bool, Error>;
    async fn count_memories(&self, tag_id: Uuid) -> Result<i64, Error>;

    // 记忆-标签关联
    async fn add_memory_tag(&self, memory_id: Uuid, tag_id: Uuid) -> Result<(), Error>;
    async fn remove_memory_tag(&self, memory_id: Uuid, tag_id: Uuid) -> Result<(), Error>;
    async fn list_memory_tags(&self, memory_id: Uuid) -> Result<Vec<TagDb>, Error>;
}

/// PostgreSQL 标签仓储实现
pub struct PostgresTagRepository {
    pool: PgPool,
}

impl PostgresTagRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TagRepository for PostgresTagRepository {
    async fn create(&self, name: &str, user_id: Uuid) -> Result<TagDb, Error> {
        sqlx::query_as::<_, TagDb>(
            r#"
            INSERT INTO tags (name, user_id)
            VALUES ($1, $2)
            ON CONFLICT (name) WHERE user_id IS NOT NULL
            DO UPDATE SET name = EXCLUDED.name
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<TagDb>, Error> {
        sqlx::query_as::<_, TagDb>("SELECT * FROM tags WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_name(&self, name: &str, user_id: Uuid) -> Result<Option<TagDb>, Error> {
        sqlx::query_as::<_, TagDb>("SELECT * FROM tags WHERE name = $1 AND user_id = $2")
            .bind(name)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TagDb>, Error> {
        sqlx::query_as::<_, TagDb>(
            r#"
            SELECT t.* FROM tags t
            LEFT JOIN memory_tags mt ON t.id = mt.tag_id
            WHERE t.user_id = $1 OR t.user_id IS NULL
            GROUP BY t.id
            ORDER BY COUNT(mt.memory_id) DESC, t.name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn update(&self, id: Uuid, name: &str) -> Result<TagDb, Error> {
        sqlx::query_as::<_, TagDb>(
            r#"
            UPDATE tags SET name = $2 WHERE id = $1 RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .fetch_one(&self.pool)
        .await
    }

    async fn delete(&self, id: Uuid) -> Result<bool, Error> {
        let result = sqlx::query("DELETE FROM tags WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn count_memories(&self, tag_id: Uuid) -> Result<i64, Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM memory_tags WHERE tag_id = $1")
            .bind(tag_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(result.0)
    }

    // 记忆-标签关联
    async fn add_memory_tag(&self, memory_id: Uuid, tag_id: Uuid) -> Result<(), Error> {
        sqlx::query(
            r#"
            INSERT INTO memory_tags (memory_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(memory_id)
        .bind(tag_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_memory_tag(&self, memory_id: Uuid, tag_id: Uuid) -> Result<(), Error> {
        sqlx::query("DELETE FROM memory_tags WHERE memory_id = $1 AND tag_id = $2")
            .bind(memory_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_memory_tags(&self, memory_id: Uuid) -> Result<Vec<TagDb>, Error> {
        sqlx::query_as::<_, TagDb>(
            r#"
            SELECT t.* FROM tags t
            INNER JOIN memory_tags mt ON t.id = mt.tag_id
            WHERE mt.memory_id = $1
            ORDER BY t.name ASC
            "#,
        )
        .bind(memory_id)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tagdb_creation() {
        let tag = TagDb {
            id: Uuid::new_v4(),
            name: "旅行".to_string(),
            user_id: Some(Uuid::new_v4()),
            created_at: Utc::now(),
        };

        assert!(!tag.name.is_empty());
        assert!(tag.user_id.is_some());
    }

    #[test]
    fn test_system_tag() {
        // 系统标签 (user_id = None)
        let system_tag = TagDb {
            id: Uuid::new_v4(),
            name: "important".to_string(),
            user_id: None,
            created_at: Utc::now(),
        };

        assert!(system_tag.user_id.is_none());
    }
}
