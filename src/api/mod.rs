//! API 路由模块

use axum::Router;

use crate::state::AppState;

mod ai;
mod auth;
mod embedding;
mod health;
mod lens_runs;
mod lenses;
mod memories;
mod search;
mod semantic;
mod spaces;
mod tags;
mod upload;

/// 聚合所有 API 路由
pub fn routes() -> Router<AppState> {
    Router::new()
        // 健康检查
        .route("/api/v1/health", axum::routing::get(health::check))
        // 认证
        .route("/api/v1/auth/login", axum::routing::post(auth::login))
        .route("/api/v1/auth/register", axum::routing::post(auth::register))
        .route("/api/v1/auth/me", axum::routing::get(auth::me))
        // Cognitive Space
        .route("/api/v1/spaces", axum::routing::get(spaces::list))
        .route("/api/v1/spaces", axum::routing::post(spaces::create))
        .route("/api/v1/spaces/:id", axum::routing::get(spaces::get))
        // Lens
        .route("/api/v1/lenses", axum::routing::get(lenses::list))
        .route("/api/v1/lenses", axum::routing::post(lenses::create))
        .route("/api/v1/lenses/:id", axum::routing::get(lenses::get))
        .route("/api/v1/lens-runs", axum::routing::get(lens_runs::list))
        .route("/api/v1/lens-runs", axum::routing::post(lens_runs::create))
        .route("/api/v1/lens-runs/:id", axum::routing::get(lens_runs::get))
        // 记忆 CRUD
        .route("/api/v1/memories", axum::routing::get(memories::list))
        .route("/api/v1/memories", axum::routing::post(memories::create))
        .route("/api/v1/memories/:id", axum::routing::get(memories::get))
        .route(
            "/api/v1/memories/:id",
            axum::routing::patch(memories::update),
        )
        .route(
            "/api/v1/memories/:id",
            axum::routing::delete(memories::delete),
        )
        // 标签 CRUD
        .route("/api/v1/tags", axum::routing::get(tags::list))
        .route("/api/v1/tags", axum::routing::post(tags::create))
        .route("/api/v1/tags/:id", axum::routing::get(tags::get))
        .route("/api/v1/tags/:id", axum::routing::patch(tags::update))
        .route("/api/v1/tags/:id", axum::routing::delete(tags::delete))
        // 搜索
        .route("/api/v1/search", axum::routing::get(search::search))
        .route(
            "/api/v1/search/suggest",
            axum::routing::get(search::suggest),
        )
        // AI 功能
        .route("/api/v1/ai/summarize", axum::routing::post(ai::summarize))
        .route("/api/v1/ai/autotag", axum::routing::post(ai::auto_tag))
        .route("/api/v1/ai/config", axum::routing::get(ai::get_config))
        .route(
            "/api/v1/memories/:id/summarize",
            axum::routing::post(ai::summarize_memory),
        )
        // 向量管理
        .route(
            "/api/v1/embeddings",
            axum::routing::post(embedding::create_embedding),
        )
        .route(
            "/api/v1/embeddings/batch",
            axum::routing::post(embedding::batch_create_embeddings),
        )
        .route(
            "/api/v1/embeddings/:id",
            axum::routing::delete(embedding::delete_embeddings),
        )
        .route(
            "/api/v1/embeddings/:id",
            axum::routing::get(embedding::check_embedding),
        )
        // 语义搜索
        .route(
            "/api/v1/search/semantic",
            axum::routing::post(semantic::semantic_search),
        )
        .route(
            "/api/v1/search/semantic",
            axum::routing::get(semantic::semantic_search_get),
        )
    // 文件上传 (预留)
    // .route("/api/v1/upload", axum::routing::post(upload::upload))
    // .route("/api/v1/media/:key", axum::routing::get(upload::get_media))
}
