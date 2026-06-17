//! 记忆 API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::feedback_loop::FeedbackLoopDb;
use crate::db::memory::{
    CreateMemory, MemoryDb, MemoryListFilter, MemoryListSort, MemoryType, UpdateMemory,
};
use crate::db::space::SpaceMemberRole;
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
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
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
    pub namespace_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub tag: Option<String>,
    pub memory_type: Option<ApiMemoryType>,
    #[serde(default)]
    pub sort: MemoryListSort,
}

fn default_limit() -> i64 {
    20
}

/// 记忆列表响应
#[derive(Serialize)]
pub struct MemoryListResponse {
    pub items: Vec<MemoryListItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// 记忆列表项响应，保留完整 Memory 字段并补充列表展示字段。
#[derive(Debug, Clone, Serialize)]
pub struct MemoryListItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: String,
    pub snippet: String,
    pub memory_type: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub is_shared: bool,
    pub source_type: String,
    pub source_metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

/// 记忆详情响应，保留完整 Memory 字段并补充 tags。
#[derive(Debug, Clone, Serialize)]
pub struct MemoryDetailResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub feedback_loop_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: String,
    pub memory_type: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub is_shared: bool,
    pub source_type: String,
    pub source_metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl MemoryDetailResponse {
    fn from_memory(memory: MemoryDb, tags: Vec<String>) -> Self {
        Self {
            id: memory.id,
            user_id: memory.user_id,
            space_id: memory.space_id,
            namespace_id: memory.namespace_id,
            feedback_loop_id: memory.feedback_loop_id,
            title: memory.title,
            content: memory.content,
            memory_type: memory.memory_type,
            file_path: memory.file_path,
            thumbnail_path: memory.thumbnail_path,
            is_shared: memory.is_shared,
            source_type: memory.source_type,
            source_metadata: memory.source_metadata,
            created_at: memory.created_at,
            updated_at: memory.updated_at,
            tags,
        }
    }
}

impl MemoryListItem {
    fn from_memory(memory: MemoryDb, tags: Vec<String>) -> Self {
        let snippet = memory_snippet(&memory.content);
        Self {
            id: memory.id,
            user_id: memory.user_id,
            space_id: memory.space_id,
            namespace_id: memory.namespace_id,
            feedback_loop_id: memory.feedback_loop_id,
            title: memory.title,
            content: memory.content,
            snippet,
            memory_type: memory.memory_type,
            file_path: memory.file_path,
            thumbnail_path: memory.thumbnail_path,
            is_shared: memory.is_shared,
            source_type: memory.source_type,
            source_metadata: memory.source_metadata,
            created_at: memory.created_at,
            updated_at: memory.updated_at,
            tags,
        }
    }
}

fn memory_snippet(content: &str) -> String {
    const MAX_SNIPPET_CHARS: usize = 180;
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= MAX_SNIPPET_CHARS {
        return compact;
    }

    format!(
        "{}...",
        compact.chars().take(MAX_SNIPPET_CHARS).collect::<String>()
    )
}

/// GET /api/v1/memories - 列出记忆
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<MemoryListResponse>>, AppError> {
    let space = resolve_space(&state, auth_user.user_id, params.space_id).await?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);
    let filter = MemoryListFilter {
        namespace_id: validate_namespace(&state, auth_user.user_id, space.id, params.namespace_id)
            .await?,
        tag: params.tag,
        memory_type: params.memory_type.map(Into::into),
        sort: params.sort,
    };

    let memories = state
        .repositories
        .memories
        .list_by_space(auth_user.user_id, space.id, limit, offset, filter.clone())
        .await
        .map_err(AppError::Database)?;

    let mut items = Vec::with_capacity(memories.len());
    for memory in memories {
        let tags = memory_tags(&state, memory.id).await?;
        items.push(MemoryListItem::from_memory(memory, tags));
    }

    let total = state
        .repositories
        .memories
        .count_by_space(auth_user.user_id, space.id, filter)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(MemoryListResponse {
        items,
        total,
        limit,
        offset,
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
    require_space_writer(&state, space.id, auth_user.user_id).await?;
    let provenance = validate_provenance(
        &state,
        auth_user.user_id,
        space.id,
        req.namespace_id,
        req.feedback_loop_id,
    )
    .await?;

    let create_memory = CreateMemory {
        user_id: auth_user.user_id,
        space_id: space.id,
        namespace_id: provenance.namespace_id,
        feedback_loop_id: provenance.feedback_loop_id,
        title: title.clone(),
        content: content.clone(),
        memory_type,
        file_path: None,
        is_shared: req.is_shared,
        source_type: "manual".to_string(),
        source_metadata: serde_json::json!({}),
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

async fn memory_tags(state: &AppState, memory_id: Uuid) -> Result<Vec<String>, AppError> {
    state
        .repositories
        .tags
        .list_memory_tags(memory_id)
        .await
        .map_err(AppError::Database)
        .map(|tags| tags.into_iter().map(|tag| tag.name).collect())
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

pub(crate) async fn index_memory_embedding(state: &AppState, memory: &MemoryDb) {
    let Some(vector_store) = state.vector_store.as_ref() else {
        return;
    };

    let Some(embedder) = state.ai.embedder.as_ref() else {
        tracing::warn!("跳过记忆向量索引：embedding provider 未配置");
        return;
    };

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
            namespace_id: memory.namespace_id,
            source_type: "memory".to_string(),
            created_at: memory.created_at.to_rfc3339(),
            visibility: if memory.is_shared {
                "shared".to_string()
            } else {
                "private".to_string()
            },
            title: memory.title.clone(),
            memory_type: memory.memory_type.clone(),
            is_shared: memory.is_shared,
        },
    };

    if let Err(error) = vector_store.upsert_memory(point).await {
        tracing::warn!(?error, memory_id = %memory.id, "写入 Qdrant 失败");
    }
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
    let namespace_id = validate_feedback_loop_scope(space_id, namespace_id, &feedback_loop)?;

    Ok(ProvenanceScope {
        namespace_id: Some(namespace_id),
        feedback_loop_id: Some(feedback_loop_id),
    })
}

fn validate_feedback_loop_scope(
    space_id: Uuid,
    namespace_id: Option<Uuid>,
    feedback_loop: &FeedbackLoopDb,
) -> Result<Uuid, AppError> {
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
    Ok(feedback_loop.namespace_id)
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

/// GET /api/v1/memories/:id - 获取单个记忆
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<MemoryDetailResponse>>, AppError> {
    let memory = state
        .repositories
        .memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;

    // Memory visibility is scoped by Cognitive Space membership.
    if memory.user_id != auth_user.user_id {
        require_space_member(&state, memory.space_id, auth_user.user_id).await?;
    }

    let tags = memory_tags(&state, memory.id).await?;

    Ok(Json(ApiResponse::success(
        MemoryDetailResponse::from_memory(memory, tags),
    )))
}

/// PATCH /api/v1/memories/:id - 更新记忆
pub async fn update(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMemoryRequest>,
) -> Result<Json<ApiResponse<MemoryDetailResponse>>, AppError> {
    if req
        .content
        .as_ref()
        .is_some_and(|content| content.trim().is_empty())
    {
        return Err(AppError::BadRequest("内容不能为空".to_string()));
    }

    // 获取现有记忆
    let existing = state
        .repositories
        .memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;

    require_memory_writer(&state, &existing, auth_user.user_id).await?;

    // 更新字段
    let memory = state
        .repositories
        .memories
        .update(
            id,
            UpdateMemory {
                title: req.title,
                content: req.content,
                memory_type: req.memory_type.map(|t| t.into()),
                is_shared: req.is_shared,
                tags: req.tags,
            },
        )
        .await
        .map_err(AppError::Database)?;

    index_memory_embedding(&state, &memory).await;

    let tags = memory_tags(&state, memory.id).await?;

    Ok(Json(ApiResponse::success(
        MemoryDetailResponse::from_memory(memory, tags),
    )))
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

    require_memory_writer(&state, &existing, auth_user.user_id).await?;

    let deleted = state
        .repositories
        .memories
        .delete(id)
        .await
        .map_err(AppError::Database)?;

    if !deleted {
        return Err(AppError::NotFound(format!("Memory {} not found", id)));
    }

    delete_memory_embedding(&state, id).await;

    Ok(Json(ApiResponse::success(())))
}

async fn delete_memory_embedding(state: &AppState, memory_id: Uuid) {
    let Some(vector_store) = state.vector_store.as_ref() else {
        return;
    };

    if let Err(error) = vector_store.delete_memory(memory_id).await {
        tracing::warn!(?error, %memory_id, "删除 Qdrant 记忆向量失败");
    }
}

async fn require_space_member(
    state: &AppState,
    space_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    state
        .repositories
        .spaces
        .find_member(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .map(|_| ())
        .ok_or(AppError::Unauthorized)
}

async fn require_space_writer(
    state: &AppState,
    space_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let member = state
        .repositories
        .spaces
        .find_member(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if member.parsed_role().is_some_and(SpaceMemberRole::can_write) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

async fn require_memory_writer(
    state: &AppState,
    memory: &MemoryDb,
    user_id: Uuid,
) -> Result<(), AppError> {
    if memory.user_id == user_id {
        return Ok(());
    }

    let member = state
        .repositories
        .spaces
        .find_member(memory.space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    if member.parsed_role() == Some(SpaceMemberRole::Owner) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feedback_loop(space_id: Uuid, namespace_id: Uuid) -> FeedbackLoopDb {
        FeedbackLoopDb {
            id: Uuid::new_v4(),
            space_id,
            namespace_id,
            goal: "Improve fraction word problems".to_string(),
            task: "Solve one practice problem".to_string(),
            attempt: None,
            evaluation: None,
            feedback: None,
            adjustment: None,
            next_task: None,
            status: "active".to_string(),
            created_by: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

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
    fn create_memory_request_accepts_namespace_and_feedback_loop_provenance() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let feedback_loop_id = Uuid::new_v4();
        let json = serde_json::json!({
            "space_id": space_id,
            "namespace_id": namespace_id,
            "feedback_loop_id": feedback_loop_id,
            "content": "Practice snapshot",
            "memory_type": "text"
        });

        let req: CreateMemoryRequest = serde_json::from_value(json).unwrap();

        assert_eq!(req.space_id, Some(space_id));
        assert_eq!(req.namespace_id, Some(namespace_id));
        assert_eq!(req.feedback_loop_id, Some(feedback_loop_id));
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
        assert_eq!(query.namespace_id, None);
        assert_eq!(query.limit, 20);
        assert_eq!(query.offset, 0);
    }

    #[test]
    fn list_query_accepts_namespace_filter() {
        let namespace_id = Uuid::new_v4();
        let query: ListQuery = serde_json::from_value(serde_json::json!({
            "namespace_id": namespace_id,
            "limit": 10
        }))
        .unwrap();

        assert_eq!(query.namespace_id, Some(namespace_id));
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn feedback_loop_provenance_rejects_cross_space_loop() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let loop_from_other_space = feedback_loop(Uuid::new_v4(), namespace_id);

        let error =
            validate_feedback_loop_scope(space_id, Some(namespace_id), &loop_from_other_space)
                .unwrap_err();

        assert!(
            matches!(error, AppError::BadRequest(message) if message.contains("Cognitive Space"))
        );
    }

    #[test]
    fn feedback_loop_provenance_rejects_namespace_mismatch() {
        let space_id = Uuid::new_v4();
        let loop_namespace_id = Uuid::new_v4();
        let requested_namespace_id = Uuid::new_v4();
        let feedback_loop = feedback_loop(space_id, loop_namespace_id);

        let error =
            validate_feedback_loop_scope(space_id, Some(requested_namespace_id), &feedback_loop)
                .unwrap_err();

        assert!(matches!(error, AppError::BadRequest(message) if message.contains("namespace_id")));
    }

    #[test]
    fn feedback_loop_provenance_infers_namespace_when_omitted() {
        let space_id = Uuid::new_v4();
        let namespace_id = Uuid::new_v4();
        let feedback_loop = feedback_loop(space_id, namespace_id);

        let inferred = validate_feedback_loop_scope(space_id, None, &feedback_loop).unwrap();

        assert_eq!(inferred, namespace_id);
    }

    #[test]
    fn list_query_accepts_filters_and_oldest_sort() {
        let json = r#"{"tag":"thought-review","memory_type":"text","sort":"oldest"}"#;
        let query: ListQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.tag.as_deref(), Some("thought-review"));
        assert_eq!(query.memory_type, Some(ApiMemoryType::Text));
        assert_eq!(query.sort, MemoryListSort::Oldest);
    }

    #[test]
    fn memory_list_item_uses_title_snippet_and_tags() {
        let memory = MemoryDb {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: None,
            feedback_loop_id: None,
            title: Some("A saved thought".to_string()),
            content: "This thought has enough content to become a short snippet.".to_string(),
            memory_type: "text".to_string(),
            file_path: None,
            thumbnail_path: None,
            is_shared: false,
            source_type: "manual".to_string(),
            source_metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let item = MemoryListItem::from_memory(memory, vec!["thought-review".to_string()]);

        assert_eq!(item.title.as_deref(), Some("A saved thought"));
        assert_eq!(
            item.snippet,
            "This thought has enough content to become a short snippet."
        );
        assert_eq!(item.memory_type, "text");
        assert_eq!(item.tags, vec!["thought-review".to_string()]);
    }

    #[test]
    fn memory_detail_response_includes_full_fields_and_tags() {
        let memory = MemoryDb {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            namespace_id: None,
            feedback_loop_id: None,
            title: Some("Detail".to_string()),
            content: "Full saved thought".to_string(),
            memory_type: "text".to_string(),
            file_path: None,
            thumbnail_path: None,
            is_shared: false,
            source_type: "manual".to_string(),
            source_metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let memory_id = memory.id;

        let detail = MemoryDetailResponse::from_memory(
            memory,
            vec!["thought-review".to_string(), "project".to_string()],
        );

        assert_eq!(detail.id, memory_id);
        assert_eq!(detail.title.as_deref(), Some("Detail"));
        assert_eq!(detail.content, "Full saved thought");
        assert_eq!(
            detail.tags,
            vec!["thought-review".to_string(), "project".to_string()]
        );
    }
}
