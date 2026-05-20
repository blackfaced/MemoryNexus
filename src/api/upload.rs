//! 文件上传 API

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

/// 上传查询参数
#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    /// 是否生成缩略图
    #[serde(default)]
    #[allow(dead_code)]
    pub thumbnail: bool,

    /// 缩略图尺寸
    #[serde(default)]
    #[allow(dead_code)]
    pub size: Option<String>,
}

/// 上传响应
#[derive(Serialize)]
pub struct UploadResponse {
    pub id: Uuid,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub size: u64,
    pub content_type: String,
}

#[allow(dead_code)]
/// POST /api/v1/upload - 上传文件
pub async fn upload(
    State(_state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Query(_query): Query<UploadQuery>,
) -> Result<Json<ApiResponse<UploadResponse>>, AppError> {
    // TODO: 实现 multipart 文件上传
    Err(AppError::NotImplemented("文件上传".to_string()))
}

/// POST /api/v1/memories/:id/media - 上传记忆媒体
#[allow(dead_code)]
pub async fn upload_memory_media(
    State(_state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Path(_memory_id): Path<Uuid>,
    Query(_query): Query<UploadQuery>,
) -> Result<Json<ApiResponse<UploadResponse>>, AppError> {
    // TODO: 实现记忆媒体上传
    // 1. 验证记忆存在且属于当前用户
    // 2. 处理文件上传
    // 3. 生成缩略图（如果是图片）
    // 4. 更新记忆记录
    Err(AppError::NotImplemented("媒体上传".to_string()))
}

/// GET /api/v1/media/:key - 获取媒体文件
#[allow(dead_code)]
pub async fn get_media(
    State(_state): State<AppState>,
    Path(_key): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // TODO: 实现媒体文件获取
    // 可以直接从存储返回文件，或者重定向到预签名 URL
    Err(AppError::NotImplemented("媒体获取".to_string()))
}

/// DELETE /api/v1/media/:key - 删除媒体文件
#[allow(dead_code)]
pub async fn delete_media(
    State(_state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Path(_key): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // TODO: 实现媒体删除
    // 需要验证用户权限
    Err(AppError::NotImplemented("媒体删除".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_query_defaults() {
        let json = r#"{}"#;
        let query: UploadQuery = serde_json::from_str(json).unwrap();
        assert!(!query.thumbnail);
        assert!(query.size.is_none());
    }

    #[test]
    fn test_upload_response_serde() {
        let response = UploadResponse {
            id: Uuid::new_v4(),
            url: "https://storage.example.com/bucket/key.jpg".to_string(),
            thumbnail_url: Some("https://storage.example.com/thumbnails/key_thumb.jpg".to_string()),
            size: 1024,
            content_type: "image/jpeg".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"url\":"));
        assert!(json.contains("\"thumbnail_url\":"));
    }
}
