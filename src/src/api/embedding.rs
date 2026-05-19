//! 向量管理 API
//!
//! 用于管理记忆的向量嵌入

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;
use crate::vector::repository::{MemoryVector, VectorPayload};

/// 创建向量请求
#[derive(Debug, Deserialize)]
pub struct CreateEmbeddingRequest {
    /// 记忆 ID
    pub memory_id: Uuid,

    /// 要向量化的文本内容
    pub content: String,

    /// 可选标题
    pub title: Option<String>,

    /// 可选标签
    #[serde(default)]
    pub tags: Vec<String>,

    /// 记忆类型
    #[serde(default = "default_memory_type")]
    pub memory_type: String,
}

/// 创建向量响应
#[derive(Debug, Serialize)]
pub struct CreateEmbeddingResponse {
    pub memory_id: Uuid,
    pub dimension: usize,
    pub model: String,
}

/// 批量创建向量请求
#[derive(Debug, Deserialize)]
pub struct BatchCreateEmbeddingRequest {
    pub items: Vec<BatchEmbeddingItem>,
}

/// 批量向量项
#[derive(Debug, Deserialize)]
pub struct BatchEmbeddingItem {
    pub memory_id: Uuid,
    pub content: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub memory_type: String,
    pub created_at: String,
}

/// 批量创建向量响应
#[derive(Debug, Serialize)]
pub struct BatchCreateEmbeddingResponse {
    pub created: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// 删除向量请求
#[derive(Debug, Deserialize)]
pub struct DeleteEmbeddingRequest {
    pub memory_ids: Vec<Uuid>,
}

/// 检查向量是否存在请求
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CheckEmbeddingRequest {
    pub memory_id: Uuid,
}

/// 检查向量是否存在响应
#[derive(Debug, Serialize)]
pub struct CheckEmbeddingResponse {
    pub memory_id: Uuid,
    pub exists: bool,
}

#[allow(dead_code)]
fn default_memory_type() -> String {
    "text".to_string()
}

/// POST /api/v1/embeddings - 创建记忆向量
pub async fn create_embedding(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateEmbeddingRequest>,
) -> Result<Json<ApiResponse<CreateEmbeddingResponse>>, AppError> {
    // 检查 AI 功能是否可用
    let embedder = state
        .ai
        .embedder
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("AI 功能未配置".to_string()))?;

    // 生成嵌入
    let embedding_result = embedder
        .embed(&req.content)
        .await
        .map_err(|e| AppError::BadRequest(format!("嵌入生成失败: {}", e)))?;

    // 构建向量
    let memory_vector = MemoryVector {
        memory_id: req.memory_id,
        user_id: auth_user.user_id,
        vector: embedding_result.embedding.clone(),
        payload: Some(VectorPayload {
            title: req.title,
            content_snippet: Some(req.content.chars().take(500).collect()),
            tags: req.tags,
            memory_type: req.memory_type,
            created_at: chrono::Utc::now(),
        }),
    };

    // 存储向量
    state
        .repositories
        .vectors
        .store(memory_vector)
        .await
        .map_err(|e| AppError::BadRequest(format!("向量存储失败: {}", e)))?;

    Ok(Json(ApiResponse::success(CreateEmbeddingResponse {
        memory_id: req.memory_id,
        dimension: embedding_result.embedding.len(),
        model: embedding_result.model,
    })))
}

/// POST /api/v1/embeddings/batch - 批量创建记忆向量
pub async fn batch_create_embeddings(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<BatchCreateEmbeddingRequest>,
) -> Result<Json<ApiResponse<BatchCreateEmbeddingResponse>>, AppError> {
    // 检查 AI 功能是否可用
    let embedder = state
        .ai
        .embedder
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("AI 功能未配置".to_string()))?;

    let mut created = 0;
    let mut failed = 0;
    let mut errors: Vec<String> = Vec::new();

    // 批量生成嵌入
    let contents: Vec<String> = req.items.iter().map(|i| i.content.clone()).collect();
    let embeddings = embedder
        .embed_batch(contents)
        .await
        .map_err(|e| AppError::BadRequest(format!("批量嵌入生成失败: {}", e)))?;

    // 构建向量列表
    let mut vectors: Vec<MemoryVector> = Vec::new();
    for (i, item) in req.items.iter().enumerate() {
        let embedding = match embeddings.get(i) {
            Some(e) => e,
            None => {
                errors.push(format!("Memory {}: 缺少嵌入结果", item.memory_id));
                failed += 1;
                continue;
            }
        };

        let created_at = chrono::DateTime::parse_from_rfc3339(&item.created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        vectors.push(MemoryVector {
            memory_id: item.memory_id,
            user_id: auth_user.user_id,
            vector: embedding.embedding.clone(),
            payload: Some(VectorPayload {
                title: item.title.clone(),
                content_snippet: Some(item.content.chars().take(500).collect()),
                tags: item.tags.clone(),
                memory_type: item.memory_type.clone(),
                created_at,
            }),
        });

        created += 1;
    }

    // 批量存储
    let vectors_count = vectors.len();
    if !vectors.is_empty() {
        if let Err(e) = state.repositories.vectors.store_batch(vectors).await {
            errors.push(format!("批量存储失败: {}", e));
            failed = vectors_count;
            created = 0;
        }
    }

    Ok(Json(ApiResponse::success(BatchCreateEmbeddingResponse {
        created,
        failed,
        errors,
    })))
}

/// DELETE /api/v1/embeddings - 删除记忆向量
pub async fn delete_embeddings(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Json(req): Json<DeleteEmbeddingRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state
        .repositories
        .vectors
        .delete_batch(req.memory_ids)
        .await
        .map_err(|e| AppError::BadRequest(format!("向量删除失败: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// GET /api/v1/embeddings/:id - 检查向量是否存在
pub async fn check_embedding(
    State(state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Path(memory_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CheckEmbeddingResponse>>, AppError> {
    let exists = state
        .repositories
        .vectors
        .exists(memory_id)
        .await
        .map_err(|e| AppError::BadRequest(format!("向量查询失败: {}", e)))?;

    Ok(Json(ApiResponse::success(CheckEmbeddingResponse {
        memory_id,
        exists,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_embedding_request_deserialize() {
        let json = r#"{"memory_id":"550e8400-e29b-41d4-a716-446655440000","content":"测试内容"}"#;
        let req: CreateEmbeddingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "测试内容");
        assert_eq!(req.memory_type, "text");
    }

    #[test]
    fn test_batch_create_embedding_request_deserialize() {
        let json = r#"{
            "items": [{
                "memory_id": "550e8400-e29b-41d4-a716-446655440000",
                "content": "测试",
                "tags": ["test"],
                "memory_type": "text",
                "created_at": "2024-01-01T00:00:00Z"
            }]
        }"#;
        let req: BatchCreateEmbeddingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.items.len(), 1);
    }

    #[test]
    fn test_create_embedding_response_serialize() {
        let response = CreateEmbeddingResponse {
            memory_id: Uuid::new_v4(),
            dimension: 1536,
            model: "text-embedding-ada-002".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("1536"));
    }
}
