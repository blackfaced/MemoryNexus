//! 健康检查 API

use axum::{Json, http::StatusCode};
use serde::Serialize;

/// 健康检查响应
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// GET /api/v1/health
pub async fn check() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let (status, body) = check().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.status, "healthy");
    }
}
