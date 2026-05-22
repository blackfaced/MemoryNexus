//! 搜索 API

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::lens::LensDb;
use crate::error::{ApiResponse, AppError};
use crate::search::{
    SearchEngine, SearchLensProvenance, SearchQuery, SearchResult, SemanticSearchError,
};
use crate::state::AppState;

/// GET /api/v1/search - 搜索记忆
pub async fn search(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ApiResponse<SearchResult>>, AppError> {
    let engine = SearchEngine::with_semantic_dependencies(
        state.db.clone(),
        state.vector_store.clone(),
        state.ai.embedder.clone(),
    );
    let context = resolve_search_context(&state, auth_user.user_id, &query).await?;
    let mut effective_query = query.clone();
    effective_query.space_id = Some(context.space_id);
    if context.use_semantic {
        effective_query.semantic = true;
    }

    let mut result = if effective_query.semantic {
        engine
            .semantic_search(&effective_query, auth_user.user_id, context.space_id)
            .await
            .map_err(map_semantic_search_error)?
    } else {
        engine
            .search(&effective_query, auth_user.user_id, context.space_id)
            .await
            .map_err(AppError::Database)?
    };
    result.lens = context.lens;

    Ok(Json(ApiResponse::success(result)))
}

struct SearchContext {
    space_id: Uuid,
    use_semantic: bool,
    lens: Option<SearchLensProvenance>,
}

async fn resolve_search_context(
    state: &AppState,
    user_id: Uuid,
    query: &SearchQuery,
) -> Result<SearchContext, AppError> {
    if let Some(lens_id) = query.lens_id {
        let lens = state
            .repositories
            .lenses
            .find_for_user(lens_id, user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;

        if let Some(space_id) = query.space_id {
            if space_id != lens.space_id {
                return Err(AppError::BadRequest(
                    "space_id must match the Lens Cognitive Space".to_string(),
                ));
            }
        }

        return Ok(SearchContext {
            space_id: lens.space_id,
            use_semantic: lens.retrieval_mode == "semantic",
            lens: Some(search_lens_provenance(lens)),
        });
    }

    let space = resolve_space(state, user_id, query.space_id).await?;

    Ok(SearchContext {
        space_id: space.id,
        use_semantic: query.semantic,
        lens: None,
    })
}

fn search_lens_provenance(lens: LensDb) -> SearchLensProvenance {
    SearchLensProvenance {
        id: lens.id,
        space_id: lens.space_id,
        name: lens.name,
        strategy: lens.strategy,
        output_format: lens.output_format,
        retrieval_mode: lens.retrieval_mode,
    }
}

async fn resolve_space(
    state: &AppState,
    user_id: uuid::Uuid,
    requested_space_id: Option<uuid::Uuid>,
) -> Result<crate::db::space::CognitiveSpaceDb, AppError> {
    if let Some(space_id) = requested_space_id {
        return state
            .repositories
            .spaces
            .find_for_user(space_id, user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized);
    }

    state
        .repositories
        .spaces
        .default_for_user(user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Cognitive space not found".to_string()))
}

fn map_semantic_search_error(error: SemanticSearchError) -> AppError {
    match error {
        SemanticSearchError::EmptyQuery => {
            AppError::BadRequest("语义搜索需要提供 q 参数".to_string())
        }
        SemanticSearchError::VectorStoreMissing => {
            AppError::BadRequest("Qdrant 向量存储未配置".to_string())
        }
        SemanticSearchError::EmbeddingProviderMissing => {
            AppError::BadRequest("Embedding provider 未配置".to_string())
        }
        SemanticSearchError::Database(error) => AppError::Database(error),
        SemanticSearchError::Embedding(error) => AppError::Internal(error.to_string()),
        SemanticSearchError::Vector(error) => AppError::Internal(error.to_string()),
    }
}

/// GET /api/v1/search/suggest - 搜索建议
pub async fn suggest(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(params): Query<SuggestQuery>,
) -> Result<Json<ApiResponse<SuggestResponse>>, AppError> {
    let engine = SearchEngine::new(state.db.clone());
    let space = resolve_space(&state, auth_user.user_id, params.space_id).await?;

    let suggestions = engine
        .suggest(&params.q, auth_user.user_id, space.id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(SuggestResponse { suggestions })))
}

/// 搜索建议查询
#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub q: String,
    pub space_id: Option<uuid::Uuid>,
}

/// 搜索建议响应
#[derive(serde::Serialize)]
pub struct SuggestResponse {
    pub suggestions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_query_serde() {
        let json = r#"{"q":"旅"}"#;
        let query: SuggestQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, "旅");
        assert_eq!(query.space_id, None);
    }

    #[test]
    fn test_suggest_response_serde() {
        let response = SuggestResponse {
            suggestions: vec!["旅行回忆".to_string(), "旅途".to_string()],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("旅行回忆"));
    }

    #[test]
    fn test_semantic_search_missing_query_maps_to_bad_request() {
        let error = map_semantic_search_error(SemanticSearchError::EmptyQuery);
        assert!(matches!(error, AppError::BadRequest(_)));
    }

    #[test]
    fn search_lens_provenance_keeps_strategy_and_space() {
        let lens = LensDb {
            id: uuid::Uuid::new_v4(),
            space_id: uuid::Uuid::new_v4(),
            name: "Project Context".to_string(),
            description: None,
            strategy: "project_context".to_string(),
            output_format: "brief".to_string(),
            retrieval_mode: "semantic".to_string(),
            created_by: uuid::Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let provenance = search_lens_provenance(lens.clone());

        assert_eq!(provenance.id, lens.id);
        assert_eq!(provenance.space_id, lens.space_id);
        assert_eq!(provenance.strategy, "project_context");
        assert_eq!(provenance.retrieval_mode, "semantic");
    }
}
