//! FeedbackLoop API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::{
    CreateFeedbackLoop, FeedbackLoopDb, FeedbackLoopListFilter, PatchFeedbackLoop,
};
use crate::db::space::SpaceMemberRole;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateFeedbackLoopRequest {
    pub space_id: Uuid,
    pub namespace_id: Uuid,
    pub goal: String,
    pub task: String,
    pub attempt: Option<String>,
    pub evaluation: Option<String>,
    pub feedback: Option<String>,
    pub adjustment: Option<String>,
    pub next_task: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListFeedbackLoopsQuery {
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PatchFeedbackLoopRequest {
    pub attempt: Option<String>,
    pub evaluation: Option<String>,
    pub feedback: Option<String>,
    pub adjustment: Option<String>,
    pub next_task: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeedbackLoopListResponse {
    pub items: Vec<FeedbackLoopDb>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
}

pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateFeedbackLoopRequest>,
) -> Result<(StatusCode, Json<ApiResponse<FeedbackLoopDb>>), AppError> {
    require_space_writer(&state, req.space_id, auth_user.user_id).await?;
    require_namespace_in_space(&state, req.namespace_id, req.space_id, auth_user.user_id).await?;

    let goal = normalize_required(&req.goal, "feedback loop goal is required")?;
    let task = normalize_required(&req.task, "feedback loop task is required")?;
    let status = normalize_status(req.status.as_deref())?;

    let feedback_loop = state
        .repositories
        .feedback_loops
        .create(CreateFeedbackLoop {
            space_id: req.space_id,
            namespace_id: req.namespace_id,
            goal,
            task,
            attempt: normalize_optional(req.attempt),
            evaluation: normalize_optional(req.evaluation),
            feedback: normalize_optional(req.feedback),
            adjustment: normalize_optional(req.adjustment),
            next_task: normalize_optional(req.next_task),
            status,
            created_by: auth_user.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(feedback_loop)),
    ))
}

pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<ListFeedbackLoopsQuery>,
) -> Result<Json<ApiResponse<FeedbackLoopListResponse>>, AppError> {
    require_space_member(&state, query.space_id, auth_user.user_id).await?;
    if let Some(namespace_id) = query.namespace_id {
        require_namespace_in_space(&state, namespace_id, query.space_id, auth_user.user_id).await?;
    }

    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let feedback_loops = state
        .repositories
        .feedback_loops
        .list_for_user(
            FeedbackLoopListFilter {
                space_id: query.space_id,
                namespace_id: query.namespace_id,
                limit,
                offset,
            },
            auth_user.user_id,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(FeedbackLoopListResponse {
        total: feedback_loops.len(),
        items: feedback_loops,
        limit,
        offset,
    })))
}

pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<FeedbackLoopDb>>, AppError> {
    let feedback_loop = state
        .repositories
        .feedback_loops
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::success(feedback_loop)))
}

pub async fn patch(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchFeedbackLoopRequest>,
) -> Result<Json<ApiResponse<FeedbackLoopDb>>, AppError> {
    let existing = state
        .repositories
        .feedback_loops
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    require_space_writer(&state, existing.space_id, auth_user.user_id).await?;

    let status = match req.status.as_deref() {
        Some(_) => Some(normalize_status(req.status.as_deref())?),
        None => None,
    };

    let feedback_loop = state
        .repositories
        .feedback_loops
        .patch(
            id,
            PatchFeedbackLoop {
                attempt: normalize_optional(req.attempt),
                evaluation: normalize_optional(req.evaluation),
                feedback: normalize_optional(req.feedback),
                adjustment: normalize_optional(req.adjustment),
                next_task: normalize_optional(req.next_task),
                status,
            },
        )
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("feedback loop not found".to_string()))?;

    Ok(Json(ApiResponse::success(feedback_loop)))
}

async fn require_space_member(
    state: &AppState,
    space_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    state
        .repositories
        .spaces
        .find_member(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .map(|_| ())
        .ok_or(AppError::Unauthorized)
}

async fn require_space_writer(
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

    if role_allows_feedback_loop_write(member.parsed_role()) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

async fn require_namespace_in_space(
    state: &AppState,
    namespace_id: Uuid,
    space_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let namespace = state
        .repositories
        .namespaces
        .find_for_user(namespace_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if namespace_belongs_to_space(namespace.space_id, space_id) {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "feedback loop namespace_id must belong to the same Cognitive Space".to_string(),
        ))
    }
}

fn role_allows_feedback_loop_write(role: Option<SpaceMemberRole>) -> bool {
    role.is_some_and(SpaceMemberRole::can_write)
}

fn namespace_belongs_to_space(namespace_space_id: Uuid, requested_space_id: Uuid) -> bool {
    namespace_space_id == requested_space_id
}

fn normalize_required(value: &str, message: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::BadRequest(message.to_string()))
    } else {
        Ok(value.to_string())
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_status(status: Option<&str>) -> Result<String, AppError> {
    let status = status
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("active")
        .to_ascii_lowercase();

    match status.as_str() {
        "active" | "completed" | "paused" => Ok(status),
        _ => Err(AppError::BadRequest(format!(
            "unsupported feedback loop status: {status}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_feedback_loop_request_deserializes_minimal_payload() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "space_id":"{space_id}",
                "namespace_id":"{namespace_id}",
                "goal":"Improve fraction word problems",
                "task":"Complete five fraction problems"
            }}"#
        );
        let req: CreateFeedbackLoopRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.space_id, space_id);
        assert_eq!(req.namespace_id, namespace_id);
        assert_eq!(req.goal, "Improve fraction word problems");
        assert_eq!(req.task, "Complete five fraction problems");
        assert_eq!(req.status, None);
    }

    #[test]
    fn list_query_supports_namespace_filter_defaults() {
        let namespace_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "space_id":"{}",
                "namespace_id":"{namespace_id}"
            }}"#,
            Uuid::new_v4()
        );
        let query: ListFeedbackLoopsQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(query.namespace_id, Some(namespace_id));
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn patch_feedback_loop_request_deserializes_attempt_without_other_fields() {
        let json = r#"{
            "attempt": "  Solved 3/5 problems and mixed up denominators  "
        }"#;
        let req: PatchFeedbackLoopRequest = serde_json::from_str(json).unwrap();

        assert_eq!(
            req.attempt.as_deref(),
            Some("  Solved 3/5 problems and mixed up denominators  ")
        );
        assert_eq!(req.evaluation, None);
        assert_eq!(req.feedback, None);
        assert_eq!(req.adjustment, None);
        assert_eq!(req.next_task, None);
        assert_eq!(req.status, None);
    }

    #[test]
    fn status_accepts_only_initial_values() {
        assert_eq!(normalize_status(None).unwrap(), "active");
        assert_eq!(normalize_status(Some(" Completed ")).unwrap(), "completed");
        assert_eq!(normalize_status(Some("paused")).unwrap(), "paused");
        assert!(normalize_status(Some("cancelled")).is_err());
    }

    #[test]
    fn goal_and_task_are_required() {
        assert!(normalize_required(" Improve math ", "required").is_ok());
        assert!(normalize_required(" ", "required").is_err());
    }

    #[test]
    fn writer_permission_uses_space_roles() {
        assert!(role_allows_feedback_loop_write(Some(
            SpaceMemberRole::Owner
        )));
        assert!(role_allows_feedback_loop_write(Some(
            SpaceMemberRole::Editor
        )));
        assert!(!role_allows_feedback_loop_write(Some(
            SpaceMemberRole::Viewer
        )));
        assert!(!role_allows_feedback_loop_write(None));
    }

    #[test]
    fn namespace_validation_rejects_cross_space_namespace() {
        let namespace_space_id = Uuid::new_v4();
        let requested_space_id = Uuid::new_v4();

        assert!(namespace_belongs_to_space(
            namespace_space_id,
            namespace_space_id
        ));
        assert!(!namespace_belongs_to_space(
            namespace_space_id,
            requested_space_id
        ));
    }
}
