//! 向量存储模块
//!
//! P0 语义检索闭环使用 Qdrant REST API，避免把 SDK 细节扩散到 API 层。

pub mod repository;

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
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub source_type: String,
    pub created_at: String,
    pub visibility: String,
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
    async fn ensure_collection(&self, vector_size: usize) -> Result<(), VectorError>;
    async fn upsert_memory(&self, point: MemoryVectorPoint) -> Result<(), VectorError>;
    async fn delete_memory(&self, memory_id: Uuid) -> Result<(), VectorError>;
    async fn search_memories(
        &self,
        space_id: Uuid,
        user_id: Uuid,
        vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<VectorSearchMatch>, VectorError>;
}

#[derive(Clone)]
pub struct QdrantVectorStore {
    base_url: String,
    collection: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl QdrantVectorStore {
    pub fn new(base_url: String, collection: String) -> Self {
        Self::new_with_api_key(base_url, collection, None)
    }

    pub fn new_with_api_key(base_url: String, collection: String, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            collection,
            api_key: api_key.and_then(|key| {
                let key = key.trim().to_string();
                if key.is_empty() {
                    None
                } else {
                    Some(key)
                }
            }),
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("QDRANT_URL").ok()?;
        let collection =
            std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| DEFAULT_COLLECTION.to_string());
        let api_key = std::env::var("QDRANT_API_KEY").ok();
        Some(Self::new_with_api_key(base_url, collection, api_key))
    }

    fn authenticated_request(
        &self,
        method: reqwest::Method,
        url: String,
    ) -> reqwest::RequestBuilder {
        let builder = self.client.request(method, url);
        if let Some(api_key) = self.api_key.as_deref() {
            builder.header("api-key", api_key)
        } else {
            builder
        }
    }

    fn collection_url(&self) -> String {
        format!("{}/collections/{}", self.base_url, self.collection)
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

    fn delete_url(&self) -> String {
        format!(
            "{}/collections/{}/points/delete?wait=true",
            self.base_url, self.collection
        )
    }
}

#[derive(Debug, Serialize)]
struct QdrantCreateCollectionRequest {
    vectors: QdrantVectorConfig,
}

#[derive(Debug, Serialize)]
struct QdrantVectorConfig {
    size: usize,
    distance: &'static str,
}

impl QdrantCreateCollectionRequest {
    fn new(size: usize) -> Self {
        Self {
            vectors: QdrantVectorConfig {
                size,
                distance: "Cosine",
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct QdrantUpsertRequest {
    points: Vec<MemoryVectorPoint>,
}

#[derive(Debug, Serialize)]
struct QdrantDeleteRequest {
    points: Vec<String>,
}

impl QdrantDeleteRequest {
    fn for_memory(memory_id: Uuid) -> Self {
        Self {
            points: vec![memory_id.to_string()],
        }
    }
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
    must: Vec<QdrantCondition>,
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
    fn for_space(space_id: Uuid, user_id: Uuid, vector: Vec<f32>, limit: usize) -> Self {
        Self {
            vector,
            limit,
            filter: QdrantFilter {
                must: vec![QdrantCondition {
                    key: "space_id".to_string(),
                    r#match: Some(QdrantMatch {
                        value: serde_json::Value::String(space_id.to_string()),
                    }),
                }],
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
    async fn ensure_collection(&self, vector_size: usize) -> Result<(), VectorError> {
        let get_response = self
            .authenticated_request(reqwest::Method::GET, self.collection_url())
            .send()
            .await
            .map_err(|e| VectorError::Request(e.to_string()))?;

        if get_response.status().is_success() {
            return Ok(());
        }

        if get_response.status() != reqwest::StatusCode::NOT_FOUND {
            let status = get_response.status();
            let text = get_response.text().await.unwrap_or_default();
            return Err(VectorError::Response(format!("{} {}", status, text)));
        }

        let create_response = self
            .authenticated_request(reqwest::Method::PUT, self.collection_url())
            .json(&QdrantCreateCollectionRequest::new(vector_size))
            .send()
            .await
            .map_err(|e| VectorError::Request(e.to_string()))?;

        if create_response.status().is_success()
            || create_response.status() == reqwest::StatusCode::CONFLICT
        {
            return Ok(());
        }

        let status = create_response.status();
        let text = create_response.text().await.unwrap_or_default();
        Err(VectorError::Response(format!("{} {}", status, text)))
    }

    async fn upsert_memory(&self, point: MemoryVectorPoint) -> Result<(), VectorError> {
        let body = QdrantUpsertRequest {
            points: vec![point],
        };

        let response = self
            .authenticated_request(reqwest::Method::PUT, self.upsert_url())
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

    async fn delete_memory(&self, memory_id: Uuid) -> Result<(), VectorError> {
        let body = QdrantDeleteRequest::for_memory(memory_id);

        let response = self
            .authenticated_request(reqwest::Method::POST, self.delete_url())
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
        space_id: Uuid,
        user_id: Uuid,
        vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<VectorSearchMatch>, VectorError> {
        let body = QdrantSearchRequest::for_space(space_id, user_id, vector, limit);

        let response = self
            .authenticated_request(reqwest::Method::POST, self.search_url())
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
    fn semantic_search_request_filters_to_space_and_user_or_shared_memories() {
        let space_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let request = QdrantSearchRequest::for_space(space_id, user_id, vec![0.1, 0.2], 5);
        let json = serde_json::to_value(request).unwrap();

        assert_eq!(json["limit"], 5);
        assert_eq!(json["with_payload"], true);
        assert_eq!(json["filter"]["must"][0]["key"], "space_id");
        assert_eq!(
            json["filter"]["must"][0]["match"]["value"],
            space_id.to_string()
        );
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
            store.collection_url(),
            "http://localhost:6333/collections/memories"
        );
        assert_eq!(
            store.upsert_url(),
            "http://localhost:6333/collections/memories/points?wait=true"
        );
        assert_eq!(
            store.search_url(),
            "http://localhost:6333/collections/memories/points/search"
        );
        assert_eq!(
            store.delete_url(),
            "http://localhost:6333/collections/memories/points/delete?wait=true"
        );
    }

    #[test]
    fn qdrant_delete_request_targets_memory_point_id() {
        let memory_id = Uuid::new_v4();
        let request = QdrantDeleteRequest::for_memory(memory_id);
        let json = serde_json::to_value(request).unwrap();

        assert_eq!(json["points"][0], memory_id.to_string());
    }

    #[test]
    fn qdrant_create_collection_request_uses_cosine_distance() {
        let request = QdrantCreateCollectionRequest::new(1536);
        let json = serde_json::to_value(request).unwrap();

        assert_eq!(json["vectors"]["size"], 1536);
        assert_eq!(json["vectors"]["distance"], "Cosine");
    }

    #[test]
    fn qdrant_request_includes_api_key_when_configured() {
        let store = QdrantVectorStore::new_with_api_key(
            "https://example.qdrant.cloud".to_string(),
            "memories".to_string(),
            Some("secret-key".to_string()),
        );

        let request = store
            .authenticated_request(reqwest::Method::GET, store.collection_url())
            .build()
            .unwrap();

        assert_eq!(request.headers().get("api-key").unwrap(), "secret-key");
    }

    #[test]
    fn memory_vector_payload_includes_space_provenance_and_visibility() {
        let payload = MemoryVectorPayload {
            memory_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: Some(Uuid::new_v4()),
            source_type: "memory".to_string(),
            created_at: "2026-05-20T00:00:00Z".to_string(),
            visibility: "private".to_string(),
            title: Some("Phase 1B".to_string()),
            memory_type: "text".to_string(),
            is_shared: false,
        };
        let json = serde_json::to_value(payload).unwrap();

        assert_eq!(json["source_type"], "memory");
        assert!(json["namespace_id"].is_string());
        assert_eq!(json["created_at"], "2026-05-20T00:00:00Z");
        assert_eq!(json["visibility"], "private");
    }
}
