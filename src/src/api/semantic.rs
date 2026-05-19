//! 语义搜索 API
//!
//! 基于向量相似度的语义搜索

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;
use crate::vector::repository::VectorSearchResult;

/// 语义搜索请求
#[derive(Debug, Deserialize)]
pub struct SemanticSearchRequest {
    /// 搜索查询文本
    pub q: String,

    /// 返回数量
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// 相似度阈值 (0-1)
    #[serde(default)]
    pub threshold: Option<f32>,
}

/// 语义搜索响应
#[derive(Debug, Serialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<SemanticSearchResult>,
    pub query: String,
    pub total: usize,
}

/// 单个语义搜索结果
#[derive(Debug, Serialize)]
pub struct SemanticSearchResult {
    pub memory_id: Uuid,
    pub score: f32,
    pub title: Option<String>,
    pub content_snippet: Option<String>,
    pub tags: Vec<String>,
    pub memory_type: String,
    pub created_at: Option<String>,
}

impl From<VectorSearchResult> for SemanticSearchResult {
    fn from(v: VectorSearchResult) -> Self {
        Self {
            memory_id: v.memory_id,
            score: v.score,
            title: v.payload.as_ref().and_then(|p| p.title.clone()),
            content_snippet: v.payload.as_ref().and_then(|p| p.content_snippet.clone()),
            tags: v
                .payload
                .as_ref()
                .map(|p| p.tags.clone())
                .unwrap_or_default(),
            memory_type: v
                .payload
                .as_ref()
                .map(|p| p.memory_type.clone())
                .unwrap_or_default(),
            created_at: v.payload.as_ref().map(|p| p.created_at.to_rfc3339()),
        }
    }
}

fn default_limit() -> usize {
    20
}

/// POST /api/v1/search/semantic - 语义搜索
pub async fn semantic_search(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<SemanticSearchRequest>,
) -> Result<Json<ApiResponse<SemanticSearchResponse>>, AppError> {
    // 检查 AI 功能是否可用
    let embedder = state
        .ai
        .embedder
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("AI 功能未配置".to_string()))?;

    // 生成查询向量
    let embedding_result = embedder
        .embed(&req.q)
        .await
        .map_err(|e| AppError::BadRequest(format!("嵌入生成失败: {}", e)))?;

    // 执行向量搜索
    let results = state
        .repositories
        .vectors
        .search(
            &embedding_result.embedding,
            auth_user.user_id,
            req.limit,
            req.threshold,
        )
        .await
        .map_err(|e| AppError::BadRequest(format!("向量搜索失败: {}", e)))?;

    let total = results.len();
    let response = SemanticSearchResponse {
        results: results
            .into_iter()
            .map(SemanticSearchResult::from)
            .collect(),
        query: req.q,
        total,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// GET /api/v1/search/semantic - 语义搜索 (GET 版本)
pub async fn semantic_search_get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<SemanticSearchRequest>,
) -> Result<Json<ApiResponse<SemanticSearchResponse>>, AppError> {
    semantic_search_impl(&state, &auth_user, query).await
}

async fn semantic_search_impl(
    state: &AppState,
    auth_user: &AuthenticatedUser,
    req: SemanticSearchRequest,
) -> Result<Json<ApiResponse<SemanticSearchResponse>>, AppError> {
    // 检查 AI 功能是否可用
    let embedder = state
        .ai
        .embedder
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("AI 功能未配置".to_string()))?;

    // 生成查询向量
    let embedding_result = embedder
        .embed(&req.q)
        .await
        .map_err(|e| AppError::BadRequest(format!("嵌入生成失败: {}", e)))?;

    // 执行向量搜索
    let results = state
        .repositories
        .vectors
        .search(
            &embedding_result.embedding,
            auth_user.user_id,
            req.limit,
            req.threshold,
        )
        .await
        .map_err(|e| AppError::BadRequest(format!("向量搜索失败: {}", e)))?;

    let total = results.len();
    let response = SemanticSearchResponse {
        results: results
            .into_iter()
            .map(SemanticSearchResult::from)
            .collect(),
        query: req.q,
        total,
    };

    Ok(Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_search_request_deserialize() {
        let json = r#"{"q":"周末旅行","limit":10}"#;
        let req: SemanticSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.q, "周末旅行");
        assert_eq!(req.limit, 10);
    }

    #[test]
    fn test_semantic_search_response_serialize() {
        let response = SemanticSearchResponse {
            results: vec![],
            query: "test".to_string(),
            total: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_semantic_search_result_from_vector() {
        let vector_result = VectorSearchResult {
            memory_id: Uuid::new_v4(),
            score: 0.95,
            payload: None,
        };

        let result: SemanticSearchResult = vector_result.into();
        assert_eq!(result.score, 0.95);
    }
}
