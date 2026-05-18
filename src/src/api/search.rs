//! 搜索 API

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::search::{SearchEngine, SearchQuery, SearchResult, SemanticSearchError};
use crate::state::AppState;

/// GET /api/v1/search - 搜索记忆
pub async fn search(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ApiResponse<SearchResult>>, AppError> {
    let engine = SearchEngine::with_vector_store(state.db.clone(), state.vector_store.clone());

    let result = if query.semantic {
        engine
            .semantic_search(&query, auth_user.user_id)
            .await
            .map_err(map_semantic_search_error)?
    } else {
        engine
            .search(&query, auth_user.user_id)
            .await
            .map_err(AppError::Database)?
    };

    Ok(Json(ApiResponse::success(result)))
}

fn map_semantic_search_error(error: SemanticSearchError) -> AppError {
    match error {
        SemanticSearchError::EmptyQuery => {
            AppError::BadRequest("语义搜索需要提供 q 参数".to_string())
        }
        SemanticSearchError::VectorStoreMissing => {
            AppError::BadRequest("Qdrant 向量存储未配置".to_string())
        }
        SemanticSearchError::EmbeddingKeyMissing => {
            AppError::BadRequest("OPENAI_API_KEY 未配置".to_string())
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

    let suggestions = engine
        .suggest(&params.q, auth_user.user_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(SuggestResponse { suggestions })))
}

/// 搜索建议查询
#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub q: String,
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
}
