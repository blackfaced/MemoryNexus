//! 记忆 API

use axum::{
    Json, extract::{Query, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiResponse, AppError};

/// 记忆类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
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

/// 记忆数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Uuid,
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub memory_type: MemoryType,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_shared: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建记忆请求
#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub memory_type: MemoryType,
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
    pub items: Vec<Memory>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// GET /api/v1/memories
pub async fn list(
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<MemoryListResponse>>, AppError> {
    // TODO: 从数据库查询
    let items = vec![];
    let total = 0;

    Ok(Json(ApiResponse::success(MemoryListResponse {
        items,
        total,
        limit: params.limit,
        offset: params.offset,
    })))
}

/// POST /api/v1/memories
pub async fn create(
    Json(req): Json<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Memory>>), AppError> {
    // 验证输入
    if req.content.trim().is_empty() {
        return Err(AppError::BadRequest("内容不能为空".to_string()));
    }

    let now = chrono::Utc::now();
    let memory = Memory {
        id: Uuid::new_v4(),
        title: req.title,
        content: req.content,
        memory_type: req.memory_type,
        tags: req.tags,
        is_shared: req.is_shared,
        created_at: now,
        updated_at: now,
    };

    // TODO: 保存到数据库

    Ok((StatusCode::CREATED, Json(ApiResponse::success(memory))))
}

/// GET /api/v1/memories/:id
pub async fn get(
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    // TODO: 从数据库查询
    Err(AppError::NotFound(format!("Memory {} not found", id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_memory_validation() {
        // 空内容应该返回错误
        let req = CreateMemoryRequest {
            title: None,
            content: "".to_string(),
            memory_type: MemoryType::Text,
            tags: vec![],
            is_shared: false,
        };

        let result = create(Json(req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_memory_success() {
        let req = CreateMemoryRequest {
            title: Some("测试记忆".to_string()),
            content: "这是一条测试记忆".to_string(),
            memory_type: MemoryType::Text,
            tags: vec!["测试".to_string()],
            is_shared: false,
        };

        let result = create(Json(req)).await;
        assert!(result.is_ok());

        let (status, body) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert!(body.data.id.ne(&Uuid::nil()));
    }

    #[test]
    fn test_memory_type_default() {
        assert_eq!(MemoryType::default(), MemoryType::Text);
    }
}
