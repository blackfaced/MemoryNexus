//! API 路由模块

use axum::Router;

mod auth;
mod health;
mod memories;

/// 聚合所有 API 路由
pub fn routes() -> Router {
    Router::new()
        // 健康检查
        .route("/api/v1/health", axum::routing::get(health::check))
        // 认证
        .route("/api/v1/auth/login", axum::routing::post(auth::login))
        .route("/api/v1/auth/register", axum::routing::post(auth::register))
        .route("/api/v1/auth/me", axum::routing::get(auth::me))
        // 记忆 CRUD
        .route("/api/v1/memories", axum::routing::get(memories::list))
        .route("/api/v1/memories", axum::routing::post(memories::create))
        .route("/api/v1/memories/:id", axum::routing::get(memories::get))
        .route("/api/v1/memories/:id", axum::routing::patch(memories::update))
        .route("/api/v1/memories/:id", axum::routing::delete(memories::delete))
}
