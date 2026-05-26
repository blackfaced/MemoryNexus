//! AI 模块
//!
//! 支持 AI 摘要生成、智能标签推荐
//! 使用 OpenAI GPT API 或兼容 API

pub mod embedding;
pub mod summary;
pub mod transcription;

#[allow(unused_imports)]
pub use embedding::{Embedder, EmbeddingError, EmbeddingResult, OpenAIEmbedder};
#[allow(unused_imports)]
pub use summary::{
    deterministic_summarize, suggest_smart_tags, OpenAISummarizer, SmartTagSuggestion, Summarizer,
    SummaryOptions, SummaryResult, SummaryStyle,
};
