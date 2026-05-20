//! Cognitive Space API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::space::{CognitiveSpaceDb, CreateCognitiveSpace};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateSpaceRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SpaceListResponse {
    pub items: Vec<CognitiveSpaceDb>,
    pub total: usize,
}

/// POST /api/v1/spaces - Create a Cognitive Space
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateSpaceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CognitiveSpaceDb>>), AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("空间名称不能为空".to_string()));
    }

    let space = state
        .repositories
        .spaces
        .create(CreateCognitiveSpace {
            name: name.to_string(),
            description: req.description,
            owner_user_id: auth_user.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(space))))
}

/// GET /api/v1/spaces - List spaces visible to the current user
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
) -> Result<Json<ApiResponse<SpaceListResponse>>, AppError> {
    let spaces = state
        .repositories
        .spaces
        .list_for_user(auth_user.user_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(SpaceListResponse {
        total: spaces.len(),
        items: spaces,
    })))
}

/// GET /api/v1/spaces/:id - Get a space visible to the current user
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CognitiveSpaceDb>>, AppError> {
    let space = state
        .repositories
        .spaces
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::success(space)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_space_request_deserializes() {
        let json = r#"{"name":"Personal Space","description":"Private"}"#;
        let req: CreateSpaceRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.name, "Personal Space");
        assert_eq!(req.description, Some("Private".to_string()));
    }

    #[test]
    fn space_list_response_serializes() {
        let response = SpaceListResponse {
            items: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"items\""));
        assert!(json.contains("\"total\":0"));
    }
}
