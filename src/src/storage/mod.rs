//! 存储模块
//!
//! 支持本地存储和 S3 兼容存储（MinIO, AWS S3）
//!
//! 使用策略模式，支持：
//! - LocalStorage: 本地文件系统（开发环境）
//! - S3Storage: S3 兼容对象存储（生产环境）

pub mod s3;
pub mod thumbnail;

pub use s3::{Storage, StorageConfig, StorageError};
pub use thumbnail::{ThumbnailGenerator, ThumbnailSize};

/// 默认存储桶
pub const DEFAULT_BUCKET: &str = "memorynexus";

/// 默认缩略图桶
pub const THUMBNAIL_BUCKET: &str = "thumbnails";
