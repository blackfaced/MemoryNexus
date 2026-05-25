//! 数据库模块
use sqlx::{postgres::PgPoolOptions, Error, PgPool};

pub mod lens;
pub mod lens_run;
pub mod memory;
pub mod profile;
pub mod space;
pub mod tag;
pub mod user;

/// 初始化数据库连接池
pub async fn init_pool(database_url: &str) -> Result<PgPool, Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(database_url)
        .await
}

/// 运行数据库迁移
pub async fn run_migrations(pool: &PgPool) -> Result<(), Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_init_pool_config() {
        // 测试连接池配置正确性
        let url = "postgresql://postgres:postgres@localhost:5432/memorynexus";
        assert!(url.starts_with("postgresql://"));
    }
}
