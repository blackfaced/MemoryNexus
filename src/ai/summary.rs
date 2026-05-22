//! AI 摘要生成模块

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 摘要错误
#[derive(Error, Debug)]
pub enum SummaryError {
    #[error("API 调用失败: {0}")]
    ApiError(String),

    #[error("内容过长: {0}")]
    ContentTooLong(String),

    #[error("API 密钥未配置")]
    ApiKeyMissing,

    #[error("模型不支持")]
    ModelNotSupported,
}

/// 摘要选项
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SummaryOptions {
    /// 摘要长度 (words)
    #[serde(default = "default_summary_length")]
    pub max_words: usize,

    /// 语言
    #[serde(default = "default_language")]
    pub language: String,

    /// 是否包含关键词
    #[serde(default = "default_include_keywords")]
    pub include_keywords: bool,

    /// 摘要风格
    #[serde(default)]
    pub style: SummaryStyle,
}

impl Default for SummaryOptions {
    fn default() -> Self {
        Self {
            max_words: default_summary_length(),
            language: default_language(),
            include_keywords: default_include_keywords(),
            style: SummaryStyle::default(),
        }
    }
}

fn default_summary_length() -> usize {
    50
}
fn default_language() -> String {
    "zh".to_string()
}
fn default_include_keywords() -> bool {
    true
}

/// 摘要风格
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SummaryStyle {
    /// 简洁摘要
    #[default]
    Concise,
    /// 详细摘要
    Detailed,
    /// 要点列表
    BulletPoints,
    /// 问答形式
    QnA,
}

impl std::fmt::Display for SummaryStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Concise => write!(f, "concise"),
            Self::Detailed => write!(f, "detailed"),
            Self::BulletPoints => write!(f, "bullet_points"),
            Self::QnA => write!(f, "qna"),
        }
    }
}

/// 摘要结果
#[derive(Debug, Clone, Serialize)]
pub struct SummaryResult {
    /// 摘要文本
    pub summary: String,

    /// 提取的关键词
    pub keywords: Vec<String>,

    /// 使用的语言
    pub language: String,

    /// 原始内容长度
    pub original_length: usize,

    /// 摘要长度
    pub summary_length: usize,

    /// 处理耗时 (ms)
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SmartTagSuggestion {
    pub tag: String,
    pub category: String,
}

/// 摘要器 trait
#[async_trait::async_trait]
pub trait Summarizer: Send + Sync {
    /// 生成摘要
    async fn summarize(
        &self,
        content: &str,
        options: &SummaryOptions,
    ) -> Result<SummaryResult, SummaryError>;

    /// 批量摘要
    async fn summarize_batch(
        &self,
        contents: Vec<String>,
        options: &SummaryOptions,
    ) -> Result<Vec<SummaryResult>, SummaryError>;
}

pub fn deterministic_summarize(content: &str, options: &SummaryOptions) -> SummaryResult {
    let start = std::time::Instant::now();
    let summary = build_deterministic_summary(content, options);
    let keywords = if options.include_keywords {
        extract_keywords_simple(content)
    } else {
        vec![]
    };

    SummaryResult {
        summary_length: summary.len(),
        summary,
        keywords,
        language: options.language.clone(),
        original_length: content.len(),
        processing_time_ms: start.elapsed().as_millis() as u64,
    }
}

pub fn suggest_smart_tags(content: &str) -> Vec<SmartTagSuggestion> {
    let normalized = content.to_lowercase();
    let mut suggestions = Vec::new();

    let rules = [
        (
            "rust",
            "technology",
            ["rust", "cargo", "borrow", "ownership", "trait", "axum"].as_slice(),
        ),
        (
            "cognitive-lens",
            "cognition",
            ["lens", "cognitive", "reflection", "belief", "contradiction"].as_slice(),
        ),
        (
            "memory-space",
            "architecture",
            ["memory", "space", "cognitivespace", "substrate"].as_slice(),
        ),
        (
            "risk",
            "review",
            ["risk", "contradiction", "uncertain", "failure", "blocked"].as_slice(),
        ),
        (
            "learning",
            "personal-growth",
            ["learn", "learning", "practice", "study", "复盘"].as_slice(),
        ),
        (
            "family",
            "personal",
            ["family", "孩子", "家庭", "parent", "child"].as_slice(),
        ),
        (
            "project",
            "work",
            ["project", "roadmap", "phase", "mvp", "todo"].as_slice(),
        ),
    ];

    for (tag, category, needles) in rules {
        if needles.iter().any(|needle| normalized.contains(needle)) {
            suggestions.push(SmartTagSuggestion {
                tag: tag.to_string(),
                category: category.to_string(),
            });
        }
    }

    for keyword in extract_keywords_simple(content) {
        if suggestions.len() >= 5 {
            break;
        }
        if suggestions
            .iter()
            .all(|suggestion| suggestion.tag != keyword)
        {
            suggestions.push(SmartTagSuggestion {
                tag: keyword,
                category: "keyword".to_string(),
            });
        }
    }

    suggestions.truncate(5);
    suggestions
}

/// OpenAI 摘要器实现
pub struct OpenAISummarizer {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAISummarizer {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "gpt-3.5-turbo".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = normalize_openai_base_url(&base_url);
        self
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    fn build_prompt(&self, content: &str, options: &SummaryOptions) -> String {
        let style_instruction = match options.style {
            SummaryStyle::Concise => "简洁明了地概括要点",
            SummaryStyle::Detailed => "详细地描述内容要点",
            SummaryStyle::BulletPoints => "用要点列表形式呈现",
            SummaryStyle::QnA => "用问答题形式呈现关键信息",
        };

        format!(
            r#"请用{}语言{},限制在{}个字以内。
如果需要包含关键词，请以"关键词:"开头列出。
如果需要包含要点，请以"要点:"开头列出。

内容:
{}"#,
            options.language,
            style_instruction,
            options.max_words * 2, // 中文字符估算
            content
        )
    }
}

fn build_deterministic_summary(content: &str, options: &SummaryOptions) -> String {
    let limit = options.max_words.max(20) * 8;
    let mut sentences = content
        .split(['.', '。', '!', '！', '?', '？', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .take(match options.style {
            SummaryStyle::Detailed => 5,
            SummaryStyle::BulletPoints => 5,
            SummaryStyle::QnA => 3,
            SummaryStyle::Concise => 3,
        })
        .collect::<Vec<_>>();

    if sentences.is_empty() {
        sentences.push(content.trim());
    }

    let mut summary = match options.style {
        SummaryStyle::BulletPoints => sentences
            .iter()
            .map(|sentence| format!("- {sentence}"))
            .collect::<Vec<_>>()
            .join("\n"),
        SummaryStyle::QnA => format!(
            "Q: What is the key information?\nA: {}",
            sentences.join(" ")
        ),
        _ => sentences.join(" "),
    };

    if summary.len() > limit {
        summary.truncate(limit);
        summary = summary.trim_end().to_string();
    }

    summary
}

#[async_trait::async_trait]
impl Summarizer for OpenAISummarizer {
    async fn summarize(
        &self,
        content: &str,
        options: &SummaryOptions,
    ) -> Result<SummaryResult, SummaryError> {
        let start = std::time::Instant::now();

        // 检查内容长度
        if content.len() > 10000 {
            return Err(SummaryError::ContentTooLong(
                "内容超过10000字符限制".to_string(),
            ));
        }

        let prompt = self.build_prompt(content, options);

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "你是一个专业的文本摘要助手。"},
                {"role": "user", "content": prompt}
            ],
            "max_tokens": 500,
            "temperature": 0.3,
        });

        let response = self
            .client
            .post(self.chat_completions_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SummaryError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SummaryError::ApiError(error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SummaryError::ApiError(e.to_string()))?;

        let summary = extract_chat_completion_text(&response_json);

        // 提取关键词（简单实现）
        let keywords = if options.include_keywords {
            extract_keywords_simple(&summary)
        } else {
            vec![]
        };

        let summary_length = summary.len();

        Ok(SummaryResult {
            summary,
            keywords,
            language: options.language.clone(),
            original_length: content.len(),
            summary_length,
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn summarize_batch(
        &self,
        contents: Vec<String>,
        options: &SummaryOptions,
    ) -> Result<Vec<SummaryResult>, SummaryError> {
        let mut results = Vec::with_capacity(contents.len());

        for content in contents {
            let result = self.summarize(&content, options).await?;
            results.push(result);
        }

        Ok(results)
    }
}

fn extract_chat_completion_text(response_json: &serde_json::Value) -> String {
    let choice = &response_json["choices"][0];
    let message = &choice["message"];
    let content = &message["content"];

    if let Some(text) = content.as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(parts) = content.as_array() {
        let text = parts
            .iter()
            .filter_map(|part| {
                part.as_str()
                    .or_else(|| part["text"].as_str())
                    .or_else(|| part["content"].as_str())
            })
            .collect::<Vec<_>>()
            .join("");
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    for fallback in [&message["reasoning"], &message["refusal"], &choice["text"]] {
        if let Some(text) = fallback.as_str() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    String::new()
}

fn normalize_openai_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.trim_end_matches("/chat/completions").to_string()
    } else {
        trimmed.to_string()
    }
}

/// 简单的关键词提取（基于词频）
fn extract_keywords_simple(text: &str) -> Vec<String> {
    // 停用词列表
    let stop_words = vec![
        "的", "了", "是", "在", "我", "有", "和", "就", "不", "人", "都", "一", "一个", "上", "也",
        "很", "到", "说", "要", "去", "你", "会", "着", "没有", "看", "好", "自己", "这", "那",
        "他", "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
        "had", "do", "does", "did", "will", "would", "could", "should", "the", "and", "or", "but",
        "if", "then", "of", "for", "in", "on", "to",
    ];

    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.len() > 1 && !stop_words.contains(&w.as_str()))
        .collect();

    // 简单词频统计
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for word in &words {
        *freq.entry(word.clone()).or_insert(0) += 1;
    }

    // 返回前5个高频词
    let mut sorted: Vec<_> = freq.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

    sorted.into_iter().take(5).map(|(w, _)| w).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_options_default() {
        let options = SummaryOptions::default();
        assert_eq!(options.max_words, 50);
        assert_eq!(options.language, "zh");
        assert!(options.include_keywords);
    }

    #[test]
    fn test_summary_style_display() {
        assert_eq!(SummaryStyle::Concise.to_string(), "concise");
        assert_eq!(SummaryStyle::BulletPoints.to_string(), "bullet_points");
    }

    #[test]
    fn test_extract_keywords_simple() {
        let text = "这是一个测试文本，测试关键词提取功能。测试关键词，测试。";
        let keywords = extract_keywords_simple(text);

        // 应该能提取到"测试"和"关键词"
        assert!(keywords.contains(&"测试".to_string()) || keywords.contains(&"关键词".to_string()));
    }

    #[test]
    fn test_openai_summarizer_creation() {
        let summarizer = OpenAISummarizer::new("test-key".to_string());
        assert_eq!(summarizer.model, "gpt-3.5-turbo");
        assert_eq!(summarizer.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_openai_summarizer_with_model() {
        let summarizer =
            OpenAISummarizer::new("test-key".to_string()).with_model("gpt-4".to_string());
        assert_eq!(summarizer.model, "gpt-4");
    }

    #[test]
    fn test_openai_summarizer_with_base_url() {
        let summarizer = OpenAISummarizer::new("test-key".to_string())
            .with_base_url("https://openrouter.ai/api/v1/".to_string());
        assert_eq!(summarizer.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(
            summarizer.chat_completions_url(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
    }

    #[test]
    fn test_openai_summarizer_normalizes_chat_completions_url() {
        let summarizer = OpenAISummarizer::new("test-key".to_string())
            .with_base_url("https://example.com/v1/chat/completions".to_string());
        assert_eq!(summarizer.base_url, "https://example.com/v1");
    }

    #[test]
    fn extracts_chat_completion_text_from_string_content() {
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "  real summary  "
                }
            }]
        });

        assert_eq!(extract_chat_completion_text(&response), "real summary");
    }

    #[test]
    fn extracts_chat_completion_text_from_content_parts() {
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": [
                        {"type": "text", "text": "part one "},
                        {"type": "text", "text": "part two"}
                    ]
                }
            }]
        });

        assert_eq!(extract_chat_completion_text(&response), "part one part two");
    }

    #[test]
    fn extracts_chat_completion_text_from_reasoning_fallback() {
        let response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "",
                    "reasoning": "reasoning text"
                }
            }]
        });

        assert_eq!(extract_chat_completion_text(&response), "reasoning text");
    }

    #[test]
    fn deterministic_summary_handles_long_content_without_provider() {
        let content = "Rust ownership and Cognitive Space memory. ".repeat(600);
        let options = SummaryOptions {
            max_words: 40,
            language: "en".to_string(),
            include_keywords: true,
            style: SummaryStyle::Concise,
        };

        let result = deterministic_summarize(&content, &options);

        assert_eq!(result.language, "en");
        assert_eq!(result.original_length, content.len());
        assert!(!result.summary.is_empty());
        assert!(result.summary.len() <= 320);
        assert!(result.keywords.iter().any(|keyword| keyword == "rust"));
    }

    #[test]
    fn deterministic_summary_supports_bullet_points() {
        let options = SummaryOptions {
            max_words: 80,
            language: "zh".to_string(),
            include_keywords: false,
            style: SummaryStyle::BulletPoints,
        };

        let result = deterministic_summarize("第一点。第二点。第三点。", &options);

        assert!(result.summary.starts_with("- 第一点"));
        assert!(result.keywords.is_empty());
    }

    #[test]
    fn smart_tags_include_categories_and_keywords() {
        let tags = suggest_smart_tags(
            "Rust project roadmap for Cognitive Space, Lens Run, and contradiction review.",
        );

        assert!(tags.iter().any(|tag| tag.tag == "rust"));
        assert!(tags.iter().any(|tag| tag.tag == "cognitive-lens"));
        assert!(tags.iter().any(|tag| tag.category == "architecture"));
        assert!(tags.len() <= 5);
    }
}
