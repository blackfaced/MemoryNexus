//! JWT 认证

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    AsyncReadExt,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT 配置
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration: Duration,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev_secret_change_in_production".to_string()),
            expiration: Duration::days(7),
        }
    }
}

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,       // user_id
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

impl Claims {
    pub fn new(user_id: Uuid, email: String, expiration: Duration) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id,
            email,
            exp: (now + expiration).timestamp(),
            iat: now.timestamp(),
        }
    }
}

/// JWT 认证器
#[derive(Clone)]
pub struct JwtAuth {
    config: JwtConfig,
}

impl JwtAuth {
    pub fn new(config: JwtConfig) -> Self {
        Self { config }
    }

    /// 生成 Token
    pub fn generate(&self, user_id: Uuid, email: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims::new(user_id, email.to_string(), self.config.expiration);
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.secret.as_bytes()),
        )
    }

    /// 验证 Token
    pub fn verify(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.secret.as_bytes()),
            &Validation::default(),
        )?;
        Ok(token_data.claims)
    }
}

/// 从请求中提取认证信息
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: String,
}

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthenticatedUser {
    type Rejection = crate::error::AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 从 Authorization header 获取 token
        let auth_header = parts.headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(crate::error::AppError::Unauthorized)?;

        // 解析 Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(crate::error::AppError::Unauthorized)?;

        // 验证 token
        let jwt = JwtAuth::default();
        let claims = jwt.verify(token)
            .map_err(|_| crate::error::AppError::Unauthorized)?;

        Ok(AuthenticatedUser {
            user_id: claims.sub,
            email: claims.email,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_claims_creation() {
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let claims = Claims::new(user_id, email.to_string(), Duration::days(7));

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_jwt_generate_and_verify() {
        let jwt = JwtAuth::default();
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = jwt.generate(user_id, email).unwrap();
        let claims = jwt.verify(&token).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
    }

    #[test]
    fn test_jwt_invalid_token() {
        let jwt = JwtAuth::default();
        let result = jwt.verify("invalid_token");
        assert!(result.is_err());
    }
}
