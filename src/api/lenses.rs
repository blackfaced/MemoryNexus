//! Lens API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::lens::{CreateLens, LensDb};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateLensRequest {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub strategy: Option<String>,
    pub output_format: Option<String>,
    pub retrieval_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListLensesQuery {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct LensListResponse {
    pub items: Vec<LensDb>,
    pub total: usize,
}

/// POST /api/v1/lenses - Create a Lens in a Cognitive Space
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateLensRequest>,
) -> Result<(StatusCode, Json<ApiResponse<LensDb>>), AppError> {
    let space = state
        .repositories
        .spaces
        .find_for_user(req.space_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Lens 名称不能为空".to_string()));
    }

    let lens = state
        .repositories
        .lenses
        .create(CreateLens {
            space_id: space.id,
            namespace_id: validate_namespace(&state, auth_user.user_id, space.id, req.namespace_id)
                .await?,
            name: name.to_string(),
            description: req.description,
            strategy: normalize_optional(req.strategy, "default"),
            output_format: normalize_optional(req.output_format, "summary"),
            retrieval_mode: normalize_optional(req.retrieval_mode, "semantic"),
            created_by: auth_user.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(lens))))
}

/// GET /api/v1/lenses?space_id=<SPACE_ID> - List Lenses in a Cognitive Space
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<ListLensesQuery>,
) -> Result<Json<ApiResponse<LensListResponse>>, AppError> {
    state
        .repositories
        .spaces
        .find_for_user(query.space_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    let lenses = state
        .repositories
        .lenses
        .list_for_space(
            query.space_id,
            auth_user.user_id,
            validate_namespace(
                &state,
                auth_user.user_id,
                query.space_id,
                query.namespace_id,
            )
            .await?,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(LensListResponse {
        total: lenses.len(),
        items: lenses,
    })))
}

/// GET /api/v1/lenses/:id - Get a Lens visible to the current user
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<LensDb>>, AppError> {
    let lens = state
        .repositories
        .lenses
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::success(lens)))
}

fn normalize_optional(value: Option<String>, default: &str) -> String {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}

async fn validate_namespace(
    state: &AppState,
    user_id: Uuid,
    space_id: Uuid,
    namespace_id: Option<Uuid>,
) -> Result<Option<Uuid>, AppError> {
    let Some(namespace_id) = namespace_id else {
        return Ok(None);
    };
    let namespace = state
        .repositories
        .namespaces
        .find_for_user(namespace_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;
    if namespace.space_id != space_id {
        return Err(AppError::BadRequest(
            "namespace_id must belong to the requested Cognitive Space".to_string(),
        ));
    }
    Ok(Some(namespace_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lens_request_deserializes() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "space_id":"{space_id}",
                "namespace_id":"{namespace_id}",
                "name":"Project Context",
                "description":"Project interpretation",
                "strategy":"project_context",
                "output_format":"brief",
                "retrieval_mode":"semantic"
            }}"#
        );
        let req: CreateLensRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.space_id, space_id);
        assert_eq!(req.namespace_id, Some(namespace_id));
        assert_eq!(req.name, "Project Context");
        assert_eq!(req.strategy, Some("project_context".to_string()));
    }

    #[test]
    fn list_lenses_query_accepts_namespace_filter() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let query: ListLensesQuery = serde_json::from_value(serde_json::json!({
            "space_id": space_id,
            "namespace_id": namespace_id
        }))
        .unwrap();

        assert_eq!(query.space_id, space_id);
        assert_eq!(query.namespace_id, Some(namespace_id));
    }

    #[test]
    fn lens_list_response_serializes() {
        let response = LensListResponse {
            items: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"items\""));
        assert!(json.contains("\"total\":0"));
    }
}
