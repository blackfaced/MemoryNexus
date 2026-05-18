//! 应用状态管理
use sqlx::PgPool;
use std::sync::Arc;

use crate::ai::embedding::{Embedder, OpenAIEmbedder};
use crate::vector::repository::VectorRepository;
use crate::vector::VectorStore;

/// AI 配置
#[derive(Clone)]
pub struct AiConfig {
    pub openai_api_key: Option<String>,
    pub embedder: Option<Arc<dyn Embedder>>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            embedder: std::env::var("OPENAI_API_KEY")
                .ok()
                .map(|key| Arc::new(OpenAIEmbedder::new(key)) as Arc<dyn Embedder>),
        }
    }
}

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub repositories: Repositories,
    pub vector_store: Option<Arc<dyn VectorStore>>,
    pub ai: AiConfig,
}

/// 仓储聚合
#[derive(Clone)]
pub struct Repositories {
    pub memories: Arc<dyn super::db::memory::MemoryRepository>,
    pub tags: Arc<dyn super::db::tag::TagRepository>,
    pub users: Arc<dyn super::db::user::UserRepository>,
    pub vectors: Arc<dyn VectorRepository>,
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
            ai: AiConfig::default(),
        }
    }
}
