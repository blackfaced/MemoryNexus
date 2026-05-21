//! Lens Run API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

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
    let memories = result.items.iter().map(memory_output).collect();
    let output = build_lens_output(LensOutputInput {
        lens_id: lens.id,
        lens_name: &lens.name,
        strategy: &lens.strategy,
        output_format: &lens.output_format,
        retrieval_mode: &lens.retrieval_mode,
        query,
        search_mode: &result.search_mode,
        memories,
    });

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
    memories: Vec<Value>,
}

fn build_lens_output(input: LensOutputInput<'_>) -> Value {
    let summary = deterministic_summary(
        input.lens_name,
        input.strategy,
        input.query,
        input.memories.len(),
    );
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
        "memory_count": input.memories.len(),
        "memories": input.memories,
        "summary": summary,
    })
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
    use serde_json::json;
    use uuid::Uuid;

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

    #[test]
    fn lens_run_output_keeps_traceable_shape() {
        let lens_id = Uuid::new_v4();
        let memory_id = Uuid::new_v4();
        let output = build_lens_output(LensOutputInput {
            lens_id,
            lens_name: "Project Context",
            strategy: "project_context",
            output_format: "brief",
            retrieval_mode: "semantic",
            query: "Summarize the project direction",
            search_mode: "semantic",
            memories: vec![json!({
                "id": memory_id,
                "title": "Direction",
                "content": "MemoryNexus is Rust-first.",
                "memory_type": "text",
                "relevance": 0.9,
            })],
        });

        assert_eq!(output["lens"]["id"], lens_id.to_string());
        assert_eq!(output["query"], "Summarize the project direction");
        assert_eq!(output["search_mode"], "semantic");
        assert_eq!(output["memory_count"], 1);
        assert_eq!(output["memories"][0]["id"], memory_id.to_string());
        assert!(output["summary"]
            .as_str()
            .unwrap()
            .contains("Project Context"));
    }
}
