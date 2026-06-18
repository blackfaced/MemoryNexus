# ADR-002: 存储层抽象设计（S3/云存储）

## 状态
✅ 已接受；适用于 MemoryNexus 托管对象。外部媒体证据引用由 ADR-021 补充。

## 背景
需要支持本地开发（MinIO）和云端生产（AWS S3/阿里云 OSS 等），同时保持代码统一。

## 决策

### 适用范围

`StorageBackend` 只适用于 MemoryNexus 管理媒体字节的场景。外部媒体证据不需要复制到
S3/MinIO；`StorageBackend` 也不替代 provider-neutral `EvidenceRef` 或可选的
`EvidenceResolver`。外部引用及其失败、安全和权限语义见 ADR-021。

### 选择：Trait 抽象层

```rust
pub trait StorageBackend: Send + Sync {
    async fn upload(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<String>;
    async fn download(&self, key: &str) -> Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn get_url(&self, key: &str) -> Result<String>;
}
```

### 实现方案

| 后端 | 实现 | 适用场景 |
|------|------|----------|
| MinIO | `minio-rs` | 本地开发/Demo |
| AWS S3 | `aws-sdk-s3` | AWS 生产环境 |
| 阿里云 OSS | `oss-rs` | 阿里云 |
| 腾讯云 COS | `cos-rs` | 腾讯云 |

### 配置示例

```yaml
storage:
  backend: "minio"  # development
  # backend: "s3"    # production
  
minio:
  endpoint: "localhost:9000"
  access_key: "minioadmin"
  secret_key: "minioadmin"
  bucket: "memorynexus"
  use_ssl: false

s3:
  region: "ap-east-1"
  bucket: "memorynexus-prod"
```

## 后果

**正面：**
- 一套代码支持多种存储后端
- 开发/生产环境无缝切换
- 便于扩展新存储提供商

**负面：**
- 抽象层增加复杂度
- 需要维护多种 SDK 依赖

## 相关决策
- ADR-001: Rust 后端选择（依赖此抽象）
- [ADR-021: External Media Evidence References](ADR-021-external-media-evidence-references.md)
