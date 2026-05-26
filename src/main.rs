//! MemoryNexus - 家庭AI记忆中心
//!
//! 使用 Axum 构建的高性能 Rust 后端

#![allow(dead_code)] // 项目早期阶段，允许预留但未启用的代码

mod ai;
mod api;
mod auth;
mod db;
mod domain;
mod error;
mod search;
mod state;
mod storage;
mod vector;

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 健康检查端点
async fn health() -> &'static str {
    "OK"
}

/// 创建应用
fn create_app() -> Router<state::AppState> {
    Router::new()
        .route("/health", axum::routing::get(health))
        .merge(api::routes())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("memorynexus=info".parse().unwrap()),
        )
        .init();

    tracing::info!("🚀 MemoryNexus 启动中...");

    // 数据库配置
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/memorynexus".to_string()
    });

    // 初始化数据库连接池
    tracing::info!("📦 连接数据库...");
    let pool = db::init_pool(&database_url).await?;

    // 运行迁移
    tracing::info!("🔄 运行数据库迁移...");
    db::run_migrations(&pool).await?;

    // 创建应用状态
    use ai::embedding::EMBEDDING_DIM;
    use vector::repository::QdrantVectorRepository;
    let vector_store = vector::QdrantVectorStore::from_env()
        .map(|store| Arc::new(store) as Arc<dyn vector::VectorStore>);

    if let Some(store) = vector_store.as_ref() {
        tracing::info!("🧭 初始化 Qdrant collection...");
        store.ensure_collection(EMBEDDING_DIM).await?;
    }

    let vector_repo: Arc<dyn vector::repository::VectorRepository> = match &vector_store {
        Some(store) => Arc::new(QdrantVectorRepository::new(
            store.clone(),
            uuid::Uuid::nil(),
        )),
        None => Arc::new(QdrantVectorRepository::new(
            Arc::new(vector::QdrantVectorStore::new(
                std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string()),
                std::env::var("QDRANT_COLLECTION")
                    .unwrap_or_else(|_| "memorynexus_memories".to_string()),
            )),
            uuid::Uuid::nil(),
        )),
    };

    let repositories = state::Repositories {
        lenses: Arc::new(db::lens::PostgresLensRepository::new(pool.clone())),
        lens_runs: Arc::new(db::lens_run::PostgresLensRunRepository::new(pool.clone())),
        memories: Arc::new(db::memory::PostgresMemoryRepository::new(pool.clone())),
        namespaces: Arc::new(db::namespace::PostgresNamespaceRepository::new(
            pool.clone(),
        )),
        profiles: Arc::new(db::profile::PostgresCognitiveProfileRepository::new(
            pool.clone(),
        )),
        reminders: Arc::new(db::reminder::PostgresReminderRepository::new(pool.clone())),
        review_reports: Arc::new(
            db::review_report::PostgresCognitiveReviewReportRepository::new(pool.clone()),
        ),
        spaces: Arc::new(db::space::PostgresCognitiveSpaceRepository::new(
            pool.clone(),
        )),
        tags: Arc::new(db::tag::PostgresTagRepository::new(pool.clone())),
        users: Arc::new(db::user::PostgresUserRepository::new(pool.clone())),
        vectors: vector_repo,
    };

    let app_state = state::AppState::new(pool, repositories, vector_store);

    // 创建应用
    let app = create_app().with_state(app_state);

    // 监听地址
    let addr = std::env::var("MEMORYNEXUS_BIND_ADDR")
        .ok()
        .and_then(|value| value.parse::<SocketAddr>().ok())
        .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080)));
    tracing::info!("📍 监听地址: http://{}", addr);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
