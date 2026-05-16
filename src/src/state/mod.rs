//! 应用状态管理
use std::sync::Arc;
use sqlx::PgPool;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub repositories: Repositories,
}

/// 仓储聚合
#[derive(Clone)]
pub struct Repositories {
    pub memories: Arc<dyn super::db::memory::MemoryRepository>,
    pub users: Arc<dyn super::db::user::UserRepository>,
}

impl AppState {
    pub fn new(db: PgPool, repositories: Repositories) -> Self {
        Self { db, repositories }
    }
}
