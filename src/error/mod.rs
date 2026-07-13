//! 统一错误处理
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// 应用错误类型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("未找到资源: {0}")]
    NotFound(String),

    #[error("认证失败")]
    Unauthorized,

    #[error("参数错误: {0}")]
    BadRequest(String),

    #[error("冲突: {0}")]
    Conflict(String),

    #[error("服务器内部错误: {0}")]
    Internal(String),

    #[error("功能尚未实现: {0}")]
    NotImplemented(String),

    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::NotImplemented(msg) => (StatusCode::NOT_IMPLEMENTED, msg.clone()),
            AppError::Database(e) => {
                tracing::error!("数据库错误: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "数据库错误".to_string())
            }
        };

        let body = Json(json!({
            "ok": false,
            "error": {
                "code": status.as_u16(),
                "message": error_message,
            }
        }));

        (status, body).into_response()
    }
}

/// API 响应封装
#[derive(serde::Serialize)]
pub struct ApiResponse<T: serde::Serialize> {
    pub ok: bool,
    pub data: T,
}

impl<T: serde::Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { ok: true, data }
    }
}
