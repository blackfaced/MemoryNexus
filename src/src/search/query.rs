//! 搜索查询模块

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::memory::MemoryDb;
use super::filter::{MemoryFilter, SortOrder};

/// 搜索查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
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
            tags: None,
            memory_type: None,
            from: None,
            to: None,
            own_only: false,
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
}

/// 单个记忆搜索项（带相关性得分）
#[derive(Debug, Clone, Serialize)]
pub struct MemorySearchItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub is_shared: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub relevance: Option<f32>,  // 相关性得分
    pub matched_on: Option<Vec<String>>,  // 匹配字段
}

impl From<MemoryDb> for MemorySearchItem {
    fn from(m: MemoryDb) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
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
}

impl SearchEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// 执行搜索
    pub async fn search(&self, query: &SearchQuery, user_id: Uuid) -> Result<SearchResult, sqlx::Error> {
        // 构建 SQL 查询
        let mut sql = String::from(
            r#"
            SELECT DISTINCT m.*, 
            ts_rank(to_tsvector('simple', coalesce(title, '') || ' ' || content), plainto_tsquery('simple', $1)) as rank
            FROM memories m
            LEFT JOIN memory_tags mt ON m.id = mt.memory_id
            LEFT JOIN tags t ON mt.tag_id = t.id
            WHERE 1=1
            "#
        );
        
        let mut params: Vec<String> = Vec::new();
        let mut param_idx = 2;
        
        // 关键词搜索
        if let Some(ref q) = query.q {
            if !q.is_empty() {
                params.push(q.clone());
                sql.push_str(&format!(
                    " AND (title ILIKE ${} OR content ILIKE ${} OR to_tsvector('simple', coalesce(title, '') || ' ' || content) @@ plainto_tsquery('simple', ${}))",
                    param_idx, param_idx, param_idx
                ));
                param_idx += 1;
            }
        }
        
        // 标签过滤
        if let Some(ref tags) = query.tags {
            if !tags.is_empty() {
                let tag_placeholders: Vec<String> = tags.iter()
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
            sql.push_str(" AND (m.user_id = $1 OR m.is_shared = true)");
        }
        
        // 排序
        sql.push_str(" ORDER BY rank DESC NULLS LAST, m.created_at DESC");
        
        // 分页
        params.push(query.limit.to_string());
        sql.push_str(&format!(" LIMIT ${}", param_idx));
        param_idx += 1;
        
        params.push(query.offset.to_string());
        sql.push_str(&format!(" OFFSET ${}", param_idx));
        
        // 构建绑定参数
        let mut builder = sqlx::query_as::<_, MemoryDb>(&sql);
        builder = builder.bind(&user_id.to_string()); // $1
        
        for param in &params {
            builder = builder.bind(param);
        }
        
        let items = builder.fetch_all(&self.pool).await?;
        
        // 统计总数
        let count_sql = sql.replace("SELECT DISTINCT m.*,", "SELECT COUNT(DISTINCT m.id)")
            .replace(&format!(" LIMIT ${}", param_idx - 1), "")
            .replace(&format!(" OFFSET ${}", param_idx), "");
        
        let total: (i64,) = sqlx::query_as(&count_sql)
            .bind(&user_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        
        Ok(SearchResult {
            items: items.into_iter().map(MemorySearchItem::from).collect(),
            total: total.0,
            limit: query.limit,
            offset: query.offset,
            query: query.q.clone(),
        })
    }
    
    /// 全文搜索建议（简单实现）
    pub async fn suggest(&self, prefix: &str, user_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let suggestions = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT title FROM memories
            WHERE user_id = $1 AND title ILIKE $2 AND title IS NOT NULL
            ORDER BY created_at DESC
            LIMIT 10
            "#
        )
        .bind(user_id)
        .bind(format!("{}%", prefix))
        .fetch_all(&self.pool)
        .await?;
        
        Ok(suggestions)
    }
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
    fn test_memory_search_item_from_memorydb() {
        let memory = MemoryDb {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
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
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"query\":\"test\""));
    }
}
