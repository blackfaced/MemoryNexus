//! 标签 API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

/// 创建标签请求
#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

/// 更新标签请求
#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: String,
}

/// 标签响应
#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub id: Uuid,
    pub name: String,
    pub memory_count: Option<i64>,
}

/// 标签列表响应
#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub items: Vec<TagResponse>,
}

/// POST /api/v1/tags - 创建标签
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TagResponse>>), AppError> {
    // 验证输入
    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest(
            "标签名长度需在1-100字符之间".to_string(),
        ));
    }

    // 检查标签是否已存在
    let existing = state
        .repositories
        .tags
        .find_by_name(name, auth_user.user_id)
        .await
        .map_err(AppError::Database)?;

    if existing.is_some() {
        return Err(AppError::BadRequest("标签已存在".to_string()));
    }

    // 创建标签
    let tag = state
        .repositories
        .tags
        .create(name, auth_user.user_id)
        .await
        .map_err(AppError::Database)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(TagResponse {
            id: tag.id,
            name: tag.name,
            memory_count: None,
        })),
    ))
}

/// GET /api/v1/tags - 列出用户标签
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<Json<ApiResponse<TagListResponse>>, AppError> {
    let tags = state
        .repositories
        .tags
        .list_by_user(auth_user.user_id)
        .await
        .map_err(AppError::Database)?;

    // 获取每个标签的记忆数量
    let mut items = Vec::new();
    for tag in tags {
        let count = state
            .repositories
            .tags
            .count_memories(tag.id)
            .await
            .map_err(AppError::Database)?;

        items.push(TagResponse {
            id: tag.id,
            name: tag.name,
            memory_count: Some(count),
        });
    }

    Ok(Json(ApiResponse::success(TagListResponse { items })))
}

/// GET /api/v1/tags/:id - 获取标签详情
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<TagResponse>>, AppError> {
    let tag = state
        .repositories
        .tags
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Tag {} not found", id)))?;

    // 验证权限（用户创建的标签）
    if tag.user_id != Some(auth_user.user_id) {
        return Err(AppError::Unauthorized);
    }

    let count = state
        .repositories
        .tags
        .count_memories(tag.id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(TagResponse {
        id: tag.id,
        name: tag.name,
        memory_count: Some(count),
    })))
}

/// PATCH /api/v1/tags/:id - 更新标签
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<Json<ApiResponse<TagResponse>>, AppError> {
    // 验证输入
    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest(
            "标签名长度需在1-100字符之间".to_string(),
        ));
    }

    // 获取现有标签
    let existing = state
        .repositories
        .tags
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Tag {} not found", id)))?;

    // 验证权限
    if existing.user_id != Some(auth_user.user_id) {
        return Err(AppError::Unauthorized);
    }

    // 更新标签
    let tag = state
        .repositories
        .tags
        .update(id, name)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(TagResponse {
        id: tag.id,
        name: tag.name,
        memory_count: None,
    })))
}

/// DELETE /api/v1/tags/:id - 删除标签
pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 获取现有标签验证权限
    let existing = state
        .repositories
        .tags
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Tag {} not found", id)))?;

    if existing.user_id != Some(auth_user.user_id) {
        return Err(AppError::Unauthorized);
    }

    state
        .repositories
        .tags
        .delete(id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tag_request_serde() {
        let json = r#"{"name":"旅行"}"#;
        let req: CreateTagRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "旅行");
    }

    #[test]
    fn test_update_tag_request_serde() {
        let json = r#"{"name":"新标签名"}"#;
        let req: UpdateTagRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "新标签名");
    }

    #[test]
    fn test_tag_response_serde() {
        let response = TagResponse {
            id: Uuid::new_v4(),
            name: "旅行".to_string(),
            memory_count: Some(10),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"旅行\""));
        assert!(json.contains("\"memory_count\":10"));
    }
}
