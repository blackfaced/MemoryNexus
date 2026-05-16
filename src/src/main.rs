//! MemoryNexus - 家庭AI记忆中心
//!
//! 使用 Axum 构建的高性能 Rust 后端

mod api;
mod error;
mod state;

use std::net::SocketAddr;
use axum::{Router, routing::get};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// 健康检查端点
async fn health() -> &'static str {
    "OK"
}

/// 创建应用
fn create_app() -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(api::routes())
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("memorynexus=info".parse().unwrap()))
        .init();

    tracing::info!("🚀 MemoryNexus 启动中...");

    // 创建应用
    let app = create_app();

    // 监听地址
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("📍 监听地址: http://{}", addr);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
