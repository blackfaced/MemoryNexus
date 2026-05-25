//! Cognitive Space API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::space::{
    CognitiveSpaceDb, CognitiveSpaceInviteDb, CognitiveSpaceMemberDb, CognitiveSpaceType,
    CreateCognitiveSpace, CreateSpaceInvite, SpaceMemberRole,
};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateSpaceRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub space_type: CognitiveSpaceType,
}

#[derive(Debug, Serialize)]
pub struct SpaceListResponse {
    pub items: Vec<CognitiveSpaceDb>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SpaceMemberListResponse {
    pub items: Vec<CognitiveSpaceMemberDb>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub role: SpaceMemberRole,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInviteRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: SpaceMemberRole,
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
            space_type: req.space_type,
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

/// GET /api/v1/spaces/:id/members - List members visible to a space member.
pub async fn list_members(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<SpaceMemberListResponse>>, AppError> {
    let members = state
        .repositories
        .spaces
        .list_members_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?;

    if members.is_empty() {
        return Err(AppError::Unauthorized);
    }

    Ok(Json(ApiResponse::success(SpaceMemberListResponse {
        total: members.len(),
        items: members,
    })))
}

/// PATCH /api/v1/spaces/:id/members/:user_id - Update a member role.
pub async fn update_member_role(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateMemberRoleRequest>,
) -> Result<Json<ApiResponse<CognitiveSpaceMemberDb>>, AppError> {
    require_member_manager(&state, id, auth_user.user_id).await?;
    if req.role == SpaceMemberRole::Owner {
        return Err(AppError::BadRequest(
            "owner role cannot be assigned through member update".to_string(),
        ));
    }

    let member = state
        .repositories
        .spaces
        .update_member_role(id, user_id, req.role)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("space member not found".to_string()))?;

    Ok(Json(ApiResponse::success(member)))
}

/// POST /api/v1/spaces/:id/invites - Create a one-time invite code.
pub async fn create_invite(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateInviteRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CognitiveSpaceInviteDb>>), AppError> {
    require_member_manager(&state, id, auth_user.user_id).await?;
    if !req.role.is_invitable() {
        return Err(AppError::BadRequest(
            "invite role must be editor or viewer".to_string(),
        ));
    }
    if req.expires_in_days.is_some_and(|days| days <= 0) {
        return Err(AppError::BadRequest(
            "expires_in_days must be positive".to_string(),
        ));
    }

    let invite = state
        .repositories
        .spaces
        .create_invite(CreateSpaceInvite {
            space_id: id,
            code: new_invite_code(),
            role: req.role,
            created_by: auth_user.user_id,
            expires_in_days: req.expires_in_days,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(invite))))
}

/// POST /api/v1/spaces/invites/accept - Accept a one-time invite code.
pub async fn accept_invite(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<AcceptInviteRequest>,
) -> Result<Json<ApiResponse<CognitiveSpaceInviteDb>>, AppError> {
    let code = req.code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest("invite code is required".to_string()));
    }

    let invite = state
        .repositories
        .spaces
        .accept_invite(code, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("invite code not found or expired".to_string()))?;

    Ok(Json(ApiResponse::success(invite)))
}

async fn require_member_manager(
    state: &AppState,
    space_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let member = state
        .repositories
        .spaces
        .find_member(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if member
        .parsed_role()
        .is_some_and(SpaceMemberRole::can_manage_members)
    {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

fn new_invite_code() -> String {
    Uuid::new_v4().simple().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_space_request_deserializes() {
        let json = r#"{"name":"Personal Space","description":"Private","space_type":"family"}"#;
        let req: CreateSpaceRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.name, "Personal Space");
        assert_eq!(req.description, Some("Private".to_string()));
        assert_eq!(req.space_type, CognitiveSpaceType::Family);
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

    #[test]
    fn invite_request_rejects_owner_role_at_validation_layer() {
        let req: CreateInviteRequest = serde_json::from_str(r#"{"role":"owner"}"#).unwrap();
        assert!(!req.role.is_invitable());
    }

    #[test]
    fn invite_code_is_url_safe_uuid_text() {
        let code = new_invite_code();
        assert_eq!(code.len(), 32);
        assert!(code.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
