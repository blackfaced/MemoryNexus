//! 向量嵌入模块
//!
//! 支持文本向量化，用于相似记忆搜索
//! 使用 OpenAI Embeddings API 或本地模型

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 嵌入错误
#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("API 调用失败: {0}")]
    ApiError(String),

    #[error("内容为空")]
    EmptyContent,

    #[error("API 密钥未配置")]
    ApiKeyMissing,
}

/// 嵌入维度
pub const EMBEDDING_DIM: usize = 1536; // OpenAI text-embedding-ada-002

/// 嵌入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// 向量
    pub embedding: Vec<f32>,

    /// 模型
    pub model: String,

    /// token 数量
    pub tokens: usize,
}

/// 向量嵌入 trait
#[async_trait::async_trait]
pub trait Embedder: Send + Sync {
    /// 生成单个嵌入
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError>;

    /// 批量生成嵌入
    async fn embed_batch(&self, texts: Vec<String>)
        -> Result<Vec<EmbeddingResult>, EmbeddingError>;
}

/// OpenAI 嵌入实现
pub struct OpenAIEmbedder {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

/// Deterministic local embedder for development and smoke tests.
///
/// This is not intended to match production embedding quality. It makes the
/// Embedding -> Qdrant -> search path testable without an external API key.
pub struct LocalHashEmbedder {
    dimensions: usize,
}

impl Default for LocalHashEmbedder {
    fn default() -> Self {
        Self {
            dimensions: EMBEDDING_DIM,
        }
    }
}

impl LocalHashEmbedder {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl OpenAIEmbedder {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "text-embedding-ada-002".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait::async_trait]
impl Embedder for LocalHashEmbedder {
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError> {
        if text.trim().is_empty() {
            return Err(EmbeddingError::EmptyContent);
        }

        let mut embedding = vec![0.0; self.dimensions];
        for token in text.split_whitespace().map(str::to_lowercase) {
            let index = stable_hash(&token) % self.dimensions;
            embedding[index] += 1.0;
        }

        let norm = embedding
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        if norm > 0.0 {
            for value in &mut embedding {
                *value /= norm;
            }
        }

        Ok(EmbeddingResult {
            embedding,
            model: "local-hash".to_string(),
            tokens: text.split_whitespace().count(),
        })
    }

    async fn embed_batch(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<EmbeddingResult>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(&text).await?);
        }
        Ok(results)
    }
}

fn stable_hash(text: &str) -> usize {
    let mut hash = 14_695_981_039_346_656_037u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash as usize
}

#[async_trait::async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError> {
        if text.trim().is_empty() {
            return Err(EmbeddingError::EmptyContent);
        }

        let request_body = serde_json::json!({
            "model": self.model,
            "input": text,
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::ApiError(error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        let embedding: Vec<f32> = response_json["data"][0]["embedding"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();

        let tokens = response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as usize;

        Ok(EmbeddingResult {
            embedding,
            model: self.model.clone(),
            tokens,
        })
    }

    async fn embed_batch(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<EmbeddingResult>, EmbeddingError> {
        let mut results = Vec::with_capacity(texts.len());

        // OpenAI API 每次最多 2048 个输入
        let batch_size = 100;

        for chunk in texts.chunks(batch_size) {
            let request_body = serde_json::json!({
                "model": self.model,
                "input": chunk,
            });

            let response = self
                .client
                .post("https://api.openai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(EmbeddingError::ApiError(error_text));
            }

            let response_json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

            for item in response_json["data"].as_array().unwrap() {
                let embedding: Vec<f32> = item["embedding"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap() as f32)
                    .collect();

                results.push(EmbeddingResult {
                    embedding,
                    model: self.model.clone(),
                    tokens: 0,
                });
            }
        }

        Ok(results)
    }
}

/// 余弦相似度计算
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b)).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_openai_embedder_creation() {
        let embedder = OpenAIEmbedder::new("test-key".to_string());
        assert_eq!(embedder.model, "text-embedding-ada-002");
    }

    #[test]
    fn test_openai_embedder_with_model() {
        let embedder = OpenAIEmbedder::new("test-key".to_string())
            .with_model("text-embedding-3-small".to_string());
        assert_eq!(embedder.model, "text-embedding-3-small");
    }

    #[test]
    fn test_embedding_result_serde() {
        let result = EmbeddingResult {
            embedding: vec![0.1, 0.2, 0.3],
            model: "test-model".to_string(),
            tokens: 10,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"model\":\"test-model\""));
        assert!(json.contains("\"tokens\":10"));
    }

    #[tokio::test]
    async fn local_hash_embedder_is_deterministic() {
        let embedder = LocalHashEmbedder::new(8);
        let first = embedder.embed("cognitive space memory").await.unwrap();
        let second = embedder.embed("cognitive space memory").await.unwrap();

        assert_eq!(first.embedding, second.embedding);
        assert_eq!(first.embedding.len(), 8);
        assert_eq!(first.model, "local-hash");
    }
}
