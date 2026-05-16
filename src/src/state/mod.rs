//! 应用状态管理
use std::sync::Arc;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    // TODO: 添加更多共享状态
    // pub redis: redis::Client,
    // pub storage: Arc<dyn StorageBackend>,
}
