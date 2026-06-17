use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::{CreateFeedbackLoop, FeedbackLoopDb, PatchFeedbackLoop};
use crate::db::memory::{CreateMemory, MemoryType};
use crate::db::space::SpaceMemberRole;
use crate::db::trace::{
    CreateCompletedTrace, TraceMode, TraceRuntime, TraceSourceType, TraceTaskType,
};
use crate::domain::event::{
    EngineEvent, EngineEventEnvelope, EnginePayloadRef, EnginePayloadRefKind,
};
use crate::domain::surface::{
    RuntimePreference, Surface, SurfaceAction, SurfaceAdapter, SurfaceRequest, SurfaceResponse,
    SurfaceVisibility,
};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct CapturePayload {
    title: Option<String>,
    content: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
struct SubmitAttemptPayload {
    space_id: Uuid,
    feedback_loop_id: Option<Uuid>,
    goal: Option<String>,
    task: Option<String>,
    attempt: Value,
}

pub async fn handle(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(request): Json<SurfaceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    request
        .validate()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;

    if request.actor != auth_user.user_id {
        return Err(AppError::Unauthorized);
    }

    match (request.surface, request.action) {
        (Surface::Capture, SurfaceAction::CaptureObservation) => {
            capture_observation(&state, auth_user.user_id, request).await
        }
        (Surface::Performance, SurfaceAction::SubmitAttempt) => {
            submit_attempt(&state, auth_user.user_id, request).await
        }
        _ => Err(AppError::NotImplemented(format!(
            "{:?} {:?} is not implemented yet",
            request.surface, request.action
        ))),
    }
}

async fn capture_observation(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: CapturePayload = serde_json::from_value(request.payload.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid capture payload: {error}")))?;

    let content = payload.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::BadRequest(
            "capture payload content cannot be empty".to_string(),
        ));
    }

    let space = state
        .repositories
        .spaces
        .default_for_user(user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Cognitive space not found".to_string()))?;
    require_space_writer(state, space.id, user_id).await?;
    let namespace = resolve_namespace_by_name(state, user_id, space.id, &request.namespace).await?;

    let memory = state
        .repositories
        .memories
        .create(CreateMemory {
            user_id,
            space_id: space.id,
            namespace_id: Some(namespace.id),
            feedback_loop_id: None,
            title: payload.title.clone(),
            content: content.clone(),
            memory_type: MemoryType::Text,
            file_path: None,
            is_shared: false,
            source_type: "surface_capture".to_string(),
            source_metadata: json!({
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
                "actor": request.actor,
                "namespace": request.namespace,
                "context": request.context,
                "metadata": payload.metadata,
            }),
            tags: payload.tags,
        })
        .await
        .map_err(AppError::Database)?;

    crate::api::memories::index_memory_embedding(state, &memory).await;

    let completed_at = Utc::now();
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: space.id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Capture,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: trace_runtime(request.context.runtime_preference),
            input_summary: Some(redacted_summary(&content)),
            output_summary: Some(format!("Captured observation as memory {}", memory.id)),
            started_at,
            completed_at,
            latency_ms: Some((completed_at - started_at).num_milliseconds().max(0)),
            model_provider: Some("deterministic".to_string()),
            model_name: None,
            token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
            estimated_cost_usd: Some(0.0),
            local_processing_ratio: Some(1.0),
            related_memory_ids: Vec::new(),
            generated_memory_ids: vec![memory.id],
            generated_lens_run_ids: Vec::new(),
            generated_review_report_ids: Vec::new(),
            generated_feedback_loop_ids: Vec::new(),
            user_feedback: None,
            error: None,
            metadata: json!({
                "surface_gateway": true,
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
                "event": "observation_captured",
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let event = EngineEvent::ObservationCaptured(EngineEventEnvelope {
        space_id: space.id,
        namespace_id: namespace.id,
        source_trace_id: trace.id,
        payload_refs: vec![EnginePayloadRef {
            kind: EnginePayloadRefKind::Observation,
            id: memory.id,
        }],
    });

    let response = SurfaceResponse::new(
        Surface::Capture,
        SurfaceAction::CaptureObservation,
        json!({
            "memory_id": memory.id,
            "namespace_id": namespace.id,
            "status": "captured",
            "event": event,
        }),
        trace.id,
        vec!["Use Performance Surface when this observation becomes an attempt.".to_string()],
        SurfaceVisibility::User,
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

async fn submit_attempt(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: SubmitAttemptPayload = serde_json::from_value(request.payload.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid submitAttempt payload: {error}")))?;

    require_space_writer(state, payload.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, payload.space_id, &request.namespace).await?;

    let attempt = attempt_summary(&payload.attempt);
    let feedback_loop = match payload.feedback_loop_id {
        Some(feedback_loop_id) => {
            let existing = state
                .repositories
                .feedback_loops
                .find_for_user(feedback_loop_id, user_id)
                .await
                .map_err(AppError::Database)?
                .ok_or(AppError::Unauthorized)?;
            if existing.space_id != payload.space_id || existing.namespace_id != namespace.id {
                return Err(AppError::BadRequest(
                    "FeedbackLoop must belong to the requested Space and Namespace".to_string(),
                ));
            }
            patch_feedback_loop_attempt(state, feedback_loop_id, attempt).await?
        }
        None => {
            let task = payload
                .task
                .or_else(|| task_from_attempt(&payload.attempt))
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "submitAttempt requires task when feedback_loop_id is omitted".to_string(),
                    )
                })?;
            let goal = payload
                .goal
                .unwrap_or_else(|| format!("Practice {}", request.namespace));
            state
                .repositories
                .feedback_loops
                .create(CreateFeedbackLoop {
                    space_id: payload.space_id,
                    namespace_id: namespace.id,
                    goal,
                    task,
                    attempt: Some(attempt),
                    evaluation: None,
                    feedback: None,
                    adjustment: None,
                    next_task: None,
                    status: "active".to_string(),
                    created_by: user_id,
                })
                .await
                .map_err(AppError::Database)?
        }
    };

    let evaluation = deterministic_evaluation(&payload.attempt);
    let completed_at = Utc::now();
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: payload.space_id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Practice,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: trace_runtime(request.context.runtime_preference),
            input_summary: Some(format!(
                "Performance submitAttempt in namespace {}",
                request.namespace
            )),
            output_summary: Some(format!(
                "Attempt recorded for FeedbackLoop {}",
                feedback_loop.id
            )),
            started_at,
            completed_at,
            latency_ms: Some((completed_at - started_at).num_milliseconds().max(0)),
            model_provider: Some("deterministic".to_string()),
            model_name: None,
            token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
            estimated_cost_usd: Some(0.0),
            local_processing_ratio: Some(1.0),
            related_memory_ids: Vec::new(),
            generated_memory_ids: Vec::new(),
            generated_lens_run_ids: Vec::new(),
            generated_review_report_ids: Vec::new(),
            generated_feedback_loop_ids: vec![feedback_loop.id],
            user_feedback: Some(json!({"attempt": payload.attempt})),
            error: None,
            metadata: json!({
                "surface_gateway": true,
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
                "deep_consolidation": false,
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let response = SurfaceResponse::new(
        Surface::Performance,
        SurfaceAction::SubmitAttempt,
        json!({
            "status": "attempt_recorded",
            "feedback_loop_id": feedback_loop.id,
            "namespace_id": namespace.id,
            "evaluation": evaluation,
            "deep_consolidation": false,
        }),
        trace.id,
        vec!["Review this attempt later for recurring mistake patterns".to_string()],
        SurfaceVisibility::User,
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

async fn patch_feedback_loop_attempt(
    state: &AppState,
    feedback_loop_id: Uuid,
    attempt: String,
) -> Result<FeedbackLoopDb, AppError> {
    state
        .repositories
        .feedback_loops
        .patch(
            feedback_loop_id,
            PatchFeedbackLoop {
                attempt: Some(attempt),
                evaluation: None,
                feedback: None,
                adjustment: None,
                next_task: None,
                status: None,
            },
        )
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("FeedbackLoop not found".to_string()))
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

    if member.parsed_role().is_some_and(SpaceMemberRole::can_write) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

fn attempt_summary(attempt: &Value) -> String {
    match attempt {
        Value::String(value) => value.trim().to_string(),
        Value::Object(object) => {
            let target = object.get("target").and_then(Value::as_str);
            let submitted = object.get("submitted").and_then(Value::as_str);
            match (target, submitted) {
                (Some(target), Some(submitted)) => {
                    format!("Target: {target}\nSubmitted: {submitted}")
                }
                _ => attempt.to_string(),
            }
        }
        _ => attempt.to_string(),
    }
}

fn task_from_attempt(attempt: &Value) -> Option<String> {
    attempt
        .get("target")
        .and_then(Value::as_str)
        .map(|target| format!("Attempt target: {target}"))
}

fn deterministic_evaluation(attempt: &Value) -> &'static str {
    let Some(target) = attempt.get("target").and_then(Value::as_str) else {
        return "recorded";
    };
    let Some(submitted) = attempt.get("submitted").and_then(Value::as_str) else {
        return "recorded";
    };
    if target.trim() == submitted.trim() {
        "correct"
    } else {
        "needs_review"
    }
}

async fn resolve_namespace_by_name(
    state: &AppState,
    user_id: Uuid,
    space_id: Uuid,
    namespace_name: &str,
) -> Result<crate::db::namespace::NamespaceDb, AppError> {
    let namespace_name = namespace_name.trim();
    if namespace_name.is_empty() {
        return Err(AppError::BadRequest(
            "namespace cannot be empty".to_string(),
        ));
    }

    state
        .repositories
        .namespaces
        .list_for_space(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .find(|namespace| namespace.name == namespace_name && namespace.status == "active")
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "namespace '{namespace_name}' was not found in the active Cognitive Space"
            ))
        })
}

fn trace_source_type(adapter: SurfaceAdapter) -> TraceSourceType {
    match adapter {
        SurfaceAdapter::Cli => TraceSourceType::Cli,
        SurfaceAdapter::Mcp => TraceSourceType::Mcp,
        SurfaceAdapter::Web | SurfaceAdapter::Mobile | SurfaceAdapter::Voice => TraceSourceType::Ui,
        SurfaceAdapter::Chat | SurfaceAdapter::Dashboard => TraceSourceType::Http,
    }
}

fn trace_mode(mode: Option<&str>) -> Result<TraceMode, AppError> {
    match mode.unwrap_or("fast").trim().to_ascii_lowercase().as_str() {
        "fast" => Ok(TraceMode::Fast),
        "focused" => Ok(TraceMode::Focused),
        "deep" => Ok(TraceMode::Deep),
        "none" => Ok(TraceMode::None),
        other => Err(AppError::BadRequest(format!(
            "unsupported surface context mode: {other}"
        ))),
    }
}

fn trace_runtime(preference: Option<RuntimePreference>) -> TraceRuntime {
    match preference.unwrap_or(RuntimePreference::Deterministic) {
        RuntimePreference::Auto | RuntimePreference::Deterministic => TraceRuntime::Deterministic,
        RuntimePreference::Local => TraceRuntime::Local,
        RuntimePreference::Cloud => TraceRuntime::Cloud,
        RuntimePreference::Hybrid => TraceRuntime::Hybrid,
    }
}

fn redacted_summary(content: &str) -> String {
    const MAX_SUMMARY_CHARS: usize = 180;
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= MAX_SUMMARY_CHARS {
        compact
    } else {
        format!(
            "{}...",
            compact.chars().take(MAX_SUMMARY_CHARS).collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_summary_keeps_target_and_submitted_text() {
        let attempt = json!({
            "target": "because",
            "submitted": "becuase"
        });

        let summary = attempt_summary(&attempt);

        assert!(summary.contains("Target: because"));
        assert!(summary.contains("Submitted: becuase"));
    }

    #[test]
    fn deterministic_evaluation_compares_target_and_submitted_text() {
        assert_eq!(
            deterministic_evaluation(&json!({
                "target": "because",
                "submitted": "because"
            })),
            "correct"
        );
        assert_eq!(
            deterministic_evaluation(&json!({
                "target": "because",
                "submitted": "becuase"
            })),
            "needs_review"
        );
    }

    #[test]
    fn submit_attempt_trace_defaults_to_fast_deterministic_runtime() {
        assert_eq!(trace_mode(Some("deep")).unwrap(), TraceMode::Deep);
        assert_eq!(trace_mode(None).unwrap(), TraceMode::Fast);
        assert_eq!(
            trace_runtime(Some(RuntimePreference::Auto)),
            TraceRuntime::Deterministic
        );
        assert_eq!(
            trace_runtime(Some(RuntimePreference::Local)),
            TraceRuntime::Local
        );
    }
}
