//! API 路由模块

use axum::{Router, routing::get, routing::post};

mod memories;
mod health;

/// 聚合所有 API 路由
pub fn routes() -> Router {
    Router::new()
        .route("/api/v1/health", get(health::check))
        .route("/api/v1/memories", get(memories::list))
        .route("/api/v1/memories", post(memories::create))
}
