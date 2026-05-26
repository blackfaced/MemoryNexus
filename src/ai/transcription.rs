//! Audio transcription providers.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AudioTranscriptionInput {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TranscriptionOptions {
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub provider: String,
    pub model: String,
    pub language: Option<String>,
    pub duration_seconds: Option<f64>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Error)]
pub enum TranscriptionProviderError {
    #[error("transcription API error: {0}")]
    Api(String),

    #[error("transcription provider returned empty text")]
    EmptyText,
}

#[async_trait::async_trait]
pub trait TranscriptionProvider: Send + Sync {
    async fn transcribe(
        &self,
        input: AudioTranscriptionInput,
        options: TranscriptionOptions,
    ) -> Result<TranscriptionResult, TranscriptionProviderError>;
}

pub struct OpenAITranscriptionProvider {
    api_key: String,
    model: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAITranscriptionProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "whisper-1".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url.trim_end_matches('/').to_string();
        self
    }
}

#[derive(Debug, Deserialize)]
struct OpenAITranscriptionResponse {
    text: String,
    language: Option<String>,
    duration: Option<f64>,
}

#[async_trait::async_trait]
impl TranscriptionProvider for OpenAITranscriptionProvider {
    async fn transcribe(
        &self,
        input: AudioTranscriptionInput,
        options: TranscriptionOptions,
    ) -> Result<TranscriptionResult, TranscriptionProviderError> {
        let filename = input
            .filename
            .clone()
            .unwrap_or_else(|| "voice-capture.webm".to_string());
        let mut file_part = reqwest::multipart::Part::bytes(input.bytes).file_name(filename);
        if let Some(content_type) = input.content_type.as_deref() {
            file_part = file_part
                .mime_str(content_type)
                .map_err(|error| TranscriptionProviderError::Api(error.to_string()))?;
        }

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "verbose_json");
        if let Some(language) = options.language.as_deref() {
            form = form.text("language", language.to_string());
        }

        let response = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|error| TranscriptionProviderError::Api(error.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| TranscriptionProviderError::Api(error.to_string()))?;
        if !status.is_success() {
            return Err(TranscriptionProviderError::Api(format!(
                "HTTP {status}: {body}"
            )));
        }

        let parsed: OpenAITranscriptionResponse = serde_json::from_str(&body)
            .map_err(|error| TranscriptionProviderError::Api(error.to_string()))?;
        let text = parsed.text.trim().to_string();
        if text.is_empty() {
            return Err(TranscriptionProviderError::EmptyText);
        }

        Ok(TranscriptionResult {
            text,
            provider: "openai".to_string(),
            model: self.model.clone(),
            language: parsed.language.or(options.language),
            duration_seconds: parsed.duration,
            metadata: json!({"response_format": "verbose_json"}),
        })
    }
}
