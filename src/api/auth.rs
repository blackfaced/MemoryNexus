//! 认证 API

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{JwtAuth, PasswordHasher};
use crate::db::user::CreateUser;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// 注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

/// 认证响应
#[derive(Serialize)]
pub struct AuthResponse {
    pub user: UserPublicResponse,
    pub token: String,
}

/// 用户公开信息
#[derive(Serialize)]
pub struct UserPublicResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
}

/// 登录
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, AppError> {
    // 验证输入
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest("邮箱和密码不能为空".to_string()));
    }

    // 查找用户
    let user = state
        .repositories
        .users
        .find_by_email(&req.email)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    // 验证密码
    if !PasswordHasher::verify(&req.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    // 生成 JWT
    let jwt = JwtAuth::default();
    let token = jwt
        .generate(user.id, &user.email)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(AuthResponse {
        user: UserPublicResponse {
            id: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
        },
        token,
    })))
}

/// 注册
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<ApiResponse<AuthResponse>>), AppError> {
    // 验证输入
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest("邮箱和密码不能为空".to_string()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("密码至少6位".to_string()));
    }
    if !req.email.contains('@') {
        return Err(AppError::BadRequest("无效的邮箱格式".to_string()));
    }

    // 检查邮箱是否已存在
    let existing = state
        .repositories
        .users
        .find_by_email(&req.email)
        .await
        .map_err(AppError::Database)?;
    if existing.is_some() {
        return Err(AppError::BadRequest("邮箱已被注册".to_string()));
    }

    // 哈希密码
    let password_hash = PasswordHasher::hash(&req.password)
        .map_err(|e| AppError::Internal(format!("密码哈希失败: {}", e)))?;

    // 创建用户
    let create_user = CreateUser {
        email: req.email.clone(),
        username: req.username.clone(),
        password_hash,
    };

    let user = state
        .repositories
        .users
        .create(create_user)
        .await
        .map_err(AppError::Database)?;

    state
        .repositories
        .spaces
        .ensure_default_for_user(user.id, &user.username)
        .await
        .map_err(AppError::Database)?;

    // 生成 JWT
    let jwt = JwtAuth::default();
    let token = jwt
        .generate(user.id, &user.email)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(AuthResponse {
            user: UserPublicResponse {
                id: user.id,
                email: user.email,
                username: user.username,
            },
            token,
        })),
    ))
}

/// 获取当前用户信息
pub async fn me(
    State(state): State<AppState>,
    auth_user: crate::auth::AuthenticatedUser,
) -> Result<Json<ApiResponse<UserPublicResponse>>, AppError> {
    let user = state
        .repositories
        .users
        .find_by_id(auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound("用户不存在".to_string()))?;

    Ok(Json(ApiResponse::success(UserPublicResponse {
        id: user.id,
        email: user.email,
        username: user.username,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_request_serde() {
        let json = r#"{"email":"test@example.com","password":"password123"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "test@example.com");
        assert_eq!(req.password, "password123");
    }

    #[test]
    fn test_register_request_serde() {
        let json = r#"{"email":"test@example.com","username":"testuser","password":"password123"}"#;
        let req: RegisterRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "test@example.com");
        assert_eq!(req.username, "testuser");
        assert_eq!(req.password, "password123");
    }

    #[test]
    fn test_auth_response_serde() {
        let response = AuthResponse {
            user: UserPublicResponse {
                id: Uuid::new_v4(),
                email: "test@example.com".to_string(),
                username: "testuser".to_string(),
            },
            token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"user\":"));
        assert!(json.contains("\"token\":"));
    }
}
