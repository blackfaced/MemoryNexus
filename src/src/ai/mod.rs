//! AI 模块
//!
//! 支持 AI 摘要生成、智能标签推荐
//! 使用 OpenAI GPT API 或兼容 API

pub mod embedding;
pub mod summary;

pub use embedding::{Embedder, EmbeddingError, EmbeddingResult, OpenAIEmbedder};
pub use summary::{OpenAISummarizer, Summarizer, SummaryOptions, SummaryResult, SummaryStyle};
