use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Error, PgPool};
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
use crate::domain::event::{
    EngineEvent, EngineEventEnvelope, EnginePayloadRef, EnginePayloadRefKind,
};
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
        (Surface::Reflection, SurfaceAction::ReviewEvidence) => {
            review_evidence(&state, auth_user.user_id, request).await
        }
        (Surface::Planning, SurfaceAction::GenerateNextTask) => {
            generate_next_task(&state, auth_user.user_id, request).await
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
}
