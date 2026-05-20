//! 用户数据库操作

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

/// 用户数据库模型
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserDb {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub family_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 用户公开信息（不含敏感字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub family_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl From<UserDb> for UserPublic {
    fn from(user: UserDb) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            avatar_url: user.avatar_url,
            family_id: user.family_id,
            created_at: user.created_at,
        }
    }
}

/// 创建用户参数
#[derive(Debug, Clone)]
pub struct CreateUser {
    pub email: String,
    pub username: String,
    pub password_hash: String,
}

/// 用户仓储 trait
#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: CreateUser) -> Result<UserDb, Error>;
    async fn find_by_email(&self, email: &str) -> Result<Option<UserDb>, Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserDb>, Error>;
}

/// PostgreSQL 用户仓储实现
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, user: CreateUser) -> Result<UserDb, Error> {
        sqlx::query_as::<_, UserDb>(
            r#"
            INSERT INTO users (email, username, password_hash)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(&user.email)
        .bind(&user.username)
        .bind(&user.password_hash)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDb>, Error> {
        sqlx::query_as::<_, UserDb>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserDb>, Error> {
        sqlx::query_as::<_, UserDb>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_public_from_userdb() {
        let user = UserDb {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password_hash: "secret_hash".to_string(),
            avatar_url: None,
            family_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let public: UserPublic = user.clone().into();

        assert_eq!(public.email, user.email);
        assert_eq!(public.username, user.username);
        // password_hash 不应该出现在 UserPublic 中
    }

    #[test]
    fn test_create_user_validation() {
        let user = CreateUser {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password_hash: "hashed_password".to_string(),
        };

        assert!(user.email.contains('@'));
        assert!(!user.username.is_empty());
        assert!(!user.password_hash.is_empty());
    }
}
