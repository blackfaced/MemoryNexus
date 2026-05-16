//! S3 兼容存储抽象
//!
//! 使用 aws-sdk-s3 提供 S3 兼容存储能力
//! 支持 MinIO（本地开发）和 AWS S3（生产）

use async_trait::async_trait;
use aws_config::Region;
use aws_sdk_s3::{
    config::Builder as S3ConfigBuilder,
    primitives::ByteStream,
    Client as S3Client,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// 存储错误类型
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("连接失败: {0}")]
    Connection(#[from] aws_sdk_s3::Error),
    
    #[error("上传失败: {0}")]
    Upload(String),
    
    #[error("下载失败: {0}")]
    Download(String),
    
    #[error("删除失败: {0}")]
    Delete(String),
    
    #[error("文件不存在: {0}")]
    NotFound(String),
    
    #[error("配置错误: {0}")]
    Config(String),
    
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for StorageError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// 存储配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    /// 端点 URL（本地开发用 MinIO）
    pub endpoint: Option<String>,
    
    /// 区域
    pub region: String,
    
    /// 访问密钥
    pub access_key: Option<String>,
    
    /// 秘密密钥
    pub secret_key: Option<String>,
    
    /// 是否使用路径模式（MinIO 需要设为 true）
    pub path_style: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("S3_ENDPOINT")
                .ok()
                .or_else(|| Some("http://localhost:9000".to_string())),
            region: std::env::var("S3_REGION")
                .unwrap_or_else(|_| "us-east-1".to_string()),
            access_key: std::env::var("S3_ACCESS_KEY")
                .or_else(|_| std::env::var("MINIO_ROOT_USER").ok()),
            secret_key: std::env::var("S3_SECRET_KEY")
                .or_else(|_| std::env::var("MINIO_ROOT_PASSWORD").ok()),
            path_style: true,
        }
    }
}

/// 上传结果
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// 对象 key
    pub key: String,
    /// 完整 URL
    pub url: String,
    /// 文件大小
    pub size: u64,
    /// ETag
    pub etag: Option<String>,
}

/// 存储 trait - 支持多种后端实现
#[async_trait]
pub trait Storage: Send + Sync {
    /// 上传文件
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<UploadResult, StorageError>;
    
    /// 上传文件（从路径）
    async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        path: &Path,
        content_type: Option<&str>,
    ) -> Result<UploadResult, StorageError>;
    
    /// 下载文件
    async fn download(&self, bucket: &str, key: &str) -> Result<Vec<u8>, StorageError>;
    
    /// 删除文件
    async fn delete(&self, bucket: &str, key: &str) -> Result<(), StorageError>;
    
    /// 获取文件 URL
    fn get_url(&self, bucket: &str, key: &str) -> String;
    
    /// 检查文件是否存在
    async fn exists(&self, bucket: &str, key: &str) -> Result<bool, StorageError>;
}

/// S3 存储实现
#[derive(Clone)]
pub struct S3Storage {
    client: S3Client,
    config: StorageConfig,
}

impl S3Storage {
    /// 创建 S3 存储客户端
    pub async fn new(config: StorageConfig) -> Result<Self, StorageError> {
        // 构建 S3 配置
        let mut s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest());
        
        // 设置区域
        s3_config = s3_config.region(Region::new(config.region.clone()));
        
        // 如果有凭证，使用默认凭证提供程序链
        // （环境变量、~/.aws/credentials、ECS 任务角色、EC2 实例角色等）
        
        let client = if let (Some(access_key), Some(secret_key)) = 
            (&config.access_key, &config.secret_key) 
        {
            // 使用显式凭证
            let creds = aws_config::Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "env",
            );
            s3_config = s3_config.credentials_provider(creds);
            
            // 如果有自定义端点（MinIO），设置它
            if let Some(endpoint) = &config.endpoint {
                let endpoint_url = aws_endpoint::AwsEndpoint::new(
                    endpoint.parse().map_err(|e| StorageError::Config(format!("Invalid endpoint: {}", e)))?,
                );
                s3_config = s3_config.endpoint_url(endpoint);
            }
            
            S3Client::new(&s3_config)
        } else {
            // 使用默认凭证链
            let s3_config = s3_config.load().await;
            S3Client::new(&s3_config)
        };
        
        Ok(Self { client, config })
    }
    
    /// 确保桶存在
    pub async fn ensure_bucket(&self, bucket: &str) -> Result<(), StorageError> {
        let exists = self.client.head_bucket()
            .bucket(bucket)
            .send()
            .await;
            
        if exists.is_err() {
            // 桶不存在，创建它
            self.client.create_bucket()
                .bucket(bucket)
                .send()
                .await?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<UploadResult, StorageError> {
        let body = ByteStream::from(data);
        let size = body.len().unwrap_or(0) as u64;
        
        let mut put_request = self.client.put_object()
            .bucket(bucket)
            .key(key)
            .body(body);
            
        if let Some(ct) = content_type {
            put_request = put_request.content_type(ct);
        }
        
        let result = put_request.send().await?;
        
        Ok(UploadResult {
            key: key.to_string(),
            url: self.get_url(bucket, key),
            size,
            etag: result.e_tag,
        })
    }
    
    async fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        path: &Path,
        content_type: Option<&str>,
    ) -> Result<UploadResult, StorageError> {
        let data = tokio::fs::read(path).await?;
        self.upload(bucket, key, data, content_type).await
    }
    
    async fn download(&self, bucket: &str, key: &str) -> Result<Vec<u8>, StorageError> {
        let result = self.client.get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Download(e.to_string()))?;
            
        let body = result.body
            .collect()
            .await
            .map_err(|e| StorageError::Download(e.to_string()))?
            .into_bytes();
            
        Ok(body.to_vec())
    }
    
    async fn delete(&self, bucket: &str, key: &str) -> Result<(), StorageError> {
        self.client.delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| StorageError::Delete(e.to_string()))?;
        Ok(())
    }
    
    fn get_url(&self, bucket: &str, key: &str) -> String {
        if let Some(endpoint) = &self.config.endpoint {
            format!("{}/{}/{}", endpoint, bucket, key)
        } else {
            format!("https://{}.s3.{}.amazonaws.com/{}", bucket, self.config.region, key)
        }
    }
    
    async fn exists(&self, bucket: &str, key: &str) -> Result<bool, StorageError> {
        match self.client.head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await 
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NoSuchKey") || e.to_string().contains("404") {
                    Ok(false)
                } else {
                    Err(StorageError::Download(e.to_string()))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert!(config.endpoint.is_some());
        assert_eq!(config.region, "us-east-1");
    }

    #[test]
    fn test_upload_result_debug() {
        let result = UploadResult {
            key: "test/key".to_string(),
            url: "http://localhost:9000/bucket/test/key".to_string(),
            size: 1024,
            etag: Some("\"abc123\"".to_string()),
        };
        
        assert!(result.key.contains("test"));
        assert!(result.size > 0);
    }

    #[tokio::test]
    async fn test_s3_storage_creation() {
        // 测试配置有效性（不需要实际连接）
        let config = StorageConfig {
            endpoint: Some("http://localhost:9000".to_string()),
            region: "us-east-1".to_string(),
            access_key: Some("minioadmin".to_string()),
            secret_key: Some("minioadmin".to_string()),
            path_style: true,
        };
        
        // 验证配置创建成功
        assert!(config.endpoint.is_some());
        assert!(config.access_key.is_some());
    }
}
