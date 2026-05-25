//! Cognitive profile projection API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::lens::LensDb;
use crate::db::lens_run::{LensRunDb, LensRunListFilter};
use crate::db::memory::MemoryDb;
use crate::db::profile::{CognitiveProfileSnapshotDb, CreateCognitiveProfileSnapshot};
use crate::db::space::CognitiveSpaceDb;
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    pub space_id: Option<Uuid>,
    pub lens_id: Option<Uuid>,
    #[serde(default = "default_target")]
    pub target: String,
    pub limit: Option<i64>,
}

fn default_target() -> String {
    "llm_context".to_string()
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub snapshot: CognitiveProfileSnapshotDb,
}

/// POST /api/v1/profiles - Project and persist a compact cognitive profile.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateProfileRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ProfileResponse>>), AppError> {
    let target = normalize_target(&req.target)?;
    let limit = req.limit.unwrap_or(12).clamp(1, 30);

    let (space, lens) = resolve_profile_context(&state, &req, auth_user.user_id).await?;

    let memories = state
        .repositories
        .memories
        .list_by_space(auth_user.user_id, space.id, limit, 0)
        .await
        .map_err(AppError::Database)?;

    let lens_runs = state
        .repositories
        .lens_runs
        .list_for_user(
            LensRunListFilter {
                lens_id: req.lens_id,
                space_id: Some(space.id),
                limit: limit.min(10),
            },
            auth_user.user_id,
        )
        .await
        .map_err(AppError::Database)?;

    let profile = build_profile_json(&space, lens.as_ref(), &target, &memories, &lens_runs);
    let source_memory_ids = memories.iter().map(|memory| memory.id).collect::<Vec<_>>();
    let source_lens_run_ids = lens_runs.iter().map(|run| run.id).collect::<Vec<_>>();

    let snapshot = state
        .repositories
        .profiles
        .create(CreateCognitiveProfileSnapshot {
            space_id: space.id,
            lens_id: req.lens_id,
            target,
            profile,
            source_memory_ids,
            source_lens_run_ids,
            created_by: auth_user.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(ProfileResponse { snapshot })),
    ))
}

async fn resolve_profile_context(
    state: &AppState,
    req: &CreateProfileRequest,
    user_id: Uuid,
) -> Result<(CognitiveSpaceDb, Option<LensDb>), AppError> {
    if let Some(lens_id) = req.lens_id {
        let lens = state
            .repositories
            .lenses
            .find_for_user(lens_id, user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;

        if let Some(space_id) = req.space_id {
            if space_id != lens.space_id {
                return Err(AppError::BadRequest(
                    "space_id must match the Lens Cognitive Space".to_string(),
                ));
            }
        }

        let space = state
            .repositories
            .spaces
            .find_for_user(lens.space_id, user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?;

        return Ok((space, Some(lens)));
    }

    let space = if let Some(space_id) = req.space_id {
        state
            .repositories
            .spaces
            .find_for_user(space_id, user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::Unauthorized)?
    } else {
        state
            .repositories
            .spaces
            .default_for_user(user_id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Cognitive space not found".to_string()))?
    };

    Ok((space, None))
}

/// GET /api/v1/profiles/:id - Fetch a persisted profile snapshot.
pub async fn get(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ProfileResponse>>, AppError> {
    let snapshot = state
        .repositories
        .profiles
        .find_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(ApiResponse::success(ProfileResponse { snapshot })))
}

fn normalize_target(target: &str) -> Result<String, AppError> {
    let target = target.trim().to_ascii_lowercase();
    match target.as_str() {
        "llm_context" | "personal_context" | "preference_review" | "decision_history"
        | "risk_review" | "project_context" => Ok(target),
        _ => Err(AppError::BadRequest(format!(
            "unsupported profile target: {target}"
        ))),
    }
}

fn build_profile_json(
    space: &CognitiveSpaceDb,
    lens: Option<&LensDb>,
    target: &str,
    memories: &[MemoryDb],
    lens_runs: &[LensRunDb],
) -> Value {
    let stable_preferences = classify_memories(memories, preference_signal);
    let active_projects = classify_memories(memories, project_signal);
    let decision_history = classify_memories(memories, decision_signal);
    let recent_context = memories
        .iter()
        .take(5)
        .map(memory_snippet)
        .collect::<Vec<_>>();
    let unresolved_contradictions = lens_runs
        .iter()
        .flat_map(extract_unresolved_contradictions)
        .collect::<Vec<_>>();

    json!({
        "space": {
            "id": space.id,
            "name": space.name,
            "space_type": space.space_type,
        },
        "lens": lens.map(|lens| json!({
            "id": lens.id,
            "name": lens.name,
            "strategy": lens.strategy,
            "output_format": lens.output_format,
            "retrieval_mode": lens.retrieval_mode,
        })),
        "target": target,
        "version": 1,
        "projected_at": Utc::now(),
        "summary": profile_summary(space, memories, lens_runs),
        "stable_preferences": stable_preferences,
        "active_projects": active_projects,
        "decision_history": decision_history,
        "recent_context": recent_context,
        "unresolved_contradictions": unresolved_contradictions,
        "source_memory_ids": memories.iter().map(|memory| memory.id).collect::<Vec<_>>(),
        "source_lens_run_ids": lens_runs.iter().map(|run| run.id).collect::<Vec<_>>(),
        "usage": {
            "search": "Use search_memories when the agent needs raw recall.",
            "lens_run": "Use run_lens when the agent needs interpretation with citations.",
            "profile": "Use this profile as compact working context before a personal agent task."
        }
    })
}

fn classify_memories(memories: &[MemoryDb], predicate: fn(&str) -> bool) -> Vec<Value> {
    memories
        .iter()
        .filter(|memory| {
            predicate(&format!(
                "{} {}",
                memory.title.as_deref().unwrap_or(""),
                memory.content
            ))
        })
        .take(5)
        .map(memory_snippet)
        .collect()
}

fn memory_snippet(memory: &MemoryDb) -> Value {
    json!({
        "memory_id": memory.id,
        "title": memory.title,
        "content": truncate(&memory.content, 240),
        "created_at": memory.created_at,
    })
}

fn extract_unresolved_contradictions(run: &LensRunDb) -> Vec<Value> {
    run.output
        .as_ref()
        .and_then(|output| output.get("unresolved_contradictions"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    json!({
                        "lens_run_id": run.id,
                        "contradiction": item,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn profile_summary(
    space: &CognitiveSpaceDb,
    memories: &[MemoryDb],
    lens_runs: &[LensRunDb],
) -> String {
    format!(
        "Cognitive profile for '{}' using {} recent memories and {} Lens Runs.",
        space.name,
        memories.len(),
        lens_runs.len()
    )
}

fn preference_signal(text: &str) -> bool {
    contains_any(
        text,
        &[
            "prefer",
            "preference",
            "like",
            "dislike",
            "喜欢",
            "偏好",
            "不喜欢",
        ],
    )
}

fn project_signal(text: &str) -> bool {
    contains_any(
        text,
        &[
            "project",
            "phase",
            "roadmap",
            "todo",
            "项目",
            "阶段",
            "路线图",
        ],
    )
}

fn decision_signal(text: &str) -> bool {
    contains_any(
        text,
        &[
            "decision", "decided", "adr", "choose", "chosen", "决定", "选择",
        ],
    )
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn memory(title: &str, content: &str) -> MemoryDb {
        MemoryDb {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            title: Some(title.to_string()),
            content: content.to_string(),
            memory_type: "text".to_string(),
            file_path: None,
            thumbnail_path: None,
            is_shared: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn profile_projection_extracts_agent_context_sections() {
        let space = CognitiveSpaceDb {
            id: Uuid::new_v4(),
            name: "Personal Agent Space".to_string(),
            description: None,
            owner_user_id: Uuid::new_v4(),
            default_lens_id: None,
            space_type: "personal".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let memories = vec![
            memory("Preference", "The user prefers Rust-first backend work."),
            memory("Project", "The MemoryNexus project is in Phase 3."),
            memory("Decision", "Decision: use CognitiveSpace as the boundary."),
        ];
        let profile = build_profile_json(&space, None, "personal_context", &memories, &[]);

        assert_eq!(profile["target"], "personal_context");
        assert_eq!(profile["stable_preferences"].as_array().unwrap().len(), 1);
        assert_eq!(profile["active_projects"].as_array().unwrap().len(), 1);
        assert_eq!(profile["decision_history"].as_array().unwrap().len(), 1);
        assert_eq!(profile["source_memory_ids"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn unsupported_target_is_rejected() {
        let error = normalize_target("agent_private_memory").unwrap_err();
        assert!(matches!(error, AppError::BadRequest(_)));
    }

    #[test]
    fn create_profile_request_allows_default_space() {
        let req: CreateProfileRequest =
            serde_json::from_str(r#"{"target":"personal_context","limit":8}"#).unwrap();

        assert_eq!(req.space_id, None);
        assert_eq!(req.target, "personal_context");
        assert_eq!(req.limit, Some(8));
    }
}
