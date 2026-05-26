//! 记忆数据库操作

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// 记忆类型枚举
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum MemoryType {
    /// 文本
    #[default]
    Text,
    /// 图片
    Image,
    /// 音频
    Audio,
    /// 视频
    Video,
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
    pub space_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub is_shared: bool,
    pub source_type: String,
    pub source_metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建记忆参数
#[derive(Debug, Clone)]
pub struct CreateMemory {
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub memory_type: MemoryType,
    pub file_path: Option<String>,
    pub is_shared: bool,
    pub source_type: String,
    pub source_metadata: Value,
    pub tags: Vec<String>,
}

/// 更新记忆参数
#[derive(Debug, Clone)]
pub struct UpdateMemory {
    pub title: Option<String>,
    pub content: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub is_shared: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// 记忆仓储 trait
#[async_trait::async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn create(&self, memory: CreateMemory) -> Result<MemoryDb, sqlx::Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MemoryDb>, sqlx::Error>;
    async fn list_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MemoryDb>, sqlx::Error>;
    async fn count_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error>;
    async fn list_by_space(
        &self,
        user_id: Uuid,
        space_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MemoryDb>, sqlx::Error>;
    async fn list_by_space_window(
        &self,
        user_id: Uuid,
        space_id: Uuid,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<MemoryDb>, sqlx::Error>;
    async fn count_by_space(&self, user_id: Uuid, space_id: Uuid) -> Result<i64, sqlx::Error>;
    async fn update(&self, id: Uuid, update: UpdateMemory) -> Result<MemoryDb, sqlx::Error>;
    async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
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

fn normalize_tag_names(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() || !seen.insert(tag.to_string()) {
            continue;
        }
        normalized.push(tag.to_string());
    }

    normalized
}

fn update_memory_sql() -> &'static str {
    r#"
            UPDATE memories
            SET content = COALESCE($2, content),
                title = CASE WHEN $3 IS NULL THEN title ELSE NULLIF($3, '') END,
                memory_type = COALESCE($4, memory_type),
                is_shared = COALESCE($5, is_shared),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
}

#[async_trait::async_trait]
impl MemoryRepository for PostgresMemoryRepository {
    async fn create(&self, memory: CreateMemory) -> Result<MemoryDb, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query_as::<_, MemoryDb>(
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(memory.user_id)
        .bind(memory.space_id)
        .bind(&memory.title)
        .bind(&memory.content)
        .bind(memory.memory_type.to_string())
        .bind(&memory.file_path)
        .bind(memory.is_shared)
        .bind(&memory.source_type)
        .bind(&memory.source_metadata)
        .fetch_one(&mut *tx)
        .await?;

        for tag_name in normalize_tag_names(memory.tags) {
            let (tag_id,): (Uuid,) = sqlx::query_as(
                r#"
                INSERT INTO tags (name, user_id)
                VALUES ($1, $2)
                ON CONFLICT (user_id, name) WHERE user_id IS NOT NULL
                DO UPDATE SET name = EXCLUDED.name
                RETURNING id
                "#,
            )
            .bind(&tag_name)
            .bind(memory.user_id)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO memory_tags (memory_id, tag_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(result.id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(result)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<MemoryDb>, sqlx::Error> {
        sqlx::query_as::<_, MemoryDb>("SELECT * FROM memories WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn list_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MemoryDb>, sqlx::Error> {
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

    async fn count_by_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM memories 
            WHERE user_id = $1 OR is_shared = true
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.0)
    }

    async fn list_by_space(
        &self,
        user_id: Uuid,
        space_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<MemoryDb>, sqlx::Error> {
        sqlx::query_as::<_, MemoryDb>(
            r#"
            SELECT * FROM memories
            WHERE space_id = $2
              AND (
                user_id = $1
                OR is_shared = true
                OR EXISTS (
                    SELECT 1 FROM cognitive_space_members
                    WHERE cognitive_space_members.space_id = memories.space_id
                      AND cognitive_space_members.user_id = $1
                )
              )
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(space_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    async fn count_by_space(&self, user_id: Uuid, space_id: Uuid) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM memories
            WHERE space_id = $2
              AND (
                user_id = $1
                OR is_shared = true
                OR EXISTS (
                    SELECT 1 FROM cognitive_space_members
                    WHERE cognitive_space_members.space_id = memories.space_id
                      AND cognitive_space_members.user_id = $1
                )
              )
            "#,
        )
        .bind(user_id)
        .bind(space_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.0)
    }

    async fn list_by_space_window(
        &self,
        user_id: Uuid,
        space_id: Uuid,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<MemoryDb>, sqlx::Error> {
        sqlx::query_as::<_, MemoryDb>(
            r#"
            SELECT * FROM memories
            WHERE space_id = $2
              AND created_at >= $3
              AND created_at < $4
              AND (
                user_id = $1
                OR is_shared = true
                OR EXISTS (
                    SELECT 1 FROM cognitive_space_members
                    WHERE cognitive_space_members.space_id = memories.space_id
                      AND cognitive_space_members.user_id = $1
                )
              )
            ORDER BY created_at DESC
            LIMIT $5
            "#,
        )
        .bind(user_id)
        .bind(space_id)
        .bind(window_start)
        .bind(window_end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    async fn update(&self, id: Uuid, update: UpdateMemory) -> Result<MemoryDb, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let memory_type = update
            .memory_type
            .map(|memory_type| memory_type.to_string());

        let memory = sqlx::query_as::<_, MemoryDb>(update_memory_sql())
            .bind(id)
            .bind(&update.content)
            .bind(&update.title)
            .bind(&memory_type)
            .bind(update.is_shared)
            .fetch_one(&mut *tx)
            .await?;

        if let Some(tags) = update.tags {
            sqlx::query("DELETE FROM memory_tags WHERE memory_id = $1")
                .bind(memory.id)
                .execute(&mut *tx)
                .await?;

            for tag_name in normalize_tag_names(tags) {
                let (tag_id,): (Uuid,) = sqlx::query_as(
                    r#"
                    INSERT INTO tags (name, user_id)
                    VALUES ($1, $2)
                    ON CONFLICT (user_id, name) WHERE user_id IS NOT NULL
                    DO UPDATE SET name = EXCLUDED.name
                    RETURNING id
                    "#,
                )
                .bind(&tag_name)
                .bind(memory.user_id)
                .fetch_one(&mut *tx)
                .await?;

                sqlx::query(
                    r#"
                    INSERT INTO memory_tags (memory_id, tag_id)
                    VALUES ($1, $2)
                    ON CONFLICT DO NOTHING
                    "#,
                )
                .bind(memory.id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(memory)
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
            space_id: Uuid::new_v4(),
            title: Some("Test".to_string()),
            content: "Content".to_string(),
            memory_type: MemoryType::Text,
            file_path: None,
            is_shared: false,
            source_type: "manual".to_string(),
            source_metadata: serde_json::json!({}),
            tags: vec![],
        };

        assert!(!memory.content.is_empty());
    }

    #[test]
    fn normalize_tag_names_trims_deduplicates_and_drops_empty_values() {
        let tags = normalize_tag_names(vec![
            " rust ".to_string(),
            "".to_string(),
            "lens".to_string(),
            "rust".to_string(),
        ]);

        assert_eq!(tags, vec!["rust".to_string(), "lens".to_string()]);
    }

    #[test]
    fn test_update_memory_struct() {
        let update = UpdateMemory {
            title: Some("New Title".to_string()),
            content: Some("New Content".to_string()),
            memory_type: Some(MemoryType::Image),
            is_shared: Some(true),
            tags: Some(vec!["review".to_string()]),
        };

        assert!(update.title.is_some());
        assert!(update.content.is_some());
        assert!(update.memory_type.is_some());
        assert_eq!(update.tags.as_deref(), Some(&["review".to_string()][..]));
    }

    #[test]
    fn update_memory_sql_clears_empty_title_but_preserves_absent_title() {
        assert!(update_memory_sql()
            .contains("title = CASE WHEN $3 IS NULL THEN title ELSE NULLIF($3, '') END"));
    }
}
