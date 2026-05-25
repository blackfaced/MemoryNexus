//! Cognitive review report API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::ai::{SummaryOptions, SummaryStyle};
use crate::auth::AuthenticatedUser;
use crate::db::lens::LensDb;
use crate::db::memory::MemoryDb;
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
        "next_actions": next_actions(input.memories),
        "summary_provider": input.summary.provider,
        "summary_source": input.summary.source,
        "summary_model": input.summary.model,
        "summary_fallback_reason": input.summary.fallback_reason,
    })
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
}
