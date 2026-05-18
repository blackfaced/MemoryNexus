//! 向量存储模块
//!
//! P0 语义检索闭环使用 Qdrant REST API，避免把 SDK 细节扩散到 API 层。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_COLLECTION: &str = "memorynexus_memories";

#[derive(Debug, Error)]
pub enum VectorError {
    #[error("向量存储请求失败: {0}")]
    Request(String),

    #[error("向量存储返回异常: {0}")]
    Response(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryVectorPayload {
    pub memory_id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub memory_type: String,
    pub is_shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryVectorPoint {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub payload: MemoryVectorPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorSearchMatch {
    pub memory_id: Uuid,
    pub score: f32,
}

#[async_trait::async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert_memory(&self, point: MemoryVectorPoint) -> Result<(), VectorError>;
    async fn search_memories(
        &self,
        user_id: Uuid,
        vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<VectorSearchMatch>, VectorError>;
}

#[derive(Clone)]
pub struct QdrantVectorStore {
    base_url: String,
    collection: String,
    client: reqwest::Client,
}

impl QdrantVectorStore {
    pub fn new(base_url: String, collection: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            collection,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("QDRANT_URL").ok()?;
        let collection =
            std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| DEFAULT_COLLECTION.to_string());
        Some(Self::new(base_url, collection))
    }

    fn upsert_url(&self) -> String {
        format!(
            "{}/collections/{}/points?wait=true",
            self.base_url, self.collection
        )
    }

    fn search_url(&self) -> String {
        format!(
            "{}/collections/{}/points/search",
            self.base_url, self.collection
        )
    }
}

#[derive(Debug, Serialize)]
struct QdrantUpsertRequest {
    points: Vec<MemoryVectorPoint>,
}

#[derive(Debug, Serialize)]
struct QdrantSearchRequest {
    vector: Vec<f32>,
    limit: usize,
    filter: QdrantFilter,
    with_payload: bool,
}

#[derive(Debug, Serialize)]
struct QdrantFilter {
    should: Vec<QdrantCondition>,
}

#[derive(Debug, Serialize)]
struct QdrantCondition {
    key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#match: Option<QdrantMatch>,
}

#[derive(Debug, Serialize)]
struct QdrantMatch {
    value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResponse {
    result: Vec<QdrantScoredPoint>,
}

#[derive(Debug, Deserialize)]
struct QdrantScoredPoint {
    score: f32,
    payload: Option<HashMap<String, serde_json::Value>>,
}

impl QdrantSearchRequest {
    fn for_user(user_id: Uuid, vector: Vec<f32>, limit: usize) -> Self {
        Self {
            vector,
            limit,
            filter: QdrantFilter {
                should: vec![
                    QdrantCondition {
                        key: "user_id".to_string(),
                        r#match: Some(QdrantMatch {
                            value: serde_json::Value::String(user_id.to_string()),
                        }),
                    },
                    QdrantCondition {
                        key: "is_shared".to_string(),
                        r#match: Some(QdrantMatch {
                            value: serde_json::Value::Bool(true),
                        }),
                    },
                ],
            },
            with_payload: true,
        }
    }
}

#[async_trait::async_trait]
impl VectorStore for QdrantVectorStore {
    async fn upsert_memory(&self, point: MemoryVectorPoint) -> Result<(), VectorError> {
        let body = QdrantUpsertRequest {
            points: vec![point],
        };

        let response = self
            .client
            .put(self.upsert_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| VectorError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(VectorError::Response(format!("{} {}", status, text)));
        }

        Ok(())
    }

    async fn search_memories(
        &self,
        user_id: Uuid,
        vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<VectorSearchMatch>, VectorError> {
        let body = QdrantSearchRequest::for_user(user_id, vector, limit);

        let response = self
            .client
            .post(self.search_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| VectorError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(VectorError::Response(format!("{} {}", status, text)));
        }

        let response: QdrantSearchResponse = response
            .json()
            .await
            .map_err(|e| VectorError::Response(e.to_string()))?;

        Ok(response
            .result
            .into_iter()
            .filter_map(|point| {
                let payload = point.payload?;
                let memory_id = payload.get("memory_id")?.as_str()?;
                let memory_id = Uuid::parse_str(memory_id).ok()?;
                Some(VectorSearchMatch {
                    memory_id,
                    score: point.score,
                })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_search_request_filters_to_user_or_shared_memories() {
        let user_id = Uuid::new_v4();
        let request = QdrantSearchRequest::for_user(user_id, vec![0.1, 0.2], 5);
        let json = serde_json::to_value(request).unwrap();

        assert_eq!(json["limit"], 5);
        assert_eq!(json["with_payload"], true);
        assert_eq!(json["filter"]["should"][0]["key"], "user_id");
        assert_eq!(
            json["filter"]["should"][0]["match"]["value"],
            user_id.to_string()
        );
        assert_eq!(json["filter"]["should"][1]["key"], "is_shared");
        assert_eq!(json["filter"]["should"][1]["match"]["value"], true);
    }

    #[test]
    fn qdrant_urls_are_built_without_double_slashes() {
        let store =
            QdrantVectorStore::new("http://localhost:6333/".to_string(), "memories".to_string());

        assert_eq!(
            store.upsert_url(),
            "http://localhost:6333/collections/memories/points?wait=true"
        );
        assert_eq!(
            store.search_url(),
            "http://localhost:6333/collections/memories/points/search"
        );
    }
}
