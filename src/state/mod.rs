//! 应用状态管理
use sqlx::PgPool;
use std::sync::Arc;

use crate::ai::embedding::{Embedder, LocalHashEmbedder, OpenAIEmbedder};
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
        let local_embedding = std::env::var("MEMORYNEXUS_EMBEDDING_PROVIDER")
            .or_else(|_| std::env::var("EMBEDDING_PROVIDER"))
            .map(|provider| provider.eq_ignore_ascii_case("local"))
            .unwrap_or(false);
        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
        let embedder = if local_embedding {
            Some(Arc::new(LocalHashEmbedder::default()) as Arc<dyn Embedder>)
        } else {
            openai_api_key.clone().map(|key| {
                let model = std::env::var("OPENAI_EMBEDDING_MODEL")
                    .or_else(|_| std::env::var("EMBEDDING_MODEL"))
                    .unwrap_or_else(|_| "text-embedding-ada-002".to_string());
                Arc::new(OpenAIEmbedder::new(key).with_model(model)) as Arc<dyn Embedder>
            })
        };

        Self {
            openai_api_key,
            embedder,
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
    pub spaces: Arc<dyn super::db::space::CognitiveSpaceRepository>,
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
