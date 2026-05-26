//! 应用状态管理
use sqlx::PgPool;
use std::sync::Arc;

use crate::ai::embedding::{Embedder, LocalHashEmbedder, OpenAIEmbedder};
use crate::ai::summary::{OpenAISummarizer, Summarizer};
use crate::ai::transcription::{OpenAITranscriptionProvider, TranscriptionProvider};
use crate::vector::repository::VectorRepository;
use crate::vector::VectorStore;

/// AI 配置
#[derive(Clone)]
pub struct AiConfig {
    pub openai_api_key: Option<String>,
    pub embedder: Option<Arc<dyn Embedder>>,
    pub summarizer: Option<Arc<dyn Summarizer>>,
    pub transcriber: Option<Arc<dyn TranscriptionProvider>>,
    pub summary_provider: Option<String>,
    pub summary_model: Option<String>,
    pub transcription_provider: Option<String>,
    pub transcription_model: Option<String>,
    pub transcription_config_error: Option<String>,
    pub summary_max_words: Option<usize>,
}

impl Default for AiConfig {
    fn default() -> Self {
        let local_embedding = std::env::var("MEMORYNEXUS_EMBEDDING_PROVIDER")
            .or_else(|_| std::env::var("EMBEDDING_PROVIDER"))
            .map(|provider| provider.eq_ignore_ascii_case("local"))
            .unwrap_or(false);
        let openai_api_key = non_empty_env("OPENAI_API_KEY");
        let embedder = if local_embedding {
            Some(Arc::new(LocalHashEmbedder::default()) as Arc<dyn Embedder>)
        } else {
            openai_api_key.clone().map(|key| {
                let model = std::env::var("OPENAI_EMBEDDING_MODEL")
                    .or_else(|_| std::env::var("EMBEDDING_MODEL"))
                    .unwrap_or_else(|_| "text-embedding-ada-002".to_string());
                Arc::new(OpenAIEmbedder::new(key).with_model(model)) as Arc<dyn Embedder>
            })
        };
        let summary_config =
            resolve_summary_config(&SummaryEnv::from_process(openai_api_key.clone()));
        let transcription_config =
            resolve_transcription_config(&TranscriptionEnv::from_process(openai_api_key.clone()));
        let summarizer = if summary_config.disabled {
            None
        } else {
            summary_config
                .api_key
                .clone()
                .zip(summary_config.model.clone())
                .map(|(key, model)| {
                    let summarizer = OpenAISummarizer::new(key).with_model(model);
                    let summarizer = if let Some(base_url) = summary_config.base_url.clone() {
                        summarizer.with_base_url(base_url)
                    } else {
                        summarizer
                    };
                    Arc::new(summarizer) as Arc<dyn Summarizer>
                })
        };
        let summary_provider = summarizer.as_ref().and(summary_config.provider);
        let transcriber = if transcription_config.disabled || transcription_config.error.is_some() {
            None
        } else {
            transcription_config
                .api_key
                .clone()
                .zip(transcription_config.model.clone())
                .map(|(key, model)| {
                    let provider = OpenAITranscriptionProvider::new(key).with_model(model);
                    let provider = if let Some(base_url) = transcription_config.base_url.clone() {
                        provider.with_base_url(base_url)
                    } else {
                        provider
                    };
                    Arc::new(provider) as Arc<dyn TranscriptionProvider>
                })
        };
        let transcription_provider = transcriber
            .as_ref()
            .and(transcription_config.provider.clone());

        Self {
            openai_api_key,
            embedder,
            summarizer,
            transcriber,
            summary_provider,
            summary_model: summary_config.model,
            transcription_provider,
            transcription_model: transcription_config.model,
            transcription_config_error: transcription_config.error,
            summary_max_words: summary_config.max_words,
        }
    }
}

struct SummaryEnv {
    summary_provider: Option<String>,
    summary_api_key: Option<String>,
    openai_api_key: Option<String>,
    openrouter_api_key: Option<String>,
    summary_model: Option<String>,
    openai_model: Option<String>,
    summary_base_url: Option<String>,
    openai_base_url: Option<String>,
    summary_max_words: Option<String>,
}

impl SummaryEnv {
    fn from_process(openai_api_key: Option<String>) -> Self {
        Self {
            summary_provider: non_empty_env("MEMORYNEXUS_SUMMARY_PROVIDER"),
            summary_api_key: non_empty_env("MEMORYNEXUS_SUMMARY_API_KEY"),
            openai_api_key,
            openrouter_api_key: non_empty_env("OPENROUTER_API_KEY"),
            summary_model: non_empty_env("MEMORYNEXUS_SUMMARY_MODEL"),
            openai_model: non_empty_env("OPENAI_MODEL"),
            summary_base_url: non_empty_env("MEMORYNEXUS_AI_BASE_URL"),
            openai_base_url: non_empty_env("OPENAI_BASE_URL"),
            summary_max_words: non_empty_env("LENS_RUN_SUMMARY_MAX_WORDS"),
        }
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

struct SummaryConfig {
    provider: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    max_words: Option<usize>,
    disabled: bool,
}

struct TranscriptionEnv {
    transcription_provider: Option<String>,
    transcription_api_key: Option<String>,
    openai_api_key: Option<String>,
    transcription_model: Option<String>,
    transcription_base_url: Option<String>,
    openai_base_url: Option<String>,
}

impl TranscriptionEnv {
    fn from_process(openai_api_key: Option<String>) -> Self {
        Self {
            transcription_provider: non_empty_env("MEMORYNEXUS_TRANSCRIPTION_PROVIDER"),
            transcription_api_key: non_empty_env("MEMORYNEXUS_TRANSCRIPTION_API_KEY"),
            openai_api_key,
            transcription_model: non_empty_env("MEMORYNEXUS_TRANSCRIPTION_MODEL")
                .or_else(|| non_empty_env("OPENAI_TRANSCRIPTION_MODEL")),
            transcription_base_url: non_empty_env("MEMORYNEXUS_TRANSCRIPTION_BASE_URL"),
            openai_base_url: non_empty_env("OPENAI_BASE_URL"),
        }
    }
}

struct TranscriptionConfig {
    provider: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    disabled: bool,
    error: Option<String>,
}

fn resolve_summary_config(env: &SummaryEnv) -> SummaryConfig {
    let provider = env
        .summary_provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .clone()
        .unwrap_or_else(|| infer_summary_provider(env))
        .to_lowercase();
    let disabled = matches!(provider.as_str(), "none" | "disabled" | "off");
    let api_key = clean_opt(env.summary_api_key.as_deref()).or_else(|| {
        if provider == "openrouter" {
            clean_opt(env.openrouter_api_key.as_deref())
                .or_else(|| clean_opt(env.openai_api_key.as_deref()))
        } else {
            clean_opt(env.openai_api_key.as_deref())
                .or_else(|| clean_opt(env.openrouter_api_key.as_deref()))
        }
    });
    let model = api_key.as_ref().and_then(|_| {
        clean_opt(env.summary_model.as_deref())
            .or_else(|| clean_opt(env.openai_model.as_deref()))
            .or_else(|| default_summary_model(&provider))
    });
    let base_url = clean_opt(env.summary_base_url.as_deref())
        .or_else(|| clean_opt(env.openai_base_url.as_deref()))
        .or_else(|| default_summary_base_url(&provider));
    let max_words = env
        .summary_max_words
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok());

    SummaryConfig {
        provider: Some(provider),
        api_key,
        model,
        base_url,
        max_words,
        disabled,
    }
}

fn infer_summary_provider(env: &SummaryEnv) -> String {
    if clean_opt(env.openrouter_api_key.as_deref()).is_some()
        && clean_opt(env.openai_api_key.as_deref()).is_none()
    {
        "openrouter".to_string()
    } else {
        "openai".to_string()
    }
}

fn clean_opt(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn default_summary_model(provider: &str) -> Option<String> {
    if provider == "openrouter" {
        Some("openrouter/free".to_string())
    } else {
        Some("gpt-3.5-turbo".to_string())
    }
}

fn default_summary_base_url(provider: &str) -> Option<String> {
    if provider == "openrouter" {
        Some("https://openrouter.ai/api/v1".to_string())
    } else {
        None
    }
}

fn resolve_transcription_config(env: &TranscriptionEnv) -> TranscriptionConfig {
    let provider = env
        .transcription_provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "openai".to_string())
        .to_lowercase();
    let disabled = matches!(provider.as_str(), "none" | "disabled" | "off");
    let error = match provider.as_str() {
        "openai" | "none" | "disabled" | "off" => None,
        unsupported => Some(format!(
            "unsupported transcription provider '{unsupported}'; supported providers are openai, none, disabled, off"
        )),
    };
    let api_key = clean_opt(env.transcription_api_key.as_deref())
        .or_else(|| clean_opt(env.openai_api_key.as_deref()));
    let model = api_key.as_ref().filter(|_| error.is_none()).map(|_| {
        clean_opt(env.transcription_model.as_deref()).unwrap_or_else(|| "whisper-1".to_string())
    });
    let base_url = clean_opt(env.transcription_base_url.as_deref())
        .or_else(|| clean_opt(env.openai_base_url.as_deref()));

    TranscriptionConfig {
        provider: Some(provider),
        api_key,
        model,
        base_url,
        disabled,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_config_infers_openrouter_from_openrouter_key() {
        let env = SummaryEnv {
            summary_provider: None,
            summary_api_key: None,
            openai_api_key: None,
            openrouter_api_key: Some("sk-or-test".to_string()),
            summary_model: None,
            openai_model: None,
            summary_base_url: None,
            openai_base_url: None,
            summary_max_words: None,
        };

        let config = resolve_summary_config(&env);

        assert_eq!(config.provider.as_deref(), Some("openrouter"));
        assert_eq!(config.api_key.as_deref(), Some("sk-or-test"));
        assert_eq!(config.model.as_deref(), Some("openrouter/free"));
        assert_eq!(
            config.base_url.as_deref(),
            Some("https://openrouter.ai/api/v1")
        );
    }

    #[test]
    fn summary_config_respects_explicit_provider() {
        let env = SummaryEnv {
            summary_provider: Some("openai".to_string()),
            summary_api_key: None,
            openai_api_key: None,
            openrouter_api_key: Some("sk-or-test".to_string()),
            summary_model: None,
            openai_model: None,
            summary_base_url: None,
            openai_base_url: None,
            summary_max_words: None,
        };

        let config = resolve_summary_config(&env);

        assert_eq!(config.provider.as_deref(), Some("openai"));
        assert_eq!(config.api_key.as_deref(), Some("sk-or-test"));
        assert_eq!(config.model.as_deref(), Some("gpt-3.5-turbo"));
        assert_eq!(config.base_url, None);
    }

    #[test]
    fn summary_config_ignores_empty_keys() {
        let env = SummaryEnv {
            summary_provider: None,
            summary_api_key: Some("".to_string()),
            openai_api_key: None,
            openrouter_api_key: Some("".to_string()),
            summary_model: None,
            openai_model: None,
            summary_base_url: None,
            openai_base_url: None,
            summary_max_words: None,
        };

        let config = resolve_summary_config(&env);

        assert_eq!(config.provider.as_deref(), Some("openai"));
        assert_eq!(config.api_key, None);
        assert_eq!(config.model, None);
    }

    #[test]
    fn transcription_config_requires_api_key_before_enabling_provider() {
        let env = TranscriptionEnv {
            transcription_provider: None,
            transcription_api_key: None,
            openai_api_key: None,
            transcription_model: None,
            transcription_base_url: None,
            openai_base_url: None,
        };

        let config = resolve_transcription_config(&env);

        assert_eq!(config.provider.as_deref(), Some("openai"));
        assert_eq!(config.api_key, None);
        assert_eq!(config.model, None);
    }

    #[test]
    fn transcription_config_defaults_to_whisper_when_key_exists() {
        let env = TranscriptionEnv {
            transcription_provider: None,
            transcription_api_key: Some("sk-test".to_string()),
            openai_api_key: None,
            transcription_model: None,
            transcription_base_url: None,
            openai_base_url: None,
        };

        let config = resolve_transcription_config(&env);

        assert_eq!(config.provider.as_deref(), Some("openai"));
        assert_eq!(config.api_key.as_deref(), Some("sk-test"));
        assert_eq!(config.model.as_deref(), Some("whisper-1"));
        assert_eq!(config.error, None);
    }

    #[test]
    fn transcription_config_rejects_unsupported_provider_names() {
        let env = TranscriptionEnv {
            transcription_provider: Some("opneai".to_string()),
            transcription_api_key: Some("sk-test".to_string()),
            openai_api_key: None,
            transcription_model: None,
            transcription_base_url: None,
            openai_base_url: None,
        };

        let config = resolve_transcription_config(&env);

        assert_eq!(config.provider.as_deref(), Some("opneai"));
        assert_eq!(config.api_key.as_deref(), Some("sk-test"));
        assert_eq!(config.model, None);
        assert!(config
            .error
            .as_deref()
            .is_some_and(|error| error.contains("unsupported transcription provider")));
    }

    #[test]
    fn transcription_config_rejects_local_provider_name() {
        let env = TranscriptionEnv {
            transcription_provider: Some("local".to_string()),
            transcription_api_key: None,
            openai_api_key: None,
            transcription_model: None,
            transcription_base_url: None,
            openai_base_url: None,
        };

        let config = resolve_transcription_config(&env);

        assert!(!config.disabled);
        assert!(config.error.is_some());
    }
}

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub repositories: Repositories,
    pub vector_store: Option<Arc<dyn VectorStore>>,
    pub ai: AiConfig,
}

/// 仓储聚合
#[derive(Clone)]
pub struct Repositories {
    pub lenses: Arc<dyn super::db::lens::LensRepository>,
    pub lens_runs: Arc<dyn super::db::lens_run::LensRunRepository>,
    pub memories: Arc<dyn super::db::memory::MemoryRepository>,
    pub profiles: Arc<dyn super::db::profile::CognitiveProfileRepository>,
    pub reminders: Arc<dyn super::db::reminder::ReminderRepository>,
    pub review_reports: Arc<dyn super::db::review_report::CognitiveReviewReportRepository>,
    pub spaces: Arc<dyn super::db::space::CognitiveSpaceRepository>,
    pub tags: Arc<dyn super::db::tag::TagRepository>,
    pub users: Arc<dyn super::db::user::UserRepository>,
    pub vectors: Arc<dyn VectorRepository>,
}

impl AppState {
    pub fn new(
        db: PgPool,
        repositories: Repositories,
        vector_store: Option<Arc<dyn VectorStore>>,
    ) -> Self {
        Self {
            db,
            repositories,
            vector_store,
            ai: AiConfig::default(),
        }
    }
}
