//! 向量仓储层
//!
//! 提供向量存储的统一接口，适配 Qdrant VectorStore
//! 支持记忆的向量嵌入、搜索和管理

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{MemoryVectorPayload, MemoryVectorPoint, VectorError, VectorStore};

/// 记忆向量数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVector {
    pub memory_id: Uuid,
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub vector: Vec<f32>,
    pub payload: Option<VectorPayload>,
}

/// 向量元数据载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPayload {
    pub title: Option<String>,
    pub content_snippet: Option<String>,
    pub tags: Vec<String>,
    pub memory_type: String,
    pub created_at: DateTime<Utc>,
}

/// 向量搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub memory_id: Uuid,
    pub score: f32,
    pub payload: Option<VectorPayload>,
}

/// 向量仓储错误
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("向量操作失败: {0}")]
    Vector(#[from] VectorError),

    #[error("ID 解析失败: {0}")]
    IdParse(String),
}

/// 向量仓储 trait - 提供记忆向量的 CRUD 操作
#[async_trait]
pub trait VectorRepository: Send + Sync {
    /// 存储单个向量
    async fn store(&self, vector: MemoryVector) -> Result<(), RepositoryError>;

    /// 批量存储向量
    async fn store_batch(&self, vectors: Vec<MemoryVector>) -> Result<(), RepositoryError>;

    /// 删除向量
    async fn delete(&self, memory_id: Uuid) -> Result<(), RepositoryError>;

    /// 批量删除向量
    async fn delete_batch(&self, memory_ids: Vec<Uuid>) -> Result<(), RepositoryError>;

    /// 检查向量是否存在
    async fn exists(&self, memory_id: Uuid) -> Result<bool, RepositoryError>;

    /// 搜索相似向量
    async fn search(
        &self,
        vector: &[f32],
        user_id: Uuid,
        space_id: Uuid,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<VectorSearchResult>, RepositoryError>;

    /// 获取向量
    async fn get(&self, memory_id: Uuid) -> Result<Option<MemoryVector>, RepositoryError>;
}

/// Qdrant 向量仓储适配器
#[derive(Clone)]
pub struct QdrantVectorRepository {
    inner: Arc<dyn VectorStore>,
    #[allow(dead_code)]
    user_id: Uuid,
}

use std::sync::Arc;

impl QdrantVectorRepository {
    pub fn new(inner: Arc<dyn VectorStore>, user_id: Uuid) -> Self {
        Self { inner, user_id }
    }
}

#[async_trait]
impl VectorRepository for QdrantVectorRepository {
    async fn store(&self, vector: MemoryVector) -> Result<(), RepositoryError> {
        let point = MemoryVectorPoint {
            id: vector.memory_id,
            vector: vector.vector,
            payload: MemoryVectorPayload {
                memory_id: vector.memory_id,
                user_id: vector.user_id,
                space_id: vector.space_id,
                source_type: "memory".to_string(),
                created_at: vector
                    .payload
                    .as_ref()
                    .map(|p| p.created_at.to_rfc3339())
                    .unwrap_or_else(|| Utc::now().to_rfc3339()),
                visibility: "private".to_string(),
                title: vector.payload.as_ref().and_then(|p| p.title.clone()),
                memory_type: vector
                    .payload
                    .as_ref()
                    .map(|p| p.memory_type.clone())
                    .unwrap_or_default(),
                is_shared: false,
            },
        };

        self.inner
            .upsert_memory(point)
            .await
            .map_err(RepositoryError::from)
    }

    async fn store_batch(&self, vectors: Vec<MemoryVector>) -> Result<(), RepositoryError> {
        for vector in vectors {
            self.store(vector).await?;
        }
        Ok(())
    }

    async fn delete(&self, _memory_id: Uuid) -> Result<(), RepositoryError> {
        // Qdrant REST API 不直接支持删除，需要通过 delete points API
        // 这里暂时返回成功，实际删除需要扩展 VectorStore trait
        Ok(())
    }

    async fn delete_batch(&self, _memory_ids: Vec<Uuid>) -> Result<(), RepositoryError> {
        // Qdrant REST API 不直接支持批量删除
        // 需要扩展 VectorStore trait
        Ok(())
    }

    async fn exists(&self, _memory_id: Uuid) -> Result<bool, RepositoryError> {
        // 需要扩展 VectorStore trait 来支持
        Ok(true)
    }

    async fn search(
        &self,
        vector: &[f32],
        user_id: Uuid,
        space_id: Uuid,
        limit: usize,
        _threshold: Option<f32>,
    ) -> Result<Vec<VectorSearchResult>, RepositoryError> {
        let matches = self
            .inner
            .search_memories(space_id, user_id, vector.to_vec(), limit)
            .await
            .map_err(RepositoryError::from)?;

        Ok(matches
            .into_iter()
            .map(|m| VectorSearchResult {
                memory_id: m.memory_id,
                score: m.score,
                payload: None,
            })
            .collect())
    }

    async fn get(&self, _memory_id: Uuid) -> Result<Option<MemoryVector>, RepositoryError> {
        // 需要扩展 VectorStore trait 来支持
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_vector_serialization() {
        let vector = MemoryVector {
            memory_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            vector: vec![0.1, 0.2, 0.3],
            payload: Some(VectorPayload {
                title: Some("测试记忆".to_string()),
                content_snippet: Some("这是内容摘要".to_string()),
                tags: vec!["test".to_string()],
                memory_type: "text".to_string(),
                created_at: Utc::now(),
            }),
        };

        let json = serde_json::to_string(&vector).unwrap();
        let deserialized: MemoryVector = serde_json::from_str(&json).unwrap();

        assert_eq!(vector.memory_id, deserialized.memory_id);
        assert_eq!(vector.vector, deserialized.vector);
    }

    #[test]
    fn test_vector_search_result_default_payload() {
        let result = VectorSearchResult {
            memory_id: Uuid::new_v4(),
            score: 0.95,
            payload: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("0.95"));
    }
}
