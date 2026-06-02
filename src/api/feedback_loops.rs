//! FeedbackLoop API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::memories::index_memory_embedding;
use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::{
    CreateFeedbackLoop, FeedbackLoopDb, FeedbackLoopListFilter, PatchFeedbackLoop,
};
use crate::db::memory::{CreateMemory, MemoryType};
use crate::db::space::SpaceMemberRole;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

const FEEDBACK_LOOP_MEMORY_SOURCE_TYPE: &str = "feedback_loop_event";

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
    #[serde(default, alias = "create_memory_snapshot")]
    pub capture_memory: Option<bool>,
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
    #[serde(default, alias = "create_memory_snapshot")]
    pub capture_memory: Option<bool>,
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

    if capture_memory_requested(req.capture_memory) {
        capture_feedback_loop_memory(&state, &feedback_loop, auth_user.user_id, "create").await?;
    }

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

    let capture_memory =
        capture_memory_requested(req.capture_memory) && patch_has_meaningful_practice_content(&req);

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

    if capture_memory {
        capture_feedback_loop_memory(&state, &feedback_loop, auth_user.user_id, "patch").await?;
    }

    Ok(Json(ApiResponse::success(feedback_loop)))
}

async fn capture_feedback_loop_memory(
    state: &AppState,
    feedback_loop: &FeedbackLoopDb,
    user_id: Uuid,
    event_kind: &str,
) -> Result<Option<crate::db::memory::MemoryDb>, AppError> {
    let Some(snapshot) = feedback_loop_memory_snapshot(feedback_loop, user_id, event_kind) else {
        return Ok(None);
    };

    let memory = state
        .repositories
        .memories
        .create(snapshot)
        .await
        .map_err(AppError::Database)?;
    index_memory_embedding(state, &memory).await;

    Ok(Some(memory))
}

fn feedback_loop_memory_snapshot(
    feedback_loop: &FeedbackLoopDb,
    user_id: Uuid,
    event_kind: &str,
) -> Option<CreateMemory> {
    let practice_fields = [
        ("goal", "Practice goal", Some(feedback_loop.goal.as_str())),
        ("task", "Practice task", Some(feedback_loop.task.as_str())),
        (
            "attempt",
            "Answer / reasoning",
            feedback_loop.attempt.as_deref(),
        ),
        (
            "evaluation",
            "Mistake pattern / evaluation",
            feedback_loop.evaluation.as_deref(),
        ),
        ("feedback", "Feedback", feedback_loop.feedback.as_deref()),
        (
            "adjustment",
            "Practice adjustment",
            feedback_loop.adjustment.as_deref(),
        ),
        (
            "next_task",
            "Next exercise",
            feedback_loop.next_task.as_deref(),
        ),
    ];

    let mut included_fields = Vec::new();
    let mut lines = Vec::new();
    for (field_name, label, value) in practice_fields {
        let Some(value) = normalized_snapshot_value(value) else {
            continue;
        };
        included_fields.push(field_name);
        lines.push(format!("{label}: {value}"));
    }

    if lines.is_empty() {
        return None;
    }

    Some(CreateMemory {
        user_id,
        space_id: feedback_loop.space_id,
        title: Some("Practice snapshot".to_string()),
        content: lines.join("\n"),
        memory_type: MemoryType::Text,
        file_path: None,
        is_shared: false,
        source_type: FEEDBACK_LOOP_MEMORY_SOURCE_TYPE.to_string(),
        source_metadata: serde_json::json!({
            "feedback_loop_id": feedback_loop.id,
            "namespace_id": feedback_loop.namespace_id,
            "space_id": feedback_loop.space_id,
            "event_kind": event_kind,
            "included_fields": included_fields,
        }),
        tags: vec!["feedback-loop".to_string()],
    })
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

fn capture_memory_requested(value: Option<bool>) -> bool {
    value.unwrap_or(false)
}

fn patch_has_meaningful_practice_content(req: &PatchFeedbackLoopRequest) -> bool {
    [
        req.attempt.as_deref(),
        req.evaluation.as_deref(),
        req.feedback.as_deref(),
        req.adjustment.as_deref(),
        req.next_task.as_deref(),
    ]
    .into_iter()
    .any(|value| normalized_snapshot_value(value).is_some())
}

fn normalized_snapshot_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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
        assert_eq!(req.capture_memory, None);
    }

    #[test]
    fn create_feedback_loop_request_deserializes_memory_capture_opt_in() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "space_id":"{space_id}",
                "namespace_id":"{namespace_id}",
                "goal":"Improve fraction word problems",
                "task":"Complete five fraction problems",
                "capture_memory": true
            }}"#
        );
        let req: CreateFeedbackLoopRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.capture_memory, Some(true));
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
        assert_eq!(req.capture_memory, None);
    }

    #[test]
    fn patch_feedback_loop_request_deserializes_memory_capture_opt_in() {
        let req: PatchFeedbackLoopRequest = serde_json::from_str(
            r#"{
                "attempt": "Child mixed units before calculating",
                "capture_memory": true
            }"#,
        )
        .unwrap();

        assert_eq!(req.capture_memory, Some(true));
    }

    #[test]
    fn feedback_loop_create_snapshot_uses_parent_friendly_summary_and_traceable_metadata() {
        let feedback_loop_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let feedback_loop = FeedbackLoopDb {
            id: feedback_loop_id,
            space_id,
            namespace_id,
            goal: "Improve fraction word problems".to_string(),
            task: "Solve five fraction word problems".to_string(),
            attempt: None,
            evaluation: Some("3/5 correct; units were mixed".to_string()),
            feedback: Some("Label units before calculating".to_string()),
            adjustment: None,
            next_task: Some("Try three unit-conversion fraction problems".to_string()),
            status: "active".to_string(),
            created_by: user_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let memory = feedback_loop_memory_snapshot(&feedback_loop, user_id, "create").unwrap();

        assert_eq!(memory.space_id, space_id);
        assert_eq!(memory.user_id, user_id);
        assert_eq!(memory.memory_type, crate::db::memory::MemoryType::Text);
        assert!(!memory.is_shared);
        assert_eq!(memory.source_type, "feedback_loop_event");
        assert!(memory
            .content
            .contains("Practice goal: Improve fraction word problems"));
        assert!(memory
            .content
            .contains("Practice task: Solve five fraction word problems"));
        assert!(memory
            .content
            .contains("Mistake pattern / evaluation: 3/5 correct; units were mixed"));
        assert!(memory
            .content
            .contains("Feedback: Label units before calculating"));
        assert!(memory
            .content
            .contains("Next exercise: Try three unit-conversion fraction problems"));
        assert_eq!(
            memory.source_metadata["feedback_loop_id"],
            feedback_loop_id.to_string()
        );
        assert_eq!(
            memory.source_metadata["namespace_id"],
            namespace_id.to_string()
        );
        assert_eq!(memory.source_metadata["space_id"], space_id.to_string());
        assert_eq!(memory.source_metadata["event_kind"], "create");
        assert_eq!(memory.source_metadata["included_fields"][0], "goal");
    }

    #[test]
    fn feedback_loop_patch_snapshot_includes_attempt_feedback_and_omits_absent_fields() {
        let user_id = Uuid::new_v4();
        let feedback_loop = FeedbackLoopDb {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: Uuid::new_v4(),
            goal: "Improve fraction word problems".to_string(),
            task: "Solve five fraction word problems".to_string(),
            attempt: Some("Child added denominators directly".to_string()),
            evaluation: Some("Needs common-denominator step".to_string()),
            feedback: Some("Find a common denominator first".to_string()),
            adjustment: None,
            next_task: None,
            status: "active".to_string(),
            created_by: user_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let memory = feedback_loop_memory_snapshot(&feedback_loop, user_id, "patch").unwrap();

        assert!(memory
            .content
            .contains("Answer / reasoning: Child added denominators directly"));
        assert!(memory
            .content
            .contains("Mistake pattern / evaluation: Needs common-denominator step"));
        assert!(memory
            .content
            .contains("Feedback: Find a common denominator first"));
        assert!(!memory.content.contains("Next exercise:"));
        assert_eq!(memory.source_metadata["event_kind"], "patch");
    }

    #[test]
    fn feedback_loop_snapshot_skips_empty_practice_content() {
        let user_id = Uuid::new_v4();
        let feedback_loop = FeedbackLoopDb {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: Uuid::new_v4(),
            goal: " ".to_string(),
            task: "\n".to_string(),
            attempt: None,
            evaluation: Some(" ".to_string()),
            feedback: None,
            adjustment: None,
            next_task: None,
            status: "active".to_string(),
            created_by: user_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert!(feedback_loop_memory_snapshot(&feedback_loop, user_id, "patch").is_none());
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
