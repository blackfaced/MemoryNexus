//! 搜索模块
//!
//! 支持全文搜索、标签过滤、日期范围筛选
//! 使用 PostgreSQL 全文搜索 + pg_trgm 扩展

pub mod filter;
pub mod query;

#[allow(unused_imports)]
pub use filter::{DateRange, MemoryFilter, SortOrder};
pub use query::{SearchEngine, SearchQuery, SearchResult, SemanticSearchError};
