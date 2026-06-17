use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
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
