//! Namespace API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::namespace::{CreateNamespace, NamespaceDb, NamespaceKind, NamespaceStatus};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateNamespaceRequest {
    pub space_id: Uuid,
    pub name: String,
    pub kind: NamespaceKind,
    pub description: Option<String>,
    #[serde(default)]
    pub status: NamespaceStatus,
}

#[derive(Debug, Deserialize)]
pub struct ListNamespacesQuery {
    pub space_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct NamespaceListResponse {
    pub items: Vec<NamespaceDb>,
    pub total: usize,
}

/// POST /api/v1/namespaces - Create a Namespace in an accessible Cognitive Space
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateNamespaceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<NamespaceDb>>), AppError> {
    let space = state
        .repositories
        .spaces
        .find_for_user(req.space_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    let name = normalize_namespace_name(&req.name)?;

    let namespace = state
        .repositories
        .namespaces
        .create(CreateNamespace {
            space_id: space.id,
            name,
            kind: req.kind,
            description: req.description,
            status: req.status,
            created_by: auth_user.user_id,
        })
        .await
        .map_err(map_create_namespace_error)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(namespace))))
}

/// GET /api/v1/namespaces?space_id=<SPACE_ID> - List Namespaces in a Cognitive Space
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<ListNamespacesQuery>,
) -> Result<Json<ApiResponse<NamespaceListResponse>>, AppError> {
    state
        .repositories
        .spaces
        .find_for_user(query.space_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    let namespaces = state
        .repositories
        .namespaces
        .list_for_space(query.space_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(NamespaceListResponse {
        total: namespaces.len(),
        items: namespaces,
    })))
}

/// GET /api/v1/namespaces/:id - Get a Namespace visible to the current user
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<NamespaceDb>>, AppError> {
    let namespace = state
        .repositories
        .namespaces
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::success(namespace)))
}

fn normalize_namespace_name(name: &str) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "Namespace name is required".to_string(),
        ));
    }

    if !name.chars().all(is_namespace_name_char) {
        return Err(AppError::BadRequest(
            "Namespace name may contain lowercase letters, numbers, dots, underscores, and hyphens"
                .to_string(),
        ));
    }

    if name.starts_with('.') || name.ends_with('.') || name.contains("..") {
        return Err(AppError::BadRequest(
            "Namespace name must use non-empty dotted segments".to_string(),
        ));
    }

    Ok(name.to_string())
}

fn is_namespace_name_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
}

fn map_create_namespace_error(error: sqlx::Error) -> AppError {
    if is_unique_violation(&error) {
        AppError::BadRequest("Namespace name already exists in this space".to_string())
    } else {
        AppError::Database(error)
    }
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(database_error) => {
            database_error.code().as_deref() == Some("23505")
                || database_error.constraint() == Some("namespaces_space_id_name_key")
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_namespace_request_accepts_reflective_and_skill_kind() {
        let space_id = Uuid::new_v4();
        let reflective = format!(
            r#"{{
                "space_id":"{space_id}",
                "name":"personal.thoughts",
                "kind":"reflective"
            }}"#
        );
        let skill = format!(
            r#"{{
                "space_id":"{space_id}",
                "name":"learning.math",
                "kind":"skill",
                "status":"active"
            }}"#
        );

        let reflective: CreateNamespaceRequest = serde_json::from_str(&reflective).unwrap();
        let skill: CreateNamespaceRequest = serde_json::from_str(&skill).unwrap();

        assert_eq!(reflective.kind, NamespaceKind::Reflective);
        assert_eq!(reflective.status, NamespaceStatus::Active);
        assert_eq!(skill.kind, NamespaceKind::Skill);
    }

    #[test]
    fn create_namespace_request_rejects_unknown_kind() {
        let space_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "space_id":"{space_id}",
                "name":"project.memorynexus",
                "kind":"project"
            }}"#
        );

        assert!(serde_json::from_str::<CreateNamespaceRequest>(&json).is_err());
    }

    #[test]
    fn namespace_name_validation_trims_and_requires_stable_identifier() {
        assert_eq!(
            normalize_namespace_name(" learning.math ").unwrap(),
            "learning.math"
        );
        assert!(normalize_namespace_name("").is_err());
        assert!(normalize_namespace_name("Learning.Math").is_err());
        assert!(normalize_namespace_name("learning..math").is_err());
        assert!(normalize_namespace_name(".learning").is_err());
    }

    #[test]
    fn namespace_list_response_serializes_items_and_total() {
        let response = NamespaceListResponse {
            items: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"items\""));
        assert!(json.contains("\"total\":0"));
    }
}
