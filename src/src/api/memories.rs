//! 记忆 API

use axum::{
    Json, extract::{Query, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiResponse, AppError};
use crate::state::AppState;
use crate::db::memory::{MemoryDb, MemoryType, CreateMemory};

/// 记忆类型枚举（API 层）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiMemoryType {
    Text,
    Image,
    Audio,
    Video,
}

impl Default for ApiMemoryType {
    fn default() -> Self {
        Self::Text
    }
}

impl From<ApiMemoryType> for MemoryType {
    fn from(t: ApiMemoryType) -> Self {
        match t {
            ApiMemoryType::Text => MemoryType::Text,
            ApiMemoryType::Image => MemoryType::Image,
            ApiMemoryType::Audio => MemoryType::Audio,
            ApiMemoryType::Video => MemoryType::Video,
        }
    }
}

/// 创建记忆请求
#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub memory_type: ApiMemoryType,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_shared: bool,
}

/// 列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub tag: Option<String>,
}

fn default_limit() -> i64 {
    20
}

/// 记忆列表响应
#[derive(Serialize)]
pub struct MemoryListResponse {
    pub items: Vec<MemoryDb>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// GET /api/v1/memories
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<MemoryListResponse>>, AppError> {
    // TODO: 从 state 获取当前用户 ID（待认证模块）
    let user_id = Uuid::nil();

    let memories = state.repositories.memories
        .list_by_user(user_id, params.limit, params.offset)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(MemoryListResponse {
        items: memories,
        total: memories.len() as i64,
        limit: params.limit,
        offset: params.offset,
    })))
}

/// POST /api/v1/memories
pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<MemoryDb>>), AppError> {
    // 验证输入
    if req.content.trim().is_empty() {
        return Err(AppError::BadRequest("内容不能为空".to_string()));
    }

    // TODO: 从 state 获取当前用户 ID（待认证模块）
    let user_id = Uuid::nil();

    let create_memory = CreateMemory {
        user_id,
        title: req.title,
        content: req.content,
        memory_type: req.memory_type.into(),
        file_path: None,
        is_shared: req.is_shared,
        tags: req.tags,
    };

    let memory = state.repositories.memories
        .create(create_memory)
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(memory))))
}

/// GET /api/v1/memories/:id
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MemoryDb>>, AppError> {
    let memory = state.repositories.memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;

    Ok(Json(ApiResponse::success(memory)))
}

/// DELETE /api/v1/memories/:id
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let deleted = state.repositories.memories
        .delete(id)
        .await
        .map_err(AppError::Database)?;

    if !deleted {
        return Err(AppError::NotFound(format!("Memory {} not found", id)));
    }

    Ok(Json(ApiResponse::success(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_memory_type_default() {
        assert_eq!(ApiMemoryType::default(), ApiMemoryType::Text);
    }

    #[test]
    fn test_api_memory_type_to_db_type() {
        assert_eq!(MemoryType::Text, ApiMemoryType::Text.into());
        assert_eq!(MemoryType::Image, ApiMemoryType::Image.into());
        assert_eq!(MemoryType::Audio, ApiMemoryType::Audio.into());
        assert_eq!(MemoryType::Video, ApiMemoryType::Video.into());
    }

    #[test]
    fn test_create_memory_request_serde() {
        let json = r#"{"content":"test","memory_type":"text"}"#;
        let req: CreateMemoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "test");
        assert_eq!(req.memory_type, ApiMemoryType::Text);
    }

    #[test]
    fn test_list_query_defaults() {
        let json = r#"{}"#;
        let query: ListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 0);
    }
}
