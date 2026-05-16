# ADR-003: 向量数据库选择（Qdrant 本地/云端）

## 状态
✅ 已接受

## 背景
需要向量存储支持语义搜索，同时需要兼容本地开发和云端部署。

## 决策

### 选择：Qdrant + 抽象层

```rust
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, collection: &str, points: Vec<Point>) -> Result<()>;
    async fn search(&self, collection: &str, query: Vec<f32>, top_k: usize) -> Result<Vec<SearchResult>>;
    async fn delete(&self, collection: &str, id: &str) -> Result<()>;
}
```

### 实现方案

| 后端 | 实现 | 适用场景 |
|------|------|----------|
| Qdrant (本地) | `qdrant-client` | Docker 一键启动 |
| Qdrant Cloud | `qdrant-client` | 云端托管 |
| Milvus | `milvus-sdk` | 大规模部署 |

### 配置示例

```yaml
vector:
  backend: "qdrant"
  
qdrant:
  url: "http://localhost:6333"
  # 本地模式无需 API key
  
qdrant_cloud:
  url: "https://xxx.qdrant.cloud"
  api_key: "${QDRANT_API_KEY}"
```

### Qdrant vs 其他选型对比

| 特性 | Qdrant | Milvus | Pinecone |
|------|--------|--------|----------|
| 本地部署 | ✅ | ✅ | ❌ |
| 开源 | ✅ | ✅ | ❌ |
| 免费额度 | 1M 向量 | 开源免费 | 100K 向量 |
| 性能 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| 复杂度 | 低 | 中 | 低 |

## 后果

**正面：**
- Qdrant 性能优秀，支持 HNSW/量化
- 轻量级，易于部署
- 纯 Rust 实现，与后端栈一致

**负面：**
- 相比 Milvus 生态稍小
- 超大规模场景可能需 Milvus

## 相关决策
- ADR-001: Rust 后端选择
