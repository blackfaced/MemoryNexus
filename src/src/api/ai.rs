//! AI API 端点

use axum::{
    Json, extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ai::{Summarizer, OpenAISummarizer, SummaryOptions, SummaryStyle};
use crate::auth::AuthenticatedUser;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

/// 生成摘要请求
#[derive(Debug, Deserialize)]
pub struct SummarizeRequest {
    pub content: String,
    
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
    pub confidence: f32,
}

/// 获取 AI 配置（仅管理员）
#[derive(Debug, Serialize)]
pub struct AiConfigResponse {
    pub model: String,
    pub embedding_model: String,
    pub enabled: bool,
}

/// POST /api/v1/ai/summarize - 生成摘要
pub async fn summarize(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<SummarizeRequest>,
) -> Result<Json<ApiResponse<SummarizeResponse>>, AppError> {
    // 获取 API Key
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| AppError::BadRequest("AI 功能未配置".to_string()))?;
    
    // 创建摘要器
    let summarizer = OpenAISummarizer::new(api_key);
    
    // 使用提供的选项或默认值
    let options = req.options.unwrap_or_default();
    
    // 生成摘要
    let result = summarizer.summarize(&req.content, &options)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    Ok(Json(ApiResponse::success(SummarizeResponse {
        summary: result.summary,
        keywords: result.keywords,
        language: result.language,
        original_length: result.original_length,
        summary_length: result.summary_length,
        processing_time_ms: result.processing_time_ms,
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
    let memory = state.repositories.memories
        .find_by_id(id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Memory {} not found", id)))?;
    
    // 验证权限
    if memory.user_id != auth_user.user_id {
        return Err(AppError::Unauthorized);
    }
    
    // 获取 API Key
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| AppError::BadRequest("AI 功能未配置".to_string()))?;
    
    // 创建摘要器
    let summarizer = OpenAISummarizer::new(api_key);
    
    // 生成摘要
    let options = req.options.unwrap_or_default();
    let result = summarizer.summarize(&memory.content, &options)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 更新记忆记录（如果用户想要保存）
    // 这里只是返回结果，实际保存由客户端决定
    
    Ok(Json(ApiResponse::success(SummarizeResponse {
        summary: result.summary,
        keywords: result.keywords,
        language: result.language,
        original_length: result.original_length,
        summary_length: result.summary_length,
        processing_time_ms: result.processing_time_ms,
    })))
}

/// POST /api/v1/ai/autotag - 智能标签推荐
pub async fn auto_tag(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<AutoTagRequest>,
) -> Result<Json<ApiResponse<AutoTagResponse>>, AppError> {
    // 获取 API Key
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| AppError::BadRequest("AI 功能未配置".to_string()))?;
    
    // 创建摘要器
    let summarizer = OpenAISummarizer::new(api_key);
    
    // 构建标签推荐 prompt
    let prompt = format!(
        r#"请分析以下内容，推荐3-5个最相关的标签。
只返回标签名称，用逗号分隔，不要包含其他说明。
标签应该简洁，通常是1-2个词。

内容:
{}"#,
        req.content
    );
    
    let options = SummaryOptions {
        max_words: 20,
        language: "zh".to_string(),
        include_keywords: true,
        style: SummaryStyle::Concise,
    };
    
    let result = summarizer.summarize(&prompt, &options)
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 解析标签（从摘要中提取）
    let tags: Vec<String> = result.summary
        .split(|c: char| !c.is_alphanumeric() && c != '，' && c != ',' && c != '、')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() <= 10) // 过滤太长的词
        .take(5)
        .collect();
    
    Ok(Json(ApiResponse::success(AutoTagResponse {
        suggested_tags: tags,
        confidence: 0.8, // 简化实现，实际可计算置信度
    })))
}

/// GET /api/v1/ai/config - 获取 AI 配置
pub async fn get_config() -> Result<Json<ApiResponse<AiConfigResponse>>, AppError> {
    let enabled = std::env::var("OPENAI_API_KEY").is_ok();
    
    Ok(Json(ApiResponse::success(AiConfigResponse {
        model: std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
        embedding_model: std::env::var("OPENAI_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-ada-002".to_string()),
        enabled,
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
    fn test_summarize_response_serde() {
        let response = SummarizeResponse {
            summary: "测试摘要".to_string(),
            keywords: vec!["测试".to_string(), "摘要".to_string()],
            language: "zh".to_string(),
            original_length: 100,
            summary_length: 50,
            processing_time_ms: 100,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("测试摘要"));
        assert!(json.contains("\"keywords\":"));
    }

    #[test]
    fn test_auto_tag_request_serde() {
        let json = r#"{"content":"这是一段关于旅行的内容"}"#;
        let req: AutoTagRequest = serde_json::from_str(json).unwrap();
        assert!(req.content.contains("旅行"));
    }

    #[test]
    fn test_ai_config_response_serde() {
        let config = AiConfigResponse {
            model: "gpt-4".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            enabled: true,
        };
        
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("\"enabled\":true"));
    }
}
