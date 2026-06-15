//! AI API 端点

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ai::{
    deterministic_summarize, suggest_smart_tags, SmartTagSuggestion, SummaryOptions, SummaryResult,
    SummaryStyle,
};
use crate::auth::AuthenticatedUser;
use crate::db::lens::LensDb;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

/// 生成摘要请求
#[derive(Debug, Deserialize)]
pub struct SummarizeRequest {
    pub content: String,

    #[serde(default)]
    pub lens_id: Option<Uuid>,

    #[serde(default)]
    pub options: Option<SummaryOptions>,
}

/// 摘要响应
#[derive(Debug, Serialize)]
pub struct SummarizeResponse {
    pub summary: String,
    pub keywords: Vec<String>,
    pub language: String,
    pub original_length: usize,
    pub summary_length: usize,
    pub processing_time_ms: u64,
    pub summary_source: String,
    pub summary_provider: String,
    pub fallback_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lens: Option<LensSummaryProvenance>,
}

#[derive(Debug, Serialize)]
pub struct LensSummaryProvenance {
    pub id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub name: String,
    pub strategy: String,
    pub output_format: String,
    pub retrieval_mode: String,
}

/// 智能标签请求
#[derive(Debug, Deserialize)]
pub struct AutoTagRequest {
    pub content: String,
}

/// 智能标签响应
#[derive(Debug, Serialize)]
pub struct AutoTagResponse {
    pub suggested_tags: Vec<String>,
    pub categories: Vec<String>,
    pub suggestions: Vec<SmartTagSuggestion>,
    pub confidence: f32,
    pub source: String,
    pub editable: bool,
}

/// 获取 AI 配置（仅管理员）
#[derive(Debug, Serialize)]
pub struct AiConfigResponse {
    pub model: String,
    pub embedding_model: String,
    pub embedding_provider: String,
    pub enabled: bool,
    pub summary_enabled: bool,
    pub summary_provider: Option<String>,
    pub summary_model: Option<String>,
    pub summary_max_words: Option<usize>,
}

/// POST /api/v1/ai/summarize - 生成摘要
#[allow(dead_code)]
pub async fn summarize(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<SummarizeRequest>,
) -> Result<Json<ApiResponse<SummarizeResponse>>, AppError> {
    let lens = resolve_summary_lens(&state, auth_user.user_id, req.lens_id, None).await?;

    // 使用提供的选项，或让 Lens 输出格式决定默认摘要风格
    let options = summary_options_for_lens(req.options, lens.as_ref());

    // 生成摘要
    let summary = summarize_with_configured_provider(&state, &req.content, &options).await;

    Ok(Json(ApiResponse::success(SummarizeResponse {
        summary: summary.result.summary,
        keywords: summary.result.keywords,
        language: summary.result.language,
        original_length: summary.result.original_length,
        summary_length: summary.result.summary_length,
        processing_time_ms: summary.result.processing_time_ms,
        summary_source: summary.source,
        summary_provider: summary.provider,
        fallback_reason: summary.fallback_reason,
        lens,
    })))
}

/// POST /api/v1/memories/:id/summarize - 为记忆生成摘要
pub async fn summarize_memory(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SummarizeRequest>,
) -> Result<Json<ApiResponse<SummarizeResponse>>, AppError> {
    // 获取记忆
    let memory = state
        .repositories
        .memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;

    // 验证权限
    if memory.user_id != auth_user.user_id {
        return Err(AppError::Unauthorized);
    }
    let lens = resolve_summary_lens(
        &state,
        auth_user.user_id,
        req.lens_id,
        Some(memory.space_id),
    )
    .await?;

    // 生成摘要
    let options = summary_options_for_lens(req.options, lens.as_ref());
    let summary = summarize_with_configured_provider(&state, &memory.content, &options).await;

    // 更新记忆记录（如果用户想要保存）
    // 这里只是返回结果，实际保存由客户端决定

    Ok(Json(ApiResponse::success(SummarizeResponse {
        summary: summary.result.summary,
        keywords: summary.result.keywords,
        language: summary.result.language,
        original_length: summary.result.original_length,
        summary_length: summary.result.summary_length,
        processing_time_ms: summary.result.processing_time_ms,
        summary_source: summary.source,
        summary_provider: summary.provider,
        fallback_reason: summary.fallback_reason,
        lens,
    })))
}

struct SummaryExecution {
    result: SummaryResult,
    source: String,
    provider: String,
    fallback_reason: Option<String>,
}

async fn summarize_with_configured_provider(
    state: &AppState,
    content: &str,
    options: &SummaryOptions,
) -> SummaryExecution {
    let fallback_provider = state
        .ai
        .summary_provider
        .clone()
        .unwrap_or_else(|| "deterministic".to_string());

    if let Some(summarizer) = &state.ai.summarizer {
        match summarizer.summarize(content, options).await {
            Ok(result) if !result.summary.trim().is_empty() => {
                return SummaryExecution {
                    result,
                    source: "ai".to_string(),
                    provider: fallback_provider,
                    fallback_reason: None,
                };
            }
            Ok(_) => {
                return SummaryExecution {
                    result: deterministic_summarize(content, options),
                    source: "deterministic".to_string(),
                    provider: fallback_provider,
                    fallback_reason: Some("summary provider returned empty output".to_string()),
                };
            }
            Err(error) => {
                return SummaryExecution {
                    result: deterministic_summarize(content, options),
                    source: "deterministic".to_string(),
                    provider: fallback_provider,
                    fallback_reason: Some(error.to_string()),
                };
            }
        }
    }

    SummaryExecution {
        result: deterministic_summarize(content, options),
        source: "deterministic".to_string(),
        provider: "deterministic".to_string(),
        fallback_reason: Some("summary provider not configured".to_string()),
    }
}

async fn resolve_summary_lens(
    state: &AppState,
    user_id: Uuid,
    lens_id: Option<Uuid>,
    expected_space_id: Option<Uuid>,
) -> Result<Option<LensSummaryProvenance>, AppError> {
    let Some(lens_id) = lens_id else {
        return Ok(None);
    };

    let lens = state
        .repositories
        .lenses
        .find_for_user(lens_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if let Some(space_id) = expected_space_id {
        if lens.space_id != space_id {
            return Err(AppError::BadRequest(
                "lens_id must belong to the same Cognitive Space as the memory".to_string(),
            ));
        }
    }

    Ok(Some(summary_lens_provenance(lens)))
}

fn summary_lens_provenance(lens: LensDb) -> LensSummaryProvenance {
    LensSummaryProvenance {
        id: lens.id,
        space_id: lens.space_id,
        namespace_id: lens.namespace_id,
        name: lens.name,
        strategy: lens.strategy,
        output_format: lens.output_format,
        retrieval_mode: lens.retrieval_mode,
    }
}

fn summary_options_for_lens(
    explicit: Option<SummaryOptions>,
    lens: Option<&LensSummaryProvenance>,
) -> SummaryOptions {
    if let Some(options) = explicit {
        return options;
    }

    let mut options = SummaryOptions::default();
    if let Some(lens) = lens {
        match lens.output_format.as_str() {
            "brief" => {
                options.max_words = 80;
                options.style = SummaryStyle::Concise;
            }
            "bullets" | "bullet_points" => {
                options.style = SummaryStyle::BulletPoints;
            }
            _ => {}
        }
    }
    options
}

/// POST /api/v1/ai/autotag - 智能标签推荐
pub async fn auto_tag(
    State(_state): State<AppState>,
    _auth_user: AuthenticatedUser,
    Json(req): Json<AutoTagRequest>,
) -> Result<Json<ApiResponse<AutoTagResponse>>, AppError> {
    let suggestions = suggest_smart_tags(&req.content);
    let tags = suggestions
        .iter()
        .map(|suggestion| suggestion.tag.clone())
        .collect::<Vec<_>>();
    let categories = suggestions
        .iter()
        .map(|suggestion| suggestion.category.clone())
        .fold(Vec::<String>::new(), |mut categories, category| {
            if !categories.contains(&category) {
                categories.push(category);
            }
            categories
        });

    Ok(Json(ApiResponse::success(AutoTagResponse {
        suggested_tags: tags,
        categories,
        suggestions,
        confidence: 0.8,
        source: "deterministic".to_string(),
        editable: true,
    })))
}

/// GET /api/v1/ai/config - 获取 AI 配置
pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AiConfigResponse>>, AppError> {
    let enabled = std::env::var("OPENAI_API_KEY").is_ok();

    Ok(Json(ApiResponse::success(AiConfigResponse {
        model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
        embedding_model: std::env::var("OPENAI_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-ada-002".to_string()),
        embedding_provider: std::env::var("MEMORYNEXUS_EMBEDDING_PROVIDER")
            .or_else(|_| std::env::var("EMBEDDING_PROVIDER"))
            .unwrap_or_else(|_| "openai".to_string()),
        enabled,
        summary_enabled: state.ai.summarizer.is_some(),
        summary_provider: state.ai.summary_provider,
        summary_model: state.ai.summary_model,
        summary_max_words: state.ai.summary_max_words,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_request_serde() {
        let json = r#"{"content":"测试内容"}"#;
        let req: SummarizeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "测试内容");
        assert!(req.options.is_none());
    }

    #[test]
    fn test_summarize_request_with_options() {
        let json = r#"{"content":"测试内容","options":{"max_words":100,"language":"en"}}"#;
        let req: SummarizeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "测试内容");
        assert!(req.options.is_some());
    }

    #[test]
    fn test_summarize_request_with_lens_id() {
        let lens_id = Uuid::new_v4();
        let json = format!(r#"{{"content":"测试内容","lens_id":"{}"}}"#, lens_id);
        let req: SummarizeRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.lens_id, Some(lens_id));
    }

    #[test]
    fn test_summarize_response_serde() {
        let response = SummarizeResponse {
            summary: "测试摘要".to_string(),
            keywords: vec!["测试".to_string(), "摘要".to_string()],
            language: "zh".to_string(),
            original_length: 100,
            summary_length: 50,
            processing_time_ms: 100,
            summary_source: "deterministic".to_string(),
            summary_provider: "deterministic".to_string(),
            fallback_reason: Some("summary provider not configured".to_string()),
            lens: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("测试摘要"));
        assert!(json.contains("\"keywords\":"));
    }

    #[test]
    fn test_summarize_response_can_include_lens_provenance() {
        let lens_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let response = SummarizeResponse {
            summary: "测试摘要".to_string(),
            keywords: vec![],
            language: "zh".to_string(),
            original_length: 100,
            summary_length: 50,
            processing_time_ms: 100,
            summary_source: "deterministic".to_string(),
            summary_provider: "deterministic".to_string(),
            fallback_reason: None,
            lens: Some(LensSummaryProvenance {
                id: lens_id,
                space_id,
                namespace_id: None,
                name: "Project Context".to_string(),
                strategy: "project_context".to_string(),
                output_format: "brief".to_string(),
                retrieval_mode: "semantic".to_string(),
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"strategy\":\"project_context\""));
        assert!(json.contains(&lens_id.to_string()));
    }

    #[test]
    fn test_auto_tag_request_serde() {
        let json = r#"{"content":"这是一段关于旅行的内容"}"#;
        let req: AutoTagRequest = serde_json::from_str(json).unwrap();
        assert!(req.content.contains("旅行"));
    }

    #[test]
    fn auto_tag_response_marks_suggestions_editable() {
        let suggestions = suggest_smart_tags("Rust Cognitive Space project");
        let response = AutoTagResponse {
            suggested_tags: suggestions
                .iter()
                .map(|suggestion| suggestion.tag.clone())
                .collect(),
            categories: suggestions
                .iter()
                .map(|suggestion| suggestion.category.clone())
                .collect(),
            suggestions,
            confidence: 0.8,
            source: "deterministic".to_string(),
            editable: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"editable\":true"));
        assert!(json.contains("\"suggestions\":"));
        assert!(json.contains("rust"));
    }

    #[test]
    fn test_ai_config_response_serde() {
        let config = AiConfigResponse {
            model: "gpt-4".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            embedding_provider: "local".to_string(),
            enabled: true,
            summary_enabled: true,
            summary_provider: Some("openrouter".to_string()),
            summary_model: Some("openrouter/free".to_string()),
            summary_max_words: Some(120),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"summary_provider\":\"openrouter\""));
    }

    #[test]
    fn summary_lens_provenance_keeps_strategy_and_space() {
        let lens = LensDb {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: None,
            name: "Project Context".to_string(),
            description: None,
            strategy: "project_context".to_string(),
            output_format: "brief".to_string(),
            retrieval_mode: "semantic".to_string(),
            created_by: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let provenance = summary_lens_provenance(lens.clone());

        assert_eq!(provenance.id, lens.id);
        assert_eq!(provenance.space_id, lens.space_id);
        assert_eq!(provenance.namespace_id, lens.namespace_id);
        assert_eq!(provenance.strategy, "project_context");
    }

    #[test]
    fn summary_options_follow_lens_output_format_without_explicit_options() {
        let lens = LensSummaryProvenance {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: None,
            name: "Learning Review".to_string(),
            strategy: "learning_review".to_string(),
            output_format: "bullets".to_string(),
            retrieval_mode: "semantic".to_string(),
        };

        let options = summary_options_for_lens(None, Some(&lens));

        assert_eq!(options.style, SummaryStyle::BulletPoints);
    }

    #[test]
    fn summary_options_keep_explicit_options_over_lens_defaults() {
        let lens = LensSummaryProvenance {
            id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: None,
            name: "Project Context".to_string(),
            strategy: "project_context".to_string(),
            output_format: "brief".to_string(),
            retrieval_mode: "semantic".to_string(),
        };
        let explicit = SummaryOptions {
            max_words: 200,
            language: "en".to_string(),
            include_keywords: false,
            style: SummaryStyle::Detailed,
        };

        let options = summary_options_for_lens(Some(explicit), Some(&lens));

        assert_eq!(options.max_words, 200);
        assert_eq!(options.style, SummaryStyle::Detailed);
    }
}
