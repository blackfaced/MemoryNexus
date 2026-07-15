use std::time::Duration;

use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Error, FromRow, PgPool, Postgres};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::{
    insert_feedback_loop_in_tx, CreateFeedbackLoop, FeedbackLoopDb, PatchFeedbackLoop,
};
use crate::db::memory::{CreateMemory, MemoryType};
use crate::db::sleep_cycles::{
    CompleteSleepCycle, CreateSleepCycle, PostgresSleepCycleRepository, SleepCycleRepository,
};
use crate::db::space::SpaceMemberRole;
use crate::db::trace::{
    insert_completed_trace_in_tx, CreateCompletedTrace, TraceMode, TraceRuntime, TraceSourceType,
    TraceTaskType,
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
use crate::domain::personal_feedback::{
    ConfirmedSleepEnergyCheckIn, SleepEnergyCheckInInput, PERSONAL_HEALTH_SLEEP_NAMESPACE,
};
use crate::domain::personal_feedback_observation::{
    build_personal_feedback_observation_summary, PersonalFeedbackObservationSummary,
    SleepObservationEvidenceRecord,
};
use crate::domain::personal_feedback_planning::{
    select_personal_feedback_experiment, PersonalFeedbackPlanningStatus, WakeTimeWindow,
    WakeTimeWindowInput,
};
use crate::domain::practice_plan::{
    build_adjusted_plan, build_next_task_plan, AdjustPlanRequest, PlanningRequest,
};
use crate::domain::reflection::{
    build_reflection_insight, EvidenceRef, EvidenceRefKind, ReflectionEvidence, ReflectionRequest,
};
use crate::domain::sleep_cycle::{SleepCycleStatus, SleepCycleType};
use crate::domain::surface::{
    RuntimePreference, Surface, SurfaceAction, SurfaceAdapter, SurfaceRequest, SurfaceResponse,
    SurfaceVisibility,
};
use crate::domain::LensStrategyRef;
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
    trace_input_summary: Option<String>,
    sleep_check_in: Option<ConfirmedSleepEnergyCheckIn>,
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
    source_event_id: Option<String>,
    normalized_outcome: Option<NormalizedLearningOutcome>,
}

/// A deliberately small Adapter-to-Performance contract. It contains only
/// confirmed text outcomes, never provider conversations or raw media.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NormalizedLearningOutcome {
    summary: String,
    mistake: Option<ConfirmedLearningMistake>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfirmedLearningMistake {
    expected_text: String,
    actual_text: String,
    mistake_type: String,
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
    idempotency: Option<PerformanceIdempotency>,
}

#[derive(Debug)]
struct PerformanceIdempotency {
    source_event_id: String,
    payload_fingerprint: String,
}

#[derive(Debug, Deserialize)]
struct ManualConsolidationPayload {
    space_id: Uuid,
    evidence_window_start: DateTime<Utc>,
    evidence_window_end: DateTime<Utc>,
    #[serde(default)]
    knowledge_context_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ReviewEvidencePayload {
    space_id: Uuid,
    lens_strategy: Option<LensStrategyRef>,
    question: Option<String>,
    #[serde(default)]
    evidence: Vec<ReflectionEvidence>,
}

#[derive(Debug, Deserialize)]
struct GenerateNextTaskPayload {
    space_id: Uuid,
    objective: Option<String>,
    owner_selected_wake_time_window: Option<WakeTimeWindowInput>,
}

#[derive(Debug, FromRow)]
struct PersonalFeedbackLifecycleRow {
    id: Uuid,
    feedback_loop_id: Uuid,
    planning_trace_id: Option<Uuid>,
    policy_version: String,
    action_id: String,
    action: Value,
    selected_evidence_ids: Value,
    expected_signal: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct AdjustPlanPayload {
    space_id: Uuid,
    proposed_plan: Value,
    #[serde(default)]
    evidence: Vec<Value>,
    #[serde(default)]
    constraints: Vec<String>,
    objective: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetStateSummaryPayload {
    space_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct KnowledgeSourceCandidateEnvelope {
    knowledge_source_candidate: KnowledgeSourceCandidateInput,
}

#[derive(Debug, Deserialize)]
struct KnowledgeContextEnvelope {
    knowledge_context: KnowledgeContextInput,
}

#[derive(Debug, Deserialize)]
struct KnowledgeSourceCandidateInput {
    id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
    state: String,
    proposed_source: Value,
    proposed_use: String,
    proposer: String,
    acquisition_trace: AcquisitionTraceInput,
    private_context_used: bool,
    opt_in_proof: Option<Value>,
    provenance: Value,
    quality_signals: Value,
    freshness: Value,
    expiry: DateTime<Utc>,
    #[serde(default)]
    downstream_link_candidates: Value,
    decision: Option<Value>,
    #[serde(default)]
    metadata: Value,
    source_policy: Option<KnowledgeSourcePolicyInput>,
}

#[derive(Debug, Deserialize)]
struct KnowledgeSourcePolicyInput {
    id: Uuid,
    state: String,
    source_descriptor: Value,
    #[serde(default)]
    allowed_use: Value,
    #[serde(default)]
    disallowed_use: Value,
    #[serde(default)]
    privacy_policy: Value,
    #[serde(default)]
    refresh_policy: Value,
    #[serde(default)]
    quality_thresholds: Value,
    #[serde(default)]
    freshness_requirements: Value,
    expiry: DateTime<Utc>,
    approved_by: String,
    approved_at: DateTime<Utc>,
    revoked_or_paused_reason: Option<String>,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
struct KnowledgeContextInput {
    id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
    source_policy_id: Uuid,
    source_candidate_id: Uuid,
    acquisition_trace: AcquisitionTraceInput,
    state: String,
    context_type: String,
    structured_claims: Value,
    provenance: Value,
    quality_signals: Value,
    freshness: Value,
    expiry: DateTime<Utc>,
    evidence_snippets: Value,
    private_context_used: bool,
    opt_in_proof: Option<Value>,
    downstream_links: Value,
    #[serde(default)]
    conflict_notes: Value,
    #[serde(default)]
    metadata: Value,
}

#[derive(Debug, Deserialize)]
struct AcquisitionTraceInput {
    id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
    submitted_by: String,
    acquisition_kind: String,
    discovery_method: String,
    extraction_method: String,
    private_context_used: bool,
    private_context_basis: Option<Value>,
    opt_in_proof: Option<Value>,
    source_handles: Value,
    source_observed_at: DateTime<Utc>,
    extraction_run_id: Option<String>,
    tool_or_adapter_version: Option<String>,
    validation_summary: Value,
    #[serde(default)]
    redacted_diagnostics: Value,
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
        (Surface::Planning, SurfaceAction::AdjustPlan) => {
            adjust_plan(&state, auth_user.user_id, request).await
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
    let namespace_observation = load_namespace_observation(
        &state.db,
        payload.space_id,
        namespace.id,
        &request.namespace,
        started_at + chrono::Duration::minutes(1),
    )
    .await?;
    let knowledge_refresh =
        load_knowledge_refresh_observation(&state.db, payload.space_id, namespace.id).await?;
    let output_summary = format!(
        "Observed {}: {} memories, {} traces, {} feedback loops",
        request.namespace,
        counts.memory_count,
        counts.trace_count,
        counts.feedback_loop_total()
    );

    let mut result = json!({
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
        "knowledge_refresh": knowledge_refresh,
    });
    result
        .as_object_mut()
        .expect("state summary result is an object")
        .insert(
            namespace_observation.response_key.to_string(),
            namespace_observation.response.clone(),
        );

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
            related_memory_ids: namespace_observation.related_memory_ids.clone(),
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
                "dictation_observation_status": if namespace_observation.response_key == "dictation_observation" { namespace_observation.trace_metadata["status"].clone() } else { Value::Null },
                "dictation_observation_evidence_record_count": if namespace_observation.response_key == "dictation_observation" { namespace_observation.trace_metadata["evidence_record_count"].clone() } else { Value::Null },
                "namespace_observation": namespace_observation.trace_metadata,
                "knowledge_refresh_source_candidate_count": knowledge_refresh["source_candidates"].as_array().map(Vec::len).unwrap_or(0),
                "knowledge_refresh_source_policy_count": knowledge_refresh["source_policies"].as_array().map(Vec::len).unwrap_or(0),
                "knowledge_refresh_context_count": knowledge_refresh["knowledge_contexts"].as_array().map(Vec::len).unwrap_or(0),
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let response = SurfaceResponse::new(
        Surface::Observation,
        SurfaceAction::GetStateSummary,
        result,
        trace.id,
        if request.namespace == PERSONAL_HEALTH_SLEEP_NAMESPACE {
            Vec::new()
        } else {
            vec![
                "Use Planning Surface when the observed state should become a next task."
                    .to_string(),
            ]
        },
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

    if request.namespace == PERSONAL_HEALTH_SLEEP_NAMESPACE {
        return generate_personal_feedback_next_task(
            state,
            user_id,
            request,
            payload,
            namespace.id,
        )
        .await;
    }

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

async fn generate_personal_feedback_next_task(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
    payload: GenerateNextTaskPayload,
    namespace_id: Uuid,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let evidence =
        load_personal_feedback_observation_evidence(&state.db, payload.space_id, namespace_id)
            .await?;
    let wake_window = payload
        .owner_selected_wake_time_window
        .map(WakeTimeWindow::parse)
        .transpose()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let planning = select_personal_feedback_experiment(evidence, wake_window.as_ref());
    let evidence_memory_ids = planning
        .supporting_evidence_ids
        .iter()
        .filter_map(|id| match id {
            EvidenceId::Memory(id) => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();

    let (result, generated_feedback_loop_ids, lifecycle_id, output_summary) = match planning.status
    {
        PersonalFeedbackPlanningStatus::ExperimentReady => {
            let action = planning.action.as_ref().expect("ready policy has action");
            let lifecycle = create_or_reuse_personal_feedback_lifecycle(
                &state.db,
                payload.space_id,
                namespace_id,
                user_id,
                action,
            )
            .await?;
            let result = json!({
                "status": "experiment_ready",
                "policy_version": lifecycle.policy_version,
                "experiment": {
                    "lifecycle_id": lifecycle.id,
                    "status": lifecycle.status,
                    "action_id": lifecycle.action_id,
                    "action": lifecycle.action,
                    "expected_observable_signal": lifecycle.expected_signal,
                    "selected_evidence_ids": lifecycle.selected_evidence_ids,
                    "feedback_loop_id": lifecycle.feedback_loop_id,
                },
                "window": planning.window,
                "valid_record_count": planning.valid_record_count,
                "threshold": planning.threshold,
            });
            (
                result,
                vec![lifecycle.feedback_loop_id],
                Some(lifecycle.id),
                "Generated or reused one reviewed personal experiment.".to_string(),
            )
        }
        PersonalFeedbackPlanningStatus::NeedsMoreEvidence
        | PersonalFeedbackPlanningStatus::ActionEvidenceGap => {
            let result = serde_json::to_value(&planning).map_err(|error| {
                AppError::Internal(format!("planning response failed: {error}"))
            })?;
            (
                result,
                Vec::new(),
                None,
                "Planning returned an evidence gap without an experiment lifecycle.".to_string(),
            )
        }
    };
    let completed_at = Utc::now();
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: payload.space_id,
            namespace_id: Some(namespace_id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Planning,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: deterministic_trace_runtime(request.context.runtime_preference),
            input_summary: Some(
                "Planning personal feedback experiment from confirmed evidence.".to_string(),
            ),
            output_summary: Some(output_summary),
            started_at,
            completed_at,
            latency_ms: Some((completed_at - started_at).num_milliseconds().max(0)),
            model_provider: Some("deterministic".to_string()),
            model_name: None,
            token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
            estimated_cost_usd: Some(0.0),
            local_processing_ratio: Some(1.0),
            related_memory_ids: evidence_memory_ids,
            generated_memory_ids: Vec::new(),
            generated_lens_run_ids: Vec::new(),
            generated_review_report_ids: Vec::new(),
            generated_feedback_loop_ids: generated_feedback_loop_ids.clone(),
            user_feedback: None,
            error: None,
            metadata: json!({
                "surface_gateway": true, "surface": request.surface, "action": request.action,
                "adapter": request.adapter, "namespace": request.namespace, "deterministic": true,
                "policy_version": planning.policy_version,
                "valid_record_count": planning.valid_record_count,
                "threshold": planning.threshold,
                "window_kind": planning.window.kind,
                "window_start_local_date": planning.window.start_local_date,
                "window_end_local_date": planning.window.end_local_date,
                "result_status": planning.status,
            }),
        })
        .await
        .map_err(AppError::Database)?;
    if let Some(lifecycle_id) = lifecycle_id {
        // The lifecycle is created before the repository trace API is called.
        // Attach the completed trace in a second idempotent step so a retry can
        // repair a transient trace write failure without creating another active
        // experiment (the partial unique index remains authoritative).
        sqlx::query("UPDATE planning_lifecycles SET planning_trace_id = $2, updated_at = NOW() WHERE id = $1 AND planning_trace_id IS NULL")
            .bind(lifecycle_id).bind(trace.id).execute(&state.db).await.map_err(AppError::Database)?;
    }
    let guidance = match planning.status {
        PersonalFeedbackPlanningStatus::NeedsMoreEvidence => {
            vec!["Add confirmed daily records before starting an experiment.".to_string()]
        }
        PersonalFeedbackPlanningStatus::ActionEvidenceGap => vec![
            "Provide the confirmed fields required by a reviewed sleep experiment, including a valid owner-selected wake-time window when using the wake-time action.".to_string(),
        ],
        PersonalFeedbackPlanningStatus::ExperimentReady => vec![
            "Keep recording confirmed daily check-ins while trying the selected experiment."
                .to_string(),
        ],
    };
    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(SurfaceResponse::new(
            Surface::Planning,
            SurfaceAction::GenerateNextTask,
            result,
            trace.id,
            guidance,
            SurfaceVisibility::User,
        ))),
    ))
}

#[allow(clippy::too_many_arguments)]
async fn create_or_reuse_personal_feedback_lifecycle(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    user_id: Uuid,
    action: &crate::domain::personal_feedback_planning::PersonalFeedbackExperimentAction,
) -> Result<PersonalFeedbackLifecycleRow, AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    // Serializes competing first creations for this Space/Namespace without
    // making Namespace a permission boundary. The partial unique index remains
    // the durable invariant if a caller bypasses this code path.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(space_id.to_string())
        .bind(namespace_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    if let Some(existing) =
        find_active_personal_feedback_lifecycle(&mut tx, space_id, namespace_id).await?
    {
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(existing);
    }
    let loop_row = insert_feedback_loop_in_tx(
        &mut tx,
        &CreateFeedbackLoop {
            space_id,
            namespace_id,
            goal: "Try one owner-selected sleep routine experiment".to_string(),
            task: action.advisory_text.to_string(),
            attempt: None,
            evaluation: None,
            feedback: None,
            adjustment: None,
            next_task: None,
            status: "active".to_string(),
            created_by: user_id,
        },
    )
    .await
    .map_err(AppError::Database)?;
    let row = sqlx::query_as::<_, PersonalFeedbackLifecycleRow>(
        "INSERT INTO planning_lifecycles (space_id, namespace_id, feedback_loop_id, policy_version, action_id, action, selected_evidence_ids, expected_signal) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id, feedback_loop_id, planning_trace_id, policy_version, action_id, action, selected_evidence_ids, expected_signal, status"
    ).bind(space_id).bind(namespace_id).bind(loop_row.id)
        .bind(crate::domain::personal_feedback_planning::PERSONAL_FEEDBACK_POLICY_VERSION)
        .bind(action.action_id).bind(serde_json::to_value(action).expect("action serialize"))
        .bind(serde_json::to_value(&action.selected_evidence_ids).expect("evidence ids serialize"))
        .bind(&action.expected_observable_signal).fetch_one(&mut *tx).await.map_err(AppError::Database)?;
    tx.commit().await.map_err(AppError::Database)?;
    Ok(row)
}

async fn find_active_personal_feedback_lifecycle(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<Option<PersonalFeedbackLifecycleRow>, AppError> {
    sqlx::query_as::<_, PersonalFeedbackLifecycleRow>(
        "SELECT id, feedback_loop_id, planning_trace_id, policy_version, action_id, action, selected_evidence_ids, expected_signal, status FROM planning_lifecycles WHERE space_id = $1 AND namespace_id = $2 AND status = 'active' FOR UPDATE"
    ).bind(space_id).bind(namespace_id).fetch_optional(&mut **tx).await.map_err(AppError::Database)
}

async fn adjust_plan(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let payload: AdjustPlanPayload = serde_json::from_value(request.payload.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid adjustPlan payload: {error}")))?;

    require_space_writer(state, payload.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, payload.space_id, &request.namespace).await?;

    let adjusted = build_adjusted_plan(&AdjustPlanRequest {
        space_id: payload.space_id,
        namespace_id: namespace.id,
        namespace: request.namespace.clone(),
        proposed_plan: payload.proposed_plan.clone(),
        evidence: payload.evidence.clone(),
        constraints: payload.constraints.clone(),
        objective: payload.objective.clone(),
    });
    let output_summary = format!(
        "Adjusted response-only plan for {}: {}",
        adjusted.namespace, adjusted.adjusted_plan.prompt
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
                "Planning adjustPlan in namespace {}",
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
                "plan_kind": adjusted.plan_kind.clone(),
                "persistence": adjusted.persistence.clone(),
                "evidence_record_count": adjusted.evidence_summary.record_count,
                "constraint_count": adjusted.evidence_summary.constraint_count,
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let result = serde_json::to_value(adjusted)
        .map_err(|error| AppError::Internal(format!("planning response failed: {error}")))?;
    let response = SurfaceResponse::new(
        Surface::Planning,
        SurfaceAction::AdjustPlan,
        result,
        trace.id,
        vec![
            "Submit the adjusted task through Performance Surface after it is attempted."
                .to_string(),
        ],
        SurfaceVisibility::User,
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

async fn capture_observation(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    if request.payload.get("knowledge_source_candidate").is_some() {
        return capture_knowledge_source_candidate(state, user_id, request).await;
    }
    if request.payload.get("knowledge_context").is_some() {
        return capture_knowledge_context(state, user_id, request).await;
    }

    let started_at = Utc::now();
    let prepared = prepare_capture_payload(&request.namespace, request.payload.clone())?;

    let space = state
        .repositories
        .spaces
        .default_for_user(user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Cognitive space not found".to_string()))?;
    require_space_writer(state, space.id, user_id).await?;
    let namespace = resolve_namespace_by_name(state, user_id, space.id, &request.namespace).await?;

    if prepared.sleep_check_in.is_some() {
        return capture_sleep_observation_with_lock(
            state,
            user_id,
            request,
            space.id,
            namespace.id,
            prepared,
            started_at,
        )
        .await;
    }

    persist_capture_observation(
        state,
        user_id,
        request,
        space.id,
        namespace.id,
        prepared,
        started_at,
    )
    .await
}

async fn capture_sleep_observation_with_lock(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
    space_id: Uuid,
    namespace_id: Uuid,
    prepared: PreparedCapturePayload,
    started_at: DateTime<Utc>,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let check_in = prepared
        .sleep_check_in
        .as_ref()
        .expect("sleep Capture path requires a sleep check-in");
    let lock_key = format!(
        "personal.health.sleep:{space_id}:{namespace_id}:{}",
        check_in.local_date
    );
    let lock_transaction = acquire_sleep_check_in_lock(state, &lock_key).await?;

    let result = persist_capture_observation(
        state,
        user_id,
        request,
        space_id,
        namespace_id,
        prepared,
        started_at,
    )
    .await;

    match result {
        Ok(response) => {
            lock_transaction
                .commit()
                .await
                .map_err(AppError::Database)?;
            Ok(response)
        }
        Err(error) => {
            let _ = lock_transaction.rollback().await;
            Err(error)
        }
    }
}

async fn acquire_sleep_check_in_lock<'a>(
    state: &'a AppState,
    lock_key: &str,
) -> Result<sqlx::Transaction<'a, Postgres>, AppError> {
    loop {
        let mut transaction = state.db.begin().await.map_err(AppError::Database)?;
        let acquired: bool =
            sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))")
                .bind(lock_key)
                .fetch_one(&mut *transaction)
                .await
                .map_err(AppError::Database)?;
        if acquired {
            return Ok(transaction);
        }
        transaction.rollback().await.map_err(AppError::Database)?;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

async fn persist_capture_observation(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
    space_id: Uuid,
    namespace_id: Uuid,
    prepared: PreparedCapturePayload,
    started_at: DateTime<Utc>,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    validate_sleep_check_in_scope(state, space_id, namespace_id, &prepared).await?;

    let memory = state
        .repositories
        .memories
        .create(CreateMemory {
            user_id,
            space_id,
            namespace_id: Some(namespace_id),
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
            space_id,
            namespace_id: Some(namespace_id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Capture,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: trace_runtime(request.context.runtime_preference),
            input_summary: Some(
                prepared
                    .trace_input_summary
                    .clone()
                    .unwrap_or_else(|| redacted_summary(&prepared.content)),
            ),
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

    if let Some(check_in) = &prepared.sleep_check_in {
        if let Some(corrects_record_id) = check_in.corrects_record_id {
            mark_sleep_check_in_superseded(state, corrects_record_id, memory.id).await?;
        }
    }

    let event = EngineEvent::ObservationCaptured(EngineEventEnvelope {
        space_id,
        namespace_id,
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
            "namespace_id": namespace_id,
            "status": "captured",
            "event": event,
        }),
        trace.id,
        vec!["Use Performance Surface when this observation becomes an attempt.".to_string()],
        SurfaceVisibility::User,
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

async fn capture_knowledge_source_candidate(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let envelope: KnowledgeSourceCandidateEnvelope =
        serde_json::from_value(request.payload.clone()).map_err(|error| {
            AppError::BadRequest(format!("invalid KnowledgeSourceCandidateInput: {error}"))
        })?;
    let input = envelope.knowledge_source_candidate;

    validate_knowledge_source_candidate_input(&input)?;
    require_space_writer(state, input.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, input.space_id, &request.namespace).await?;
    validate_knowledge_scope(
        input.space_id,
        input.namespace_id,
        input.space_id,
        namespace.id,
    )?;
    validate_acquisition_trace(&input.acquisition_trace, input.space_id, namespace.id)?;

    if let Some(policy) = &input.source_policy {
        validate_knowledge_policy_input(policy)?;
        if input.state != "approved" {
            return Err(AppError::BadRequest(
                "KnowledgeSourcePolicy requires an approved source candidate".to_string(),
            ));
        }
    }

    insert_acquisition_trace(&state.db, &input.acquisition_trace).await?;
    insert_knowledge_source_candidate(&state.db, &input).await?;
    if let Some(policy) = &input.source_policy {
        insert_knowledge_source_policy(&state.db, &input, policy).await?;
    }

    let completed_at = Utc::now();
    let output_summary = format!(
        "Captured KnowledgeSourceCandidate {} in state {}",
        input.id, input.state
    );
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: input.space_id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Capture,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: deterministic_trace_runtime(request.context.runtime_preference),
            input_summary: Some(format!(
                "Capture KnowledgeSourceCandidate in namespace {}",
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
                "knowledge_refresh": {
                    "kind": "source_candidate",
                    "source_candidate_id": input.id,
                    "source_policy_id": input.source_policy.as_ref().map(|policy| policy.id),
                    "acquisition_trace_id": input.acquisition_trace.id,
                    "state": input.state,
                },
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let response = SurfaceResponse::new(
        Surface::Capture,
        SurfaceAction::CaptureObservation,
        json!({
            "status": "knowledge_source_candidate_accepted",
            "source_candidate_id": input.id,
            "source_policy_id": input.source_policy.as_ref().map(|policy| policy.id),
            "acquisition_trace_id": input.acquisition_trace.id,
            "namespace_id": namespace.id,
            "state": input.state,
        }),
        trace.id,
        vec!["Use Observation Surface to inspect Knowledge Refresh state.".to_string()],
        SurfaceVisibility::Adapter,
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

async fn capture_knowledge_context(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    let envelope: KnowledgeContextEnvelope = serde_json::from_value(request.payload.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid KnowledgeContextInput: {error}")))?;
    let input = envelope.knowledge_context;

    validate_knowledge_context_input(&input)?;
    require_space_writer(state, input.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, input.space_id, &request.namespace).await?;
    validate_knowledge_scope(
        input.space_id,
        input.namespace_id,
        input.space_id,
        namespace.id,
    )?;
    validate_acquisition_trace(&input.acquisition_trace, input.space_id, namespace.id)?;
    validate_context_policy_state(&state.db, &input).await?;

    insert_acquisition_trace(&state.db, &input.acquisition_trace).await?;
    insert_knowledge_context(&state.db, &input).await?;

    let completed_at = Utc::now();
    let output_summary = format!("Captured KnowledgeContext {}", input.id);
    let trace = state
        .repositories
        .traces
        .create_completed(CreateCompletedTrace {
            space_id: input.space_id,
            namespace_id: Some(namespace.id),
            source_type: trace_source_type(request.adapter),
            task_type: TraceTaskType::Capture,
            mode: trace_mode(request.context.mode.as_deref())?,
            runtime: deterministic_trace_runtime(request.context.runtime_preference),
            input_summary: Some(format!(
                "Capture KnowledgeContext in namespace {}",
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
                "knowledge_refresh": {
                    "kind": "knowledge_context",
                    "knowledge_context_id": input.id,
                    "source_candidate_id": input.source_candidate_id,
                    "source_policy_id": input.source_policy_id,
                    "acquisition_trace_id": input.acquisition_trace.id,
                    "state": input.state,
                },
            }),
        })
        .await
        .map_err(AppError::Database)?;

    let response = SurfaceResponse::new(
        Surface::Capture,
        SurfaceAction::CaptureObservation,
        json!({
            "status": "knowledge_context_accepted",
            "knowledge_context_id": input.id,
            "source_candidate_id": input.source_candidate_id,
            "source_policy_id": input.source_policy_id,
            "acquisition_trace_id": input.acquisition_trace.id,
            "namespace_id": namespace.id,
            "state": input.state,
        }),
        trace.id,
        vec!["Use Observation Surface to inspect KnowledgeContext state.".to_string()],
        SurfaceVisibility::Adapter,
    );

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

fn validate_knowledge_source_candidate_input(
    input: &KnowledgeSourceCandidateInput,
) -> Result<(), AppError> {
    validate_allowed_state(
        &input.state,
        &["proposed", "approved", "rejected", "expired"],
        "KnowledgeSourceCandidate.state",
    )?;
    validate_required_text(&input.proposed_use, "KnowledgeSourceCandidate.proposed_use")?;
    validate_required_text(&input.proposer, "KnowledgeSourceCandidate.proposer")?;
    require_json_object(
        &input.proposed_source,
        "KnowledgeSourceCandidate.proposed_source",
    )?;
    require_json_object(&input.provenance, "KnowledgeSourceCandidate.provenance")?;
    require_json_object(
        &input.quality_signals,
        "KnowledgeSourceCandidate.quality_signals",
    )?;
    require_json_object(&input.freshness, "KnowledgeSourceCandidate.freshness")?;
    validate_future_expiry_unless_expired(
        &input.state,
        input.expiry,
        "KnowledgeSourceCandidate.expiry",
    )?;
    require_json_array(
        &input.downstream_link_candidates,
        "KnowledgeSourceCandidate.downstream_link_candidates",
    )?;
    require_json_object(&input.metadata, "KnowledgeSourceCandidate.metadata")?;
    validate_private_context_opt_in(
        input.private_context_used,
        input.opt_in_proof.as_ref(),
        input.namespace_id,
        "KnowledgeSourceCandidate.opt_in_proof",
    )?;
    validate_knowledge_value_safety(
        &json!({
            "proposed_source": input.proposed_source,
            "provenance": input.provenance,
            "quality_signals": input.quality_signals,
            "freshness": input.freshness,
            "downstream_link_candidates": input.downstream_link_candidates,
            "decision": input.decision,
            "metadata": input.metadata,
            "opt_in_proof": input.opt_in_proof,
        }),
        "KnowledgeSourceCandidate",
    )?;
    Ok(())
}

fn validate_knowledge_policy_input(input: &KnowledgeSourcePolicyInput) -> Result<(), AppError> {
    validate_allowed_state(
        &input.state,
        &["active", "paused", "revoked", "expired"],
        "KnowledgeSourcePolicy.state",
    )?;
    validate_required_text(&input.approved_by, "KnowledgeSourcePolicy.approved_by")?;
    require_json_object(
        &input.source_descriptor,
        "KnowledgeSourcePolicy.source_descriptor",
    )?;
    require_json_array(&input.allowed_use, "KnowledgeSourcePolicy.allowed_use")?;
    require_json_array(
        &input.disallowed_use,
        "KnowledgeSourcePolicy.disallowed_use",
    )?;
    require_json_object(
        &input.privacy_policy,
        "KnowledgeSourcePolicy.privacy_policy",
    )?;
    require_json_object(
        &input.refresh_policy,
        "KnowledgeSourcePolicy.refresh_policy",
    )?;
    require_json_object(
        &input.quality_thresholds,
        "KnowledgeSourcePolicy.quality_thresholds",
    )?;
    require_json_object(
        &input.freshness_requirements,
        "KnowledgeSourcePolicy.freshness_requirements",
    )?;
    validate_future_expiry_unless_expired(
        &input.state,
        input.expiry,
        "KnowledgeSourcePolicy.expiry",
    )?;
    require_json_object(&input.metadata, "KnowledgeSourcePolicy.metadata")?;
    validate_knowledge_value_safety(
        &json!({
            "source_descriptor": input.source_descriptor,
            "allowed_use": input.allowed_use,
            "disallowed_use": input.disallowed_use,
            "privacy_policy": input.privacy_policy,
            "refresh_policy": input.refresh_policy,
            "quality_thresholds": input.quality_thresholds,
            "freshness_requirements": input.freshness_requirements,
            "revoked_or_paused_reason": input.revoked_or_paused_reason,
            "metadata": input.metadata,
        }),
        "KnowledgeSourcePolicy",
    )?;
    Ok(())
}

fn validate_knowledge_context_input(input: &KnowledgeContextInput) -> Result<(), AppError> {
    validate_allowed_state(
        &input.state,
        &["candidate", "valid", "rejected", "expired"],
        "KnowledgeContext.state",
    )?;
    validate_allowed_state(
        &input.context_type,
        &[
            "reference_claims",
            "rubric_context",
            "practice_context",
            "trend_context",
            "contradiction_context",
            "review_context",
        ],
        "KnowledgeContext.context_type",
    )?;
    require_json_array(
        &input.structured_claims,
        "KnowledgeContext.structured_claims",
    )?;
    require_non_empty_array(
        &input.structured_claims,
        "KnowledgeContext.structured_claims",
    )?;
    require_json_object(&input.provenance, "KnowledgeContext.provenance")?;
    require_json_object(&input.quality_signals, "KnowledgeContext.quality_signals")?;
    require_json_object(&input.freshness, "KnowledgeContext.freshness")?;
    require_json_array(
        &input.evidence_snippets,
        "KnowledgeContext.evidence_snippets",
    )?;
    require_non_empty_array(
        &input.evidence_snippets,
        "KnowledgeContext.evidence_snippets",
    )?;
    validate_future_expiry_unless_expired(&input.state, input.expiry, "KnowledgeContext.expiry")?;
    require_json_array(&input.downstream_links, "KnowledgeContext.downstream_links")?;
    require_json_array(&input.conflict_notes, "KnowledgeContext.conflict_notes")?;
    require_json_object(&input.metadata, "KnowledgeContext.metadata")?;
    validate_private_context_opt_in(
        input.private_context_used,
        input.opt_in_proof.as_ref(),
        input.namespace_id,
        "KnowledgeContext.opt_in_proof",
    )?;
    validate_knowledge_value_safety(
        &json!({
            "structured_claims": input.structured_claims,
            "provenance": input.provenance,
            "quality_signals": input.quality_signals,
            "freshness": input.freshness,
            "evidence_snippets": input.evidence_snippets,
            "opt_in_proof": input.opt_in_proof,
            "downstream_links": input.downstream_links,
            "conflict_notes": input.conflict_notes,
            "metadata": input.metadata,
        }),
        "KnowledgeContext",
    )?;
    Ok(())
}

fn validate_acquisition_trace(
    input: &AcquisitionTraceInput,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<(), AppError> {
    validate_knowledge_scope(input.space_id, input.namespace_id, space_id, namespace_id)?;
    validate_allowed_state(
        &input.acquisition_kind,
        &[
            "source_candidate",
            "source_policy_review",
            "knowledge_context",
            "revalidation",
        ],
        "AcquisitionTrace.acquisition_kind",
    )?;
    validate_required_text(&input.submitted_by, "AcquisitionTrace.submitted_by")?;
    validate_required_text(&input.discovery_method, "AcquisitionTrace.discovery_method")?;
    validate_required_text(
        &input.extraction_method,
        "AcquisitionTrace.extraction_method",
    )?;
    require_json_array(&input.source_handles, "AcquisitionTrace.source_handles")?;
    require_json_object(
        &input.validation_summary,
        "AcquisitionTrace.validation_summary",
    )?;
    require_json_object(
        &input.redacted_diagnostics,
        "AcquisitionTrace.redacted_diagnostics",
    )?;
    require_json_object(&input.metadata, "AcquisitionTrace.metadata")?;
    validate_private_context_opt_in(
        input.private_context_used,
        input.opt_in_proof.as_ref(),
        namespace_id,
        "AcquisitionTrace.opt_in_proof",
    )?;
    validate_knowledge_value_safety(
        &json!({
            "private_context_basis": input.private_context_basis,
            "opt_in_proof": input.opt_in_proof,
            "source_handles": input.source_handles,
            "extraction_run_id": input.extraction_run_id,
            "tool_or_adapter_version": input.tool_or_adapter_version,
            "validation_summary": input.validation_summary,
            "redacted_diagnostics": input.redacted_diagnostics,
            "metadata": input.metadata,
        }),
        "AcquisitionTrace",
    )?;
    Ok(())
}

fn validate_allowed_state(value: &str, allowed: &[&str], field: &str) -> Result<(), AppError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!("{field} is invalid")))
    }
}

fn validate_required_text(value: &str, field: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        Err(AppError::BadRequest(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn validate_future_expiry_unless_expired(
    state: &str,
    expiry: DateTime<Utc>,
    field: &str,
) -> Result<(), AppError> {
    if state != "expired" && expiry <= Utc::now() {
        Err(AppError::BadRequest(format!(
            "{field} must be in the future unless state is expired"
        )))
    } else {
        Ok(())
    }
}

fn require_json_object(value: &Value, field: &str) -> Result<(), AppError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!("{field} must be an object")))
    }
}

fn require_json_array(value: &Value, field: &str) -> Result<(), AppError> {
    if value.is_array() {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!("{field} must be an array")))
    }
}

fn require_non_empty_array(value: &Value, field: &str) -> Result<(), AppError> {
    if value.as_array().is_some_and(|items| !items.is_empty()) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!("{field} cannot be empty")))
    }
}

fn validate_knowledge_scope(
    object_space_id: Uuid,
    object_namespace_id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<(), AppError> {
    if object_space_id != space_id || object_namespace_id != namespace_id {
        return Err(AppError::BadRequest(
            "Knowledge Refresh object must belong to the requested Space and Namespace".to_string(),
        ));
    }
    Ok(())
}

fn validate_private_context_opt_in(
    private_context_used: bool,
    opt_in_proof: Option<&Value>,
    namespace_id: Uuid,
    field: &str,
) -> Result<(), AppError> {
    if !private_context_used {
        return Ok(());
    }
    let Some(proof) = opt_in_proof else {
        return Err(AppError::BadRequest(format!(
            "{field} is required when private_context_used is true"
        )));
    };
    if !opt_in_proof_matches_namespace(proof, namespace_id) {
        return Err(AppError::BadRequest(format!(
            "{field} must be scoped to the requested Namespace"
        )));
    }
    Ok(())
}

fn opt_in_proof_matches_namespace(proof: &Value, namespace_id: Uuid) -> bool {
    let namespace = namespace_id.to_string();
    proof
        .get("namespace_id")
        .and_then(Value::as_str)
        .is_some_and(|value| value == namespace)
        || proof
            .get("namespace_ids")
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value.as_str().is_some_and(|value| value == namespace))
            })
}

fn validate_knowledge_value_safety(value: &Value, path: &str) -> Result<(), AppError> {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                let nested_path = format!("{path}.{key}");
                if denied_knowledge_key(key) {
                    return Err(invalid_knowledge_reference(&nested_path));
                }
                validate_knowledge_value_safety(nested, &nested_path)?;
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_knowledge_value_safety(item, &format!("{path}[{index}]"))?;
            }
        }
        Value::String(text) if has_secret_like_value(text) => {
            return Err(invalid_knowledge_reference(path));
        }
        Value::String(_) => {}
        _ => {}
    }
    Ok(())
}

fn denied_knowledge_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "authorization"
            | "cookie"
            | "credentials"
            | "credential"
            | "password"
            | "secret"
            | "signed_url"
            | "api_key"
            | "access_key"
            | "secret_key"
            | "x-amz-signature"
            | "x-amz-credential"
            | "signature"
    )
}

fn has_secret_like_value(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("bearer ")
        || value.starts_with("-----BEGIN PRIVATE KEY-----")
        || value.starts_with("sk-")
        || value.starts_with("AKIA")
        || value.starts_with("AIza")
        || value.starts_with("ghp_")
        || lower.contains("x-amz-signature=")
        || lower.contains("x-amz-credential=")
        || lower.contains("sig=")
        || is_jwt_like_value(value)
}

fn is_jwt_like_value(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| part.len() >= 8 && part.chars().all(is_base64_url_char))
}

fn is_base64_url_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, '-' | '_')
}

fn invalid_knowledge_reference(path: &str) -> AppError {
    AppError::BadRequest(format!(
        "invalid_knowledge_reference: secret-bearing value rejected at {path}"
    ))
}

async fn insert_acquisition_trace(
    pool: &PgPool,
    input: &AcquisitionTraceInput,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO knowledge_acquisition_traces (
            id,
            space_id,
            namespace_id,
            submitted_by,
            acquisition_kind,
            discovery_method,
            extraction_method,
            private_context_used,
            private_context_basis,
            opt_in_proof,
            source_handles,
            source_observed_at,
            extraction_run_id,
            tool_or_adapter_version,
            validation_summary,
            redacted_diagnostics,
            metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        "#,
    )
    .bind(input.id)
    .bind(input.space_id)
    .bind(input.namespace_id)
    .bind(&input.submitted_by)
    .bind(&input.acquisition_kind)
    .bind(&input.discovery_method)
    .bind(&input.extraction_method)
    .bind(input.private_context_used)
    .bind(&input.private_context_basis)
    .bind(&input.opt_in_proof)
    .bind(&input.source_handles)
    .bind(input.source_observed_at)
    .bind(&input.extraction_run_id)
    .bind(&input.tool_or_adapter_version)
    .bind(&input.validation_summary)
    .bind(&input.redacted_diagnostics)
    .bind(&input.metadata)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(AppError::Database)
}

async fn insert_knowledge_source_candidate(
    pool: &PgPool,
    input: &KnowledgeSourceCandidateInput,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO knowledge_source_candidates (
            id,
            space_id,
            namespace_id,
            state,
            proposed_source,
            proposed_use,
            proposer,
            acquisition_trace_id,
            private_context_used,
            opt_in_proof,
            provenance,
            quality_signals,
            freshness,
            expiry,
            downstream_link_candidates,
            decision,
            metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        "#,
    )
    .bind(input.id)
    .bind(input.space_id)
    .bind(input.namespace_id)
    .bind(&input.state)
    .bind(&input.proposed_source)
    .bind(&input.proposed_use)
    .bind(&input.proposer)
    .bind(input.acquisition_trace.id)
    .bind(input.private_context_used)
    .bind(&input.opt_in_proof)
    .bind(&input.provenance)
    .bind(&input.quality_signals)
    .bind(&input.freshness)
    .bind(input.expiry)
    .bind(&input.downstream_link_candidates)
    .bind(&input.decision)
    .bind(&input.metadata)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(AppError::Database)
}

async fn insert_knowledge_source_policy(
    pool: &PgPool,
    candidate: &KnowledgeSourceCandidateInput,
    policy: &KnowledgeSourcePolicyInput,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO knowledge_source_policies (
            id,
            space_id,
            namespace_id,
            state,
            source_candidate_id,
            source_descriptor,
            allowed_use,
            disallowed_use,
            privacy_policy,
            refresh_policy,
            quality_thresholds,
            freshness_requirements,
            expiry,
            approved_by,
            approved_at,
            revoked_or_paused_reason,
            metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        "#,
    )
    .bind(policy.id)
    .bind(candidate.space_id)
    .bind(candidate.namespace_id)
    .bind(&policy.state)
    .bind(candidate.id)
    .bind(&policy.source_descriptor)
    .bind(&policy.allowed_use)
    .bind(&policy.disallowed_use)
    .bind(&policy.privacy_policy)
    .bind(&policy.refresh_policy)
    .bind(&policy.quality_thresholds)
    .bind(&policy.freshness_requirements)
    .bind(policy.expiry)
    .bind(&policy.approved_by)
    .bind(policy.approved_at)
    .bind(&policy.revoked_or_paused_reason)
    .bind(&policy.metadata)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(AppError::Database)
}

async fn insert_knowledge_context(
    pool: &PgPool,
    input: &KnowledgeContextInput,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO knowledge_contexts (
            id,
            space_id,
            namespace_id,
            source_policy_id,
            source_candidate_id,
            acquisition_trace_id,
            state,
            context_type,
            structured_claims,
            provenance,
            quality_signals,
            freshness,
            expiry,
            evidence_snippets,
            private_context_used,
            opt_in_proof,
            downstream_links,
            conflict_notes,
            metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        "#,
    )
    .bind(input.id)
    .bind(input.space_id)
    .bind(input.namespace_id)
    .bind(input.source_policy_id)
    .bind(input.source_candidate_id)
    .bind(input.acquisition_trace.id)
    .bind(&input.state)
    .bind(&input.context_type)
    .bind(&input.structured_claims)
    .bind(&input.provenance)
    .bind(&input.quality_signals)
    .bind(&input.freshness)
    .bind(input.expiry)
    .bind(&input.evidence_snippets)
    .bind(input.private_context_used)
    .bind(&input.opt_in_proof)
    .bind(&input.downstream_links)
    .bind(&input.conflict_notes)
    .bind(&input.metadata)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(AppError::Database)
}

#[derive(Debug, FromRow)]
struct KnowledgePolicyValidationRow {
    policy_space_id: Uuid,
    policy_namespace_id: Uuid,
    policy_state: String,
    policy_expiry: DateTime<Utc>,
    candidate_id: Uuid,
    candidate_space_id: Uuid,
    candidate_namespace_id: Uuid,
    candidate_state: String,
    candidate_expiry: DateTime<Utc>,
}

async fn validate_context_policy_state(
    pool: &PgPool,
    input: &KnowledgeContextInput,
) -> Result<(), AppError> {
    let row = sqlx::query_as::<_, KnowledgePolicyValidationRow>(
        r#"
        SELECT
            policy.space_id AS policy_space_id,
            policy.namespace_id AS policy_namespace_id,
            policy.state AS policy_state,
            policy.expiry AS policy_expiry,
            candidate.id AS candidate_id,
            candidate.space_id AS candidate_space_id,
            candidate.namespace_id AS candidate_namespace_id,
            candidate.state AS candidate_state,
            candidate.expiry AS candidate_expiry
        FROM knowledge_source_policies policy
        JOIN knowledge_source_candidates candidate
          ON candidate.id = policy.source_candidate_id
        WHERE policy.id = $1
        "#,
    )
    .bind(input.source_policy_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| {
        AppError::BadRequest(
            "KnowledgeContext source_policy_id was not found in the requested Space and Namespace"
                .to_string(),
        )
    })?;

    validate_knowledge_scope(
        row.policy_space_id,
        row.policy_namespace_id,
        input.space_id,
        input.namespace_id,
    )?;
    validate_knowledge_scope(
        row.candidate_space_id,
        row.candidate_namespace_id,
        input.space_id,
        input.namespace_id,
    )?;
    if row.candidate_id != input.source_candidate_id {
        return Err(AppError::BadRequest(
            "KnowledgeContext source_candidate_id must match the source policy".to_string(),
        ));
    }
    if row.candidate_state != "approved" {
        return Err(AppError::BadRequest(
            "KnowledgeContext requires an approved source candidate".to_string(),
        ));
    }
    if row.candidate_expiry <= Utc::now() {
        return Err(AppError::BadRequest(
            "KnowledgeContext source candidate is expired".to_string(),
        ));
    }
    if row.policy_state != "active" {
        return Err(AppError::BadRequest(
            "KnowledgeContext requires an active source policy".to_string(),
        ));
    }
    if row.policy_expiry <= Utc::now() {
        return Err(AppError::BadRequest(
            "KnowledgeContext source policy is expired".to_string(),
        ));
    }
    Ok(())
}

fn prepare_capture_payload(
    namespace: &str,
    payload: Value,
) -> Result<PreparedCapturePayload, AppError> {
    if namespace == PERSONAL_HEALTH_SLEEP_NAMESPACE {
        return prepare_sleep_energy_capture_payload(payload);
    }

    let payload: CapturePayload = serde_json::from_value(payload)
        .map_err(|error| AppError::BadRequest(format!("invalid capture payload: {error}")))?;
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
        trace_input_summary: None,
        sleep_check_in: None,
    })
}

fn prepare_sleep_energy_capture_payload(
    payload: Value,
) -> Result<PreparedCapturePayload, AppError> {
    let input: SleepEnergyCheckInInput = serde_json::from_value(payload).map_err(|_| {
        AppError::BadRequest("invalid personal.health.sleep capture payload".to_string())
    })?;
    let check_in = input
        .confirm()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let input_source = check_in.input_source.as_str().to_string();
    let title = format!(
        "Confirmed sleep and energy check-in — {}",
        check_in.local_date
    );

    Ok(PreparedCapturePayload {
        title: Some(title),
        content: check_in.canonical_text(),
        input_source: Some(input_source),
        tags: Vec::new(),
        metadata: json!({}),
        source_metadata: json!({
            "personal_feedback": check_in.persistence_metadata(),
        }),
        trace_metadata: json!({
            "namespace": PERSONAL_HEALTH_SLEEP_NAMESPACE,
            "personal_feedback": check_in.trace_metadata(),
        }),
        trace_input_summary: Some(check_in.trace_input_summary()),
        sleep_check_in: Some(check_in),
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
        trace_input_summary: None,
        sleep_check_in: None,
    })
}

async fn validate_sleep_check_in_scope(
    state: &AppState,
    space_id: Uuid,
    namespace_id: Uuid,
    prepared: &PreparedCapturePayload,
) -> Result<(), AppError> {
    let Some(check_in) = &prepared.sleep_check_in else {
        return Ok(());
    };
    let local_date = check_in.local_date.to_string();

    let active_record_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM memories
        WHERE space_id = $1
          AND namespace_id = $2
          AND source_type = 'surface_capture'
          AND source_metadata #>> '{capture,personal_feedback,record_type}' = 'sleep_energy_check_in'
          AND source_metadata #>> '{capture,personal_feedback,local_date}' = $3
          AND source_metadata #>> '{capture,personal_feedback,superseded_by_memory_id}' IS NULL
        LIMIT 1
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(&local_date)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    match (check_in.corrects_record_id, active_record_id) {
        (None, None) => Ok(()),
        (None, Some(_)) => Err(AppError::BadRequest(
            "a confirmed sleep check-in already exists for local_date".to_string(),
        )),
        (Some(corrects_record_id), Some(active_record_id))
            if corrects_record_id == active_record_id =>
        {
            let corrected = state
                .repositories
                .memories
                .find_by_id(corrects_record_id)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| {
                    AppError::BadRequest("corrects_record_id was not found".to_string())
                })?;
            if corrected.space_id != space_id
                || corrected.namespace_id != Some(namespace_id)
                || corrected
                    .source_metadata
                    .pointer("/capture/personal_feedback/record_type")
                    .and_then(Value::as_str)
                    != Some("sleep_energy_check_in")
                || corrected
                    .source_metadata
                    .pointer("/capture/personal_feedback/local_date")
                    .and_then(Value::as_str)
                    != Some(local_date.as_str())
            {
                return Err(AppError::BadRequest(
                    "corrects_record_id must reference an earlier same-Space, same-Namespace check-in for local_date"
                        .to_string(),
                ));
            }
            Ok(())
        }
        (Some(_), _) => Err(AppError::BadRequest(
            "corrects_record_id must reference the current same-day sleep check-in".to_string(),
        )),
    }
}

async fn mark_sleep_check_in_superseded(
    state: &AppState,
    corrected_record_id: Uuid,
    replacement_record_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE memories
        SET source_metadata = jsonb_set(
                source_metadata,
                '{capture,personal_feedback,superseded_by_memory_id}',
                to_jsonb($2::uuid),
                true
            ),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(corrected_record_id)
    .bind(replacement_record_id)
    .execute(&state.db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
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
    if payload.source_event_id.is_some() || payload.normalized_outcome.is_some() {
        return prepare_idempotent_learning_outcome(namespace, payload);
    }
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
        idempotency: None,
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
        idempotency: None,
    })
}

fn prepare_idempotent_learning_outcome(
    namespace: &str,
    payload: SubmitAttemptPayload,
) -> Result<PreparedSubmitAttemptPayload, AppError> {
    let source_event_id = payload.source_event_id.ok_or_else(|| {
        AppError::BadRequest("source_event_id is required for normalized_outcome".to_string())
    })?;
    if !valid_source_event_id(&source_event_id) {
        return Err(AppError::BadRequest("invalid source_event_id".to_string()));
    }
    if payload.feedback_loop_id.is_some()
        || payload.attempt.is_some()
        || payload.task_kind.is_some()
        || payload.source.is_some()
        || !payload.prompt_items.is_empty()
        || !payload.submitted_items.is_empty()
        || !payload.metadata.is_null()
    {
        return Err(AppError::BadRequest(
            "idempotent normalized_outcome does not accept legacy attempt, dictation, feedback_loop_id, or metadata fields".to_string(),
        ));
    }
    if matches!(payload.input_source.as_deref(), Some("typed" | "pasted"))
        && payload.input_confirmation.is_some()
    {
        return Err(AppError::BadRequest(
            "typed or pasted normalized_outcome must not include media confirmation".to_string(),
        ));
    }
    validate_surface_evidence(
        payload.input_source.as_deref(),
        payload.input_confirmation.as_ref(),
        &payload.evidence_refs,
    )?;
    let outcome = payload.normalized_outcome.ok_or_else(|| {
        AppError::BadRequest("normalized_outcome is required with source_event_id".to_string())
    })?;
    validate_normalized_outcome(&outcome)?;
    let attempt_summary = normalized_outcome_summary(&outcome);
    let task = payload
        .task
        .ok_or_else(|| AppError::BadRequest("normalized_outcome requires task".to_string()))?;
    let goal = payload
        .goal
        .unwrap_or_else(|| format!("Practice {namespace}"));
    let fingerprint_input = json!({
        "namespace": namespace,
        "goal": goal,
        "task": task,
        "input_source": payload.input_source,
        "input_confirmation": payload.input_confirmation,
        "normalized_outcome": outcome,
    });
    let payload_fingerprint = canonical_payload_fingerprint(&fingerprint_input)?;

    Ok(PreparedSubmitAttemptPayload {
        space_id: payload.space_id,
        feedback_loop_id: None,
        goal: Some(goal),
        task: Some(task),
        attempt_summary,
        evaluation: json!({"summary": "completed", "mistake_recorded": outcome.mistake.is_some()}),
        trace_metadata: json!({"normalized_learning_outcome": true}),
        input_source: payload.input_source,
        idempotency: Some(PerformanceIdempotency {
            source_event_id,
            payload_fingerprint,
        }),
    })
}

fn valid_source_event_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn validate_normalized_outcome(outcome: &NormalizedLearningOutcome) -> Result<(), AppError> {
    if outcome.summary.trim().is_empty() || outcome.summary.len() > 400 {
        return Err(AppError::BadRequest(
            "normalized_outcome.summary must be 1..=400 characters".to_string(),
        ));
    }
    if has_secret_like_value(outcome.summary.trim()) {
        return Err(AppError::BadRequest(
            "normalized_outcome contains a secret-like text value".to_string(),
        ));
    }
    if let Some(mistake) = &outcome.mistake {
        for value in [
            &mistake.expected_text,
            &mistake.actual_text,
            &mistake.mistake_type,
        ] {
            if value.trim().is_empty()
                || value.len() > 120
                || value.chars().any(char::is_control)
                || has_secret_like_value(value.trim())
            {
                return Err(AppError::BadRequest(
                    "normalized_outcome mistake fields must be bounded non-control text"
                        .to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn normalized_outcome_summary(outcome: &NormalizedLearningOutcome) -> String {
    match &outcome.mistake {
        Some(mistake) => format!(
            "{}; {} -> {} ({})",
            outcome.summary.trim(),
            mistake.expected_text.trim(),
            mistake.actual_text.trim(),
            mistake.mistake_type.trim()
        ),
        None => outcome.summary.trim().to_string(),
    }
}

fn canonical_payload_fingerprint(value: &Value) -> Result<String, AppError> {
    let canonical = serde_json::to_vec(value).map_err(|error| {
        AppError::Internal(format!("normalized outcome serialization failed: {error}"))
    })?;
    Ok(format!("{:x}", Sha256::digest(canonical)))
}

fn reject_forbidden_idempotent_outcome_fields(value: &Value) -> Result<(), AppError> {
    const FORBIDDEN: &[&str] = &[
        "raw_chat_history",
        "chat_history",
        "conversation",
        "camera_frame",
        "image_bytes",
        "media_bytes",
        "media_locator",
        "provider_session_id",
        "provider_payload",
        "model_reasoning",
        "credentials",
        "authorization",
        "password",
        "api_key",
        "token",
        "secret",
    ];
    fn visit(value: &Value, in_evidence_refs: bool) -> bool {
        match value {
            Value::Object(object) => object.iter().any(|(key, value)| {
                let key = key.to_ascii_lowercase();
                (!in_evidence_refs
                    && FORBIDDEN
                        .iter()
                        .any(|needle| key == *needle || key.contains(needle)))
                    || visit(value, in_evidence_refs || key == "evidence_refs")
            }),
            Value::Array(values) => values.iter().any(|value| visit(value, in_evidence_refs)),
            Value::String(value) => !in_evidence_refs && has_secret_like_value(value),
            _ => false,
        }
    }
    if visit(value, false) {
        return Err(AppError::BadRequest(
            "idempotent normalized_outcome contains forbidden raw or secret-bearing fields"
                .to_string(),
        ));
    }
    Ok(())
}

async fn submit_attempt(
    state: &AppState,
    user_id: Uuid,
    request: SurfaceRequest,
) -> Result<(StatusCode, Json<ApiResponse<SurfaceResponse>>), AppError> {
    let started_at = Utc::now();
    if request.payload.get("source_event_id").is_some() {
        reject_forbidden_idempotent_outcome_fields(&request.payload)?;
    }
    let payload: SubmitAttemptPayload = serde_json::from_value(request.payload.clone())
        .map_err(|error| AppError::BadRequest(format!("invalid submitAttempt payload: {error}")))?;
    let prepared = prepare_submit_attempt_payload(&request.namespace, payload)?;

    require_space_writer(state, prepared.space_id, user_id).await?;
    let namespace =
        resolve_namespace_by_name(state, user_id, prepared.space_id, &request.namespace).await?;

    if let Some(idempotency) = prepared.idempotency.as_ref() {
        let task = prepared.task.clone().ok_or_else(|| {
            AppError::Internal("idempotent outcome missing validated task".to_string())
        })?;
        let goal = prepared.goal.clone().ok_or_else(|| {
            AppError::Internal("idempotent outcome missing validated goal".to_string())
        })?;
        let result = submit_idempotent_learning_outcome(
            &state.db,
            idempotency,
            CreateFeedbackLoop {
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
            },
            AttemptTraceInput {
                space_id: prepared.space_id,
                namespace_id: namespace.id,
                request: &request,
                input_source: prepared.input_source.clone(),
                trace_metadata: prepared.trace_metadata.clone(),
                attempt_summary: prepared.attempt_summary.clone(),
                started_at,
            },
        )
        .await?;
        let (feedback_loop_id, trace_id, status) = match result {
            IdempotentSubmission::Created {
                feedback_loop_id,
                trace_id,
            } => (feedback_loop_id, trace_id, "attempt_recorded"),
            IdempotentSubmission::Replayed {
                feedback_loop_id,
                trace_id,
            } => (feedback_loop_id, trace_id, "attempt_replayed"),
        };
        let event = EngineEvent::AttemptSubmitted(EngineEventEnvelope {
            space_id: prepared.space_id,
            namespace_id: namespace.id,
            source_trace_id: trace_id,
            payload_refs: vec![EnginePayloadRef {
                kind: EnginePayloadRefKind::Attempt,
                id: feedback_loop_id,
            }],
        });
        let response = SurfaceResponse::new(
            Surface::Performance,
            SurfaceAction::SubmitAttempt,
            json!({
                "status": status,
                "feedback_loop_id": feedback_loop_id,
                "namespace_id": namespace.id,
                "evaluation": prepared.evaluation,
                "deep_consolidation": false,
                "event": event,
            }),
            trace_id,
            vec!["Review this attempt later for recurring mistake patterns".to_string()],
            SurfaceVisibility::User,
        );
        return Ok((StatusCode::OK, Json(ApiResponse::success(response))));
    }

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

struct AttemptTraceInput<'a> {
    space_id: Uuid,
    namespace_id: Uuid,
    request: &'a SurfaceRequest,
    input_source: Option<String>,
    trace_metadata: Value,
    attempt_summary: String,
    started_at: DateTime<Utc>,
}

#[derive(Debug)]
enum IdempotentSubmission {
    Created {
        feedback_loop_id: Uuid,
        trace_id: Uuid,
    },
    Replayed {
        feedback_loop_id: Uuid,
        trace_id: Uuid,
    },
}

#[derive(FromRow)]
struct IdempotencyRecord {
    payload_fingerprint: String,
    feedback_loop_id: Option<Uuid>,
    trace_id: Option<Uuid>,
}

async fn submit_idempotent_learning_outcome(
    pool: &PgPool,
    idempotency: &PerformanceIdempotency,
    feedback_loop: CreateFeedbackLoop,
    trace_input: AttemptTraceInput<'_>,
) -> Result<IdempotentSubmission, AppError> {
    let mut tx = pool.begin().await.map_err(AppError::Database)?;
    let claimed: Option<bool> = sqlx::query_scalar(
        r#"
        INSERT INTO performance_idempotency_records
            (space_id, namespace_id, source_event_id, payload_fingerprint)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (space_id, namespace_id, source_event_id) DO NOTHING
        RETURNING true
        "#,
    )
    .bind(feedback_loop.space_id)
    .bind(feedback_loop.namespace_id)
    .bind(&idempotency.source_event_id)
    .bind(&idempotency.payload_fingerprint)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    if claimed.is_none() {
        let record = sqlx::query_as::<_, IdempotencyRecord>(
            r#"
            SELECT payload_fingerprint, feedback_loop_id, trace_id
            FROM performance_idempotency_records
            WHERE space_id = $1 AND namespace_id = $2 AND source_event_id = $3
            "#,
        )
        .bind(feedback_loop.space_id)
        .bind(feedback_loop.namespace_id)
        .bind(&idempotency.source_event_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;
        if record.payload_fingerprint != idempotency.payload_fingerprint {
            return Err(AppError::Conflict(
                "source_event_id conflicts with a different normalized outcome".to_string(),
            ));
        }
        let feedback_loop_id = record
            .feedback_loop_id
            .ok_or_else(|| AppError::Internal("idempotency record is incomplete".to_string()))?;
        let trace_id = record
            .trace_id
            .ok_or_else(|| AppError::Internal("idempotency record is incomplete".to_string()))?;
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(IdempotentSubmission::Replayed {
            feedback_loop_id,
            trace_id,
        });
    }

    let feedback_loop = insert_feedback_loop_in_tx(&mut tx, &feedback_loop)
        .await
        .map_err(AppError::Database)?;
    let completed_at = Utc::now();
    let trace = insert_completed_trace_in_tx(
        &mut tx,
        CreateCompletedTrace {
            space_id: trace_input.space_id,
            namespace_id: Some(trace_input.namespace_id),
            source_type: trace_source_type(trace_input.request.adapter),
            task_type: TraceTaskType::Practice,
            mode: trace_mode(trace_input.request.context.mode.as_deref())?,
            runtime: trace_runtime(trace_input.request.context.runtime_preference),
            input_summary: Some(format!("Performance submitAttempt in namespace {}", trace_input.request.namespace)),
            output_summary: Some(format!("Attempt recorded for FeedbackLoop {}", feedback_loop.id)),
            started_at: trace_input.started_at,
            completed_at,
            latency_ms: Some((completed_at - trace_input.started_at).num_milliseconds().max(0)),
            model_provider: Some("deterministic".to_string()),
            model_name: None,
            token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
            estimated_cost_usd: Some(0.0),
            local_processing_ratio: Some(1.0),
            related_memory_ids: Vec::new(), generated_memory_ids: Vec::new(),
            generated_lens_run_ids: Vec::new(), generated_review_report_ids: Vec::new(),
            generated_feedback_loop_ids: vec![feedback_loop.id],
            user_feedback: Some(json!({"attempt": trace_input.attempt_summary})),
            error: None,
            metadata: json!({"surface_gateway": true, "surface": trace_input.request.surface,
                "action": trace_input.request.action, "adapter": trace_input.request.adapter,
                "namespace": trace_input.request.namespace, "input_source": trace_input.input_source,
                "deep_consolidation": false, "dictation": trace_input.trace_metadata,
                "event": "attempt_submitted"}),
        },
    )
    .await
    .map_err(AppError::Database)?;
    sqlx::query(
        r#"UPDATE performance_idempotency_records SET feedback_loop_id = $4, trace_id = $5
            WHERE space_id = $1 AND namespace_id = $2 AND source_event_id = $3"#,
    )
    .bind(feedback_loop.space_id)
    .bind(feedback_loop.namespace_id)
    .bind(&idempotency.source_event_id)
    .bind(feedback_loop.id)
    .bind(trace.id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;
    tx.commit().await.map_err(AppError::Database)?;
    Ok(IdempotentSubmission::Created {
        feedback_loop_id: feedback_loop.id,
        trace_id: trace.id,
    })
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
        lens_strategy: payload.lens_strategy.clone(),
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
    let candidate_context = load_manual_consolidation_candidate_context(
        &state.db,
        payload.space_id,
        namespace.id,
        &payload.knowledge_context_ids,
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
                "Selected {} Trace records and {} KnowledgeContext records for the evidence window",
                input_trace_ids.len(),
                candidate_context.selected.len()
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
                "selected_knowledge_context_count": candidate_context.selected.len(),
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
                "candidate_context_selection": candidate_context.selection_metadata(),
            }),
        })
        .await
        .map_err(map_sleep_cycle_create_error)?;

    let candidate_context_response =
        candidate_context.to_response(sleep_cycle.id, input_trace_ids.len());

    let sleep_cycle = sleep_cycle_repository
        .mark_completed(
            sleep_cycle.id,
            CompleteSleepCycle {
                generated_memory_ids: Vec::new(),
                metadata: json!({
                    "summary": deterministic_consolidation_summary(input_trace_ids.len()),
                    "selected_input_trace_count": input_trace_ids.len(),
                    "runtime": "deterministic",
                    "candidate_context": candidate_context_response,
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
            "candidate_context": candidate_context_response,
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

#[derive(Debug, FromRow)]
struct ManualKnowledgeContextRow {
    id: Uuid,
    state: String,
    context_type: String,
    source_policy_id: Uuid,
    source_candidate_id: Uuid,
    acquisition_trace_id: Uuid,
    structured_claims: Value,
    quality_signals: Value,
    conflict_notes: Value,
    expiry: DateTime<Utc>,
}

#[derive(Debug)]
struct ManualCandidateContext {
    selected: Vec<ManualKnowledgeContextRow>,
    total_in_scope_count: i64,
    explicit_reference_count: usize,
}

impl ManualCandidateContext {
    fn ignored_count(&self) -> i64 {
        (self.total_in_scope_count - self.selected.len() as i64).max(0)
    }

    fn selection_metadata(&self) -> Value {
        json!({
            "mode": if self.explicit_reference_count == 0 { "ambient" } else { "explicit" },
            "requested_knowledge_context_count": self.explicit_reference_count,
            "selected_knowledge_context_count": self.selected.len(),
            "ignored_knowledge_context_count": self.ignored_count(),
            "knowledge_context_ids": self.selected.iter().map(|row| row.id).collect::<Vec<_>>(),
        })
    }

    fn to_response(&self, sleep_cycle_id: Uuid, local_trace_count: usize) -> Value {
        let knowledge_context_ids = self.selected.iter().map(|row| row.id).collect::<Vec<_>>();
        let dream_candidates = self
            .selected
            .iter()
            .map(|row| deterministic_knowledge_context_dream_candidate(row, sleep_cycle_id))
            .collect::<Vec<_>>();

        json!({
            "mode": if self.explicit_reference_count == 0 { "ambient" } else { "explicit" },
            "requested_knowledge_context_count": self.explicit_reference_count,
            "total_in_scope_knowledge_context_count": self.total_in_scope_count,
            "selected_knowledge_context_count": self.selected.len(),
            "ignored_knowledge_context_count": self.ignored_count(),
            "knowledge_context_ids": knowledge_context_ids,
            "evidence_priority": {
                "local_trace_count": local_trace_count,
                "local_evidence": "primary",
                "external_knowledge": "candidate_context"
            },
            "persistence": "sleep_cycle_metadata_only",
            "dream_candidates": dream_candidates,
        })
    }
}

async fn load_manual_consolidation_candidate_context(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    requested_ids: &[Uuid],
) -> Result<ManualCandidateContext, AppError> {
    let total_in_scope_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_contexts
        WHERE space_id = $1
          AND namespace_id = $2
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let explicit_ids = unique_uuids(requested_ids);
    let selected = if explicit_ids.is_empty() {
        sqlx::query_as::<_, ManualKnowledgeContextRow>(
            eligible_knowledge_context_sql(
                r#"
                ORDER BY context.updated_at DESC, context.id ASC
                LIMIT 10
                "#,
            )
            .as_str(),
        )
        .bind(space_id)
        .bind(namespace_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?
    } else {
        let selected = sqlx::query_as::<_, ManualKnowledgeContextRow>(
            eligible_knowledge_context_sql(
                r#"
                  AND context.id = ANY($3::uuid[])
                ORDER BY array_position($3::uuid[], context.id), context.id ASC
                "#,
            )
            .as_str(),
        )
        .bind(space_id)
        .bind(namespace_id)
        .bind(&explicit_ids)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        if selected.len() != explicit_ids.len() {
            return Err(AppError::BadRequest(
                "knowledge_context_ids must reference approved, non-expired context for the requested space and namespace".to_string(),
            ));
        }
        selected
    };

    Ok(ManualCandidateContext {
        selected,
        total_in_scope_count,
        explicit_reference_count: explicit_ids.len(),
    })
}

fn eligible_knowledge_context_sql(suffix: &str) -> String {
    format!(
        r#"
        SELECT
            context.id,
            context.state,
            context.context_type,
            context.source_policy_id,
            context.source_candidate_id,
            context.acquisition_trace_id,
            context.structured_claims,
            context.quality_signals,
            context.conflict_notes,
            context.expiry
        FROM knowledge_contexts context
        JOIN knowledge_source_policies policy
          ON policy.id = context.source_policy_id
         AND policy.space_id = context.space_id
         AND policy.namespace_id = context.namespace_id
        JOIN knowledge_source_candidates candidate
          ON candidate.id = context.source_candidate_id
         AND candidate.space_id = context.space_id
         AND candidate.namespace_id = context.namespace_id
        JOIN knowledge_acquisition_traces acquisition
          ON acquisition.id = context.acquisition_trace_id
         AND acquisition.space_id = context.space_id
         AND acquisition.namespace_id = context.namespace_id
        WHERE context.space_id = $1
          AND context.namespace_id = $2
          AND context.state IN ('candidate', 'valid')
          AND context.expiry > NOW()
          AND policy.state = 'active'
          AND policy.expiry > NOW()
          AND candidate.state = 'approved'
          AND candidate.expiry > NOW()
        {suffix}
        "#
    )
}

fn unique_uuids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut unique = Vec::new();
    for id in ids {
        if !unique.contains(id) {
            unique.push(*id);
        }
    }
    unique
}

fn deterministic_knowledge_context_dream_candidate(
    row: &ManualKnowledgeContextRow,
    sleep_cycle_id: Uuid,
) -> Value {
    let purpose = knowledge_context_candidate_purpose(row);
    json!({
        "id": Uuid::new_v4(),
        "type": "dream_candidate",
        "status": "proposed",
        "purpose": purpose,
        "source_sleep_cycle_id": sleep_cycle_id,
        "knowledge_context_id": row.id,
        "source_knowledge_context_ids": [row.id],
        "source_policy_id": row.source_policy_id,
        "source_candidate_id": row.source_candidate_id,
        "acquisition_trace_id": row.acquisition_trace_id,
        "knowledge_context_state": row.state,
        "context_type": row.context_type,
        "content": deterministic_knowledge_context_candidate_content(row, purpose),
        "rationale": "External KnowledgeContext is used only as candidate context; local Trace, FeedbackLoop, and GrowthModel evidence remain primary.",
        "expected_effect": "Create a reviewable next-step candidate without directly changing GrowthModel or PracticePlan.",
        "structured_claim_count": json_array_len(&row.structured_claims),
        "conflict_note_count": json_array_len(&row.conflict_notes),
        "quality_signals": row.quality_signals,
        "expiry": row.expiry,
        "evidence_priority": {
            "local_evidence": "primary",
            "external_knowledge": "candidate_context"
        },
        "direct_mutation": {
            "growth_model": false,
            "practice_plan": false,
            "memory": false
        }
    })
}

fn knowledge_context_candidate_purpose(row: &ManualKnowledgeContextRow) -> &'static str {
    if row.context_type == "contradiction_context" || json_array_len(&row.conflict_notes) > 0 {
        "contradiction_exploration"
    } else {
        match row.context_type.as_str() {
            "practice_context" => "practice_generation",
            "review_context" | "rubric_context" | "reference_claims" => "review_question",
            "trend_context" => "planning_prompt",
            _ => "review_question",
        }
    }
}

fn deterministic_knowledge_context_candidate_content(
    row: &ManualKnowledgeContextRow,
    purpose: &str,
) -> String {
    let claim_text = first_structured_claim_text(&row.structured_claims).unwrap_or_else(|| {
        "Review the approved external context against local evidence.".to_string()
    });
    match purpose {
        "contradiction_exploration" => {
            format!("Treat this external claim as a hypothesis, not an overwrite: {claim_text}")
        }
        "practice_generation" => {
            format!("Draft a candidate practice idea from approved context: {claim_text}")
        }
        "planning_prompt" => {
            format!(
                "Consider whether this external trend should inform a future plan: {claim_text}"
            )
        }
        _ => format!("Ask a review question using approved external context: {claim_text}"),
    }
}

fn first_structured_claim_text(claims: &Value) -> Option<String> {
    claims.as_array()?.iter().find_map(|claim| {
        claim
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| text.chars().take(240).collect())
    })
}

fn json_array_len(value: &Value) -> usize {
    value.as_array().map_or(0, Vec::len)
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

struct NamespaceObservation {
    response_key: &'static str,
    response: Value,
    trace_metadata: Value,
    related_memory_ids: Vec<Uuid>,
}

async fn load_namespace_observation(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    namespace: &str,
    now: DateTime<Utc>,
) -> Result<NamespaceObservation, AppError> {
    if namespace == PERSONAL_HEALTH_SLEEP_NAMESPACE {
        let evidence =
            load_personal_feedback_observation_evidence(pool, space_id, namespace_id).await?;
        let summary = build_personal_feedback_observation_summary(evidence);
        return Ok(NamespaceObservation {
            response_key: "personal_feedback_observation",
            trace_metadata: personal_feedback_observation_trace_metadata(&summary),
            related_memory_ids: summary
                .supporting_evidence_ids
                .iter()
                .filter_map(|evidence| match evidence {
                    EvidenceId::Memory(id) => Some(*id),
                    _ => None,
                })
                .collect(),
            response: serde_json::to_value(summary)
                .expect("personal feedback observation serializes"),
        });
    }

    let evidence =
        load_recent_dictation_observation_evidence(pool, space_id, namespace_id, now).await?;
    let summary = build_dictation_observation_summary(space_id, namespace_id, now, evidence);
    Ok(NamespaceObservation {
        response_key: "dictation_observation",
        trace_metadata: json!({
            "kind": "dictation",
            "status": summary.status,
            "evidence_record_count": summary.evidence_record_count,
        }),
        related_memory_ids: Vec::new(),
        response: serde_json::to_value(summary).expect("dictation observation serializes"),
    })
}

#[derive(Debug, FromRow)]
struct PersonalFeedbackObservationRow {
    id: Uuid,
    source_metadata: Value,
}

async fn load_personal_feedback_observation_evidence(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<Vec<SleepObservationEvidenceRecord>, AppError> {
    let rows = sqlx::query_as::<_, PersonalFeedbackObservationRow>(
        r#"
        SELECT id, source_metadata
        FROM memories
        WHERE space_id = $1
          AND namespace_id = $2
          AND source_metadata #>> '{capture,personal_feedback,record_type}' = 'sleep_energy_check_in'
          AND source_metadata #> '{capture,personal_feedback,superseded_by_memory_id}' IS NULL
        ORDER BY source_metadata #>> '{capture,personal_feedback,local_date}' ASC, id ASC
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            SleepObservationEvidenceRecord::from_persistence_metadata(row.id, &row.source_metadata)
        })
        .collect())
}

fn personal_feedback_observation_trace_metadata(
    summary: &PersonalFeedbackObservationSummary,
) -> Value {
    json!({
        "kind": "personal_feedback_sleep",
        "status": summary.status,
        "window": summary.window,
        "valid_record_count": summary.valid_record_count,
        "threshold": summary.threshold,
        "remaining_record_count": summary.remaining_record_count,
        "selected_evidence_ids": summary.supporting_evidence_ids,
        "has_observed_baseline": summary.observed.is_some(),
    })
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

#[derive(Debug, FromRow)]
struct KnowledgeSourceCandidateObservationRow {
    id: Uuid,
    state: String,
    proposed_use: String,
    proposer: String,
    acquisition_trace_id: Uuid,
    private_context_used: bool,
    quality_signals: Value,
    freshness: Value,
    expiry: DateTime<Utc>,
    downstream_link_count: i32,
    decision: Option<Value>,
    surface_trace_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct KnowledgeSourcePolicyObservationRow {
    id: Uuid,
    state: String,
    source_candidate_id: Uuid,
    allowed_use: Value,
    disallowed_use: Value,
    privacy_policy: Value,
    refresh_policy: Value,
    quality_thresholds: Value,
    freshness_requirements: Value,
    expiry: DateTime<Utc>,
    approved_by: String,
    approved_at: DateTime<Utc>,
    revoked_or_paused_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct KnowledgeContextObservationRow {
    id: Uuid,
    state: String,
    context_type: String,
    source_policy_id: Uuid,
    source_candidate_id: Uuid,
    acquisition_trace_id: Uuid,
    structured_claim_count: i32,
    evidence_snippet_count: i32,
    quality_signals: Value,
    freshness: Value,
    expiry: DateTime<Utc>,
    private_context_used: bool,
    downstream_links: Value,
    conflict_notes: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

async fn load_knowledge_refresh_observation(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
) -> Result<Value, AppError> {
    let source_candidates = sqlx::query_as::<_, KnowledgeSourceCandidateObservationRow>(
        r#"
        SELECT
            candidate.id,
            candidate.state,
            candidate.proposed_use,
            candidate.proposer,
            candidate.acquisition_trace_id,
            candidate.private_context_used,
            candidate.quality_signals,
            candidate.freshness,
            candidate.expiry,
            jsonb_array_length(candidate.downstream_link_candidates) AS downstream_link_count,
            candidate.decision,
            (
                SELECT trace.id
                FROM traces trace
                WHERE trace.space_id = candidate.space_id
                  AND trace.namespace_id = candidate.namespace_id
                  AND trace.metadata #>> '{knowledge_refresh,kind}' = 'source_candidate'
                  AND trace.metadata #>> '{knowledge_refresh,source_candidate_id}' = candidate.id::text
                ORDER BY trace.created_at DESC, trace.id DESC
                LIMIT 1
            ) AS surface_trace_id,
            candidate.created_at,
            candidate.updated_at
        FROM knowledge_source_candidates candidate
        WHERE candidate.space_id = $1
          AND candidate.namespace_id = $2
        ORDER BY candidate.updated_at DESC, candidate.id ASC
        LIMIT 50
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let source_policies = sqlx::query_as::<_, KnowledgeSourcePolicyObservationRow>(
        r#"
        SELECT
            id,
            state,
            source_candidate_id,
            allowed_use,
            disallowed_use,
            privacy_policy,
            refresh_policy,
            quality_thresholds,
            freshness_requirements,
            expiry,
            approved_by,
            approved_at,
            revoked_or_paused_reason,
            created_at,
            updated_at
        FROM knowledge_source_policies
        WHERE space_id = $1
          AND namespace_id = $2
        ORDER BY updated_at DESC, id ASC
        LIMIT 50
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let knowledge_contexts = sqlx::query_as::<_, KnowledgeContextObservationRow>(
        r#"
        SELECT
            id,
            state,
            context_type,
            source_policy_id,
            source_candidate_id,
            acquisition_trace_id,
            jsonb_array_length(structured_claims) AS structured_claim_count,
            jsonb_array_length(evidence_snippets) AS evidence_snippet_count,
            quality_signals,
            freshness,
            expiry,
            private_context_used,
            downstream_links,
            conflict_notes,
            created_at,
            updated_at
        FROM knowledge_contexts
        WHERE space_id = $1
          AND namespace_id = $2
        ORDER BY updated_at DESC, id ASC
        LIMIT 50
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(json!({
        "source_candidates": source_candidates.into_iter().map(|row| {
            json!({
                "id": row.id,
                "state": row.state,
                "proposed_use": row.proposed_use,
                "proposer": row.proposer,
                "acquisition_trace_id": row.acquisition_trace_id,
                "private_context_used": row.private_context_used,
                "quality_signals": row.quality_signals,
                "freshness": row.freshness,
                "expiry": row.expiry,
                "downstream_link_count": row.downstream_link_count,
                "decision": row.decision,
                "surface_trace_id": row.surface_trace_id,
                "created_at": row.created_at,
                "updated_at": row.updated_at,
            })
        }).collect::<Vec<_>>(),
        "source_policies": source_policies.into_iter().map(|row| {
            json!({
                "id": row.id,
                "state": row.state,
                "source_candidate_id": row.source_candidate_id,
                "allowed_use": row.allowed_use,
                "disallowed_use": row.disallowed_use,
                "privacy_policy": row.privacy_policy,
                "refresh_policy": row.refresh_policy,
                "quality_thresholds": row.quality_thresholds,
                "freshness_requirements": row.freshness_requirements,
                "expiry": row.expiry,
                "approved_by": row.approved_by,
                "approved_at": row.approved_at,
                "revoked_or_paused_reason": row.revoked_or_paused_reason,
                "created_at": row.created_at,
                "updated_at": row.updated_at,
            })
        }).collect::<Vec<_>>(),
        "knowledge_contexts": knowledge_contexts.into_iter().map(|row| {
            json!({
                "id": row.id,
                "state": row.state,
                "context_type": row.context_type,
                "source_policy_id": row.source_policy_id,
                "source_candidate_id": row.source_candidate_id,
                "acquisition_trace_id": row.acquisition_trace_id,
                "structured_claim_count": row.structured_claim_count,
                "evidence_snippet_count": row.evidence_snippet_count,
                "quality_signals": row.quality_signals,
                "freshness": row.freshness,
                "expiry": row.expiry,
                "private_context_used": row.private_context_used,
                "downstream_links": row.downstream_links,
                "conflict_notes": row.conflict_notes,
                "created_at": row.created_at,
                "updated_at": row.updated_at,
            })
        }).collect::<Vec<_>>(),
    }))
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
    fn idempotent_outcome_validates_identity_and_stable_fingerprint() {
        let payload = json!({
            "space_id": Uuid::nil(), "source_event_id": "study-buddy.session:2026-07-13.1",
            "task": "Daily spelling", "input_source": "typed",
            "normalized_outcome": {"summary": "Completed five words", "mistake": {
                "expected_text": "because", "actual_text": "becuase", "mistake_type": "letter_order"
            }}
        });
        let first = prepare_submit_attempt_payload(
            "child.english.spelling",
            serde_json::from_value(payload.clone()).unwrap(),
        )
        .unwrap();
        let second = prepare_submit_attempt_payload(
            "child.english.spelling",
            serde_json::from_value(payload).unwrap(),
        )
        .unwrap();
        assert_eq!(
            first.idempotency.as_ref().unwrap().payload_fingerprint,
            second.idempotency.as_ref().unwrap().payload_fingerprint
        );
        assert!(!valid_source_event_id("provider session/with spaces"));
    }

    #[test]
    fn idempotent_outcome_rejects_typed_confirmation_and_raw_provider_fields() {
        let typed_confirmation: SubmitAttemptPayload = serde_json::from_value(json!({
            "space_id": Uuid::nil(), "source_event_id": "event-1", "task": "Daily spelling",
            "input_source": "typed", "input_confirmation": {"status": "confirmed", "method": "explicit_acceptance"},
            "normalized_outcome": {"summary": "Completed session"}
        }))
        .unwrap();
        assert!(
            prepare_idempotent_learning_outcome("child.english.spelling", typed_confirmation)
                .is_err()
        );
        assert!(reject_forbidden_idempotent_outcome_fields(&json!({
            "source_event_id": "event-1", "provider_session_id": "do-not-store"
        }))
        .is_err());
    }

    #[test]
    fn idempotent_outcome_rejects_secret_like_text_values() {
        for field in ["summary", "expected_text", "actual_text", "mistake_type"] {
            let mut outcome = json!({
                "summary": "Completed session",
                "mistake": {
                    "expected_text": "because",
                    "actual_text": "becuase",
                    "mistake_type": "letter_order"
                }
            });
            if field == "summary" {
                outcome[field] = json!("Bearer should-not-persist");
            } else {
                outcome["mistake"][field] = json!("sk-should-not-persist");
            }
            let payload: SubmitAttemptPayload = serde_json::from_value(json!({
                "space_id": Uuid::nil(), "source_event_id": format!("event-{field}"),
                "task": "Daily spelling", "input_source": "typed",
                "normalized_outcome": outcome
            }))
            .unwrap();
            assert!(
                prepare_idempotent_learning_outcome("child.english.spelling", payload).is_err()
            );
        }
    }

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
    fn review_evidence_payload_preserves_lens_strategy_in_response_shape() {
        let payload: ReviewEvidencePayload = serde_json::from_value(json!({
            "space_id": Uuid::nil(),
            "lens_strategy": { "name": "learning_review" },
            "question": "Review the mistake pattern",
            "evidence": [
                {
                    "source": {
                        "kind": "trace",
                        "id": Uuid::from_u128(11)
                    },
                    "summary": "Target: because Submitted: becuase"
                }
            ]
        }))
        .unwrap();

        let insight = build_reflection_insight(&ReflectionRequest {
            space_id: payload.space_id,
            namespace_id: Uuid::from_u128(1),
            namespace: "child.english.spelling".to_string(),
            lens_strategy: payload.lens_strategy.clone(),
            question: payload.question.clone(),
            evidence: payload.evidence.clone(),
        });

        let response = serde_json::to_value(insight).unwrap();
        assert_eq!(
            response["lens_strategy"],
            json!({"name": "learning_review"})
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
            SurfaceAction::AdjustPlan,
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
        let payload = json!({
            "title": "Today's words",
            "task_kind": "english_spelling",
            "source": "typed",
            "prompt_items": [
                {"item_kind": "english_word", "expected_text": " because ", "metadata": {}},
                {"item_kind": "english_word", "expected_text": "friend", "metadata": {}}
            ],
            "tags": ["dictation"],
            "metadata": {"adapter_note": "typed list"}
        });

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
        let payload = json!({
            "task_kind": "english_spelling",
            "source": "typed",
            "input_confirmation": {
                "status": "confirmed",
                "method": "explicit_acceptance"
            },
            "prompt_items": [
                {"item_kind": "english_word", "expected_text": "because", "metadata": {}}
            ]
        });

        let err = prepare_capture_payload("child.english.spelling", payload).unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
        assert!(err
            .to_string()
            .contains("typed or pasted dictation capture cannot include input_confirmation"));
    }

    #[test]
    fn media_dictation_capture_payload_accepts_validated_refs_without_persisting_them() {
        let payload = json!({
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
        });

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
