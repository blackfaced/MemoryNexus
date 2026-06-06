//! Cognitive review report API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

use crate::ai::{SummaryOptions, SummaryStyle};
use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::{FeedbackLoopDb, FeedbackLoopWindowFilter};
use crate::db::lens::LensDb;
use crate::db::memory::MemoryDb;
use crate::db::namespace::NamespaceDb;
use crate::db::review_report::{
    CognitiveReviewReportDb, CreateCognitiveReviewReport, ReviewReportListFilter,
};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateReviewReportRequest {
    pub space_id: Uuid,
    pub lens_id: Uuid,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    #[serde(default = "default_report_type")]
    pub report_type: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLearningReviewRequest {
    pub lens_id: Uuid,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListReviewReportsQuery {
    pub space_id: Uuid,
    pub lens_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReviewReportListResponse {
    pub items: Vec<CognitiveReviewReportDb>,
    pub total: usize,
}

fn default_report_type() -> String {
    "periodic_review".to_string()
}

/// POST /api/v1/review-reports - Generate and persist a derived review report.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateReviewReportRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CognitiveReviewReportDb>>), AppError> {
    if req.window_start >= req.window_end {
        return Err(AppError::BadRequest(
            "window_start must be before window_end".to_string(),
        ));
    }

    let report_type = normalize_report_type(&req.report_type)?;
    let limit = req.limit.unwrap_or(30).clamp(1, 100);
    let lens = state
        .repositories
        .lenses
        .find_for_user(req.lens_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if lens.space_id != req.space_id {
        return Err(AppError::BadRequest(
            "lens_id must belong to the requested Cognitive Space".to_string(),
        ));
    }

    state
        .repositories
        .spaces
        .find_for_user(req.space_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    let memories = state
        .repositories
        .memories
        .list_by_space_window(
            auth_user.user_id,
            req.space_id,
            req.window_start,
            req.window_end,
            limit,
        )
        .await
        .map_err(AppError::Database)?;

    let source_memory_ids = memories.iter().map(|memory| memory.id).collect::<Vec<_>>();
    let source_lens_run_ids = Vec::new();
    let summary = generate_review_summary(&state, &lens, &report_type, &memories).await;
    let report_json = build_review_report_json(ReviewReportInput {
        lens: &lens,
        report_type: &report_type,
        window_start: req.window_start,
        window_end: req.window_end,
        memories: &memories,
        summary: &summary,
    });

    let report = state
        .repositories
        .review_reports
        .create(CreateCognitiveReviewReport {
            space_id: req.space_id,
            lens_id: req.lens_id,
            report_type,
            window_start: req.window_start,
            window_end: req.window_end,
            report: report_json,
            source_memory_ids,
            source_lens_run_ids,
            summary_provider: summary.provider,
            summary_source: summary.source,
            summary_model: summary.model,
            summary_fallback_reason: summary.fallback_reason,
            created_by: auth_user.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(report))))
}

/// POST /api/v1/namespaces/:namespace_id/learning-reviews - Generate a weekly
/// learning report from practice sessions in one skill Namespace.
pub async fn create_learning_review(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(namespace_id): Path<Uuid>,
    Json(req): Json<CreateLearningReviewRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CognitiveReviewReportDb>>), AppError> {
    if req.window_start >= req.window_end {
        return Err(AppError::BadRequest(
            "window_start must be before window_end".to_string(),
        ));
    }

    let namespace = state
        .repositories
        .namespaces
        .find_for_user(namespace_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if namespace.kind != "skill" {
        return Err(AppError::BadRequest(
            "learning review namespace must be a skill namespace".to_string(),
        ));
    }

    let lens = state
        .repositories
        .lenses
        .find_for_user(req.lens_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if lens.space_id != namespace.space_id {
        return Err(AppError::BadRequest(
            "lens_id must belong to the same Cognitive Space as the Namespace".to_string(),
        ));
    }

    let limit = req.limit.unwrap_or(50).clamp(1, 200);
    let feedback_loops = state
        .repositories
        .feedback_loops
        .list_for_user_window(
            FeedbackLoopWindowFilter {
                space_id: namespace.space_id,
                namespace_id,
                window_start: req.window_start,
                window_end: req.window_end,
                limit,
            },
            auth_user.user_id,
        )
        .await
        .map_err(AppError::Database)?;

    let memories = state
        .repositories
        .memories
        .list_by_space_window(
            auth_user.user_id,
            namespace.space_id,
            req.window_start,
            req.window_end,
            limit,
        )
        .await
        .map_err(AppError::Database)?;
    let source_memories = learning_source_memories(&memories, namespace_id, &feedback_loops);
    let source_memory_ids = source_memories
        .iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
    let summary = generate_learning_review_summary(&state, &namespace, &feedback_loops).await;
    let report_json = build_learning_review_report_json(LearningReviewInput {
        namespace: &namespace,
        window_start: req.window_start,
        window_end: req.window_end,
        feedback_loops: &feedback_loops,
        source_memories: &source_memories,
        summary: &summary,
    });

    let report = state
        .repositories
        .review_reports
        .create(CreateCognitiveReviewReport {
            space_id: namespace.space_id,
            lens_id: req.lens_id,
            report_type: "weekly_learning_review".to_string(),
            window_start: req.window_start,
            window_end: req.window_end,
            report: report_json,
            source_memory_ids,
            source_lens_run_ids: Vec::new(),
            summary_provider: summary.provider,
            summary_source: summary.source,
            summary_model: summary.model,
            summary_fallback_reason: summary.fallback_reason,
            created_by: auth_user.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(report))))
}

/// GET /api/v1/review-reports/:id - Fetch a persisted report.
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CognitiveReviewReportDb>>, AppError> {
    let report = state
        .repositories
        .review_reports
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::success(report)))
}

/// GET /api/v1/review-reports?space_id=<SPACE_ID> - List visible reports.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<ListReviewReportsQuery>,
) -> Result<Json<ApiResponse<ReviewReportListResponse>>, AppError> {
    state
        .repositories
        .spaces
        .find_for_user(query.space_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if let Some(lens_id) = query.lens_id {
        let lens = state
            .repositories
            .lenses
            .find_for_user(lens_id, auth_user.user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;
        if lens.space_id != query.space_id {
            return Err(AppError::BadRequest(
                "lens_id must belong to the requested Cognitive Space".to_string(),
            ));
        }
    }

    let reports = state
        .repositories
        .review_reports
        .list_for_user(
            ReviewReportListFilter {
                space_id: query.space_id,
                lens_id: query.lens_id,
                limit: query.limit.unwrap_or(20).clamp(1, 100),
            },
            auth_user.user_id,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(ReviewReportListResponse {
        total: reports.len(),
        items: reports,
    })))
}

fn normalize_report_type(report_type: &str) -> Result<String, AppError> {
    let report_type = report_type.trim().to_ascii_lowercase();
    match report_type.as_str() {
        "periodic_review" | "daily_review" | "weekly_review" | "monthly_review" => Ok(report_type),
        _ => Err(AppError::BadRequest(format!(
            "unsupported report_type: {report_type}"
        ))),
    }
}

struct ReviewSummary {
    text: String,
    provider: String,
    source: String,
    model: Option<String>,
    fallback_reason: Option<String>,
}

async fn generate_review_summary(
    state: &AppState,
    lens: &LensDb,
    report_type: &str,
    memories: &[MemoryDb],
) -> ReviewSummary {
    let fallback = deterministic_review_summary(lens, report_type, memories);
    let Some(summarizer) = state.ai.summarizer.as_ref() else {
        return ReviewSummary {
            text: fallback,
            provider: "deterministic".to_string(),
            source: "deterministic".to_string(),
            model: None,
            fallback_reason: Some("summary provider not configured".to_string()),
        };
    };

    let attempted_provider = state
        .ai
        .summary_provider
        .clone()
        .unwrap_or_else(|| "openai".to_string());

    if memories.is_empty() {
        return ReviewSummary {
            text: fallback,
            provider: attempted_provider,
            source: "deterministic".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: Some("no memories in review window".to_string()),
        };
    }

    match summarizer
        .summarize(
            &build_review_prompt(lens, report_type, memories),
            &SummaryOptions {
                max_words: state.ai.summary_max_words.unwrap_or(160),
                language: "zh".to_string(),
                include_keywords: false,
                style: SummaryStyle::BulletPoints,
            },
        )
        .await
    {
        Ok(result) if !result.summary.trim().is_empty() => ReviewSummary {
            text: result.summary,
            provider: attempted_provider,
            source: "ai".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: None,
        },
        Ok(_) => ReviewSummary {
            text: fallback,
            provider: attempted_provider,
            source: "deterministic".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: Some("summary provider returned empty output".to_string()),
        },
        Err(error) => ReviewSummary {
            text: fallback,
            provider: attempted_provider,
            source: "deterministic".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: Some(error.to_string()),
        },
    }
}

fn build_review_prompt(lens: &LensDb, report_type: &str, memories: &[MemoryDb]) -> String {
    let memories = memories
        .iter()
        .enumerate()
        .map(|(index, memory)| {
            format!(
                "[{}] id={} title={} created_at={}\n{}",
                index + 1,
                memory.id,
                memory.title.as_deref().unwrap_or("(untitled)"),
                memory.created_at,
                memory.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"你正在为 MemoryNexus 生成周期性认知回顾报告。

Report type: {report_type}
Lens:
- name: {lens_name}
- strategy: {strategy}
- output_format: {output_format}

Source memories:
{memories}

请基于这些 memories 输出一个认知回顾。要求：
1. 只使用 source memories，不要编造。
2. 总结本窗口内的重要变化、稳定偏好、决策、风险或矛盾。
3. 给出 2-5 个 next actions。
4. 如果信息不足，明确说明缺口。"#,
        report_type = report_type,
        lens_name = lens.name,
        strategy = lens.strategy,
        output_format = lens.output_format,
        memories = memories,
    )
}

fn deterministic_review_summary(lens: &LensDb, report_type: &str, memories: &[MemoryDb]) -> String {
    if memories.is_empty() {
        return format!(
            "{report_type} found no memories for Lens '{}' using strategy '{}'.",
            lens.name, lens.strategy
        );
    }

    format!(
        "{report_type} interpreted {} memories through Lens '{}' using strategy '{}'.",
        memories.len(),
        lens.name,
        lens.strategy
    )
}

async fn generate_learning_review_summary(
    state: &AppState,
    namespace: &NamespaceDb,
    feedback_loops: &[FeedbackLoopDb],
) -> ReviewSummary {
    let fallback = deterministic_learning_review_summary(namespace, feedback_loops);
    let Some(summarizer) = state.ai.summarizer.as_ref() else {
        return ReviewSummary {
            text: fallback,
            provider: "deterministic".to_string(),
            source: "deterministic".to_string(),
            model: None,
            fallback_reason: Some("summary provider not configured".to_string()),
        };
    };

    let attempted_provider = state
        .ai
        .summary_provider
        .clone()
        .unwrap_or_else(|| "openai".to_string());

    if feedback_loops.is_empty() {
        return ReviewSummary {
            text: fallback,
            provider: attempted_provider,
            source: "deterministic".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: Some("no practice sessions in review window".to_string()),
        };
    }

    match summarizer
        .summarize(
            &build_learning_review_prompt(namespace, feedback_loops),
            &SummaryOptions {
                max_words: state.ai.summary_max_words.unwrap_or(180),
                language: "en".to_string(),
                include_keywords: false,
                style: SummaryStyle::BulletPoints,
            },
        )
        .await
    {
        Ok(result) if !result.summary.trim().is_empty() => ReviewSummary {
            text: result.summary,
            provider: attempted_provider,
            source: "ai".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: None,
        },
        Ok(_) => ReviewSummary {
            text: fallback,
            provider: attempted_provider,
            source: "deterministic".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: Some("summary provider returned empty output".to_string()),
        },
        Err(error) => ReviewSummary {
            text: fallback,
            provider: attempted_provider,
            source: "deterministic".to_string(),
            model: state.ai.summary_model.clone(),
            fallback_reason: Some(error.to_string()),
        },
    }
}

fn build_learning_review_prompt(
    namespace: &NamespaceDb,
    feedback_loops: &[FeedbackLoopDb],
) -> String {
    let sessions = feedback_loops
        .iter()
        .enumerate()
        .map(|(index, session)| {
            format!(
                "[{}] id={} goal={} task={} answer={} mistake_pattern={} feedback={} adjustment={} next_exercise={} status={}",
                index + 1,
                session.id,
                session.goal,
                session.task,
                session.attempt.as_deref().unwrap_or(""),
                session.evaluation.as_deref().unwrap_or(""),
                session.feedback.as_deref().unwrap_or(""),
                session.adjustment.as_deref().unwrap_or(""),
                session.next_task.as_deref().unwrap_or(""),
                session.status
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"Generate a parent-friendly weekly learning review for Namespace {namespace_name}.

Use only these practice sessions:
{sessions}

Return concise learning-language bullets covering practiced topics, recurring mistake patterns,
improvement signals, current focus, and suggested next practice. Do not mention backend terms."#,
        namespace_name = namespace.name,
        sessions = sessions
    )
}

fn deterministic_learning_review_summary(
    namespace: &NamespaceDb,
    feedback_loops: &[FeedbackLoopDb],
) -> String {
    if feedback_loops.is_empty() {
        return format!(
            "No practice sessions were found for {} in this review window.",
            namespace.name
        );
    }

    format!(
        "Reviewed {} practice sessions for {} and summarized topics, mistake patterns, improvement signals, and next practice.",
        feedback_loops.len(),
        namespace.name
    )
}

struct ReviewReportInput<'a> {
    lens: &'a LensDb,
    report_type: &'a str,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    memories: &'a [MemoryDb],
    summary: &'a ReviewSummary,
}

fn build_review_report_json(input: ReviewReportInput<'_>) -> Value {
    json!({
        "report_type": input.report_type,
        "window": {
            "start": input.window_start,
            "end": input.window_end,
        },
        "lens": {
            "id": input.lens.id,
            "name": input.lens.name,
            "strategy": input.lens.strategy,
            "output_format": input.lens.output_format,
            "retrieval_mode": input.lens.retrieval_mode,
        },
        "summary": input.summary.text,
        "memory_count": input.memories.len(),
        "source_memories": input.memories.iter().map(memory_snippet).collect::<Vec<_>>(),
        "key_points": input.memories.iter().take(5).map(key_point).collect::<Vec<_>>(),
        "recurring_themes": recurring_themes(input.memories),
        "inner_tensions": inner_tensions(input.memories),
        "forming_direction": forming_direction(input.memories, &input.summary.text),
        "next_step": review_next_step(input.memories),
        "next_actions": next_actions(input.memories),
        "summary_provider": input.summary.provider,
        "summary_source": input.summary.source,
        "summary_model": input.summary.model,
        "summary_fallback_reason": input.summary.fallback_reason,
    })
}

struct LearningReviewInput<'a> {
    namespace: &'a NamespaceDb,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    feedback_loops: &'a [FeedbackLoopDb],
    source_memories: &'a [&'a MemoryDb],
    summary: &'a ReviewSummary,
}

fn build_learning_review_report_json(input: LearningReviewInput<'_>) -> Value {
    let source_feedback_loop_ids = input
        .feedback_loops
        .iter()
        .map(|session| session.id)
        .collect::<Vec<_>>();
    let source_memory_ids = input
        .source_memories
        .iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();

    json!({
        "report_type": "weekly_learning_review",
        "namespace": {
            "id": input.namespace.id,
            "name": input.namespace.name,
            "kind": input.namespace.kind,
        },
        "window": {
            "start": input.window_start,
            "end": input.window_end,
        },
        "summary": input.summary.text,
        "practice_session_count": input.feedback_loops.len(),
        "practiced_topics": practiced_topics(input.feedback_loops),
        "recurring_mistake_patterns": recurring_mistake_patterns(input.feedback_loops),
        "improvement_signals": improvement_signals(input.feedback_loops),
        "current_focus": current_focus(input.feedback_loops),
        "suggested_next_practice": suggested_next_practice(input.feedback_loops),
        "source_feedback_loop_ids": source_feedback_loop_ids,
        "source_practice_session_ids": source_feedback_loop_ids,
        "source_memory_ids": source_memory_ids,
        "source_memories": input.source_memories.iter().map(|memory| memory_snippet(memory)).collect::<Vec<_>>(),
        "provenance": {
            "space_id": input.namespace.space_id,
            "namespace_id": input.namespace.id,
            "namespace_name": input.namespace.name,
            "source_feedback_loop_ids": source_feedback_loop_ids,
            "source_memory_ids": source_memory_ids,
            "window": {
                "start": input.window_start,
                "end": input.window_end,
            },
            "summary_provider": input.summary.provider,
            "summary_source": input.summary.source,
            "summary_model": input.summary.model,
            "summary_fallback_reason": input.summary.fallback_reason,
        },
        "summary_provider": input.summary.provider,
        "summary_source": input.summary.source,
        "summary_model": input.summary.model,
        "summary_fallback_reason": input.summary.fallback_reason,
    })
}

fn practiced_topics(feedback_loops: &[FeedbackLoopDb]) -> Vec<String> {
    unique_non_empty(
        feedback_loops
            .iter()
            .flat_map(|session| [first_sentence(&session.goal), first_sentence(&session.task)]),
        8,
    )
}

fn recurring_mistake_patterns(feedback_loops: &[FeedbackLoopDb]) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for pattern in feedback_loops
        .iter()
        .filter_map(|session| session.evaluation.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        *counts.entry(pattern.to_string()).or_default() += 1;
    }

    let recurring = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(pattern, count)| format!("{pattern} ({count} sessions)"))
        .collect::<Vec<_>>();

    if recurring.is_empty() {
        unique_non_empty(counts.into_keys(), 5)
    } else {
        recurring
    }
}

fn improvement_signals(feedback_loops: &[FeedbackLoopDb]) -> Vec<String> {
    let completed_count = feedback_loops
        .iter()
        .filter(|session| session.status == "completed")
        .count();
    let mut signals = Vec::new();

    if completed_count > 0 {
        signals.push(format!(
            "{completed_count} practice sessions were completed."
        ));
    }

    signals.extend(unique_non_empty(
        feedback_loops
            .iter()
            .filter_map(|session| session.feedback.as_deref())
            .map(|feedback| format!("Feedback used: {feedback}")),
        4,
    ));

    signals
}

fn current_focus(feedback_loops: &[FeedbackLoopDb]) -> Vec<String> {
    let mistakes = recurring_mistake_patterns(feedback_loops);
    if !mistakes.is_empty() {
        return mistakes.into_iter().take(2).collect();
    }

    feedback_loops
        .last()
        .map(|session| vec![first_sentence(&session.task)])
        .unwrap_or_else(|| {
            vec!["Collect a few practice sessions before choosing next week's focus.".to_string()]
        })
}

fn suggested_next_practice(feedback_loops: &[FeedbackLoopDb]) -> Vec<String> {
    let suggestions = unique_non_empty(
        feedback_loops
            .iter()
            .rev()
            .filter_map(|session| session.next_task.as_deref())
            .map(str::to_string),
        5,
    );

    if suggestions.is_empty() {
        vec![
            "Add one short practice session and record the answer, feedback, and next exercise."
                .to_string(),
        ]
    } else {
        suggestions
    }
}

fn unique_non_empty<I>(items: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut values = Vec::new();
    for item in items {
        let item = item.trim().to_string();
        if item.is_empty() || !seen.insert(item.clone()) {
            continue;
        }
        values.push(item);
        if values.len() >= limit {
            break;
        }
    }
    values
}

fn learning_source_memories<'a>(
    memories: &'a [MemoryDb],
    namespace_id: Uuid,
    feedback_loops: &[FeedbackLoopDb],
) -> Vec<&'a MemoryDb> {
    let feedback_loop_ids = feedback_loops
        .iter()
        .map(|session| session.id.to_string())
        .collect::<HashSet<_>>();
    let namespace_id = namespace_id.to_string();

    memories
        .iter()
        .filter(|memory| memory.source_type == "feedback_loop_event")
        .filter(|memory| {
            memory.source_metadata["namespace_id"].as_str() == Some(namespace_id.as_str())
                && memory
                    .source_metadata
                    .get("feedback_loop_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| feedback_loop_ids.contains(id))
        })
        .collect()
}

fn recurring_themes(memories: &[MemoryDb]) -> Vec<String> {
    memories
        .iter()
        .take(4)
        .map(|memory| first_sentence(&memory.content))
        .filter(|theme| !theme.is_empty())
        .collect()
}

fn inner_tensions(memories: &[MemoryDb]) -> Vec<String> {
    if memories.len() < 2 {
        return Vec::new();
    }

    vec![
        "想继续推进一些事情，但也在寻找更清晰的选择标准。".to_string(),
        "想保留更多可能性，但注意力正在要求一条更稳定的主线。".to_string(),
    ]
}

fn forming_direction(memories: &[MemoryDb], summary: &str) -> String {
    if memories.is_empty() {
        return "材料还不够形成稳定主线。继续记录后再观察反复出现的方向。".to_string();
    }

    if summary.trim().is_empty() {
        "你正在从零散记录转向观察自己的长期思考模式。".to_string()
    } else {
        summary.trim().to_string()
    }
}

fn review_next_step(memories: &[MemoryDb]) -> String {
    if memories.is_empty() {
        return "写下一件今天最占空间的想法。".to_string();
    }

    "选择一个反复出现的主题，补充一条更具体的下一步想法。".to_string()
}

fn memory_snippet(memory: &MemoryDb) -> Value {
    json!({
        "memory_id": memory.id,
        "title": memory.title,
        "content": truncate(&memory.content, 240),
        "created_at": memory.created_at,
    })
}

fn key_point(memory: &MemoryDb) -> Value {
    json!({
        "memory_id": memory.id,
        "title": memory.title,
        "point": first_sentence(&memory.content),
    })
}

fn next_actions(memories: &[MemoryDb]) -> Vec<String> {
    if memories.is_empty() {
        return vec![
            "Add memories during the next review window before generating another report."
                .to_string(),
        ];
    }

    vec![
        "Review the cited memories before changing priorities.".to_string(),
        "Turn one stable pattern into a concrete next action.".to_string(),
        "Create a reminder for any follow-up that should not be lost.".to_string(),
    ]
}

fn first_sentence(content: &str) -> String {
    content
        .split(['.', '。', '!', '！', '?', '？'])
        .map(str::trim)
        .find(|sentence| !sentence.is_empty())
        .unwrap_or(content.trim())
        .to_string()
}

fn truncate(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }

    content.chars().take(max_chars).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn validates_report_type() {
        assert_eq!(
            normalize_report_type("weekly_review").unwrap(),
            "weekly_review"
        );
        assert!(normalize_report_type("random").is_err());
    }

    #[test]
    fn review_report_json_keeps_lens_window_and_provenance() {
        let lens = LensDb {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            name: "Weekly Review".to_string(),
            description: None,
            strategy: "personal_context".to_string(),
            output_format: "bullets".to_string(),
            retrieval_mode: "semantic".to_string(),
            created_by: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let summary = ReviewSummary {
            text: "review summary".to_string(),
            provider: "deterministic".to_string(),
            source: "deterministic".to_string(),
            model: None,
            fallback_reason: Some("summary provider not configured".to_string()),
        };

        let report = build_review_report_json(ReviewReportInput {
            lens: &lens,
            report_type: "weekly_review",
            window_start: Utc.with_ymd_and_hms(2026, 5, 18, 0, 0, 0).unwrap(),
            window_end: Utc.with_ymd_and_hms(2026, 5, 25, 0, 0, 0).unwrap(),
            memories: &[],
            summary: &summary,
        });

        assert_eq!(report["report_type"], "weekly_review");
        assert_eq!(report["lens"]["id"], json!(lens.id));
        assert_eq!(report["summary_provider"], "deterministic");
        assert_eq!(report["memory_count"], 0);
    }

    #[test]
    fn weekly_review_json_exposes_user_facing_sections() {
        let lens = LensDb {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            name: "最近的我".to_string(),
            description: None,
            strategy: "weekly_thought_review".to_string(),
            output_format: "bullets".to_string(),
            retrieval_mode: "semantic".to_string(),
            created_by: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let summary = ReviewSummary {
            text: "你正在从做很多东西转向选择主线。".to_string(),
            provider: "deterministic".to_string(),
            source: "deterministic".to_string(),
            model: None,
            fallback_reason: Some("summary provider not configured".to_string()),
        };

        let report = build_review_report_json(ReviewReportInput {
            lens: &lens,
            report_type: "weekly_review",
            window_start: Utc.with_ymd_and_hms(2026, 5, 18, 0, 0, 0).unwrap(),
            window_end: Utc.with_ymd_and_hms(2026, 5, 25, 0, 0, 0).unwrap(),
            memories: &[],
            summary: &summary,
        });

        assert!(report["recurring_themes"].is_array());
        assert!(report["inner_tensions"].is_array());
        assert!(report["forming_direction"]
            .as_str()
            .unwrap()
            .contains("主线"));
        assert!(report["next_step"].as_str().unwrap().contains("写下"));
    }

    #[test]
    fn learning_review_empty_state_uses_parent_friendly_sections() {
        let namespace = test_namespace("learning.stem");
        let summary = ReviewSummary {
            text: "No practice sessions were found.".to_string(),
            provider: "deterministic".to_string(),
            source: "deterministic".to_string(),
            model: None,
            fallback_reason: Some("summary provider not configured".to_string()),
        };

        let report = build_learning_review_report_json(LearningReviewInput {
            namespace: &namespace,
            window_start: Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap(),
            window_end: Utc.with_ymd_and_hms(2026, 6, 8, 0, 0, 0).unwrap(),
            feedback_loops: &[],
            source_memories: &[],
            summary: &summary,
        });

        assert_eq!(report["report_type"], "weekly_learning_review");
        assert_eq!(report["practice_session_count"], 0);
        assert_eq!(report["practiced_topics"].as_array().unwrap().len(), 0);
        assert!(report["suggested_next_practice"][0]
            .as_str()
            .unwrap()
            .contains("practice session"));
        assert_eq!(
            report["provenance"]["summary_source"].as_str(),
            Some("deterministic")
        );
    }

    #[test]
    fn learning_review_summarizes_attempt_feedback_and_repeated_mistakes() {
        let namespace = test_namespace("learning.stem");
        let sessions = vec![
            test_feedback_loop(
                namespace.space_id,
                namespace.id,
                "Improve fraction word problems",
                "Solve cup recipe fraction problem",
                Some("3/8 cup with unit labels"),
                Some("Changed units between steps"),
                Some("Write the unit next to every number"),
                Some("Try three unit-conversion fraction problems"),
                "completed",
            ),
            test_feedback_loop(
                namespace.space_id,
                namespace.id,
                "Improve fraction word problems",
                "Solve garden soil fraction problem",
                Some("2/6 bag"),
                Some("Changed units between steps"),
                Some("Check whether the unit stayed the same"),
                Some("Practice two fraction comparison word problems"),
                "completed",
            ),
        ];
        let summary = ReviewSummary {
            text: "Reviewed two practice sessions.".to_string(),
            provider: "deterministic".to_string(),
            source: "deterministic".to_string(),
            model: None,
            fallback_reason: Some("summary provider not configured".to_string()),
        };

        let report = build_learning_review_report_json(LearningReviewInput {
            namespace: &namespace,
            window_start: Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap(),
            window_end: Utc.with_ymd_and_hms(2026, 6, 8, 0, 0, 0).unwrap(),
            feedback_loops: &sessions,
            source_memories: &[],
            summary: &summary,
        });

        assert!(report["practiced_topics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("Improve fraction word problems")));
        assert_eq!(
            report["recurring_mistake_patterns"][0].as_str(),
            Some("Changed units between steps (2 sessions)")
        );
        assert!(report["improvement_signals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("2 practice sessions")));
        assert_eq!(
            report["source_feedback_loop_ids"].as_array().unwrap().len(),
            2
        );
    }

    #[test]
    fn learning_source_memories_filter_namespace_and_feedback_loop_provenance() {
        let namespace_id = Uuid::new_v4();
        let other_namespace_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let session = test_feedback_loop(
            space_id,
            namespace_id,
            "Practice fractions",
            "Solve one word problem",
            None,
            None,
            None,
            None,
            "active",
        );
        let matching_memory = test_memory(space_id, namespace_id, session.id);
        let wrong_namespace_memory = test_memory(space_id, other_namespace_id, session.id);
        let wrong_session_memory = test_memory(space_id, namespace_id, Uuid::new_v4());
        let memories = vec![
            matching_memory,
            wrong_namespace_memory,
            wrong_session_memory,
        ];

        let filtered = learning_source_memories(&memories, namespace_id, &[session]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, memories[0].id);
    }

    fn test_namespace(name: &str) -> NamespaceDb {
        NamespaceDb {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            name: name.to_string(),
            kind: "skill".to_string(),
            description: None,
            status: "active".to_string(),
            created_by: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn test_feedback_loop(
        space_id: Uuid,
        namespace_id: Uuid,
        goal: &str,
        task: &str,
        attempt: Option<&str>,
        evaluation: Option<&str>,
        feedback: Option<&str>,
        next_task: Option<&str>,
        status: &str,
    ) -> FeedbackLoopDb {
        FeedbackLoopDb {
            id: Uuid::new_v4(),
            space_id,
            namespace_id,
            goal: goal.to_string(),
            task: task.to_string(),
            attempt: attempt.map(str::to_string),
            evaluation: evaluation.map(str::to_string),
            feedback: feedback.map(str::to_string),
            adjustment: Some("Use a unit label step".to_string()),
            next_task: next_task.map(str::to_string),
            status: status.to_string(),
            created_by: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn test_memory(space_id: Uuid, namespace_id: Uuid, feedback_loop_id: Uuid) -> MemoryDb {
        MemoryDb {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id,
            title: Some("Practice snapshot".to_string()),
            content: "Practice goal: fractions".to_string(),
            memory_type: "text".to_string(),
            file_path: None,
            thumbnail_path: None,
            is_shared: false,
            source_type: "feedback_loop_event".to_string(),
            source_metadata: json!({
                "namespace_id": namespace_id,
                "feedback_loop_id": feedback_loop_id,
                "space_id": space_id,
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
