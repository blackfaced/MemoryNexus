//! 应用状态管理
use sqlx::PgPool;
use std::sync::Arc;

use crate::vector::VectorStore;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub repositories: Repositories,
    pub vector_store: Option<Arc<dyn VectorStore>>,
}

/// 仓储聚合
#[derive(Clone)]
pub struct Repositories {
    pub memories: Arc<dyn super::db::memory::MemoryRepository>,
    pub tags: Arc<dyn super::db::tag::TagRepository>,
    pub users: Arc<dyn super::db::user::UserRepository>,
}

impl AppState {
    pub fn new(
        db: PgPool,
        repositories: Repositories,
        vector_store: Option<Arc<dyn VectorStore>>,
    ) -> Self {
        Self {
            db,
            repositories,
            vector_store,
        }
    }
}
