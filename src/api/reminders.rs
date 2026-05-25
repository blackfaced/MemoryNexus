//! Reminder API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::db::reminder::{CreateReminder, ReminderDb, ReminderListFilter};
use crate::error::{ApiResponse, AppError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateReminderRequest {
    pub space_id: Uuid,
    pub memory_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: String,
    pub remind_at: DateTime<Utc>,
    pub repeat_rule: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListRemindersQuery {
    pub space_id: Uuid,
    #[serde(default)]
    pub due_only: bool,
    #[serde(default)]
    pub include_completed: bool,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ReminderListResponse {
    pub items: Vec<ReminderDb>,
    pub total: usize,
}

/// POST /api/v1/reminders - Create a scheduled recall reminder.
pub async fn create(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(req): Json<CreateReminderRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ReminderDb>>), AppError> {
    let content = req.content.trim();
    if content.is_empty() {
        return Err(AppError::BadRequest(
            "reminder content is required".to_string(),
        ));
    }
    validate_repeat_rule(req.repeat_rule.as_deref())?;
    require_space_member(&state, req.space_id, auth_user.user_id).await?;
    require_memory_in_space(&state, req.memory_id, req.space_id).await?;

    let reminder = state
        .repositories
        .reminders
        .create(CreateReminder {
            user_id: auth_user.user_id,
            space_id: req.space_id,
            memory_id: req.memory_id,
            title: req.title,
            content: content.to_string(),
            remind_at: req.remind_at,
            repeat_rule: req.repeat_rule,
        })
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(reminder))))
}

/// GET /api/v1/reminders?space_id=<SPACE_ID> - List reminders in a space.
pub async fn list(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Query(query): Query<ListRemindersQuery>,
) -> Result<Json<ApiResponse<ReminderListResponse>>, AppError> {
    require_space_member(&state, query.space_id, auth_user.user_id).await?;
    let reminders = state
        .repositories
        .reminders
        .list_for_user(
            ReminderListFilter {
                space_id: query.space_id,
                include_completed: query.include_completed,
                due_only: query.due_only,
                limit: query.limit.unwrap_or(20).clamp(1, 100),
            },
            auth_user.user_id,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(ApiResponse::success(ReminderListResponse {
        total: reminders.len(),
        items: reminders,
    })))
}

/// POST /api/v1/reminders/:id/complete - Mark a reminder as completed.
pub async fn complete(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ReminderDb>>, AppError> {
    let reminder = state
        .repositories
        .reminders
        .complete_for_user(id, auth_user.user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("pending reminder not found".to_string()))?;

    Ok(Json(ApiResponse::success(reminder)))
}

async fn require_space_member(
    state: &AppState,
    space_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    state
        .repositories
        .spaces
        .find_for_user(space_id, user_id)
        .await
        .map_err(AppError::Database)?
        .map(|_| ())
        .ok_or(AppError::Unauthorized)
}

async fn require_memory_in_space(
    state: &AppState,
    memory_id: Option<Uuid>,
    space_id: Uuid,
) -> Result<(), AppError> {
    let Some(memory_id) = memory_id else {
        return Ok(());
    };

    let memory = state
        .repositories
        .memories
        .find_by_id(memory_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("memory not found".to_string()))?;

    if memory.space_id != space_id {
        return Err(AppError::BadRequest(
            "reminder memory_id must belong to the same Cognitive Space".to_string(),
        ));
    }

    Ok(())
}

fn validate_repeat_rule(rule: Option<&str>) -> Result<(), AppError> {
    match rule {
        None | Some("daily" | "weekly" | "monthly") => Ok(()),
        Some(rule) => Err(AppError::BadRequest(format!(
            "unsupported repeat_rule: {rule}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_reminder_request_deserializes() {
        let req: CreateReminderRequest = serde_json::from_str(
            r#"{
                "space_id":"24f7166f-3a6f-475b-a409-bd11a00a5734",
                "title":"Review",
                "content":"Review MemoryNexus",
                "remind_at":"2026-05-26T09:00:00Z",
                "repeat_rule":"weekly"
            }"#,
        )
        .unwrap();

        assert_eq!(req.title.as_deref(), Some("Review"));
        assert_eq!(req.repeat_rule.as_deref(), Some("weekly"));
    }

    #[test]
    fn validates_repeat_rules() {
        assert!(validate_repeat_rule(None).is_ok());
        assert!(validate_repeat_rule(Some("daily")).is_ok());
        assert!(validate_repeat_rule(Some("hourly")).is_err());
    }
}
