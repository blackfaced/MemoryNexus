//! API 路由模块

use axum::Router;

mod auth;
mod health;
mod memories;
mod search;
mod tags;
mod upload;

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
        // 标签 CRUD
        .route("/api/v1/tags", axum::routing::get(tags::list))
        .route("/api/v1/tags", axum::routing::post(tags::create))
        .route("/api/v1/tags/:id", axum::routing::get(tags::get))
        .route("/api/v1/tags/:id", axum::routing::patch(tags::update))
        .route("/api/v1/tags/:id", axum::routing::delete(tags::delete))
        // 搜索
        .route("/api/v1/search", axum::routing::get(search::search))
        .route("/api/v1/search/suggest", axum::routing::get(search::suggest))
        // 文件上传 (预留)
        // .route("/api/v1/upload", axum::routing::post(upload::upload))
        // .route("/api/v1/media/:key", axum::routing::get(upload::get_media))
}
