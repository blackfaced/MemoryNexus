//! Namespace-driven practice session API with learning.math compatibility routes.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::feedback_loops::{
    capture_memory_requested, feedback_loop_memory_snapshot, normalize_optional,
    normalize_required, normalize_status, require_namespace_in_space, require_space_member,
    require_space_writer,
};
use crate::api::memories::index_memory_embedding;
use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::{
    CreateFeedbackLoop, FeedbackLoopDb, FeedbackLoopListFilter, FeedbackLoopWithMemorySnapshot,
    PatchFeedbackLoop,
};
use crate::db::namespace::{CreateNamespace, NamespaceDb, NamespaceKind, NamespaceStatus};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

const LEARNING_MATH_NAMESPACE: &str = "learning.math";
const LEARNING_MATH_DESCRIPTION: &str = "Parent-assisted elementary math practice feedback loop";

#[derive(Debug, Deserialize)]
pub struct CreatePracticeSessionRequest {
    pub space_id: Option<Uuid>,
    pub namespace_id: Option<Uuid>,
    #[serde(alias = "goal")]
    pub practice_goal: String,
    #[serde(alias = "task")]
    pub exercise: String,
    #[serde(alias = "attempt")]
    pub answer: Option<String>,
    #[serde(alias = "evaluation")]
    pub mistake_pattern: Option<String>,
    pub feedback: Option<String>,
    #[serde(alias = "adjustment")]
    pub practice_adjustment: Option<String>,
    #[serde(alias = "next_task")]
    pub next_exercise: Option<String>,
    pub status: Option<String>,
    #[serde(default, alias = "create_memory_snapshot")]
    pub capture_memory: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListPracticeSessionsQuery {
    pub space_id: Option<Uuid>,
    pub namespace_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PatchPracticeAttemptRequest {
    #[serde(alias = "attempt")]
    pub answer: Option<String>,
    #[serde(default, alias = "create_memory_snapshot")]
    pub capture_memory: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PatchPracticeFeedbackRequest {
    #[serde(alias = "evaluation")]
    pub mistake_pattern: Option<String>,
    pub feedback: Option<String>,
    #[serde(alias = "adjustment")]
    pub practice_adjustment: Option<String>,
    #[serde(alias = "next_task")]
    pub next_exercise: Option<String>,
    pub status: Option<String>,
    #[serde(default, alias = "create_memory_snapshot")]
    pub capture_memory: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PracticeSessionResponse {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Uuid,
    pub practice_goal: String,
    pub exercise: String,
    pub answer: Option<String>,
    pub mistake_pattern: Option<String>,
    pub feedback: Option<String>,
    pub practice_adjustment: Option<String>,
    pub next_exercise: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PracticeSessionListResponse {
    pub items: Vec<PracticeSessionResponse>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
}

pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreatePracticeSessionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<PracticeSessionResponse>>), AppError> {
    let space_id = req
        .space_id
        .ok_or_else(|| AppError::BadRequest("space_id is required".to_string()))?;
    require_space_writer(&state, space_id, auth_user.user_id).await?;
    let namespace =
        ensure_learning_math_namespace(&state, space_id, req.namespace_id, auth_user.user_id)
            .await?;

    create_practice_session(&state, auth_user.user_id, space_id, namespace.id, req).await
}

pub async fn create_in_namespace(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(namespace_id): Path<Uuid>,
    Json(req): Json<CreatePracticeSessionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<PracticeSessionResponse>>), AppError> {
    let namespace = require_practice_namespace(&state, namespace_id, auth_user.user_id).await?;
    require_space_writer(&state, namespace.space_id, auth_user.user_id).await?;
    if let Some(space_id) = req.space_id {
        require_matching_space_id(space_id, namespace.space_id)?;
    }

    create_practice_session(
        &state,
        auth_user.user_id,
        namespace.space_id,
        namespace.id,
        req,
    )
    .await
}

async fn create_practice_session(
    state: &AppState,
    user_id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
    req: CreatePracticeSessionRequest,
) -> Result<(StatusCode, Json<ApiResponse<PracticeSessionResponse>>), AppError> {
    let goal = normalize_required(&req.practice_goal, "practice goal is required")?;
    let task = normalize_required(&req.exercise, "exercise is required")?;
    let attempt = normalize_optional(req.answer);
    let evaluation = normalize_optional(req.mistake_pattern);
    let feedback = normalize_optional(req.feedback);
    let adjustment = normalize_optional(req.practice_adjustment);
    let next_task = normalize_optional(req.next_exercise);
    let status = normalize_status(req.status.as_deref())?;

    let snapshot = capture_memory_requested(req.capture_memory).then(|| {
        feedback_loop_memory_snapshot(
            user_id,
            "create",
            [
                ("goal", "Practice goal", Some(goal.as_str())),
                ("task", "Practice task", Some(task.as_str())),
                ("attempt", "Answer / reasoning", attempt.as_deref()),
                (
                    "evaluation",
                    "Mistake pattern / evaluation",
                    evaluation.as_deref(),
                ),
                ("feedback", "Feedback", feedback.as_deref()),
                ("adjustment", "Practice adjustment", adjustment.as_deref()),
                ("next_task", "Next exercise", next_task.as_deref()),
            ],
        )
    });

    let create_feedback_loop = CreateFeedbackLoop {
        space_id,
        namespace_id,
        goal,
        task,
        attempt,
        evaluation,
        feedback,
        adjustment,
        next_task,
        status,
        created_by: user_id,
    };

    let result = match snapshot.flatten() {
        Some(snapshot) => state
            .repositories
            .feedback_loops
            .create_with_memory_snapshot(create_feedback_loop, snapshot)
            .await
            .map_err(AppError::Database)?,
        None => FeedbackLoopWithMemorySnapshot {
            feedback_loop: state
                .repositories
                .feedback_loops
                .create(create_feedback_loop)
                .await
                .map_err(AppError::Database)?,
            memory: None,
        },
    };

    if let Some(memory) = result.memory.as_ref() {
        index_memory_embedding(state, memory).await;
    }

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(PracticeSessionResponse::from(
            result.feedback_loop,
        ))),
    ))
}

pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<ListPracticeSessionsQuery>,
) -> Result<Json<ApiResponse<PracticeSessionListResponse>>, AppError> {
    let space_id = query
        .space_id
        .ok_or_else(|| AppError::BadRequest("space_id is required".to_string()))?;
    require_space_member(&state, space_id, auth_user.user_id).await?;
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let Some(namespace) =
        find_learning_math_namespace(&state, space_id, query.namespace_id, auth_user.user_id)
            .await?
    else {
        return Ok(Json(ApiResponse::success(PracticeSessionListResponse {
            items: vec![],
            total: 0,
            limit,
            offset,
        })));
    };

    list_practice_sessions(
        &state,
        space_id,
        namespace.id,
        Some(limit),
        Some(offset),
        auth_user.user_id,
    )
    .await
}

pub async fn list_in_namespace(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(namespace_id): Path<Uuid>,
    Query(query): Query<ListPracticeSessionsQuery>,
) -> Result<Json<ApiResponse<PracticeSessionListResponse>>, AppError> {
    let namespace = require_practice_namespace(&state, namespace_id, auth_user.user_id).await?;
    require_space_member(&state, namespace.space_id, auth_user.user_id).await?;
    if let Some(space_id) = query.space_id {
        require_matching_space_id(space_id, namespace.space_id)?;
    }

    list_practice_sessions(
        &state,
        namespace.space_id,
        namespace.id,
        query.limit,
        query.offset,
        auth_user.user_id,
    )
    .await
}

pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let feedback_loop = find_learning_math_session(&state, id, auth_user.user_id).await?;

    Ok(Json(ApiResponse::success(PracticeSessionResponse::from(
        feedback_loop,
    ))))
}

pub async fn get_in_namespace(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path((namespace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let feedback_loop =
        find_practice_session_in_namespace(&state, namespace_id, id, auth_user.user_id).await?;

    Ok(Json(ApiResponse::success(PracticeSessionResponse::from(
        feedback_loop,
    ))))
}

pub async fn patch_attempt(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchPracticeAttemptRequest>,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let existing = find_learning_math_session(&state, id, auth_user.user_id).await?;
    require_space_writer(&state, existing.space_id, auth_user.user_id).await?;
    require_learning_math_namespace(
        &state,
        existing.namespace_id,
        existing.space_id,
        auth_user.user_id,
    )
    .await?;

    patch_practice_attempt(&state, auth_user.user_id, id, req).await
}

async fn patch_practice_attempt(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: PatchPracticeAttemptRequest,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let attempt = normalize_optional(req.answer);
    let patch = PatchFeedbackLoop {
        attempt,
        evaluation: None,
        feedback: None,
        adjustment: None,
        next_task: None,
        status: None,
    };
    let snapshot = capture_memory_requested(req.capture_memory).then(|| {
        feedback_loop_memory_snapshot(
            user_id,
            "patch",
            [
                ("attempt", "Answer / reasoning", patch.attempt.as_deref()),
                ("evaluation", "Mistake pattern / evaluation", None),
                ("feedback", "Feedback", None),
                ("adjustment", "Practice adjustment", None),
                ("next_task", "Next exercise", None),
            ],
        )
    });

    patch_session(state, id, patch, snapshot.flatten()).await
}

pub async fn patch_attempt_in_namespace(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path((namespace_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PatchPracticeAttemptRequest>,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let existing =
        find_practice_session_in_namespace(&state, namespace_id, id, auth_user.user_id).await?;
    require_space_writer(&state, existing.space_id, auth_user.user_id).await?;
    patch_practice_attempt(&state, auth_user.user_id, id, req).await
}

pub async fn patch_feedback(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchPracticeFeedbackRequest>,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let existing = find_learning_math_session(&state, id, auth_user.user_id).await?;
    require_space_writer(&state, existing.space_id, auth_user.user_id).await?;
    require_learning_math_namespace(
        &state,
        existing.namespace_id,
        existing.space_id,
        auth_user.user_id,
    )
    .await?;

    patch_practice_feedback(&state, auth_user.user_id, id, req).await
}

async fn patch_practice_feedback(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    req: PatchPracticeFeedbackRequest,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let status = match req.status.as_deref() {
        Some(_) => Some(normalize_status(req.status.as_deref())?),
        None => None,
    };
    let patch = PatchFeedbackLoop {
        attempt: None,
        evaluation: normalize_optional(req.mistake_pattern),
        feedback: normalize_optional(req.feedback),
        adjustment: normalize_optional(req.practice_adjustment),
        next_task: normalize_optional(req.next_exercise),
        status,
    };
    let snapshot = capture_memory_requested(req.capture_memory).then(|| {
        feedback_loop_memory_snapshot(
            user_id,
            "patch",
            [
                ("attempt", "Answer / reasoning", None),
                (
                    "evaluation",
                    "Mistake pattern / evaluation",
                    patch.evaluation.as_deref(),
                ),
                ("feedback", "Feedback", patch.feedback.as_deref()),
                (
                    "adjustment",
                    "Practice adjustment",
                    patch.adjustment.as_deref(),
                ),
                ("next_task", "Next exercise", patch.next_task.as_deref()),
            ],
        )
    });

    patch_session(state, id, patch, snapshot.flatten()).await
}

pub async fn patch_feedback_in_namespace(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path((namespace_id, id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PatchPracticeFeedbackRequest>,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let existing =
        find_practice_session_in_namespace(&state, namespace_id, id, auth_user.user_id).await?;
    require_space_writer(&state, existing.space_id, auth_user.user_id).await?;
    patch_practice_feedback(&state, auth_user.user_id, id, req).await
}

async fn patch_session(
    state: &AppState,
    id: Uuid,
    patch: PatchFeedbackLoop,
    snapshot: Option<crate::db::feedback_loop::FeedbackLoopMemorySnapshot>,
) -> Result<Json<ApiResponse<PracticeSessionResponse>>, AppError> {
    let result = match snapshot {
        Some(snapshot) => state
            .repositories
            .feedback_loops
            .patch_with_memory_snapshot(id, patch, snapshot)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("practice session not found".to_string()))?,
        None => FeedbackLoopWithMemorySnapshot {
            feedback_loop: state
                .repositories
                .feedback_loops
                .patch(id, patch)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound("practice session not found".to_string()))?,
            memory: None,
        },
    };

    if let Some(memory) = result.memory.as_ref() {
        index_memory_embedding(state, memory).await;
    }

    Ok(Json(ApiResponse::success(PracticeSessionResponse::from(
        result.feedback_loop,
    ))))
}

async fn list_practice_sessions(
    state: &AppState,
    space_id: Uuid,
    namespace_id: Uuid,
    limit: Option<i64>,
    offset: Option<i64>,
    user_id: Uuid,
) -> Result<Json<ApiResponse<PracticeSessionListResponse>>, AppError> {
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let offset = offset.unwrap_or(0).max(0);
    let feedback_loops = state
        .repositories
        .feedback_loops
        .list_for_user(
            FeedbackLoopListFilter {
                space_id,
                namespace_id: Some(namespace_id),
                limit,
                offset,
            },
            user_id,
        )
        .await
        .map_err(AppError::Database)?;

    let items = feedback_loops
        .into_iter()
        .map(PracticeSessionResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::success(PracticeSessionListResponse {
        total: items.len(),
        items,
        limit,
        offset,
    })))
}

async fn find_practice_session_in_namespace(
    state: &AppState,
    namespace_id: Uuid,
    id: Uuid,
    user_id: Uuid,
) -> Result<FeedbackLoopDb, AppError> {
    let feedback_loop = state
        .repositories
        .feedback_loops
        .find_for_user(id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;
    let namespace = require_practice_namespace(state, namespace_id, user_id).await?;
    require_namespace_in_space(state, namespace_id, namespace.space_id, user_id).await?;

    if feedback_loop.space_id != namespace.space_id || feedback_loop.namespace_id != namespace.id {
        return Err(AppError::Unauthorized);
    }

    Ok(feedback_loop)
}

async fn find_learning_math_session(
    state: &AppState,
    id: Uuid,
    user_id: Uuid,
) -> Result<FeedbackLoopDb, AppError> {
    let feedback_loop = state
        .repositories
        .feedback_loops
        .find_for_user(id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    require_learning_math_namespace(
        state,
        feedback_loop.namespace_id,
        feedback_loop.space_id,
        user_id,
    )
    .await?;

    Ok(feedback_loop)
}

async fn require_practice_namespace(
    state: &AppState,
    namespace_id: Uuid,
    user_id: Uuid,
) -> Result<NamespaceDb, AppError> {
    let namespace = state
        .repositories
        .namespaces
        .find_for_user(namespace_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;
    require_namespace_in_space(state, namespace_id, namespace.space_id, user_id).await?;

    if namespace.kind == NamespaceKind::Skill.as_str() {
        Ok(namespace)
    } else {
        Err(AppError::BadRequest(
            "practice session namespace must be a skill namespace".to_string(),
        ))
    }
}

fn require_matching_space_id(
    requested_space_id: Uuid,
    namespace_space_id: Uuid,
) -> Result<(), AppError> {
    if requested_space_id == namespace_space_id {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "space_id must match the namespace Space".to_string(),
        ))
    }
}

async fn ensure_learning_math_namespace(
    state: &AppState,
    space_id: Uuid,
    namespace_id: Option<Uuid>,
    user_id: Uuid,
) -> Result<NamespaceDb, AppError> {
    if let Some(namespace_id) = namespace_id {
        require_learning_math_namespace(state, namespace_id, space_id, user_id).await?;
        return state
            .repositories
            .namespaces
            .find_for_user(namespace_id, user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized);
    }

    if let Some(namespace) = state
        .repositories
        .namespaces
        .list_for_space(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .find(|namespace| namespace.name == LEARNING_MATH_NAMESPACE)
    {
        if is_learning_math_namespace(&namespace.name, &namespace.kind) {
            return Ok(namespace);
        }
        return Err(wrong_learning_math_namespace_kind());
    }

    state
        .repositories
        .namespaces
        .create(CreateNamespace {
            space_id,
            name: LEARNING_MATH_NAMESPACE.to_string(),
            kind: NamespaceKind::Skill,
            description: Some(LEARNING_MATH_DESCRIPTION.to_string()),
            status: NamespaceStatus::Active,
            created_by: user_id,
        })
        .await
        .map_err(AppError::Database)
}

async fn find_learning_math_namespace(
    state: &AppState,
    space_id: Uuid,
    namespace_id: Option<Uuid>,
    user_id: Uuid,
) -> Result<Option<NamespaceDb>, AppError> {
    if let Some(namespace_id) = namespace_id {
        require_learning_math_namespace(state, namespace_id, space_id, user_id).await?;
        return state
            .repositories
            .namespaces
            .find_for_user(namespace_id, user_id)
            .await
            .map_err(AppError::Database);
    }

    let namespace = state
        .repositories
        .namespaces
        .list_for_space(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .find(|namespace| namespace.name == LEARNING_MATH_NAMESPACE);

    match namespace {
        Some(namespace) if is_learning_math_namespace(&namespace.name, &namespace.kind) => {
            Ok(Some(namespace))
        }
        Some(_) => Err(wrong_learning_math_namespace_kind()),
        None => Ok(None),
    }
}

async fn require_learning_math_namespace(
    state: &AppState,
    namespace_id: Uuid,
    space_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    require_namespace_in_space(state, namespace_id, space_id, user_id).await?;
    let namespace = state
        .repositories
        .namespaces
        .find_for_user(namespace_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if is_learning_math_namespace(&namespace.name, &namespace.kind) {
        Ok(())
    } else {
        Err(wrong_learning_math_namespace_kind())
    }
}

fn is_learning_math_namespace(name: &str, kind: &str) -> bool {
    name == LEARNING_MATH_NAMESPACE && kind == NamespaceKind::Skill.as_str()
}

fn wrong_learning_math_namespace_kind() -> AppError {
    AppError::BadRequest("practice session namespace must be learning.math skill".to_string())
}

impl From<FeedbackLoopDb> for PracticeSessionResponse {
    fn from(feedback_loop: FeedbackLoopDb) -> Self {
        Self {
            id: feedback_loop.id,
            space_id: feedback_loop.space_id,
            namespace_id: feedback_loop.namespace_id,
            practice_goal: feedback_loop.goal,
            exercise: feedback_loop.task,
            answer: feedback_loop.attempt,
            mistake_pattern: feedback_loop.evaluation,
            feedback: feedback_loop.feedback,
            practice_adjustment: feedback_loop.adjustment,
            next_exercise: feedback_loop.next_task,
            status: feedback_loop.status,
            created_at: feedback_loop.created_at,
            updated_at: feedback_loop.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_request_accepts_product_language_and_feedback_loop_aliases() {
        let space_id = Uuid::new_v4();
        let product_json = format!(
            r#"{{
                "space_id":"{space_id}",
                "practice_goal":"Improve fraction word problems",
                "exercise":"Solve five fraction problems",
                "answer":"I solved 3 out of 5",
                "mistake_pattern":"Mixed up units",
                "practice_adjustment":"Label units first",
                "next_exercise":"Try three unit-conversion problems"
            }}"#
        );
        let alias_json = format!(
            r#"{{
                "space_id":"{space_id}",
                "goal":"Improve fraction word problems",
                "task":"Solve five fraction problems",
                "attempt":"I solved 3 out of 5",
                "evaluation":"Mixed up units",
                "adjustment":"Label units first",
                "next_task":"Try three unit-conversion problems"
            }}"#
        );

        let product: CreatePracticeSessionRequest = serde_json::from_str(&product_json).unwrap();
        let alias: CreatePracticeSessionRequest = serde_json::from_str(&alias_json).unwrap();

        assert_eq!(product.practice_goal, alias.practice_goal);
        assert_eq!(product.exercise, alias.exercise);
        assert_eq!(product.answer, alias.answer);
        assert_eq!(product.mistake_pattern, alias.mistake_pattern);
        assert_eq!(product.practice_adjustment, alias.practice_adjustment);
        assert_eq!(product.next_exercise, alias.next_exercise);
    }

    #[test]
    fn attempt_patch_accepts_answer_language() {
        let req: PatchPracticeAttemptRequest = serde_json::from_str(
            r#"{
                "answer": "Child added denominators directly",
                "capture_memory": true
            }"#,
        )
        .unwrap();

        assert_eq!(
            req.answer.as_deref(),
            Some("Child added denominators directly")
        );
        assert_eq!(req.capture_memory, Some(true));
    }

    #[test]
    fn feedback_patch_accepts_parent_friendly_learning_fields() {
        let req: PatchPracticeFeedbackRequest = serde_json::from_str(
            r#"{
                "mistake_pattern": "Units changed between steps",
                "feedback": "Write units next to every number",
                "practice_adjustment": "Add a unit check before calculating",
                "next_exercise": "Three unit-conversion fraction problems",
                "status": "completed"
            }"#,
        )
        .unwrap();

        assert_eq!(
            req.mistake_pattern.as_deref(),
            Some("Units changed between steps")
        );
        assert_eq!(
            req.feedback.as_deref(),
            Some("Write units next to every number")
        );
        assert_eq!(
            req.practice_adjustment.as_deref(),
            Some("Add a unit check before calculating")
        );
        assert_eq!(
            req.next_exercise.as_deref(),
            Some("Three unit-conversion fraction problems")
        );
        assert_eq!(req.status.as_deref(), Some("completed"));
    }

    #[test]
    fn learning_math_namespace_requires_skill_kind() {
        assert!(is_learning_math_namespace("learning.math", "skill"));
        assert!(!is_learning_math_namespace("learning.math", "reflective"));
        assert!(!is_learning_math_namespace("personal.thoughts", "skill"));
    }
}
