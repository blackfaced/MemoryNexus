//! 记忆 API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ai::{Embedder, OpenAIEmbedder};
use crate::auth::AuthenticatedUser;
use crate::db::memory::{CreateMemory, MemoryDb, MemoryType};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;
use crate::vector::{MemoryVectorPayload, MemoryVectorPoint};

/// 记忆类型枚举（API 层）
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApiMemoryType {
    /// 文本
    #[default]
    Text,
    /// 图片
    Image,
    /// 音频
    Audio,
    /// 视频
    Video,
}

impl From<ApiMemoryType> for MemoryType {
    fn from(t: ApiMemoryType) -> Self {
        match t {
            ApiMemoryType::Text => MemoryType::Text,
            ApiMemoryType::Image => MemoryType::Image,
            ApiMemoryType::Audio => MemoryType::Audio,
            ApiMemoryType::Video => MemoryType::Video,
        }
    }
}

impl From<MemoryType> for ApiMemoryType {
    fn from(t: MemoryType) -> Self {
        match t {
            MemoryType::Text => ApiMemoryType::Text,
            MemoryType::Image => ApiMemoryType::Image,
            MemoryType::Audio => ApiMemoryType::Audio,
            MemoryType::Video => ApiMemoryType::Video,
        }
    }
}

/// 创建记忆请求
#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub space_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub memory_type: ApiMemoryType,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_shared: bool,
}

/// 更新记忆请求
#[derive(Debug, Deserialize)]
pub struct UpdateMemoryRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub memory_type: Option<ApiMemoryType>,
    #[allow(dead_code)]
    pub tags: Option<Vec<String>>,
    #[allow(dead_code)]
    pub is_shared: Option<bool>,
}

/// 列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub space_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[allow(dead_code)]
    pub tag: Option<String>,
    #[allow(dead_code)]
    pub memory_type: Option<ApiMemoryType>,
}

fn default_limit() -> i64 {
    20
}

/// 记忆列表响应
#[derive(Serialize)]
pub struct MemoryListResponse {
    pub items: Vec<MemoryDb>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// GET /api/v1/memories - 列出记忆
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<MemoryListResponse>>, AppError> {
    let space = resolve_space(&state, auth_user.user_id, params.space_id).await?;

    let memories = state
        .repositories
        .memories
        .list_by_space(auth_user.user_id, space.id, params.limit, params.offset)
        .await
        .map_err(AppError::Database)?;

    let total = state
        .repositories
        .memories
        .count_by_space(auth_user.user_id, space.id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(MemoryListResponse {
        items: memories,
        total,
        limit: params.limit,
        offset: params.offset,
    })))
}

/// POST /api/v1/memories - 创建记忆
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<ApiResponse<MemoryDb>>), AppError> {
    // 验证输入
    if req.content.trim().is_empty() {
        return Err(AppError::BadRequest("内容不能为空".to_string()));
    }

    let content = req.content;
    let title = req.title;
    let memory_type: MemoryType = req.memory_type.into();
    let space = resolve_space(&state, auth_user.user_id, req.space_id).await?;

    let create_memory = CreateMemory {
        user_id: auth_user.user_id,
        space_id: space.id,
        title: title.clone(),
        content: content.clone(),
        memory_type,
        file_path: None,
        is_shared: req.is_shared,
        tags: req.tags,
    };

    let memory = state
        .repositories
        .memories
        .create(create_memory)
        .await
        .map_err(AppError::Database)?;

    index_memory_embedding(&state, &memory).await;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(memory))))
}

async fn resolve_space(
    state: &AppState,
    user_id: Uuid,
    requested_space_id: Option<Uuid>,
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

async fn index_memory_embedding(state: &AppState, memory: &MemoryDb) {
    let Some(vector_store) = state.vector_store.as_ref() else {
        return;
    };

    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        tracing::warn!("跳过记忆向量索引：OPENAI_API_KEY 未配置");
        return;
    };

    let model = std::env::var("OPENAI_EMBEDDING_MODEL")
        .or_else(|_| std::env::var("EMBEDDING_MODEL"))
        .unwrap_or_else(|_| "text-embedding-ada-002".to_string());
    let embedder = OpenAIEmbedder::new(api_key).with_model(model);

    let embedding = match embedder.embed(&memory.content).await {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(?error, memory_id = %memory.id, "生成记忆 embedding 失败");
            return;
        }
    };

    let point = MemoryVectorPoint {
        id: memory.id,
        vector: embedding.embedding,
        payload: MemoryVectorPayload {
            memory_id: memory.id,
            user_id: memory.user_id,
            space_id: memory.space_id,
            title: memory.title.clone(),
            memory_type: memory.memory_type.clone(),
            is_shared: memory.is_shared,
        },
    };

    if let Err(error) = vector_store.upsert_memory(point).await {
        tracing::warn!(?error, memory_id = %memory.id, "写入 Qdrant 失败");
    }
}

/// GET /api/v1/memories/:id - 获取单个记忆
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MemoryDb>>, AppError> {
    let memory = state
        .repositories
        .memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;

    // 检查权限（自己的或共享的）
    if memory.user_id != auth_user.user_id && !memory.is_shared {
        return Err(AppError::Unauthorized);
    }

    Ok(Json(ApiResponse::success(memory)))
}

/// PATCH /api/v1/memories/:id - 更新记忆
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMemoryRequest>,
) -> Result<Json<ApiResponse<MemoryDb>>, AppError> {
    // 获取现有记忆
    let existing = state
        .repositories
        .memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;

    // 检查权限
    if existing.user_id != auth_user.user_id {
        return Err(AppError::Unauthorized);
    }

    // 更新字段
    let memory = state
        .repositories
        .memories
        .update(
            id,
            &req.content,
            &req.title,
            req.memory_type.map(|t| t.into()),
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(memory)))
}

/// DELETE /api/v1/memories/:id - 删除记忆
pub async fn delete(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 获取现有记忆检查权限
    let existing = state
        .repositories
        .memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;

    if existing.user_id != auth_user.user_id {
        return Err(AppError::Unauthorized);
    }

    let deleted = state
        .repositories
        .memories
        .delete(id)
        .await
        .map_err(AppError::Database)?;

    if !deleted {
        return Err(AppError::NotFound(format!("Memory {} not found", id)));
    }

    Ok(Json(ApiResponse::success(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_memory_type_default() {
        assert_eq!(ApiMemoryType::default(), ApiMemoryType::Text);
    }

    #[test]
    fn test_api_memory_type_to_db_type() {
        assert_eq!(MemoryType::Text, ApiMemoryType::Text.into());
        assert_eq!(MemoryType::Image, ApiMemoryType::Image.into());
        assert_eq!(MemoryType::Audio, ApiMemoryType::Audio.into());
        assert_eq!(MemoryType::Video, ApiMemoryType::Video.into());
    }

    #[test]
    fn test_create_memory_request_serde() {
        let json = r#"{"content":"test","memory_type":"text"}"#;
        let req: CreateMemoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.space_id, None);
        assert_eq!(req.content, "test");
        assert_eq!(req.memory_type, ApiMemoryType::Text);
    }

    #[test]
    fn test_update_memory_request_serde() {
        let json = r#"{"title":"New Title"}"#;
        let req: UpdateMemoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, Some("New Title".to_string()));
        assert_eq!(req.content, None);
    }

    #[test]
    fn test_list_query_defaults() {
        let json = r#"{}"#;
        let query: ListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.space_id, None);
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 0);
    }
}
