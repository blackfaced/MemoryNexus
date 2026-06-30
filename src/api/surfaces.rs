use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::{CreateFeedbackLoop, FeedbackLoopDb, PatchFeedbackLoop};
use crate::db::memory::{CreateMemory, MemoryType};
use crate::db::sleep_cycles::{
    CompleteSleepCycle, CreateSleepCycle, PostgresSleepCycleRepository, SleepCycleRepository,
};
use crate::db::space::SpaceMemberRole;
use crate::db::trace::{
    CreateCompletedTrace, TraceMode, TraceRuntime, TraceSourceType, TraceTaskType,
};
use crate::domain::dictation::{
    build_dictation_attempt, build_dictation_capture, DictationAttemptInput, DictationCaptureInput,
    DictationSource, DictationTaskKind, PromptItemInput, SubmittedItemInput,
};
use crate::domain::dictation_observation::{
    build_dictation_observation_summary, DictationObservationEvidenceRecord,
};
use crate::domain::event::{
    EngineEvent, EngineEventEnvelope, EnginePayloadRef, EnginePayloadRefKind,
};
use crate::domain::evidence::{validate_evidence_request, InputConfirmation};
use crate::domain::growth_model::{EvidenceId, GrowthEvidenceRecord};
use crate::domain::practice_plan::{build_next_task_plan, PlanningRequest};
use crate::domain::reflection::{
    build_reflection_insight, EvidenceRef, EvidenceRefKind, ReflectionEvidence, ReflectionRequest,
};
use crate::domain::sleep_cycle::{SleepCycleStatus, SleepCycleType};
use crate::domain::surface::{
    RuntimePreference, Surface, SurfaceAction, SurfaceAdapter, SurfaceRequest, SurfaceResponse,
    SurfaceVisibility,
};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct CapturePayload {
    title: Option<String>,
    content: Option<String>,
    task_kind: Option<DictationTaskKind>,
    source: Option<DictationSource>,
    input_source: Option<String>,
    input_confirmation: Option<InputConfirmation>,
    #[serde(default)]
    prompt_items: Vec<PromptItemInput>,
    #[serde(default)]
    evidence_refs: Vec<Value>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug)]
struct PreparedCapturePayload {
    title: Option<String>,
    content: String,
    input_source: Option<String>,
    tags: Vec<String>,
    metadata: Value,
    source_metadata: Value,
    trace_metadata: Value,
}

#[derive(Debug, Deserialize)]
struct SubmitAttemptPayload {
    space_id: Uuid,
    feedback_loop_id: Option<Uuid>,
    goal: Option<String>,
    task: Option<String>,
    attempt: Option<Value>,
    task_kind: Option<DictationTaskKind>,
    source: Option<DictationSource>,
    input_source: Option<String>,
    #[serde(default)]
    prompt_items: Vec<PromptItemInput>,
    #[serde(default)]
    submitted_items: Vec<SubmittedItemInput>,
    #[serde(default)]
    metadata: Value,
    input_confirmation: Option<InputConfirmation>,
    #[serde(default)]
    evidence_refs: Vec<Value>,
}

#[derive(Debug)]
struct PreparedSubmitAttemptPayload {
    space_id: Uuid,
    feedback_loop_id: Option<Uuid>,
    goal: Option<String>,
    task: Option<String>,
    attempt_summary: String,
    evaluation: Value,
    trace_metadata: Value,
    input_source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManualConsolidationPayload {
    space_id: Uuid,
    evidence_window_start: DateTime<Utc>,
    evidence_window_end: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ReviewEvidencePayload {
    space_id: Uuid,
    question: Option<String>,
    #[serde(default)]
    evidence: Vec<ReflectionEvidence>,
}

#[derive(Debug, Deserialize)]
struct GenerateNextTaskPayload {
    space_id: Uuid,
    objective: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetStateSummaryPayload {
    space_id: Uuid,
}

pub async fn handle(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(request): Json<SurfaceRequest>,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    request
        .validate()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    validate_media_fields_allowed(request.action, &request.payload)?;

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
        (Surface::Reflection, SurfaceAction::ReviewEvidence) => {
            review_evidence(&state, auth_user.user_id, request).await
        }
        (Surface::Planning, SurfaceAction::GenerateNextTask) => {
            generate_next_task(&state, auth_user.user_id, request).await
        }
        (Surface::Observation, SurfaceAction::GetStateSummary) => {
            get_state_summary(&state, auth_user.user_id, request).await
        }
        (Surface::Observation, SurfaceAction::RequestConsolidation) => {
            request_manual_consolidation(&state, auth_user.user_id, request).await
        }
        _ => Err(AppError::NotImplemented(format!(
            "{:?} {:?} is not implemented yet",
            request.surface, request.action
        ))),
    }
}

async fn get_state_summary(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: GetStateSummaryPayload =
        serde_json::from_value(request.payload.clone()).map_err(|error| {
            AppError::BadRequest(format!("invalid getStateSummary payload: {error}"))
        })?;

    require_space_member(state, payload.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, payload.space_id, &request.namespace).await?;

    let counts = load_observation_counts(&state.db, payload.space_id, namespace.id).await?;
    let latest_trace_task_type =
        latest_trace_task_type(&state.db, payload.space_id, namespace.id).await?;
    let observation_window_end = started_at + chrono::Duration::minutes(1);
    let dictation_evidence = load_recent_dictation_observation_evidence(
        &state.db,
        payload.space_id,
        namespace.id,
        observation_window_end,
    )
    .await?;
    let dictation_observation = build_dictation_observation_summary(
        payload.space_id,
        namespace.id,
        observation_window_end,
        dictation_evidence,
    );
    let output_summary = format!(
        "Observed {}: {} memories, {} traces, {} feedback loops",
        request.namespace,
        counts.memory_count,
        counts.trace_count,
        counts.feedback_loop_total()
    );

    let result = json!({
        "status": "state_summary_ready",
        "space_id": payload.space_id,
        "namespace_id": namespace.id,
        "namespace": request.namespace,
        "summary": output_summary,
        "counts": {
            "memories": counts.memory_count,
            "traces": counts.trace_count,
            "feedback_loops": {
                "active": counts.active_feedback_loop_count,
                "completed": counts.completed_feedback_loop_count,
                "paused": counts.paused_feedback_loop_count,
                "total": counts.feedback_loop_total(),
            },
            "review_reports": counts.review_report_count,
            "sleep_cycles": counts.sleep_cycle_count,
        },
        "trends": {
            "recent_trace_count": counts.trace_count,
            "latest_trace_task_type": latest_trace_task_type,
        },
        "growth_model": {
            "status": "not_persisted",
            "growth_model_id": Value::Null,
            "note": "No persisted GrowthModel is available for this namespace yet.",
        },
        "dictation_observation": dictation_observation,
    });

    let completed_at = Utc::now();
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: payload.space_id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Observation,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: deterministic_trace_runtime(request.context.runtime_preference),
            input_summary: Some(format!(
                "Observation getStateSummary in namespace {}",
                request.namespace
            )),
            output_summary: Some(output_summary),
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
            generated_feedback_loop_ids: Vec::new(),
            user_feedback: None,
            error: None,
            metadata: json!({
                "surface_gateway": true,
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
                "namespace": request.namespace,
                "deterministic": true,
                "memory_count": counts.memory_count,
                "trace_count": counts.trace_count,
                "feedback_loop_count": counts.feedback_loop_total(),
                "growth_model_status": "not_persisted",
                "dictation_observation_status": dictation_observation.status,
                "dictation_observation_evidence_record_count": dictation_observation.evidence_record_count,
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let response = SurfaceResponse::new(
        Surface::Observation,
        SurfaceAction::GetStateSummary,
        result,
        trace.id,
        vec!["Use Planning Surface when the observed state should become a next task.".to_string()],
        SurfaceVisibility::User,
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

async fn generate_next_task(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: GenerateNextTaskPayload = serde_json::from_value(request.payload.clone())
        .map_err(|error| {
            AppError::BadRequest(format!("invalid generateNextTask payload: {error}"))
        })?;

    require_space_writer(state, payload.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, payload.space_id, &request.namespace).await?;

    let plan = build_next_task_plan(&PlanningRequest {
        space_id: payload.space_id,
        namespace_id: namespace.id,
        namespace: request.namespace.clone(),
        objective: payload.objective.clone(),
    });
    let output_summary = format!(
        "Generated response-only next task for {}: {}",
        plan.namespace, plan.next_task.prompt
    );

    let completed_at = Utc::now();
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: payload.space_id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Planning,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: deterministic_trace_runtime(request.context.runtime_preference),
            input_summary: Some(format!(
                "Planning generateNextTask in namespace {}",
                request.namespace
            )),
            output_summary: Some(output_summary),
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
            generated_feedback_loop_ids: Vec::new(),
            user_feedback: payload
                .objective
                .map(|objective| json!({"objective": objective})),
            error: None,
            metadata: json!({
                "surface_gateway": true,
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
                "namespace": request.namespace,
                "deterministic": true,
                "plan_kind": plan.plan_kind.clone(),
                "persistence": plan.persistence.clone(),
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let result = serde_json::to_value(plan)
        .map_err(|error| AppError::Internal(format!("planning response failed: {error}")))?;
    let response = SurfaceResponse::new(
        Surface::Planning,
        SurfaceAction::GenerateNextTask,
        result,
        trace.id,
        vec!["Submit the next task through Performance Surface after it is attempted.".to_string()],
        SurfaceVisibility::User,
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

async fn capture_observation(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: CapturePayload = serde_json::from_value(request.payload.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid capture payload: {error}")))?;
    let prepared = prepare_capture_payload(&request.namespace, payload)?;

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
            title: prepared.title.clone(),
            content: prepared.content.clone(),
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
                "metadata": prepared.metadata,
                "input_source": prepared.input_source,
                "capture": prepared.source_metadata,
            }),
            tags: prepared.tags,
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
            input_summary: Some(redacted_summary(&prepared.content)),
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
                "namespace": request.namespace,
                "input_source": prepared.input_source,
                "capture": prepared.trace_metadata,
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

fn prepare_capture_payload(
    namespace: &str,
    payload: CapturePayload,
) -> Result<PreparedCapturePayload, AppError> {
    if payload.task_kind.is_some() || payload.source.is_some() || !payload.prompt_items.is_empty() {
        return prepare_dictation_capture_payload(namespace, payload);
    }

    let content = payload.content.unwrap_or_default().trim().to_string();
    if content.is_empty() {
        return Err(AppError::BadRequest(
            "capture payload content cannot be empty".to_string(),
        ));
    }
    validate_surface_evidence(
        payload.input_source.as_deref(),
        payload.input_confirmation.as_ref(),
        &payload.evidence_refs,
    )?;

    Ok(PreparedCapturePayload {
        title: payload.title,
        content,
        input_source: payload.input_source,
        tags: payload.tags,
        metadata: payload.metadata,
        source_metadata: json!({}),
        trace_metadata: json!({}),
    })
}

fn prepare_dictation_capture_payload(
    namespace: &str,
    payload: CapturePayload,
) -> Result<PreparedCapturePayload, AppError> {
    let task_kind = payload
        .task_kind
        .ok_or_else(|| AppError::BadRequest("dictation capture requires task_kind".to_string()))?;
    let source =
        resolve_dictation_source(payload.source, payload.input_source.as_deref(), "capture")?;
    let input = DictationCaptureInput {
        namespace: namespace.to_string(),
        task_kind,
        source,
        title: payload.title,
        prompt_items: payload.prompt_items,
        input_confirmation: payload.input_confirmation,
        evidence_refs: payload.evidence_refs,
        metadata: payload.metadata,
    };
    let capture =
        build_dictation_capture(input).map_err(|error| AppError::BadRequest(error.to_string()))?;
    let source_metadata = json!({
        "dictation": capture.persistence_metadata,
    });
    let trace_metadata = json!({
        "namespace": namespace,
        "input_source": capture.source,
        "dictation": {
            "task_kind": capture.task_kind,
            "item_count": capture.prompt_items.len(),
            "evidence_ref_count": capture.evidence_ref_count,
        },
    });

    Ok(PreparedCapturePayload {
        title: capture.title,
        content: capture.canonical_text,
        input_source: Some(capture.source.as_str().to_string()),
        tags: payload.tags,
        metadata: capture.persistence_metadata["metadata"].clone(),
        source_metadata,
        trace_metadata,
    })
}

fn resolve_dictation_source(
    source: Option<DictationSource>,
    input_source: Option<&str>,
    record_kind: &str,
) -> Result<DictationSource, AppError> {
    let input_source = match input_source {
        Some(value) => Some(
            serde_json::from_value::<DictationSource>(json!(value)).map_err(|_| {
                AppError::BadRequest(format!("unsupported dictation source: {value}"))
            })?,
        ),
        None => None,
    };

    match (source, input_source) {
        (Some(source), Some(input_source)) if source != input_source => {
            Err(AppError::BadRequest(format!(
                "dictation {record_kind} source and input_source must match when both are provided"
            )))
        }
        (Some(source), _) | (_, Some(source)) => Ok(source),
        (None, None) => Err(AppError::BadRequest(format!(
            "dictation {record_kind} requires source"
        ))),
    }
}

fn prepare_submit_attempt_payload(
    namespace: &str,
    payload: SubmitAttemptPayload,
) -> Result<PreparedSubmitAttemptPayload, AppError> {
    if payload.task_kind.is_some()
        || payload.source.is_some()
        || !payload.prompt_items.is_empty()
        || !payload.submitted_items.is_empty()
    {
        return prepare_dictation_attempt_payload(namespace, payload);
    }

    validate_surface_evidence(
        payload.input_source.as_deref(),
        payload.input_confirmation.as_ref(),
        &payload.evidence_refs,
    )?;
    let attempt = payload.attempt.ok_or_else(|| {
        AppError::BadRequest(
            "submitAttempt requires attempt unless using dictation submitted_items".to_string(),
        )
    })?;
    let attempt_summary = attempt_summary(&attempt);
    let task = payload.task.or_else(|| task_from_attempt(&attempt));
    let evaluation = json!(deterministic_evaluation(&attempt));

    Ok(PreparedSubmitAttemptPayload {
        space_id: payload.space_id,
        feedback_loop_id: payload.feedback_loop_id,
        goal: payload.goal,
        task,
        attempt_summary,
        evaluation,
        trace_metadata: Value::Null,
        input_source: payload.input_source,
    })
}

fn prepare_dictation_attempt_payload(
    namespace: &str,
    payload: SubmitAttemptPayload,
) -> Result<PreparedSubmitAttemptPayload, AppError> {
    let task_kind = payload
        .task_kind
        .ok_or_else(|| AppError::BadRequest("dictation attempt requires task_kind".to_string()))?;
    let source =
        resolve_dictation_source(payload.source, payload.input_source.as_deref(), "attempt")?;
    let input = DictationAttemptInput {
        namespace: namespace.to_string(),
        task_kind,
        source,
        task: payload.task,
        goal: payload.goal,
        prompt_items: payload.prompt_items,
        submitted_items: payload.submitted_items,
        input_confirmation: payload.input_confirmation,
        evidence_refs: payload.evidence_refs,
        metadata: payload.metadata,
    };
    let attempt =
        build_dictation_attempt(input).map_err(|error| AppError::BadRequest(error.to_string()))?;
    let evaluation = serde_json::to_value(&attempt.evaluation).map_err(|error| {
        AppError::Internal(format!(
            "dictation evaluation serialization failed: {error}"
        ))
    })?;
    let trace_metadata = json!({
        "task_kind": attempt.task_kind,
        "source": attempt.source,
        "item_count": attempt.prompt_items.len(),
        "submitted_item_count": attempt.submitted_items.len(),
        "evidence_ref_count": attempt.evidence_ref_count,
    });

    Ok(PreparedSubmitAttemptPayload {
        space_id: payload.space_id,
        feedback_loop_id: payload.feedback_loop_id,
        goal: attempt.goal,
        task: attempt.task,
        attempt_summary: attempt.summary,
        evaluation,
        trace_metadata,
        input_source: Some(attempt.source.as_str().to_string()),
    })
}

async fn submit_attempt(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: SubmitAttemptPayload = serde_json::from_value(request.payload.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid submitAttempt payload: {error}")))?;
    let prepared = prepare_submit_attempt_payload(&request.namespace, payload)?;

    require_space_writer(state, prepared.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, prepared.space_id, &request.namespace).await?;

    let feedback_loop = match prepared.feedback_loop_id {
        Some(feedback_loop_id) => {
            let existing = state
                .repositories
                .feedback_loops
                .find_for_user(feedback_loop_id, user_id)
                .await
                .map_err(AppError::Database)?
                .ok_or(AppError::Unauthorized)?;
            if existing.space_id != prepared.space_id || existing.namespace_id != namespace.id {
                return Err(AppError::BadRequest(
                    "FeedbackLoop must belong to the requested Space and Namespace".to_string(),
                ));
            }
            patch_feedback_loop_attempt(state, feedback_loop_id, prepared.attempt_summary.clone())
                .await?
        }
        None => {
            let task = prepared.task.ok_or_else(|| {
                AppError::BadRequest(
                    "submitAttempt requires task when feedback_loop_id is omitted".to_string(),
                )
            })?;
            let goal = prepared
                .goal
                .unwrap_or_else(|| format!("Practice {}", request.namespace));
            state
                .repositories
                .feedback_loops
                .create(CreateFeedbackLoop {
                    space_id: prepared.space_id,
                    namespace_id: namespace.id,
                    goal,
                    task,
                    attempt: Some(prepared.attempt_summary.clone()),
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

    let completed_at = Utc::now();
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: prepared.space_id,
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
            user_feedback: Some(json!({"attempt": prepared.attempt_summary})),
            error: None,
            metadata: json!({
                "surface_gateway": true,
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
                "namespace": request.namespace,
                "input_source": prepared.input_source,
                "deep_consolidation": false,
                "dictation": prepared.trace_metadata,
                "event": "attempt_submitted",
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let event = EngineEvent::AttemptSubmitted(EngineEventEnvelope {
        space_id: prepared.space_id,
        namespace_id: namespace.id,
        source_trace_id: trace.id,
        payload_refs: vec![EnginePayloadRef {
            kind: EnginePayloadRefKind::Attempt,
            id: feedback_loop.id,
        }],
    });

    let response = SurfaceResponse::new(
        Surface::Performance,
        SurfaceAction::SubmitAttempt,
        json!({
            "status": "attempt_recorded",
            "feedback_loop_id": feedback_loop.id,
            "namespace_id": namespace.id,
            "evaluation": prepared.evaluation,
            "deep_consolidation": false,
            "event": event,
        }),
        trace.id,
        vec!["Review this attempt later for recurring mistake patterns".to_string()],
        SurfaceVisibility::User,
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

async fn review_evidence(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: ReviewEvidencePayload =
        serde_json::from_value(request.payload.clone()).map_err(|error| {
            AppError::BadRequest(format!("invalid reviewEvidence payload: {error}"))
        })?;

    require_space_writer(state, payload.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, payload.space_id, &request.namespace).await?;
    validate_reflection_evidence_sources(
        state,
        user_id,
        payload.space_id,
        namespace.id,
        &payload.evidence,
    )
    .await?;

    let insight = build_reflection_insight(&ReflectionRequest {
        space_id: payload.space_id,
        namespace_id: namespace.id,
        namespace: request.namespace.clone(),
        question: payload.question.clone(),
        evidence: payload.evidence.clone(),
    });

    let completed_at = Utc::now();
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: payload.space_id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Review,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: deterministic_trace_runtime(request.context.runtime_preference),
            input_summary: Some(format!(
                "Reflection reviewEvidence in namespace {} with {} evidence items",
                request.namespace, insight.evidence_count
            )),
            output_summary: Some(insight.summary.clone()),
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
            generated_feedback_loop_ids: Vec::new(),
            user_feedback: payload
                .question
                .map(|question| json!({"question": question})),
            error: None,
            metadata: json!({
                "surface_gateway": true,
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
                "namespace": request.namespace,
                "evidence_count": insight.evidence_count,
                "evidence_refs": evidence_refs(&payload.evidence),
                "deterministic": true,
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let follow_up_suggestions = if insight.evidence_count == 0 {
        vec!["Provide confirmed evidence before asking for a reflection.".to_string()]
    } else {
        vec!["Use Planning Surface when this reflection should become a next task.".to_string()]
    };

    let result = serde_json::to_value(insight)
        .map_err(|error| AppError::Internal(format!("reflection response failed: {error}")))?;
    let response = SurfaceResponse::new(
        Surface::Reflection,
        SurfaceAction::ReviewEvidence,
        result,
        trace.id,
        follow_up_suggestions,
        SurfaceVisibility::User,
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

async fn validate_reflection_evidence_sources(
    state: &AppState,
    user_id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
    evidence: &[ReflectionEvidence],
) -> Result<(), AppError> {
    for item in evidence {
        match item.source.kind {
            EvidenceRefKind::Trace => {
                let trace = state
                    .repositories
                    .traces
                    .find_for_user(item.source.id, user_id)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| invalid_evidence_ref(&item.source))?;
                if trace.space_id != space_id || trace.namespace_id != Some(namespace_id) {
                    return Err(invalid_evidence_ref(&item.source));
                }
            }
            EvidenceRefKind::Memory => {
                let memory = state
                    .repositories
                    .memories
                    .find_by_id(item.source.id)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| invalid_evidence_ref(&item.source))?;
                if memory.space_id != space_id || memory.namespace_id != Some(namespace_id) {
                    return Err(invalid_evidence_ref(&item.source));
                }
            }
            EvidenceRefKind::FeedbackLoop => {
                let feedback_loop = state
                    .repositories
                    .feedback_loops
                    .find_for_user(item.source.id, user_id)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| invalid_evidence_ref(&item.source))?;
                if feedback_loop.space_id != space_id || feedback_loop.namespace_id != namespace_id
                {
                    return Err(invalid_evidence_ref(&item.source));
                }
            }
            EvidenceRefKind::ReviewReport => {
                let review_report = state
                    .repositories
                    .review_reports
                    .find_for_user(item.source.id, user_id)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| invalid_evidence_ref(&item.source))?;
                if review_report.space_id != space_id
                    || review_report.namespace_id != Some(namespace_id)
                {
                    return Err(invalid_evidence_ref(&item.source));
                }
            }
        }
    }

    Ok(())
}

fn invalid_evidence_ref(source: &EvidenceRef) -> AppError {
    AppError::BadRequest(format!(
        "reflection evidence reference {:?} {} was not found in the requested Space and Namespace",
        source.kind, source.id
    ))
}

fn evidence_refs(evidence: &[ReflectionEvidence]) -> Vec<EvidenceRef> {
    evidence.iter().map(|item| item.source.clone()).collect()
}

fn validate_surface_evidence(
    input_source: Option<&str>,
    input_confirmation: Option<&InputConfirmation>,
    evidence_refs: &[Value],
) -> Result<(), AppError> {
    let refs = if evidence_refs.is_empty() {
        None
    } else {
        Some(evidence_refs)
    };
    validate_evidence_request(input_source, input_confirmation, refs)
        .map_err(|error| AppError::BadRequest(error.to_string()))
}

fn validate_media_fields_allowed(action: SurfaceAction, payload: &Value) -> Result<(), AppError> {
    if matches!(
        action,
        SurfaceAction::CaptureObservation | SurfaceAction::SubmitAttempt
    ) {
        return Ok(());
    }

    let Some(object) = payload.as_object() else {
        return Ok(());
    };
    for field in ["evidence_refs", "input_confirmation", "input_source"] {
        if object.contains_key(field) {
            return Err(AppError::BadRequest(format!(
                "media evidence field '{field}' is only supported for capture_observation and submit_attempt"
            )));
        }
    }
    Ok(())
}

async fn request_manual_consolidation(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let payload: ManualConsolidationPayload = serde_json::from_value(request.payload.clone())
        .map_err(|_| {
            AppError::BadRequest(
                "space_id, evidence_window_start, and evidence_window_end are required".to_string(),
            )
        })?;
    if payload.evidence_window_start >= payload.evidence_window_end {
        return Err(AppError::BadRequest(
            "evidence_window_start must be before evidence_window_end".to_string(),
        ));
    }

    require_space_writer(state, payload.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, payload.space_id, &request.namespace).await?;
    let input_trace_ids = select_trace_evidence(
        &state.db,
        payload.space_id,
        namespace.id,
        payload.evidence_window_start,
        payload.evidence_window_end,
    )
    .await?;

    let now = Utc::now();
    let triggering_trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: payload.space_id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Consolidation,
            mode: manual_trace_mode(request.context.mode.as_deref())?,
            runtime: deterministic_trace_runtime(request.context.runtime_preference),
            input_summary: Some(
                "Manual consolidation requested through Surface Gateway".to_string(),
            ),
            output_summary: Some(format!(
                "Selected {} Trace records for the evidence window",
                input_trace_ids.len()
            )),
            started_at: now,
            completed_at: Utc::now(),
            latency_ms: None,
            model_provider: Some("deterministic".to_string()),
            model_name: None,
            token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
            estimated_cost_usd: Some(0.0),
            local_processing_ratio: Some(1.0),
            related_memory_ids: Vec::new(),
            generated_memory_ids: Vec::new(),
            generated_lens_run_ids: Vec::new(),
            generated_review_report_ids: Vec::new(),
            generated_feedback_loop_ids: Vec::new(),
            user_feedback: None,
            error: None,
            metadata: json!({
                "surface_gateway": true,
                "surface": request.surface,
                "action": request.action,
                "namespace": request.namespace,
                "evidence_window_start": payload.evidence_window_start,
                "evidence_window_end": payload.evidence_window_end,
                "selected_input_trace_count": input_trace_ids.len(),
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let sleep_cycle_repository = PostgresSleepCycleRepository::new(state.db.clone());
    let sleep_cycle = sleep_cycle_repository
        .create(CreateSleepCycle {
            space_id: payload.space_id,
            namespace_id: Some(namespace.id),
            cycle_type: SleepCycleType::Manual,
            status: SleepCycleStatus::Running,
            evidence_window_start: payload.evidence_window_start,
            evidence_window_end: payload.evidence_window_end,
            input_trace_ids: input_trace_ids.clone(),
            input_memory_ids: Vec::new(),
            input_feedback_loop_ids: Vec::new(),
            input_review_report_ids: Vec::new(),
            triggering_trace_id: Some(triggering_trace.id),
            metadata: json!({
                "trigger": "surface_gateway",
                "surface": request.surface,
                "action": request.action,
                "adapter": request.adapter,
            }),
        })
        .await
        .map_err(map_sleep_cycle_create_error)?;

    let sleep_cycle = sleep_cycle_repository
        .mark_completed(
            sleep_cycle.id,
            CompleteSleepCycle {
                generated_memory_ids: Vec::new(),
                metadata: json!({
                    "summary": deterministic_consolidation_summary(input_trace_ids.len()),
                    "selected_input_trace_count": input_trace_ids.len(),
                    "runtime": "deterministic",
                }),
            },
        )
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("manual consolidation record not found".to_string()))?;

    let response = SurfaceResponse::new(
        Surface::Observation,
        SurfaceAction::RequestConsolidation,
        json!({
            "cycle_id": sleep_cycle.id,
            "status": sleep_cycle.status,
            "namespace": request.namespace,
            "evidence_window_start": sleep_cycle.evidence_window_start,
            "evidence_window_end": sleep_cycle.evidence_window_end,
            "input_trace_ids": sleep_cycle.input_trace_ids,
            "input_trace_count": sleep_cycle.input_trace_ids.len(),
        }),
        triggering_trace.id,
        Vec::new(),
        SurfaceVisibility::Debug,
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
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

async fn select_trace_evidence(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Result<Vec<Uuid>, AppError> {
    sqlx::query_scalar(
        r#"
        SELECT id
        FROM traces
        WHERE space_id = $1
          AND namespace_id = $2
          AND started_at >= $3
          AND started_at < $4
          AND status = 'completed'
          AND task_type <> 'consolidation'
        ORDER BY started_at ASC, id ASC
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(window_start)
    .bind(window_end)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)
}

fn manual_trace_mode(mode: Option<&str>) -> Result<TraceMode, AppError> {
    match mode.unwrap_or("deep").trim().to_ascii_lowercase().as_str() {
        "fast" => Ok(TraceMode::Fast),
        "focused" => Ok(TraceMode::Focused),
        "deep" => Ok(TraceMode::Deep),
        "none" => Ok(TraceMode::None),
        other => Err(AppError::BadRequest(format!(
            "unsupported surface context mode: {other}"
        ))),
    }
}

fn deterministic_trace_runtime(_preference: Option<RuntimePreference>) -> TraceRuntime {
    TraceRuntime::Deterministic
}

fn deterministic_consolidation_summary(input_trace_count: usize) -> String {
    if input_trace_count == 0 {
        "No Trace evidence found in the selected window".to_string()
    } else {
        format!("Selected {input_trace_count} Trace records for deterministic consolidation")
    }
}

fn map_sleep_cycle_create_error(error: Error) -> AppError {
    if matches!(error, Error::RowNotFound) {
        AppError::Unauthorized
    } else {
        AppError::Database(error)
    }
}

#[derive(Debug, FromRow)]
struct DictationTraceEvidenceRow {
    id: Uuid,
    started_at: DateTime<Utc>,
    metadata: Value,
}

#[derive(Debug, FromRow)]
struct DictationFeedbackLoopEvidenceRow {
    id: Uuid,
    updated_at: DateTime<Utc>,
    evaluation: Option<String>,
}

async fn load_recent_dictation_observation_evidence(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Vec<DictationObservationEvidenceRecord>, AppError> {
    let window_start = now - chrono::Duration::days(7);
    let mut records = Vec::new();

    let trace_rows = sqlx::query_as::<_, DictationTraceEvidenceRow>(
        r#"
        SELECT id, started_at, metadata
        FROM traces
        WHERE space_id = $1
          AND namespace_id = $2
          AND started_at >= $3
          AND started_at <= $4
          AND status = 'completed'
          AND task_type <> 'observation'
        ORDER BY started_at ASC, id ASC
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(window_start)
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    for row in trace_rows {
        let signal_labels = dictation_signal_labels_from_value(&row.metadata);
        if signal_labels.is_empty() {
            continue;
        }
        records.push(DictationObservationEvidenceRecord {
            observed_at: row.started_at,
            growth_evidence: GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: EvidenceId::Trace(row.id),
                signal_labels,
                explanation: None,
            },
        });
    }

    let feedback_loop_rows = sqlx::query_as::<_, DictationFeedbackLoopEvidenceRow>(
        r#"
        SELECT id, updated_at, evaluation
        FROM feedback_loops
        WHERE space_id = $1
          AND namespace_id = $2
          AND updated_at >= $3
          AND updated_at <= $4
        ORDER BY updated_at ASC, id ASC
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(window_start)
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    for row in feedback_loop_rows {
        let signal_labels = row
            .evaluation
            .as_deref()
            .and_then(|evaluation| serde_json::from_str::<Value>(evaluation).ok())
            .map(|value| dictation_signal_labels_from_value(&value))
            .unwrap_or_default();
        if signal_labels.is_empty() {
            continue;
        }
        records.push(DictationObservationEvidenceRecord {
            observed_at: row.updated_at,
            growth_evidence: GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: EvidenceId::FeedbackLoop(row.id),
                signal_labels,
                explanation: None,
            },
        });
    }

    Ok(records)
}

fn dictation_signal_labels_from_value(value: &Value) -> Vec<String> {
    let mut labels = Vec::new();
    collect_dictation_signal_labels(value, &mut labels);
    labels
}

fn collect_dictation_signal_labels(value: &Value, labels: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                if matches!(key.as_str(), "signal_labels" | "mistake_types") {
                    collect_string_array_labels(nested, labels);
                } else {
                    collect_dictation_signal_labels(nested, labels);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_dictation_signal_labels(item, labels);
            }
        }
        _ => {}
    }
}

fn collect_string_array_labels(value: &Value, labels: &mut Vec<String>) {
    let Some(items) = value.as_array() else {
        return;
    };
    for item in items {
        let Some(label) = item
            .as_str()
            .map(str::trim)
            .filter(|label| !label.is_empty())
        else {
            continue;
        };
        if !labels.iter().any(|existing| existing == label) {
            labels.push(label.to_string());
        }
    }
}

#[derive(Debug)]
struct ObservationCounts {
    memory_count: i64,
    trace_count: i64,
    active_feedback_loop_count: i64,
    completed_feedback_loop_count: i64,
    paused_feedback_loop_count: i64,
    review_report_count: i64,
    sleep_cycle_count: i64,
}

impl ObservationCounts {
    fn feedback_loop_total(&self) -> i64 {
        self.active_feedback_loop_count
            + self.completed_feedback_loop_count
            + self.paused_feedback_loop_count
    }
}

async fn load_observation_counts(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<ObservationCounts, AppError> {
    let memory_count = count_namespace_rows(
        pool,
        "SELECT COUNT(*) FROM memories WHERE space_id = $1 AND namespace_id = $2",
        space_id,
        namespace_id,
    )
    .await?;
    let trace_count = count_namespace_rows(
        pool,
        "SELECT COUNT(*) FROM traces WHERE space_id = $1 AND namespace_id = $2",
        space_id,
        namespace_id,
    )
    .await?;
    let active_feedback_loop_count =
        count_feedback_loops(pool, space_id, namespace_id, "active").await?;
    let completed_feedback_loop_count =
        count_feedback_loops(pool, space_id, namespace_id, "completed").await?;
    let paused_feedback_loop_count =
        count_feedback_loops(pool, space_id, namespace_id, "paused").await?;
    let review_report_count = count_namespace_rows(
        pool,
        "SELECT COUNT(*) FROM cognitive_review_reports WHERE space_id = $1 AND namespace_id = $2",
        space_id,
        namespace_id,
    )
    .await?;
    let sleep_cycle_count = count_namespace_rows(
        pool,
        "SELECT COUNT(*) FROM sleep_cycles WHERE space_id = $1 AND namespace_id = $2",
        space_id,
        namespace_id,
    )
    .await?;

    Ok(ObservationCounts {
        memory_count,
        trace_count,
        active_feedback_loop_count,
        completed_feedback_loop_count,
        paused_feedback_loop_count,
        review_report_count,
        sleep_cycle_count,
    })
}

async fn count_namespace_rows(
    pool: &PgPool,
    sql: &str,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<i64, AppError> {
    sqlx::query_scalar(sql)
        .bind(space_id)
        .bind(namespace_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)
}

async fn count_feedback_loops(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    status: &str,
) -> Result<i64, AppError> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM feedback_loops
        WHERE space_id = $1
          AND namespace_id = $2
          AND status = $3
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)
}

async fn latest_trace_task_type(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<Option<String>, AppError> {
    sqlx::query_scalar(
        r#"
        SELECT task_type
        FROM traces
        WHERE space_id = $1
          AND namespace_id = $2
          AND status = 'completed'
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
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

    #[test]
    fn deterministic_summary_handles_empty_evidence_window() {
        assert_eq!(
            deterministic_consolidation_summary(0),
            "No Trace evidence found in the selected window"
        );
    }

    #[test]
    fn manual_consolidation_defaults_to_deep_deterministic_trace() {
        assert_eq!(manual_trace_mode(None).unwrap(), TraceMode::Deep);
        assert_eq!(
            deterministic_trace_runtime(None),
            TraceRuntime::Deterministic
        );
        assert_eq!(
            deterministic_trace_runtime(Some(RuntimePreference::Cloud)),
            TraceRuntime::Deterministic
        );
    }

    #[test]
    fn cross_space_link_validation_error_is_rejected() {
        assert!(matches!(
            map_sleep_cycle_create_error(Error::RowNotFound),
            AppError::Unauthorized
        ));
    }

    #[test]
    fn media_evidence_fields_are_rejected_on_unsupported_surface_actions() {
        for action in [
            SurfaceAction::ReviewEvidence,
            SurfaceAction::GenerateNextTask,
            SurfaceAction::GetStateSummary,
            SurfaceAction::RequestConsolidation,
        ] {
            let err = validate_media_fields_allowed(
                action,
                &json!({
                    "space_id": Uuid::new_v4(),
                    "evidence_refs": []
                }),
            )
            .unwrap_err();
            assert!(matches!(err, AppError::BadRequest(_)));

            let err = validate_media_fields_allowed(
                action,
                &json!({
                    "space_id": Uuid::new_v4(),
                    "input_confirmation": {
                        "status": "confirmed",
                        "method": "explicit_acceptance"
                    }
                }),
            )
            .unwrap_err();
            assert!(matches!(err, AppError::BadRequest(_)));

            let err = validate_media_fields_allowed(
                action,
                &json!({
                    "space_id": Uuid::new_v4(),
                    "input_source": "agent_ocr"
                }),
            )
            .unwrap_err();
            assert!(matches!(err, AppError::BadRequest(_)));
        }

        validate_media_fields_allowed(
            SurfaceAction::CaptureObservation,
            &json!({"evidence_refs": []}),
        )
        .unwrap();
        validate_media_fields_allowed(SurfaceAction::SubmitAttempt, &json!({"evidence_refs": []}))
            .unwrap();
    }

    #[test]
    fn dictation_capture_payload_builds_canonical_text_and_descriptor_free_metadata() {
        let payload: CapturePayload = serde_json::from_value(json!({
            "title": "Today's words",
            "task_kind": "english_spelling",
            "source": "typed",
            "prompt_items": [
                {"item_kind": "english_word", "expected_text": " because ", "metadata": {}},
                {"item_kind": "english_word", "expected_text": "friend", "metadata": {}}
            ],
            "tags": ["dictation"],
            "metadata": {"adapter_note": "typed list"}
        }))
        .unwrap();

        let prepared =
            prepare_capture_payload("child.english.spelling", payload).expect("valid dictation");

        assert_eq!(prepared.content, "because\nfriend");
        assert_eq!(prepared.title.as_deref(), Some("Today's words"));
        assert_eq!(prepared.input_source.as_deref(), Some("typed"));
        assert_eq!(
            prepared.source_metadata["dictation"]["task_kind"],
            "english_spelling"
        );
        assert_eq!(prepared.source_metadata["dictation"]["source"], "typed");
        assert_eq!(
            prepared.source_metadata["dictation"].get("evidence_refs"),
            None
        );
        assert_eq!(
            prepared.source_metadata["dictation"].get("input_confirmation"),
            None
        );
        assert_eq!(
            prepared.trace_metadata["namespace"],
            "child.english.spelling"
        );
        assert_eq!(prepared.trace_metadata["input_source"], "typed");
        assert_eq!(prepared.trace_metadata["dictation"]["item_count"], 2);
    }

    #[test]
    fn typed_dictation_capture_payload_rejects_media_descriptors() {
        let payload: CapturePayload = serde_json::from_value(json!({
            "task_kind": "english_spelling",
            "source": "typed",
            "input_confirmation": {
                "status": "confirmed",
                "method": "explicit_acceptance"
            },
            "prompt_items": [
                {"item_kind": "english_word", "expected_text": "because", "metadata": {}}
            ]
        }))
        .unwrap();

        let err = prepare_capture_payload("child.english.spelling", payload).unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
        assert!(err
            .to_string()
            .contains("typed or pasted dictation capture cannot include input_confirmation"));
    }

    #[test]
    fn media_dictation_capture_payload_accepts_validated_refs_without_persisting_them() {
        let payload: CapturePayload = serde_json::from_value(json!({
            "task_kind": "english_spelling",
            "source": "agent_ocr",
            "input_confirmation": {
                "status": "confirmed",
                "method": "explicit_correction"
            },
            "evidence_refs": [{
                "provider": "agent_ocr",
                "locator": "s3://dictation/archive/worksheet-2026-06-24.png",
                "media_type": "image/png",
                "metadata": {"page": 1}
            }],
            "prompt_items": [
                {"item_kind": "english_word", "expected_text": "because", "metadata": {}}
            ]
        }))
        .unwrap();

        let prepared =
            prepare_capture_payload("child.english.spelling", payload).expect("media dictation");
        let combined_persistence =
            format!("{}{}", prepared.source_metadata, prepared.trace_metadata);

        assert_eq!(prepared.input_source.as_deref(), Some("agent_ocr"));
        assert_eq!(
            prepared.trace_metadata["dictation"]["evidence_ref_count"],
            1
        );
        assert!(!combined_persistence.contains("worksheet-2026-06-24"));
        assert!(!combined_persistence.contains("evidence_refs"));
    }

    #[test]
    fn media_dictation_attempt_payload_rejects_confirmation_matrix_at_surface_boundary() {
        for source in ["agent_ocr", "agent_transcribed", "mixed"] {
            let missing = dictation_attempt_payload_with_confirmation(source, None);
            let err =
                prepare_submit_attempt_payload("child.english.spelling", missing).unwrap_err();
            assert!(matches!(err, AppError::BadRequest(_)));
            assert!(err
                .to_string()
                .contains("input_confirmation is required for media-derived dictation attempt"));

            let unconfirmed = dictation_attempt_payload_with_confirmation(
                source,
                Some(json!({
                    "status": "unknown",
                    "method": "explicit_acceptance"
                })),
            );
            let err =
                prepare_submit_attempt_payload("child.english.spelling", unconfirmed).unwrap_err();
            assert!(matches!(err, AppError::BadRequest(_)));
            assert!(err.to_string().contains(
                "input_confirmation must be confirmed by explicit acceptance or correction"
            ));

            let invalid_method = dictation_attempt_payload_with_confirmation(
                source,
                Some(json!({
                    "status": "confirmed",
                    "method": "manual_bypass"
                })),
            );
            let err = prepare_submit_attempt_payload("child.english.spelling", invalid_method)
                .unwrap_err();
            assert!(matches!(err, AppError::BadRequest(_)));
            assert!(err.to_string().contains(
                "input_confirmation must be confirmed by explicit acceptance or correction"
            ));
        }
    }

    fn dictation_attempt_payload_with_confirmation(
        source: &str,
        input_confirmation: Option<Value>,
    ) -> SubmitAttemptPayload {
        let mut payload = json!({
            "space_id": Uuid::new_v4(),
            "task_kind": "english_spelling",
            "source": source,
            "prompt_items": [
                {"item_kind": "english_word", "expected_text": "because", "metadata": {}}
            ],
            "submitted_items": [
                {"actual_text": "becaus", "metadata": {}}
            ],
            "task": "Today's spelling words",
            "goal": "Practice child.english.spelling",
            "metadata": {}
        });

        if let Some(input_confirmation) = input_confirmation {
            payload["input_confirmation"] = input_confirmation;
        }

        serde_json::from_value(payload).expect("test payload should deserialize")
    }
}
