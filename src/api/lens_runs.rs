//! Lens Run API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::ai::{Summarizer, SummaryOptions, SummaryStyle};
use crate::auth::AuthenticatedUser;
use crate::db::lens_run::{CreateCompletedLensRun, LensRunDb, LensRunListFilter};
use crate::error::{ApiResponse, AppError};
use crate::search::{MemorySearchItem, SearchEngine, SearchQuery, SemanticSearchError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RunLensRequest {
    pub lens_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
    pub query: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListLensRunsQuery {
    pub lens_id: Option<Uuid>,
    pub space_id: Option<Uuid>,
    pub namespace_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct LensRunListResponse {
    pub items: Vec<LensRunDb>,
    pub total: usize,
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
    let provenance = validate_provenance(
        &state,
        auth_user.user_id,
        lens.space_id,
        resolve_lens_namespace(lens.namespace_id, req.namespace_id)?,
        req.feedback_loop_id,
    )
    .await?;

    let limit = normalize_limit(req.limit);
    let engine = SearchEngine::with_semantic_dependencies(
        state.db.clone(),
        state.vector_store.clone(),
        state.ai.embedder.clone(),
    );
    let search_query = SearchQuery {
        space_id: Some(lens.space_id),
        namespace_id: provenance.namespace_id,
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
        namespace_id: provenance.namespace_id,
        feedback_loop_id: provenance.feedback_loop_id,
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
            namespace_id: provenance.namespace_id,
            feedback_loop_id: provenance.feedback_loop_id,
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

/// GET /api/v1/lens-runs?lens_id=<LENS_ID>|space_id=<SPACE_ID> - List visible Lens Runs.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<ListLensRunsQuery>,
) -> Result<Json<ApiResponse<LensRunListResponse>>, AppError> {
    if query.lens_id.is_none() && query.space_id.is_none() {
        return Err(AppError::BadRequest(
            "lens_id or space_id is required".to_string(),
        ));
    }

    if let Some(lens_id) = query.lens_id {
        state
            .repositories
            .lenses
            .find_for_user(lens_id, auth_user.user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;
    }

    if let Some(space_id) = query.space_id {
        state
            .repositories
            .spaces
            .find_for_user(space_id, auth_user.user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;
    }
    let namespace_id = if let Some(namespace_id) = query.namespace_id {
        let space_id = if let Some(space_id) = query.space_id {
            space_id
        } else if let Some(lens_id) = query.lens_id {
            state
                .repositories
                .lenses
                .find_for_user(lens_id, auth_user.user_id)
                .await
                .map_err(AppError::Database)?
                .ok_or(AppError::Unauthorized)?
                .space_id
        } else {
            return Err(AppError::BadRequest(
                "space_id or lens_id is required with namespace_id".to_string(),
            ));
        };
        validate_namespace(&state, auth_user.user_id, space_id, Some(namespace_id)).await?
    } else {
        None
    };

    let runs = state
        .repositories
        .lens_runs
        .list_for_user(
            LensRunListFilter {
                lens_id: query.lens_id,
                space_id: query.space_id,
                namespace_id,
                limit: normalize_limit(query.limit),
            },
            auth_user.user_id,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(LensRunListResponse {
        total: runs.len(),
        items: runs,
    })))
}

fn normalize_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(5).clamp(1, 20)
}

struct ProvenanceScope {
    namespace_id: Option<Uuid>,
    feedback_loop_id: Option<Uuid>,
}

async fn validate_provenance(
    state: &AppState,
    user_id: Uuid,
    space_id: Uuid,
    namespace_id: Option<Uuid>,
    feedback_loop_id: Option<Uuid>,
) -> Result<ProvenanceScope, AppError> {
    let namespace_id = validate_namespace(state, user_id, space_id, namespace_id).await?;
    let Some(feedback_loop_id) = feedback_loop_id else {
        return Ok(ProvenanceScope {
            namespace_id,
            feedback_loop_id: None,
        });
    };

    let feedback_loop = state
        .repositories
        .feedback_loops
        .find_for_user(feedback_loop_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;
    if feedback_loop.space_id != space_id {
        return Err(AppError::BadRequest(
            "feedback_loop_id must belong to the requested Cognitive Space".to_string(),
        ));
    }
    if let Some(namespace_id) = namespace_id {
        if feedback_loop.namespace_id != namespace_id {
            return Err(AppError::BadRequest(
                "feedback_loop_id must belong to namespace_id".to_string(),
            ));
        }
    }

    Ok(ProvenanceScope {
        namespace_id: Some(feedback_loop.namespace_id),
        feedback_loop_id: Some(feedback_loop_id),
    })
}

fn resolve_lens_namespace(
    lens_namespace_id: Option<Uuid>,
    requested_namespace_id: Option<Uuid>,
) -> Result<Option<Uuid>, AppError> {
    match (lens_namespace_id, requested_namespace_id) {
        (Some(lens_namespace_id), Some(requested_namespace_id))
            if lens_namespace_id != requested_namespace_id =>
        {
            Err(AppError::BadRequest(
                "namespace_id must match the Lens namespace".to_string(),
            ))
        }
        (Some(lens_namespace_id), _) => Ok(Some(lens_namespace_id)),
        (None, requested_namespace_id) => Ok(requested_namespace_id),
    }
}

async fn validate_namespace(
    state: &AppState,
    user_id: Uuid,
    space_id: Uuid,
    namespace_id: Option<Uuid>,
) -> Result<Option<Uuid>, AppError> {
    let Some(namespace_id) = namespace_id else {
        return Ok(None);
    };
    let namespace = state
        .repositories
        .namespaces
        .find_for_user(namespace_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;
    if namespace.space_id != space_id {
        return Err(AppError::BadRequest(
            "namespace_id must belong to the requested Cognitive Space".to_string(),
        ));
    }
    Ok(Some(namespace_id))
}

fn memory_output(memory: &MemorySearchItem) -> Value {
    json!({
        "id": memory.id,
        "namespace_id": memory.namespace_id,
        "feedback_loop_id": memory.feedback_loop_id,
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
    namespace_id: Option<Uuid>,
    feedback_loop_id: Option<Uuid>,
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
    let key_points = build_key_points(input.memories);
    let open_questions = build_open_questions(&input);
    let suggested_next_actions = build_suggested_next_actions(input.strategy, input.memories.len());
    let citations = build_citations(input.memories);
    let unresolved_contradictions = build_unresolved_contradictions();
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
        "provenance": {
            "namespace_id": input.namespace_id,
            "feedback_loop_id": input.feedback_loop_id,
        },
        "search_mode": input.search_mode,
        "memory_count": memories.len(),
        "memories": memories,
        "summary": summary.text,
        "key_points": key_points,
        "open_questions": open_questions,
        "suggested_next_actions": suggested_next_actions,
        "citations": citations,
        "unresolved_contradictions": unresolved_contradictions,
        "summary_provider": summary.provider,
        "summary_model": summary.model,
        "summary_source": summary.source,
        "summary_fallback_reason": summary.fallback_reason,
    })
}

fn build_unresolved_contradictions() -> Vec<Value> {
    Vec::new()
}

fn build_key_points(memories: &[MemorySearchItem]) -> Vec<Value> {
    memories
        .iter()
        .take(5)
        .map(|memory| {
            json!({
                "memory_id": memory.id,
                "title": memory.title,
                "point": first_sentence(&memory.content),
            })
        })
        .collect()
}

fn build_open_questions(input: &LensOutputInput<'_>) -> Vec<String> {
    if input.memories.is_empty() {
        return vec![format!(
            "No memories matched query '{}'; add more context or broaden the query.",
            input.query
        )];
    }

    match input.strategy {
        "engineering_review" => vec![
            "如果只能推进一个下一步，最小可验证动作是什么？".to_string(),
            "你会用什么评估标准决定继续或停止？".to_string(),
        ],
        "detective_review" => vec![
            "这里有没有一个你暂时不想正面回答的问题？".to_string(),
            "哪些担心是事实，哪些只是尚未验证的假设？".to_string(),
        ],
        "narrative_review" => vec![
            "这更像哪个阶段的转变：发散、选择，还是收束？".to_string(),
            "如果这是一个长期主线的开头，它会指向什么？".to_string(),
        ],
        "risk_review" => vec![
            "Which risks are still unsupported by concrete memories?".to_string(),
            "Which contradictions need a follow-up Lens Run?".to_string(),
        ],
        "learning_review" => vec![
            "Which learning gaps appear repeatedly?".to_string(),
            "What is the next smallest practice step?".to_string(),
        ],
        _ => vec![
            "What additional memories would make this interpretation more reliable?".to_string(),
        ],
    }
}

fn build_suggested_next_actions(strategy: &str, memory_count: usize) -> Vec<String> {
    if memory_count == 0 {
        return vec![
            "Add at least one memory related to this query, then rerun the Lens.".to_string(),
        ];
    }

    match strategy {
        "engineering_review" => vec![
            "写下三个判断标准，再用它们筛选当前选项。".to_string(),
            "把最小下一步限制在 30 分钟内可以完成的动作。".to_string(),
        ],
        "detective_review" => vec![
            "标出这段想法里最强的一个假设。".to_string(),
            "补一条记忆：你最不愿意承认的真实顾虑是什么？".to_string(),
        ],
        "narrative_review" => vec![
            "给当前阶段起一个名字，方便之后回看。".to_string(),
            "观察下一周是否仍然围绕同一个主题打转。".to_string(),
        ],
        "risk_review" => vec![
            "Review the cited memories and mark unresolved risks.".to_string(),
            "Add mitigation memories for the highest-risk item.".to_string(),
        ],
        "learning_review" => vec![
            "Turn the strongest learning point into a concrete next practice task.".to_string(),
            "Add a follow-up reflection after the next practice session.".to_string(),
        ],
        "family_growth" => vec![
            "Keep the cited memories as continuity anchors for future reviews.".to_string(),
            "Add new observations when the same pattern appears again.".to_string(),
        ],
        _ => vec![
            "Review the cited memories before acting on this interpretation.".to_string(),
            "Run another Lens if you need a different perspective.".to_string(),
        ],
    }
}

fn build_citations(memories: &[MemorySearchItem]) -> Vec<Value> {
    memories
        .iter()
        .map(|memory| {
            json!({
                "memory_id": memory.id,
                "title": memory.title,
                "relevance": memory.relevance,
            })
        })
        .collect()
}

fn first_sentence(content: &str) -> String {
    content
        .split(['.', '。', '!', '！', '?', '？'])
        .map(str::trim)
        .find(|sentence| !sentence.is_empty())
        .unwrap_or(content.trim())
        .to_string()
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
    match strategy {
        "engineering_review" => {
            return format!(
                "{lens_name}：你现在的问题可能不是想法太多，而是缺少评估标准。先把这件事拆成可比较的条件，再决定下一步。"
            );
        }
        "detective_review" => {
            return format!(
                "{lens_name}：这段想法背后可能藏着一个更核心的问题。先找出你正在回避的假设，再判断它是否真的成立。"
            );
        }
        "narrative_review" => {
            return format!(
                "{lens_name}：这像是一个从发散走向选择的阶段。重点不是马上得出答案，而是看清你正在形成哪条主线。"
            );
        }
        "weekly_thought_review" => {
            return format!(
                "{lens_name}：最近的记录正在形成一条可观察的线索。继续保存想法后，系统会更稳定地提炼反复主题和内在张力。"
            );
        }
        _ => {}
    }

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
            namespace_id: None,
            feedback_loop_id: None,
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
        let namespace_id = Uuid::new_v4();
        let feedback_loop_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "lens_id":"{lens_id}",
                "namespace_id":"{namespace_id}",
                "feedback_loop_id":"{feedback_loop_id}",
                "query":"Summarize the project direction",
                "limit":3
            }}"#
        );
        let req: RunLensRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(req.lens_id, lens_id);
        assert_eq!(req.namespace_id, Some(namespace_id));
        assert_eq!(req.feedback_loop_id, Some(feedback_loop_id));
        assert_eq!(req.query, "Summarize the project direction");
        assert_eq!(req.limit, Some(3));
    }

    #[test]
    fn list_lens_runs_query_deserializes() {
        let lens_id = Uuid::new_v4();
        let query: ListLensRunsQuery = serde_json::from_value(json!({
            "lens_id": lens_id,
            "namespace_id": Uuid::new_v4(),
            "limit": 3
        }))
        .unwrap();

        assert_eq!(query.lens_id, Some(lens_id));
        assert_eq!(query.space_id, None);
        assert!(query.namespace_id.is_some());
        assert_eq!(query.limit, Some(3));
    }

    #[test]
    fn lens_run_rejects_namespace_that_conflicts_with_lens_namespace() {
        let error = resolve_lens_namespace(Some(Uuid::new_v4()), Some(Uuid::new_v4())).unwrap_err();

        assert!(
            matches!(error, AppError::BadRequest(message) if message.contains("Lens namespace"))
        );
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
            namespace_id: Some(Uuid::new_v4()),
            feedback_loop_id: Some(Uuid::new_v4()),
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
        assert!(output["provenance"]["namespace_id"].is_string());
        assert!(output["provenance"]["feedback_loop_id"].is_string());
        assert_eq!(output["query"], "Summarize the project direction");
        assert_eq!(output["search_mode"], "semantic");
        assert_eq!(output["memory_count"], 1);
        assert_eq!(output["memories"][0]["id"], memory_id.to_string());
        assert_eq!(output["summary_provider"], "deterministic");
        assert_eq!(output["summary_source"], "deterministic");
        assert_eq!(output["key_points"].as_array().unwrap().len(), 1);
        assert_eq!(output["citations"][0]["memory_id"], memory_id.to_string());
        assert_eq!(
            output["unresolved_contradictions"]
                .as_array()
                .expect("Lens Run output should expose unresolved contradictions")
                .len(),
            0
        );
        assert!(!output["suggested_next_actions"]
            .as_array()
            .unwrap()
            .is_empty());
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
            namespace_id: None,
            feedback_loop_id: None,
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

    #[tokio::test]
    async fn deterministic_thought_review_lenses_return_user_facing_insights() {
        let lens_id = Uuid::new_v4();
        let memories = vec![search_item(Uuid::new_v4())];
        let output = build_lens_output(LensOutputInput {
            lens_id,
            lens_name: "工程视角",
            strategy: "engineering_review",
            output_format: "brief",
            retrieval_mode: "semantic",
            namespace_id: None,
            feedback_loop_id: None,
            query: "我最近项目很多，不知道哪个值得继续。",
            search_mode: "keyword",
            memories: &memories,
            summarizer: None,
            summary_provider: None,
            summary_model: None,
            summary_max_words: None,
        })
        .await;

        assert!(output["summary"].as_str().unwrap().contains("评估标准"));
        assert!(output["open_questions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|question| question.as_str().unwrap().contains("下一步")));
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
            namespace_id: None,
            feedback_loop_id: None,
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
