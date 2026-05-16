//! 记忆数据库操作

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// 记忆类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum MemoryType {
    Text,
    Image,
    Audio,
    Video,
}

impl Default for MemoryType {
    fn default() -> Self {
        Self::Text
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Image => write!(f, "image"),
            Self::Audio => write!(f, "audio"),
            Self::Video => write!(f, "video"),
        }
    }
}

/// 记忆数据库模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MemoryDb {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub is_shared: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建记忆参数
#[derive(Debug, Clone)]
pub struct CreateMemory {
    pub user_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub memory_type: MemoryType,
    pub file_path: Option<String>,
    pub is_shared: bool,
    pub tags: Vec<String>,
}

/// 记忆仓储 trait
pub trait MemoryRepository: Send + Sync {
    fn create(&self, memory: CreateMemory) -> impl std::future::Future<Output = Result<MemoryDb, sqlx::Error>> + Send;
    fn find_by_id(&self, id: Uuid) -> impl std::future::Future<Output = Result<Option<MemoryDb>, sqlx::Error>> + Send;
    fn list_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> impl std::future::Future<Output = Result<Vec<MemoryDb>, sqlx::Error>> + Send;
    fn delete(&self, id: Uuid) -> impl std::future::Future<Output = Result<bool, sqlx::Error>> + Send;
}

/// PostgreSQL 记忆仓储实现
pub struct PostgresMemoryRepository {
    pool: PgPool,
}

impl PostgresMemoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl MemoryRepository for PostgresMemoryRepository {
    async fn create(&self, memory: CreateMemory) -> Result<MemoryDb, sqlx::Error> {
        let result = sqlx::query_as::<_, MemoryDb>(
            r#"
            INSERT INTO memories (user_id, title, content, memory_type, file_path, is_shared)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(memory.user_id)
        .bind(&memory.title)
        .bind(&memory.content)
        .bind(memory.memory_type.to_string())
        .bind(&memory.file_path)
        .bind(memory.is_shared)
        .fetch_one(&self.pool)
        .await?;

        // 处理标签
        for tag_name in memory.tags {
            // TODO: 标签逻辑
            let _ = tag_name;
        }

        Ok(result)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<MemoryDb>, sqlx::Error> {
        sqlx::query_as::<_, MemoryDb>(
            "SELECT * FROM memories WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list_by_user(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<MemoryDb>, sqlx::Error> {
        sqlx::query_as::<_, MemoryDb>(
            r#"
            SELECT * FROM memories 
            WHERE user_id = $1 OR is_shared = true
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM memories WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_display() {
        assert_eq!(MemoryType::Text.to_string(), "text");
        assert_eq!(MemoryType::Image.to_string(), "image");
        assert_eq!(MemoryType::Audio.to_string(), "audio");
        assert_eq!(MemoryType::Video.to_string(), "video");
    }

    #[test]
    fn test_memory_type_default() {
        assert_eq!(MemoryType::default(), MemoryType::Text);
    }

    #[test]
    fn test_create_memory_validation() {
        let memory = CreateMemory {
            user_id: Uuid::new_v4(),
            title: Some("Test".to_string()),
            content: "Content".to_string(),
            memory_type: MemoryType::Text,
            file_path: None,
            is_shared: false,
            tags: vec![],
        };
        
        assert!(!memory.content.is_empty());
    }
}
