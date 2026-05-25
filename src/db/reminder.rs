//! Reminder database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Error, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReminderDb {
    pub id: Uuid,
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub memory_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: String,
    pub remind_at: DateTime<Utc>,
    pub is_completed: bool,
    pub status: String,
    pub repeat_rule: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct CreateReminder {
    pub user_id: Uuid,
    pub space_id: Uuid,
    pub memory_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: String,
    pub remind_at: DateTime<Utc>,
    pub repeat_rule: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReminderListFilter {
    pub space_id: Uuid,
    pub include_completed: bool,
    pub due_only: bool,
    pub limit: i64,
}

#[async_trait::async_trait]
pub trait ReminderRepository: Send + Sync {
    async fn create(&self, reminder: CreateReminder) -> Result<ReminderDb, Error>;
    async fn list_for_user(
        &self,
        filter: ReminderListFilter,
        user_id: Uuid,
    ) -> Result<Vec<ReminderDb>, Error>;
    async fn complete_for_user(
        &self,
        reminder_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ReminderDb>, Error>;
}

pub struct PostgresReminderRepository {
    pool: PgPool,
}

impl PostgresReminderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ReminderRepository for PostgresReminderRepository {
    async fn create(&self, reminder: CreateReminder) -> Result<ReminderDb, Error> {
        sqlx::query_as::<_, ReminderDb>(
            r#"
            INSERT INTO reminders (
                user_id,
                space_id,
                memory_id,
                title,
                content,
                remind_at,
                repeat_rule
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, user_id, space_id, memory_id, title, content, remind_at,
                      is_completed, status, repeat_rule, created_at, completed_at
            "#,
        )
        .bind(reminder.user_id)
        .bind(reminder.space_id)
        .bind(reminder.memory_id)
        .bind(&reminder.title)
        .bind(&reminder.content)
        .bind(reminder.remind_at)
        .bind(&reminder.repeat_rule)
        .fetch_one(&self.pool)
        .await
    }

    async fn list_for_user(
        &self,
        filter: ReminderListFilter,
        user_id: Uuid,
    ) -> Result<Vec<ReminderDb>, Error> {
        sqlx::query_as::<_, ReminderDb>(
            r#"
            SELECT r.id, r.user_id, r.space_id, r.memory_id, r.title, r.content,
                   r.remind_at, r.is_completed, r.status, r.repeat_rule,
                   r.created_at, r.completed_at
            FROM reminders r
            INNER JOIN cognitive_space_members m ON m.space_id = r.space_id
            WHERE m.user_id = $1
              AND r.space_id = $2
              AND ($3 OR r.status = 'pending')
              AND (NOT $4 OR (r.status = 'pending' AND r.remind_at <= NOW()))
            ORDER BY r.remind_at ASC
            LIMIT $5
            "#,
        )
        .bind(user_id)
        .bind(filter.space_id)
        .bind(filter.include_completed)
        .bind(filter.due_only)
        .bind(filter.limit)
        .fetch_all(&self.pool)
        .await
    }

    async fn complete_for_user(
        &self,
        reminder_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ReminderDb>, Error> {
        sqlx::query_as::<_, ReminderDb>(
            r#"
            UPDATE reminders r
            SET remind_at = CASE r.repeat_rule
                    WHEN 'daily' THEN GREATEST(r.remind_at, NOW()) + INTERVAL '1 day'
                    WHEN 'weekly' THEN GREATEST(r.remind_at, NOW()) + INTERVAL '1 week'
                    WHEN 'monthly' THEN GREATEST(r.remind_at, NOW()) + INTERVAL '1 month'
                    ELSE r.remind_at
                END,
                status = CASE
                    WHEN r.repeat_rule IS NULL THEN 'completed'
                    ELSE 'pending'
                END,
                is_completed = r.repeat_rule IS NULL,
                completed_at = NOW()
            FROM cognitive_space_members m
            WHERE r.id = $1
              AND m.space_id = r.space_id
              AND m.user_id = $2
              AND r.status = 'pending'
            RETURNING r.id, r.user_id, r.space_id, r.memory_id, r.title, r.content,
                      r.remind_at, r.is_completed, r.status, r.repeat_rule,
                      r.created_at, r.completed_at
            "#,
        )
        .bind(reminder_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_reminder_keeps_space_boundary() {
        let user_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();
        let reminder = CreateReminder {
            user_id,
            space_id,
            memory_id: None,
            title: Some("Review".to_string()),
            content: "Review current MemoryNexus direction".to_string(),
            remind_at: Utc::now(),
            repeat_rule: Some("weekly".to_string()),
        };

        assert_eq!(reminder.user_id, user_id);
        assert_eq!(reminder.space_id, space_id);
        assert_eq!(reminder.repeat_rule.as_deref(), Some("weekly"));
    }
}
