//! 搜索 API

use axum::{
    Json, extract::{Query, State},
};
use serde::Deserialize;

use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::search::{SearchEngine, SearchQuery, SearchResult};
use crate::state::AppState;

/// GET /api/v1/search - 搜索记忆
pub async fn search(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ApiResponse<SearchResult>>, AppError> {
    let engine = SearchEngine::new(state.db.clone());
    
    let result = engine.search(&query, auth_user.user_id)
        .await
        .map_err(AppError::Database)?;
    
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/search/suggest - 搜索建议
pub async fn suggest(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(params): Query<SuggestQuery>,
) -> Result<Json<ApiResponse<SuggestResponse>>, AppError> {
    let engine = SearchEngine::new(state.db.clone());
    
    let suggestions = engine.suggest(&params.q, auth_user.user_id)
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
            suggestions: vec![
                "旅行回忆".to_string(),
                "旅途".to_string(),
            ],
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("旅行回忆"));
    }
}
