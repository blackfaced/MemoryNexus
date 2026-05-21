//! Lens Run API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::ai::{Summarizer, SummaryOptions, SummaryStyle};
use crate::auth::AuthenticatedUser;
use crate::db::lens_run::{CreateCompletedLensRun, LensRunDb};
use crate::error::{ApiResponse, AppError};
use crate::search::{MemorySearchItem, SearchEngine, SearchQuery, SemanticSearchError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RunLensRequest {
    pub lens_id: Uuid,
    pub query: String,
    pub limit: Option<i64>,
}

/// POST /api/v1/lens-runs - Execute a Lens synchronously.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<RunLensRequest>,
) -> Result<(StatusCode, Json<ApiResponse<LensRunDb>>), AppError> {
    let query = req.query.trim();
    if query.is_empty() {
        return Err(AppError::BadRequest("Lens Run query 不能为空".to_string()));
    }

    let lens = state
        .repositories
        .lenses
        .find_for_user(req.lens_id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    let limit = normalize_limit(req.limit);
    let engine = SearchEngine::with_semantic_dependencies(
        state.db.clone(),
        state.vector_store.clone(),
        state.ai.embedder.clone(),
    );
    let search_query = SearchQuery {
        space_id: Some(lens.space_id),
        q: Some(query.to_string()),
        semantic: lens.retrieval_mode.eq_ignore_ascii_case("semantic"),
        limit,
        ..SearchQuery::default()
    };

    let result = if search_query.semantic {
        match engine
            .semantic_search(&search_query, auth_user.user_id, lens.space_id)
            .await
        {
            Ok(result) => result,
            Err(
                SemanticSearchError::VectorStoreMissing
                | SemanticSearchError::EmbeddingProviderMissing,
            ) => engine
                .search(&search_query, auth_user.user_id, lens.space_id)
                .await
                .map_err(AppError::Database)?,
            Err(SemanticSearchError::EmptyQuery) => {
                return Err(AppError::BadRequest("Lens Run query 不能为空".to_string()));
            }
            Err(SemanticSearchError::Database(error)) => return Err(AppError::Database(error)),
            Err(SemanticSearchError::Embedding(error)) => {
                return Err(AppError::Internal(error.to_string()));
            }
            Err(SemanticSearchError::Vector(error)) => {
                return Err(AppError::Internal(error.to_string()));
            }
        }
    } else {
        engine
            .search(&search_query, auth_user.user_id, lens.space_id)
            .await
            .map_err(AppError::Database)?
    };

    let memory_ids = result.items.iter().map(|memory| memory.id).collect();
    let output = build_lens_output(LensOutputInput {
        lens_id: lens.id,
        lens_name: &lens.name,
        strategy: &lens.strategy,
        output_format: &lens.output_format,
        retrieval_mode: &lens.retrieval_mode,
        query,
        search_mode: &result.search_mode,
        memories: &result.items,
        summarizer: state.ai.summarizer.clone(),
        summary_provider: state.ai.summary_provider.clone(),
        summary_model: state.ai.summary_model.clone(),
        summary_max_words: state.ai.summary_max_words,
    })
    .await;

    let run = state
        .repositories
        .lens_runs
        .create_completed(CreateCompletedLensRun {
            lens_id: lens.id,
            space_id: lens.space_id,
            query: Some(query.to_string()),
            input_memory_ids: memory_ids,
            output,
            created_by: auth_user.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(run))))
}

/// GET /api/v1/lens-runs/:id - Get a Lens Run visible to the current user.
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<LensRunDb>>, AppError> {
    let run = state
        .repositories
        .lens_runs
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::success(run)))
}

fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(5).clamp(1, 20)
}

fn memory_output(memory: &MemorySearchItem) -> Value {
    json!({
        "id": memory.id,
        "title": memory.title,
        "content": memory.content,
        "memory_type": memory.memory_type,
        "relevance": memory.relevance,
    })
}

struct LensOutputInput<'a> {
    lens_id: Uuid,
    lens_name: &'a str,
    strategy: &'a str,
    output_format: &'a str,
    retrieval_mode: &'a str,
    query: &'a str,
    search_mode: &'a str,
    memories: &'a [MemorySearchItem],
    summarizer: Option<Arc<dyn Summarizer>>,
    summary_provider: Option<String>,
    summary_model: Option<String>,
    summary_max_words: Option<usize>,
}

async fn build_lens_output(input: LensOutputInput<'_>) -> Value {
    let memories: Vec<Value> = input.memories.iter().map(memory_output).collect();
    let summary = generate_lens_summary(&input).await;

    json!({
        "lens": {
            "id": input.lens_id,
            "name": input.lens_name,
            "strategy": input.strategy,
            "output_format": input.output_format,
            "retrieval_mode": input.retrieval_mode,
        },
        "query": input.query,
        "search_mode": input.search_mode,
        "memory_count": memories.len(),
        "memories": memories,
        "summary": summary.text,
        "summary_provider": summary.provider,
        "summary_model": summary.model,
        "summary_source": summary.source,
        "summary_fallback_reason": summary.fallback_reason,
    })
}

struct LensSummary {
    text: String,
    provider: String,
    model: Option<String>,
    source: String,
    fallback_reason: Option<String>,
}

async fn generate_lens_summary(input: &LensOutputInput<'_>) -> LensSummary {
    let fallback = || {
        deterministic_summary(
            input.lens_name,
            input.strategy,
            input.query,
            input.memories.len(),
        )
    };

    let Some(summarizer) = &input.summarizer else {
        return LensSummary {
            text: fallback(),
            provider: "deterministic".to_string(),
            model: None,
            source: "deterministic".to_string(),
            fallback_reason: Some("summary provider not configured".to_string()),
        };
    };

    let attempted_provider = input
        .summary_provider
        .clone()
        .unwrap_or_else(|| "openai".to_string());

    if input.memories.is_empty() {
        return LensSummary {
            text: fallback(),
            provider: attempted_provider,
            model: input.summary_model.clone(),
            source: "deterministic".to_string(),
            fallback_reason: Some("no memories retrieved".to_string()),
        };
    }

    let prompt = build_lens_summary_prompt(input);
    let options = SummaryOptions {
        max_words: input
            .summary_max_words
            .unwrap_or_else(|| summary_word_limit(input.output_format)),
        language: "zh".to_string(),
        include_keywords: false,
        style: summary_style(input.output_format),
    };

    match summarizer.summarize(&prompt, &options).await {
        Ok(result) if !result.summary.trim().is_empty() => LensSummary {
            text: result.summary,
            provider: attempted_provider,
            model: input.summary_model.clone(),
            source: "ai".to_string(),
            fallback_reason: None,
        },
        Ok(_) => LensSummary {
            text: fallback(),
            provider: attempted_provider,
            model: input.summary_model.clone(),
            source: "deterministic".to_string(),
            fallback_reason: Some("summary provider returned empty output".to_string()),
        },
        Err(error) => LensSummary {
            text: fallback(),
            provider: attempted_provider,
            model: input.summary_model.clone(),
            source: "deterministic".to_string(),
            fallback_reason: Some(error.to_string()),
        },
    }
}

fn build_lens_summary_prompt(input: &LensOutputInput<'_>) -> String {
    let memories = input
        .memories
        .iter()
        .enumerate()
        .map(|(index, memory)| {
            format!(
                "[{}] id={} title={} relevance={:?}\n{}",
                index + 1,
                memory.id,
                memory.title.as_deref().unwrap_or("(untitled)"),
                memory.relevance,
                memory.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        r#"你正在执行 MemoryNexus Lens Run。

Lens:
- name: {lens_name}
- strategy: {strategy}
- output_format: {output_format}
- retrieval_mode: {retrieval_mode}
- actual_search_mode: {search_mode}

User query:
{query}

Retrieved memories:
{memories}

请基于检索到的 memories 回答 query。要求：
1. 只使用给定 memories 中的信息，不要编造。
2. 明确给出可执行的理解或总结。
3. 如果信息不足，说明缺口。
4. 保持输出格式符合 output_format。"#,
        lens_name = input.lens_name,
        strategy = input.strategy,
        output_format = input.output_format,
        retrieval_mode = input.retrieval_mode,
        search_mode = input.search_mode,
        query = input.query,
        memories = memories
    )
}

fn summary_word_limit(output_format: &str) -> usize {
    match output_format {
        "detailed" => 180,
        "bullets" | "bullet_points" => 120,
        _ => 80,
    }
}

fn summary_style(output_format: &str) -> SummaryStyle {
    match output_format {
        "detailed" => SummaryStyle::Detailed,
        "bullets" | "bullet_points" => SummaryStyle::BulletPoints,
        _ => SummaryStyle::Concise,
    }
}

fn deterministic_summary(
    lens_name: &str,
    strategy: &str,
    query: &str,
    memory_count: usize,
) -> String {
    if memory_count == 0 {
        return format!(
            "Lens '{lens_name}' found no matching memories for query '{query}' using strategy '{strategy}'."
        );
    }

    format!(
        "Lens '{lens_name}' interpreted {memory_count} memories for query '{query}' using strategy '{strategy}'."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::summary::SummaryError;
    use crate::ai::SummaryResult;
    use chrono::Utc;
    use uuid::Uuid;

    struct StaticSummarizer;

    #[async_trait::async_trait]
    impl Summarizer for StaticSummarizer {
        async fn summarize(
            &self,
            content: &str,
            options: &SummaryOptions,
        ) -> Result<SummaryResult, SummaryError> {
            Ok(SummaryResult {
                summary: format!(
                    "AI summary using {} words over prompt containing {} chars",
                    options.max_words,
                    content.len()
                ),
                keywords: vec![],
                language: options.language.clone(),
                original_length: content.len(),
                summary_length: 10,
                processing_time_ms: 1,
            })
        }

        async fn summarize_batch(
            &self,
            contents: Vec<String>,
            options: &SummaryOptions,
        ) -> Result<Vec<SummaryResult>, SummaryError> {
            let mut results = Vec::new();
            for content in contents {
                results.push(self.summarize(&content, options).await?);
            }
            Ok(results)
        }
    }

    fn search_item(id: Uuid) -> MemorySearchItem {
        let now = Utc::now();
        MemorySearchItem {
            id,
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            title: Some("Direction".to_string()),
            content: "MemoryNexus is Rust-first.".to_string(),
            memory_type: "text".to_string(),
            is_shared: false,
            created_at: now,
            updated_at: now,
            relevance: Some(0.9),
            matched_on: Some(vec!["semantic".to_string()]),
        }
    }

    #[test]
    fn run_lens_request_deserializes() {
        let lens_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "lens_id":"{lens_id}",
                "query":"Summarize the project direction",
                "limit":3
            }}"#
        );
        let req: RunLensRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.lens_id, lens_id);
        assert_eq!(req.query, "Summarize the project direction");
        assert_eq!(req.limit, Some(3));
    }

    #[tokio::test]
    async fn lens_run_output_keeps_traceable_shape_without_ai() {
        let lens_id = Uuid::new_v4();
        let memory_id = Uuid::new_v4();
        let memories = vec![search_item(memory_id)];
        let output = build_lens_output(LensOutputInput {
            lens_id,
            lens_name: "Project Context",
            strategy: "project_context",
            output_format: "brief",
            retrieval_mode: "semantic",
            query: "Summarize the project direction",
            search_mode: "semantic",
            memories: &memories,
            summarizer: None,
            summary_provider: None,
            summary_model: None,
            summary_max_words: None,
        })
        .await;

        assert_eq!(output["lens"]["id"], lens_id.to_string());
        assert_eq!(output["query"], "Summarize the project direction");
        assert_eq!(output["search_mode"], "semantic");
        assert_eq!(output["memory_count"], 1);
        assert_eq!(output["memories"][0]["id"], memory_id.to_string());
        assert_eq!(output["summary_provider"], "deterministic");
        assert_eq!(output["summary_source"], "deterministic");
        assert_eq!(
            output["summary_fallback_reason"],
            "summary provider not configured"
        );
        assert!(output["summary"]
            .as_str()
            .unwrap()
            .contains("Project Context"));
    }

    #[tokio::test]
    async fn lens_run_output_uses_configured_summarizer() {
        let lens_id = Uuid::new_v4();
        let memories = vec![search_item(Uuid::new_v4())];
        let output = build_lens_output(LensOutputInput {
            lens_id,
            lens_name: "Project Context",
            strategy: "project_context",
            output_format: "brief",
            retrieval_mode: "semantic",
            query: "Summarize the project direction",
            search_mode: "semantic",
            memories: &memories,
            summarizer: Some(Arc::new(StaticSummarizer)),
            summary_provider: Some("openrouter".to_string()),
            summary_model: Some("gpt-test".to_string()),
            summary_max_words: Some(42),
        })
        .await;

        assert_eq!(output["summary_provider"], "openrouter");
        assert_eq!(output["summary_model"], "gpt-test");
        assert_eq!(output["summary_source"], "ai");
        assert!(output["summary_fallback_reason"].is_null());
        assert!(output["summary"].as_str().unwrap().contains("42 words"));
    }

    struct EmptySummarizer;

    #[async_trait::async_trait]
    impl Summarizer for EmptySummarizer {
        async fn summarize(
            &self,
            content: &str,
            options: &SummaryOptions,
        ) -> Result<SummaryResult, SummaryError> {
            Ok(SummaryResult {
                summary: String::new(),
                keywords: vec![],
                language: options.language.clone(),
                original_length: content.len(),
                summary_length: 0,
                processing_time_ms: 1,
            })
        }

        async fn summarize_batch(
            &self,
            contents: Vec<String>,
            options: &SummaryOptions,
        ) -> Result<Vec<SummaryResult>, SummaryError> {
            let mut results = Vec::new();
            for content in contents {
                results.push(self.summarize(&content, options).await?);
            }
            Ok(results)
        }
    }

    #[tokio::test]
    async fn lens_run_output_keeps_attempted_provider_on_empty_summary_fallback() {
        let lens_id = Uuid::new_v4();
        let memories = vec![search_item(Uuid::new_v4())];
        let output = build_lens_output(LensOutputInput {
            lens_id,
            lens_name: "Project Context",
            strategy: "project_context",
            output_format: "brief",
            retrieval_mode: "semantic",
            query: "Summarize the project direction",
            search_mode: "semantic",
            memories: &memories,
            summarizer: Some(Arc::new(EmptySummarizer)),
            summary_provider: Some("openrouter".to_string()),
            summary_model: Some("openrouter/free".to_string()),
            summary_max_words: Some(42),
        })
        .await;

        assert_eq!(output["summary_provider"], "openrouter");
        assert_eq!(output["summary_source"], "deterministic");
        assert_eq!(
            output["summary_fallback_reason"],
            "summary provider returned empty output"
        );
    }
}
