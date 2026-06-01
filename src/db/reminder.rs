//! Reminder database operations

use chrono::{DateTime, Duration, Months, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub delivery_channel: String,
    pub delivery_status: String,
    pub delivery_attempted_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub delivery_error: Option<String>,
    pub delivery_provenance: Value,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletionDeliveryUpdate {
    status: String,
    attempted_at: Option<DateTime<Utc>>,
    delivered_at: Option<DateTime<Utc>>,
    error: Option<String>,
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
    pub delivery_channel: String,
}

#[derive(Debug, Clone)]
pub struct UpdateReminderDelivery {
    pub reminder_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub error: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct ReminderListFilter {
    pub space_id: Uuid,
    pub include_completed: bool,
    pub due_only: bool,
    pub limit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReminderRecurrenceError {
    Empty,
    UnsupportedFrequency(String),
    InvalidInterval(String),
    DateOverflow,
}

impl std::fmt::Display for ReminderRecurrenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "repeat_rule cannot be empty"),
            Self::UnsupportedFrequency(frequency) => {
                write!(f, "unsupported repeat_rule frequency: {frequency}")
            }
            Self::InvalidInterval(interval) => {
                write!(
                    f,
                    "repeat_rule interval must be a positive integer: {interval}"
                )
            }
            Self::DateOverflow => write!(f, "repeat_rule produced an out-of-range next due date"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReminderFrequency {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderRecurrenceRule {
    frequency: ReminderFrequency,
    interval: u32,
}

impl ReminderRecurrenceRule {
    pub fn parse(rule: &str) -> Result<Self, ReminderRecurrenceError> {
        if rule.is_empty() {
            return Err(ReminderRecurrenceError::Empty);
        }

        let mut parts = rule.split(':');
        let frequency = match parts.next().unwrap_or_default() {
            "daily" => ReminderFrequency::Daily,
            "weekly" => ReminderFrequency::Weekly,
            "monthly" => ReminderFrequency::Monthly,
            other => {
                return Err(ReminderRecurrenceError::UnsupportedFrequency(
                    other.to_string(),
                ))
            }
        };
        let interval = match parts.next() {
            None => 1,
            Some(raw_interval) => parse_interval(raw_interval)?,
        };
        if parts.next().is_some() {
            return Err(ReminderRecurrenceError::InvalidInterval(rule.to_string()));
        }

        Ok(Self {
            frequency,
            interval,
        })
    }

    pub fn next_due_at(
        self,
        current_due_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> Result<DateTime<Utc>, ReminderRecurrenceError> {
        let anchor = current_due_at.max(completed_at);
        match self.frequency {
            ReminderFrequency::Daily => Duration::try_days(i64::from(self.interval))
                .and_then(|duration| anchor.checked_add_signed(duration))
                .ok_or(ReminderRecurrenceError::DateOverflow),
            ReminderFrequency::Weekly => Duration::try_weeks(i64::from(self.interval))
                .and_then(|duration| anchor.checked_add_signed(duration))
                .ok_or(ReminderRecurrenceError::DateOverflow),
            ReminderFrequency::Monthly => anchor
                .checked_add_months(Months::new(self.interval))
                .ok_or(ReminderRecurrenceError::DateOverflow),
        }
    }
}

fn parse_interval(raw_interval: &str) -> Result<u32, ReminderRecurrenceError> {
    if raw_interval.is_empty()
        || raw_interval.starts_with('0')
        || !raw_interval.chars().all(|ch| ch.is_ascii_digit())
    {
        return Err(ReminderRecurrenceError::InvalidInterval(
            raw_interval.to_string(),
        ));
    }

    let interval = raw_interval
        .parse::<u32>()
        .map_err(|_| ReminderRecurrenceError::InvalidInterval(raw_interval.to_string()))?;
    Ok(interval)
}

pub fn validate_repeat_rule(rule: &str) -> Result<(), ReminderRecurrenceError> {
    ReminderRecurrenceRule::parse(rule).map(|_| ())
}

fn next_due_at_for_repeat_rule(
    rule: &str,
    current_due_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
) -> Result<DateTime<Utc>, ReminderRecurrenceError> {
    ReminderRecurrenceRule::parse(rule)?.next_due_at(current_due_at, completed_at)
}

fn completion_delivery_update(
    reminder: &ReminderDb,
    next_due_at: Option<DateTime<Utc>>,
) -> CompletionDeliveryUpdate {
    if next_due_at.is_some() {
        return CompletionDeliveryUpdate {
            status: "pending".to_string(),
            attempted_at: None,
            delivered_at: None,
            error: None,
        };
    }

    CompletionDeliveryUpdate {
        status: reminder.delivery_status.clone(),
        attempted_at: reminder.delivery_attempted_at,
        delivered_at: reminder.delivered_at,
        error: reminder.delivery_error.clone(),
    }
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
    async fn update_delivery_for_user(
        &self,
        delivery: UpdateReminderDelivery,
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
                repeat_rule,
                delivery_channel
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, space_id, memory_id, title, content, remind_at,
                      is_completed, status, repeat_rule, delivery_channel,
                      delivery_status, delivery_attempted_at, delivered_at,
                      delivery_error, delivery_provenance, created_at, completed_at
            "#,
        )
        .bind(reminder.user_id)
        .bind(reminder.space_id)
        .bind(reminder.memory_id)
        .bind(&reminder.title)
        .bind(&reminder.content)
        .bind(reminder.remind_at)
        .bind(&reminder.repeat_rule)
        .bind(&reminder.delivery_channel)
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
                   r.delivery_channel, r.delivery_status, r.delivery_attempted_at,
                   r.delivered_at, r.delivery_error, r.delivery_provenance,
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
        let mut tx = self.pool.begin().await?;
        let reminder = sqlx::query_as::<_, ReminderDb>(
            r#"
            SELECT r.id, r.user_id, r.space_id, r.memory_id, r.title, r.content,
                   r.remind_at, r.is_completed, r.status, r.repeat_rule,
                   r.delivery_channel, r.delivery_status, r.delivery_attempted_at,
                   r.delivered_at, r.delivery_error, r.delivery_provenance,
                   r.created_at, r.completed_at
            FROM reminders r
            INNER JOIN cognitive_space_members m ON m.space_id = r.space_id
            WHERE r.id = $1
              AND m.user_id = $2
              AND r.status = 'pending'
            FOR UPDATE OF r
            "#,
        )
        .bind(reminder_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(reminder) = reminder else {
            tx.commit().await?;
            return Ok(None);
        };

        let acknowledged_at = Utc::now();
        let next_due_at = match reminder.repeat_rule.as_deref() {
            Some(rule) => Some(
                next_due_at_for_repeat_rule(rule, reminder.remind_at, acknowledged_at)
                    .map_err(|err| Error::Protocol(err.to_string()))?,
            ),
            None => None,
        };
        let delivery = completion_delivery_update(&reminder, next_due_at);

        let updated = sqlx::query_as::<_, ReminderDb>(
            r#"
            UPDATE reminders
            SET remind_at = COALESCE($2, remind_at),
                status = CASE WHEN $2 IS NULL THEN 'completed' ELSE 'pending' END,
                is_completed = $2 IS NULL,
                completed_at = $3,
                delivery_status = $4,
                delivery_attempted_at = $5,
                delivered_at = $6,
                delivery_error = $7
            WHERE id = $1
            RETURNING id, user_id, space_id, memory_id, title, content, remind_at,
                      is_completed, status, repeat_rule, delivery_channel,
                      delivery_status, delivery_attempted_at, delivered_at,
                      delivery_error, delivery_provenance, created_at, completed_at
            "#,
        )
        .bind(reminder_id)
        .bind(next_due_at)
        .bind(acknowledged_at)
        .bind(&delivery.status)
        .bind(delivery.attempted_at)
        .bind(delivery.delivered_at)
        .bind(&delivery.error)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(updated))
    }

    async fn update_delivery_for_user(
        &self,
        delivery: UpdateReminderDelivery,
    ) -> Result<Option<ReminderDb>, Error> {
        sqlx::query_as::<_, ReminderDb>(
            r#"
            UPDATE reminders r
            SET delivery_status = $3,
                delivery_attempted_at = NOW(),
                delivered_at = CASE
                    WHEN $3 = 'delivered' THEN NOW()
                    ELSE NULL
                END,
                delivery_error = CASE
                    WHEN $3 = 'failed' THEN $4
                    ELSE NULL
                END,
                delivery_provenance = jsonb_build_object(
                    'channel', r.delivery_channel,
                    'source', $5::text,
                    'actor_user_id', $2::text,
                    'status', $3::text,
                    'attempted_at', NOW(),
                    'error', $4::text
                )
            FROM cognitive_space_members m
            WHERE r.id = $1
              AND m.space_id = r.space_id
              AND m.user_id = $2
              AND r.status = 'pending'
              AND r.delivery_channel = 'in_app'
              AND r.remind_at <= NOW()
            RETURNING r.id, r.user_id, r.space_id, r.memory_id, r.title, r.content,
                      r.remind_at, r.is_completed, r.status, r.repeat_rule,
                      r.delivery_channel, r.delivery_status, r.delivery_attempted_at,
                      r.delivered_at, r.delivery_error, r.delivery_provenance,
                      r.created_at, r.completed_at
            "#,
        )
        .bind(delivery.reminder_id)
        .bind(delivery.user_id)
        .bind(&delivery.status)
        .bind(&delivery.error)
        .bind(&delivery.source)
        .fetch_optional(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
            delivery_channel: "in_app".to_string(),
        };

        assert_eq!(reminder.user_id, user_id);
        assert_eq!(reminder.space_id, space_id);
        assert_eq!(reminder.repeat_rule.as_deref(), Some("weekly"));
    }

    #[test]
    fn next_due_at_advances_from_completion_when_original_due_is_past() {
        let original_due = "2026-05-01T09:00:00Z".parse().unwrap();
        let completed_at = "2026-05-10T10:30:00Z".parse().unwrap();

        let next_due = next_due_at_for_repeat_rule("weekly:2", original_due, completed_at).unwrap();

        assert_eq!(next_due.to_rfc3339(), "2026-05-24T10:30:00+00:00");
    }

    #[test]
    fn next_due_at_preserves_future_anchor_when_completed_early() {
        let original_due = "2026-05-20T09:00:00Z".parse().unwrap();
        let completed_at = "2026-05-10T10:30:00Z".parse().unwrap();

        let next_due = next_due_at_for_repeat_rule("daily:3", original_due, completed_at).unwrap();

        assert_eq!(next_due.to_rfc3339(), "2026-05-23T09:00:00+00:00");
    }

    #[test]
    fn monthly_recurrence_clamps_end_of_month_in_utc() {
        let original_due = "2026-01-31T09:00:00Z".parse().unwrap();
        let completed_at = "2026-01-31T09:05:00Z".parse().unwrap();

        let next_due = next_due_at_for_repeat_rule("monthly", original_due, completed_at).unwrap();

        assert_eq!(next_due.to_rfc3339(), "2026-02-28T09:05:00+00:00");
    }

    #[test]
    fn oversized_recurrence_interval_returns_overflow_error() {
        let original_due = "2026-05-01T09:00:00Z".parse().unwrap();
        let completed_at = "2026-05-10T10:30:00Z".parse().unwrap();

        let err = next_due_at_for_repeat_rule("daily:4294967295", original_due, completed_at)
            .unwrap_err();

        assert_eq!(err, ReminderRecurrenceError::DateOverflow);
    }

    #[test]
    fn recurring_completion_resets_delivery_for_next_occurrence() {
        let reminder = delivered_reminder(Some("weekly:2"));
        let next_due_at = Some("2026-05-24T10:30:00Z".parse().unwrap());

        let delivery = completion_delivery_update(&reminder, next_due_at);

        assert_eq!(delivery.status, "pending");
        assert_eq!(delivery.attempted_at, None);
        assert_eq!(delivery.delivered_at, None);
        assert_eq!(delivery.error, None);
        assert_eq!(
            reminder.delivery_provenance,
            json!({"source": "api_in_app_poll", "status": "delivered"})
        );
    }

    #[test]
    fn non_recurring_completion_preserves_delivery_state_and_provenance() {
        let reminder = delivered_reminder(None);

        let delivery = completion_delivery_update(&reminder, None);

        assert_eq!(delivery.status, "delivered");
        assert_eq!(
            delivery.attempted_at.map(|value| value.to_rfc3339()),
            Some("2026-05-10T10:00:00+00:00".to_string())
        );
        assert_eq!(
            delivery.delivered_at.map(|value| value.to_rfc3339()),
            Some("2026-05-10T10:01:00+00:00".to_string())
        );
        assert_eq!(delivery.error.as_deref(), Some("prior transient error"));
        assert_eq!(
            reminder.delivery_provenance,
            json!({"source": "api_in_app_poll", "status": "delivered"})
        );
    }

    fn delivered_reminder(repeat_rule: Option<&str>) -> ReminderDb {
        ReminderDb {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
            memory_id: None,
            title: Some("Review".to_string()),
            content: "Review current MemoryNexus direction".to_string(),
            remind_at: "2026-05-10T09:00:00Z".parse().unwrap(),
            is_completed: false,
            status: "pending".to_string(),
            repeat_rule: repeat_rule.map(str::to_string),
            delivery_channel: "in_app".to_string(),
            delivery_status: "delivered".to_string(),
            delivery_attempted_at: Some("2026-05-10T10:00:00Z".parse().unwrap()),
            delivered_at: Some("2026-05-10T10:01:00Z".parse().unwrap()),
            delivery_error: Some("prior transient error".to_string()),
            delivery_provenance: json!({"source": "api_in_app_poll", "status": "delivered"}),
            created_at: "2026-05-01T09:00:00Z".parse().unwrap(),
            completed_at: None,
        }
    }
}
