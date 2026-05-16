//! 搜索模块
//!
//! 支持全文搜索、标签过滤、日期范围筛选
//! 使用 PostgreSQL 全文搜索 + pg_trgm 扩展

pub mod query;
pub mod filter;

pub use query::{SearchQuery, SearchResult, SearchEngine};
pub use filter::{MemoryFilter, DateRange, SortOrder};
