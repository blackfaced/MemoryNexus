//! AI 模块
//!
//! 支持 AI 摘要生成、智能标签推荐
//! 使用 OpenAI GPT API 或兼容 API

pub mod summary;
pub mod embedding;

pub use summary::{Summarizer, SummaryResult, SummaryOptions};
pub use embedding::{Embedder, EmbeddingResult};
