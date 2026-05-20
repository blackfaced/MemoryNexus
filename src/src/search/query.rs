//! 搜索查询模块

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ai::{Embedder, EmbeddingError, OpenAIEmbedder};
use crate::db::memory::MemoryDb;
use crate::vector::{VectorError, VectorStore};

/// 搜索查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    /// Cognitive Space boundary
    pub space_id: Option<Uuid>,

    /// 搜索关键词
    pub q: Option<String>,

    /// 标签列表
    pub tags: Option<Vec<String>>,

    /// 记忆类型
    pub memory_type: Option<String>,

    /// 日期范围
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,

    /// 是否只搜索自己的记忆
    #[serde(default)]
    pub own_only: bool,

    /// 是否启用语义搜索
    #[serde(default)]
    pub semantic: bool,

    /// 分页
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            q: None,
            space_id: None,
            tags: None,
            memory_type: None,
            from: None,
            to: None,
            own_only: false,
            semantic: false,
            limit: 20,
            offset: 0,
        }
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub items: Vec<MemorySearchItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub query: Option<String>,
    pub search_mode: String,
}

/// 单个记忆搜索项（带相关性得分）
#[derive(Debug, Clone, Serialize)]
pub struct MemorySearchItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub is_shared: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub relevance: Option<f32>,          // 相关性得分
    pub matched_on: Option<Vec<String>>, // 匹配字段
}

impl From<MemoryDb> for MemorySearchItem {
    fn from(m: MemoryDb) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            space_id: m.space_id,
            title: m.title,
            content: m.content,
            memory_type: m.memory_type,
            is_shared: m.is_shared,
            created_at: m.created_at,
            updated_at: m.updated_at,
            relevance: None,
            matched_on: None,
        }
    }
}

/// 搜索引擎
pub struct SearchEngine {
    pool: PgPool,
    vector_store: Option<Arc<dyn VectorStore>>,
}

impl SearchEngine {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            vector_store: None,
        }
    }

    pub fn with_vector_store(pool: PgPool, vector_store: Option<Arc<dyn VectorStore>>) -> Self {
        Self { pool, vector_store }
    }

    /// 执行搜索
    pub async fn search(
        &self,
        query: &SearchQuery,
        user_id: Uuid,
        space_id: Uuid,
    ) -> Result<SearchResult, sqlx::Error> {
        // 构建 SQL 查询
        let mut sql = String::from(
            r#"
            SELECT DISTINCT m.*, 
            CASE WHEN $2::text <> ''
                THEN ts_rank(to_tsvector('simple', coalesce(title, '') || ' ' || content), plainto_tsquery('simple', $2))
                ELSE 0
            END as rank
            FROM memories m
            LEFT JOIN memory_tags mt ON m.id = mt.memory_id
            LEFT JOIN tags t ON mt.tag_id = t.id
            WHERE 1=1
            AND m.space_id = $3::uuid
            "#,
        );

        let mut params: Vec<String> = Vec::new();
        let keyword = query.q.clone().unwrap_or_default();
        let mut param_idx = 4;

        // 关键词搜索
        if let Some(ref q) = query.q {
            if !q.is_empty() {
                sql.push_str(&format!(
                    " AND (title ILIKE ${} OR content ILIKE ${} OR to_tsvector('simple', coalesce(title, '') || ' ' || content) @@ plainto_tsquery('simple', $2))",
                    param_idx, param_idx
                ));
                params.push(format!("%{}%", q));
                param_idx += 1;
            }
        }

        // 标签过滤
        if let Some(ref tags) = query.tags {
            if !tags.is_empty() {
                let tag_placeholders: Vec<String> = tags
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("${}", param_idx + i))
                    .collect();
                sql.push_str(&format!(
                    " AND t.name = ANY(ARRAY[{}]::text[])",
                    tag_placeholders.join(", ")
                ));
                for tag in tags {
                    params.push(tag.clone());
                }
                param_idx += tags.len();
            }
        }

        // 记忆类型过滤
        if let Some(ref memory_type) = query.memory_type {
            params.push(memory_type.clone());
            sql.push_str(&format!(" AND m.memory_type = ${}", param_idx));
            param_idx += 1;
        }

        // 日期范围过滤
        if let Some(ref from) = query.from {
            params.push(from.to_rfc3339());
            sql.push_str(&format!(" AND m.created_at >= ${}", param_idx));
            param_idx += 1;
        }

        if let Some(ref to) = query.to {
            params.push(to.to_rfc3339());
            sql.push_str(&format!(" AND m.created_at <= ${}", param_idx));
            param_idx += 1;
        }

        // 所有权过滤
        if query.own_only {
            sql.push_str(" AND m.user_id = $1");
        } else {
            sql.push_str(
                r#"
                AND (
                    m.user_id = $1
                    OR m.is_shared = true
                    OR EXISTS (
                        SELECT 1 FROM cognitive_space_members csm
                        WHERE csm.space_id = m.space_id AND csm.user_id = $1
                    )
                )
                "#,
            );
        }

        // 排序
        sql.push_str(" ORDER BY rank DESC NULLS LAST, m.created_at DESC");

        // 分页
        params.push(query.limit.to_string());
        sql.push_str(&format!(" LIMIT ${}::bigint", param_idx));
        param_idx += 1;

        params.push(query.offset.to_string());
        sql.push_str(&format!(" OFFSET ${}::bigint", param_idx));

        // 构建绑定参数
        let mut builder = sqlx::query_as::<_, MemoryDb>(&sql);
        builder = builder.bind(user_id); // $1
        builder = builder.bind(keyword); // $2
        builder = builder.bind(space_id); // $3

        for param in &params {
            builder = builder.bind(param);
        }

        let items = builder.fetch_all(&self.pool).await?;
        let total = items.len() as i64;

        Ok(SearchResult {
            items: items.into_iter().map(MemorySearchItem::from).collect(),
            total,
            limit: query.limit,
            offset: query.offset,
            query: query.q.clone(),
            search_mode: "keyword".to_string(),
        })
    }

    /// 执行语义搜索：Embedding -> Qdrant -> PostgreSQL 记忆详情
    pub async fn semantic_search(
        &self,
        query: &SearchQuery,
        user_id: Uuid,
        space_id: Uuid,
    ) -> Result<SearchResult, SemanticSearchError> {
        let text = query
            .q
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .ok_or(SemanticSearchError::EmptyQuery)?;

        let vector_store = self
            .vector_store
            .as_ref()
            .ok_or(SemanticSearchError::VectorStoreMissing)?;

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| SemanticSearchError::EmbeddingKeyMissing)?;
        let model = std::env::var("OPENAI_EMBEDDING_MODEL")
            .or_else(|_| std::env::var("EMBEDDING_MODEL"))
            .unwrap_or_else(|_| "text-embedding-ada-002".to_string());
        let embedder = OpenAIEmbedder::new(api_key).with_model(model);
        let embedding = embedder.embed(text).await?;

        let matches = vector_store
            .search_memories(space_id, user_id, embedding.embedding, query.limit as usize)
            .await?;

        if matches.is_empty() {
            return Ok(SearchResult {
                items: vec![],
                total: 0,
                limit: query.limit,
                offset: query.offset,
                query: query.q.clone(),
                search_mode: "semantic".to_string(),
            });
        }

        let ids: Vec<Uuid> = matches.iter().map(|m| m.memory_id).collect();
        let memories = sqlx::query_as::<_, MemoryDb>(
            r#"
            SELECT * FROM memories
            WHERE id = ANY($1)
              AND space_id = $3
              AND (
                user_id = $2
                OR is_shared = true
                OR EXISTS (
                    SELECT 1 FROM cognitive_space_members csm
                    WHERE csm.space_id = memories.space_id AND csm.user_id = $2
                )
              )
            "#,
        )
        .bind(&ids)
        .bind(user_id)
        .bind(space_id)
        .fetch_all(&self.pool)
        .await?;

        let mut by_id: HashMap<Uuid, MemoryDb> = memories
            .into_iter()
            .map(|memory| (memory.id, memory))
            .collect();
        let mut items = Vec::with_capacity(matches.len());

        for vector_match in matches {
            if let Some(memory) = by_id.remove(&vector_match.memory_id) {
                let mut item = MemorySearchItem::from(memory);
                item.relevance = Some(vector_match.score);
                item.matched_on = Some(vec!["semantic".to_string()]);
                items.push(item);
            }
        }

        Ok(SearchResult {
            total: items.len() as i64,
            items,
            limit: query.limit,
            offset: query.offset,
            query: query.q.clone(),
            search_mode: "semantic".to_string(),
        })
    }

    /// 全文搜索建议（简单实现）
    pub async fn suggest(
        &self,
        prefix: &str,
        user_id: Uuid,
        space_id: Uuid,
    ) -> Result<Vec<String>, sqlx::Error> {
        let suggestions = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT title FROM memories
            WHERE space_id = $3
              AND user_id = $1
              AND title ILIKE $2
              AND title IS NOT NULL
            ORDER BY created_at DESC
            LIMIT 10
            "#,
        )
        .bind(user_id)
        .bind(format!("{}%", prefix))
        .bind(space_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(suggestions)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SemanticSearchError {
    #[error("语义搜索需要非空查询")]
    EmptyQuery,

    #[error("Qdrant 向量存储未配置")]
    VectorStoreMissing,

    #[error("OPENAI_API_KEY 未配置")]
    EmbeddingKeyMissing,

    #[error("Embedding 失败: {0}")]
    Embedding(#[from] EmbeddingError),

    #[error("向量检索失败: {0}")]
    Vector(#[from] VectorError),

    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_default() {
        let query = SearchQuery::default();
        assert!(query.q.is_none());
        assert!(query.tags.is_none());
        assert_eq!(query.limit, 20);
        assert!(!query.semantic);
    }

    #[test]
    fn test_search_query_deserialize() {
        let json = r#"{"q":"旅行","limit":10,"own_only":true}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, Some("旅行".to_string()));
        assert_eq!(query.limit, 10);
        assert!(query.own_only);
    }

    #[test]
    fn test_search_query_semantic_deserialize() {
        let json = r#"{"q":"外婆做饭","semantic":true}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();

        assert!(query.semantic);
        assert_eq!(query.q, Some("外婆做饭".to_string()));
    }

    #[test]
    fn test_search_query_space_deserialize() {
        let space_id = Uuid::new_v4();
        let json = format!(r#"{{"q":"Rust","space_id":"{}"}}"#, space_id);
        let query: SearchQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(query.space_id, Some(space_id));
    }

    #[test]
    fn test_memory_search_item_from_memorydb() {
        let memory = MemoryDb {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            title: Some("Test".to_string()),
            content: "Content".to_string(),
            memory_type: "text".to_string(),
            file_path: None,
            thumbnail_path: None,
            is_shared: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let item: MemorySearchItem = memory.into();
        assert!(item.title.is_some());
        assert_eq!(item.title.unwrap(), "Test");
    }

    #[test]
    fn test_search_result_serde() {
        let result = SearchResult {
            items: vec![],
            total: 0,
            limit: 20,
            offset: 0,
            query: Some("test".to_string()),
            search_mode: "keyword".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"query\":\"test\""));
    }
}
