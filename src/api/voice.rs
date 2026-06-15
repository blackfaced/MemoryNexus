//! Voice capture API

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::ai::transcription::{
    AudioTranscriptionInput, TranscriptionOptions, TranscriptionProvider, TranscriptionResult,
};
use crate::auth::AuthenticatedUser;
use crate::db::memory::{CreateMemory, MemoryDb, MemoryRepository, MemoryType};
use crate::db::space::{CognitiveSpaceRepository, SpaceMemberRole};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

const VOICE_CAPTURE_SOURCE_TYPE: &str = "voice_transcription";

#[derive(Debug, Deserialize)]
pub struct VoiceCaptureQuery {
    pub space_id: Uuid,
    pub filename: Option<String>,
    pub title: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug)]
pub(crate) struct VoiceCaptureInput {
    pub space_id: Uuid,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub title: Option<String>,
    pub language: Option<String>,
    pub audio: Vec<u8>,
}

pub(crate) struct VoiceCaptureDeps<'a> {
    pub memories: &'a dyn MemoryRepository,
    pub spaces: &'a dyn CognitiveSpaceRepository,
    pub transcriber: Option<&'a dyn TranscriptionProvider>,
    pub transcription_config_error: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct VoiceCaptureResponse {
    pub memory: MemoryDb,
    pub transcription: TranscriptionResult,
}

pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<VoiceCaptureQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<ApiResponse<VoiceCaptureResponse>>), AppError> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let response = create_voice_capture_memory(
        VoiceCaptureDeps {
            memories: state.repositories.memories.as_ref(),
            spaces: state.repositories.spaces.as_ref(),
            transcriber: state.ai.transcriber.as_deref(),
            transcription_config_error: state.ai.transcription_config_error.as_deref(),
        },
        auth_user.user_id,
        VoiceCaptureInput {
            space_id: query.space_id,
            filename: query.filename,
            content_type,
            title: query.title,
            language: query.language,
            audio: body.to_vec(),
        },
    )
    .await?;

    crate::api::memories::index_memory_embedding(&state, &response.memory).await;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response))))
}

pub(crate) async fn create_voice_capture_memory(
    deps: VoiceCaptureDeps<'_>,
    user_id: Uuid,
    input: VoiceCaptureInput,
) -> Result<VoiceCaptureResponse, AppError> {
    if input.audio.is_empty() {
        return Err(AppError::BadRequest(
            "audio upload cannot be empty".to_string(),
        ));
    }

    let transcriber = deps.transcriber.ok_or_else(|| {
        AppError::BadRequest(deps.transcription_config_error.map(str::to_string).unwrap_or_else(
            || {
                "transcription provider is not configured; set MEMORYNEXUS_TRANSCRIPTION_PROVIDER and OPENAI_API_KEY"
                    .to_string()
            },
        ))
    })?;

    deps.spaces
        .find_for_user(input.space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;
    let member = deps
        .spaces
        .find_member(input.space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;
    if !member.parsed_role().is_some_and(SpaceMemberRole::can_write) {
        return Err(AppError::Unauthorized);
    }

    let audio_size_bytes = input.audio.len();
    let transcription = transcriber
        .transcribe(
            AudioTranscriptionInput {
                bytes: input.audio,
                filename: input.filename.clone(),
                content_type: input.content_type.clone(),
            },
            TranscriptionOptions {
                language: input.language.clone(),
            },
        )
        .await
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    if transcription.text.trim().is_empty() {
        return Err(AppError::BadRequest(
            "transcription provider returned empty text".to_string(),
        ));
    }

    let source_metadata = voice_source_metadata(
        &transcription,
        input.filename.as_deref(),
        input.content_type.as_deref(),
        audio_size_bytes,
    );
    let memory = deps
        .memories
        .create(CreateMemory {
            user_id,
            space_id: input.space_id,
            namespace_id: None,
            feedback_loop_id: None,
            title: input.title,
            content: transcription.text.trim().to_string(),
            memory_type: MemoryType::Audio,
            file_path: None,
            is_shared: false,
            source_type: VOICE_CAPTURE_SOURCE_TYPE.to_string(),
            source_metadata,
            tags: vec!["voice-capture".to_string()],
        })
        .await
        .map_err(AppError::Database)?;

    Ok(VoiceCaptureResponse {
        memory,
        transcription,
    })
}

fn voice_source_metadata(
    transcription: &TranscriptionResult,
    filename: Option<&str>,
    content_type: Option<&str>,
    audio_size_bytes: usize,
) -> Value {
    json!({
        "provider": transcription.provider,
        "model": transcription.model,
        "language": transcription.language,
        "duration_seconds": transcription.duration_seconds,
        "filename": filename,
        "content_type": content_type,
        "audio_size_bytes": audio_size_bytes,
        "transcription": transcription.metadata,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::json;
    use sqlx::Error;
    use uuid::Uuid;

    use super::*;
    use crate::ai::transcription::{
        AudioTranscriptionInput, TranscriptionOptions, TranscriptionProvider,
        TranscriptionProviderError, TranscriptionResult,
    };
    use crate::db::memory::{
        CreateMemory, MemoryDb, MemoryListFilter, MemoryRepository, MemoryType, UpdateMemory,
    };
    use crate::db::space::{
        CognitiveSpaceDb, CognitiveSpaceMemberDb, CognitiveSpaceRepository, CognitiveSpaceType,
        CreateCognitiveSpace, CreateSpaceInvite, SpaceMemberRole,
    };
    use crate::error::AppError;

    #[tokio::test]
    async fn voice_capture_returns_visible_error_when_provider_is_missing() {
        let user_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let spaces = FakeSpaces::writer(space_id, user_id);
        let memories = FakeMemories::default();

        let result = create_voice_capture_memory(
            VoiceCaptureDeps {
                memories: &memories,
                spaces: &spaces,
                transcriber: None,
                transcription_config_error: None,
            },
            user_id,
            VoiceCaptureInput {
                space_id,
                filename: Some("thought.webm".to_string()),
                content_type: Some("audio/webm".to_string()),
                title: None,
                language: Some("zh".to_string()),
                audio: b"audio bytes".to_vec(),
            },
        )
        .await;

        assert!(
            matches!(result, Err(AppError::BadRequest(message)) if message.contains("transcription provider"))
        );
        assert!(memories.created.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn voice_capture_returns_visible_error_for_unsupported_provider() {
        let user_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let spaces = FakeSpaces::writer(space_id, user_id);
        let memories = FakeMemories::default();

        let result = create_voice_capture_memory(
            VoiceCaptureDeps {
                memories: &memories,
                spaces: &spaces,
                transcriber: None,
                transcription_config_error: Some(
                    "unsupported transcription provider 'local'; supported providers are openai, none, disabled, off",
                ),
            },
            user_id,
            VoiceCaptureInput {
                space_id,
                filename: Some("thought.webm".to_string()),
                content_type: Some("audio/webm".to_string()),
                title: None,
                language: Some("zh".to_string()),
                audio: b"audio bytes".to_vec(),
            },
        )
        .await;

        assert!(
            matches!(result, Err(AppError::BadRequest(message)) if message.contains("unsupported transcription provider 'local'"))
        );
        assert!(memories.created.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn voice_capture_requires_space_write_permission_before_transcribing() {
        let user_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let spaces = FakeSpaces::viewer(space_id, user_id);
        let memories = FakeMemories::default();
        let transcriber = RecordingTranscriber::new("shared thought");

        let result = create_voice_capture_memory(
            VoiceCaptureDeps {
                memories: &memories,
                spaces: &spaces,
                transcriber: Some(&transcriber),
                transcription_config_error: None,
            },
            user_id,
            VoiceCaptureInput {
                space_id,
                filename: Some("thought.webm".to_string()),
                content_type: Some("audio/webm".to_string()),
                title: None,
                language: None,
                audio: b"audio bytes".to_vec(),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
        assert_eq!(*transcriber.calls.lock().unwrap(), 0);
        assert!(memories.created.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn voice_capture_creates_audio_memory_with_transcription_provenance() {
        let user_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let spaces = FakeSpaces::writer(space_id, user_id);
        let memories = FakeMemories::default();
        let transcriber = RecordingTranscriber::new("我今天反复想到这个决定");

        let response = create_voice_capture_memory(
            VoiceCaptureDeps {
                memories: &memories,
                spaces: &spaces,
                transcriber: Some(&transcriber),
                transcription_config_error: None,
            },
            user_id,
            VoiceCaptureInput {
                space_id,
                filename: Some("thought.webm".to_string()),
                content_type: Some("audio/webm".to_string()),
                title: Some("Voice thought".to_string()),
                language: Some("zh".to_string()),
                audio: b"audio bytes".to_vec(),
            },
        )
        .await
        .unwrap();

        let created = memories.created.lock().unwrap();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].space_id, space_id);
        assert_eq!(created[0].user_id, user_id);
        assert_eq!(created[0].content, "我今天反复想到这个决定");
        assert_eq!(created[0].memory_type, MemoryType::Audio);
        assert_eq!(created[0].source_type, "voice_transcription");
        assert_eq!(created[0].tags, vec!["voice-capture".to_string()]);
        assert_eq!(created[0].source_metadata["provider"], "openai");
        assert_eq!(created[0].source_metadata["model"], "whisper-1");
        assert_eq!(created[0].source_metadata["language"], "zh");
        assert_eq!(created[0].source_metadata["filename"], "thought.webm");
        assert_eq!(created[0].source_metadata["content_type"], "audio/webm");
        assert_eq!(created[0].source_metadata["audio_size_bytes"], 11);
        assert_eq!(response.memory.source_type, "voice_transcription");
        assert_eq!(response.transcription.text, "我今天反复想到这个决定");
    }

    #[derive(Default)]
    struct FakeMemories {
        created: Arc<Mutex<Vec<CreateMemory>>>,
    }

    #[async_trait]
    impl MemoryRepository for FakeMemories {
        async fn create(&self, memory: CreateMemory) -> Result<MemoryDb, Error> {
            self.created.lock().unwrap().push(memory.clone());
            Ok(MemoryDb {
                id: Uuid::new_v4(),
                user_id: memory.user_id,
                space_id: memory.space_id,
                namespace_id: memory.namespace_id,
                feedback_loop_id: memory.feedback_loop_id,
                title: memory.title,
                content: memory.content,
                memory_type: memory.memory_type.to_string(),
                file_path: memory.file_path,
                thumbnail_path: None,
                is_shared: memory.is_shared,
                source_type: memory.source_type,
                source_metadata: memory.source_metadata,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<MemoryDb>, Error> {
            unimplemented!()
        }

        async fn list_by_user(
            &self,
            _user_id: Uuid,
            _limit: i64,
            _offset: i64,
        ) -> Result<Vec<MemoryDb>, Error> {
            unimplemented!()
        }

        async fn count_by_user(&self, _user_id: Uuid) -> Result<i64, Error> {
            unimplemented!()
        }

        async fn list_by_space(
            &self,
            _user_id: Uuid,
            _space_id: Uuid,
            _limit: i64,
            _offset: i64,
            _filter: MemoryListFilter,
        ) -> Result<Vec<MemoryDb>, Error> {
            unimplemented!()
        }

        async fn list_by_space_window(
            &self,
            _user_id: Uuid,
            _space_id: Uuid,
            _window_start: chrono::DateTime<Utc>,
            _window_end: chrono::DateTime<Utc>,
            _limit: i64,
            _namespace_id: Option<Uuid>,
        ) -> Result<Vec<MemoryDb>, Error> {
            unimplemented!()
        }

        async fn list_feedback_loop_event_snapshots(
            &self,
            _user_id: Uuid,
            _filter: crate::db::memory::FeedbackLoopEventSnapshotFilter,
        ) -> Result<Vec<MemoryDb>, Error> {
            unimplemented!()
        }

        async fn count_by_space(
            &self,
            _user_id: Uuid,
            _space_id: Uuid,
            _filter: MemoryListFilter,
        ) -> Result<i64, Error> {
            unimplemented!()
        }

        async fn update(&self, _id: Uuid, _update: UpdateMemory) -> Result<MemoryDb, Error> {
            unimplemented!()
        }

        async fn delete(&self, _id: Uuid) -> Result<bool, Error> {
            unimplemented!()
        }
    }

    struct FakeSpaces {
        space_id: Uuid,
        user_id: Uuid,
        role: SpaceMemberRole,
    }

    impl FakeSpaces {
        fn writer(space_id: Uuid, user_id: Uuid) -> Self {
            Self {
                space_id,
                user_id,
                role: SpaceMemberRole::Editor,
            }
        }

        fn viewer(space_id: Uuid, user_id: Uuid) -> Self {
            Self {
                space_id,
                user_id,
                role: SpaceMemberRole::Viewer,
            }
        }
    }

    #[async_trait]
    impl CognitiveSpaceRepository for FakeSpaces {
        async fn create(&self, _space: CreateCognitiveSpace) -> Result<CognitiveSpaceDb, Error> {
            unimplemented!()
        }

        async fn list_for_user(&self, _user_id: Uuid) -> Result<Vec<CognitiveSpaceDb>, Error> {
            unimplemented!()
        }

        async fn list_for_user_by_type(
            &self,
            _user_id: Uuid,
            _space_type: CognitiveSpaceType,
        ) -> Result<Vec<CognitiveSpaceDb>, Error> {
            unimplemented!()
        }

        async fn find_for_user(
            &self,
            space_id: Uuid,
            user_id: Uuid,
        ) -> Result<Option<CognitiveSpaceDb>, Error> {
            if space_id != self.space_id || user_id != self.user_id {
                return Ok(None);
            }

            Ok(Some(CognitiveSpaceDb {
                id: self.space_id,
                name: "Shared Space".to_string(),
                description: None,
                owner_user_id: self.user_id,
                default_lens_id: None,
                space_type: "personal".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }))
        }

        async fn default_for_user(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<CognitiveSpaceDb>, Error> {
            unimplemented!()
        }

        async fn ensure_default_for_user(
            &self,
            _user_id: Uuid,
            _username: &str,
        ) -> Result<CognitiveSpaceDb, Error> {
            unimplemented!()
        }

        async fn find_member(
            &self,
            space_id: Uuid,
            user_id: Uuid,
        ) -> Result<Option<CognitiveSpaceMemberDb>, Error> {
            if space_id != self.space_id || user_id != self.user_id {
                return Ok(None);
            }

            Ok(Some(CognitiveSpaceMemberDb {
                space_id: self.space_id,
                user_id: self.user_id,
                role: self.role.to_string(),
                created_at: Utc::now(),
            }))
        }

        async fn list_members_for_user(
            &self,
            _space_id: Uuid,
            _user_id: Uuid,
        ) -> Result<Vec<CognitiveSpaceMemberDb>, Error> {
            unimplemented!()
        }

        async fn update_member_role(
            &self,
            _space_id: Uuid,
            _user_id: Uuid,
            _role: SpaceMemberRole,
        ) -> Result<Option<CognitiveSpaceMemberDb>, Error> {
            unimplemented!()
        }

        async fn create_invite(
            &self,
            _invite: CreateSpaceInvite,
        ) -> Result<crate::db::space::CognitiveSpaceInviteDb, Error> {
            unimplemented!()
        }

        async fn accept_invite(
            &self,
            _code: &str,
            _user_id: Uuid,
        ) -> Result<Option<crate::db::space::CognitiveSpaceInviteDb>, Error> {
            unimplemented!()
        }
    }

    struct RecordingTranscriber {
        text: String,
        calls: Arc<Mutex<usize>>,
    }

    impl RecordingTranscriber {
        fn new(text: &str) -> Self {
            Self {
                text: text.to_string(),
                calls: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl TranscriptionProvider for RecordingTranscriber {
        async fn transcribe(
            &self,
            input: AudioTranscriptionInput,
            options: TranscriptionOptions,
        ) -> Result<TranscriptionResult, TranscriptionProviderError> {
            *self.calls.lock().unwrap() += 1;
            assert_eq!(input.bytes, b"audio bytes");
            assert_eq!(input.filename.as_deref(), Some("thought.webm"));
            assert_eq!(options.language.as_deref(), Some("zh"));

            Ok(TranscriptionResult {
                text: self.text.clone(),
                provider: "openai".to_string(),
                model: "whisper-1".to_string(),
                language: options.language,
                duration_seconds: Some(1.5),
                metadata: json!({"response_format": "verbose_json"}),
            })
        }
    }
}
