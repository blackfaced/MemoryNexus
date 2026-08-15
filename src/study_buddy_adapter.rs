//! Study Buddy's narrow provider adapter boundary.
//!
//! Provider records are read only from the authenticated loopback feed and are
//! mapped into the existing provider-neutral Source Evidence contract. Raw
//! provider payloads remain transient and never enter the durable adapter ledger.

use std::{collections::HashSet, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::foundation_learning_observation::FoundationMistakeType;
pub use crate::domain::foundation_learning_observation::{
    FOUNDATION_CORRECTION_GOAL, FOUNDATION_INITIAL_GOAL, FOUNDATION_REINFORCEMENT_GOAL,
};
use crate::reference_adapter::{
    AdapterError, NormalizedGatewayRequest, Normalizer, SourceClient, SourcePage, SourceRecord,
};

const SOURCE_PRODUCT: &str = "study_buddy";
const SOURCE_SCHEMA_VERSION: u16 = 1;
const FOUNDATION_NAMESPACE: &str = "learning.foundation";

pub fn loopback_http_client(
    endpoint: &str,
    expected_path: &str,
) -> Result<(reqwest::Client, Url), AdapterError> {
    let endpoint = Url::parse(endpoint)
        .map_err(|_| AdapterError::InvalidData("invalid loopback endpoint URL".to_string()))?;
    if endpoint.scheme() != "http"
        || !endpoint
            .host_str()
            .is_some_and(|host| matches!(host, "127.0.0.1" | "localhost" | "::1"))
        || endpoint.path() != expected_path
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(AdapterError::InvalidData(
            "endpoint must be plain loopback HTTP with the expected path".to_string(),
        ));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| AdapterError::InvalidData("failed to build loopback client".to_string()))?;
    Ok((client, endpoint))
}

#[derive(Debug, Clone)]
pub struct StudyBuddyAdapterConfig {
    actor_id: Uuid,
    space_id: Uuid,
    allowed_subject_refs: HashSet<String>,
    adapter_version: String,
}

impl StudyBuddyAdapterConfig {
    pub fn new(
        actor_id: Uuid,
        space_id: Uuid,
        allowed_subject_refs: HashSet<String>,
        adapter_version: String,
    ) -> Result<Self, AdapterError> {
        if actor_id.is_nil()
            || space_id.is_nil()
            || allowed_subject_refs.len() != 1
            || allowed_subject_refs
                .iter()
                .any(|value| Uuid::parse_str(value).is_err())
            || adapter_version.is_empty()
            || adapter_version.len() > 64
            || !adapter_version
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(AdapterError::InvalidData(
                "invalid Study Buddy adapter configuration".to_string(),
            ));
        }
        Ok(Self {
            actor_id,
            space_id,
            allowed_subject_refs,
            adapter_version,
        })
    }

    pub fn subject_ref(&self) -> &str {
        self.allowed_subject_refs
            .iter()
            .next()
            .expect("validated single-subject configuration")
    }
}

#[derive(Clone)]
pub struct StudyBuddySourceClient {
    client: reqwest::Client,
    feed_url: Url,
    token: String,
    subject_ref: String,
}

impl StudyBuddySourceClient {
    pub fn new(feed_url: &str, token: String, subject_ref: String) -> Result<Self, AdapterError> {
        if token.trim().is_empty() || Uuid::parse_str(&subject_ref).is_err() {
            return Err(AdapterError::InvalidData(
                "Study Buddy feed must be authenticated loopback HTTP".to_string(),
            ));
        }
        let (client, feed_url) = loopback_http_client(feed_url, "/api/integration/source-events")?;
        Ok(Self {
            client,
            feed_url,
            token,
            subject_ref,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyBuddySourcePage {
    event_schema_version: u16,
    events: Vec<Value>,
    page: StudyBuddyPageMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyBuddyPageMetadata {
    after: u64,
    next_cursor: u64,
    end_of_page: bool,
    end_of_feed: bool,
    has_more: bool,
}

#[async_trait]
impl SourceClient for StudyBuddySourceClient {
    async fn acquire_page(
        &self,
        after_cursor: Option<&str>,
        limit: usize,
    ) -> Result<SourcePage, AdapterError> {
        if limit == 0 || limit > 1_000 {
            return Err(AdapterError::InvalidData(
                "Study Buddy page limit is invalid".to_string(),
            ));
        }
        let after = after_cursor
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|_| AdapterError::InvalidData("invalid Study Buddy cursor".to_string()))?;
        let mut url = self.feed_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("limit", &limit.to_string());
            query.append_pair("schemaVersion", &SOURCE_SCHEMA_VERSION.to_string());
            query.append_pair("after", &after.to_string());
        }
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|_| {
                AdapterError::InvalidData("Study Buddy feed request failed".to_string())
            })?;
        if !response.status().is_success() {
            return Err(AdapterError::InvalidData(format!(
                "Study Buddy feed returned HTTP {}",
                response.status().as_u16()
            )));
        }
        let page: StudyBuddySourcePage = response.json().await.map_err(|_| {
            AdapterError::InvalidData("Study Buddy feed response is invalid".to_string())
        })?;
        if page.event_schema_version != SOURCE_SCHEMA_VERSION
            || page.events.len() > limit
            || page.page.after != after
            || page.page.next_cursor < after
            || !page.page.end_of_page
            || page.page.has_more == page.page.end_of_feed
        {
            return Err(AdapterError::InvalidData(
                "Study Buddy feed returned an inconsistent page".to_string(),
            ));
        }
        let mut previous_sequence = after;
        for event in &page.events {
            let sequence = event
                .get("sequence")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    AdapterError::InvalidData(
                        "Study Buddy event is missing a valid sequence".to_string(),
                    )
                })?;
            if sequence <= previous_sequence || sequence > page.page.next_cursor {
                return Err(AdapterError::InvalidData(
                    "Study Buddy event sequence is not strictly ordered".to_string(),
                ));
            }
            previous_sequence = sequence;
        }
        if (page.events.is_empty() && page.page.next_cursor != after)
            || (!page.events.is_empty() && previous_sequence != page.page.next_cursor)
        {
            return Err(AdapterError::InvalidData(
                "Study Buddy page cursor does not match its records".to_string(),
            ));
        }
        let mut records = page
            .events
            .into_iter()
            .map(|payload| {
                let event_id = payload
                    .get("eventId")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok())
                    .filter(|value| !value.is_nil())
                    .ok_or_else(|| {
                        AdapterError::InvalidData(
                            "Study Buddy event is missing a valid identity".to_string(),
                        )
                    })?;
                Ok(SourceRecord {
                    delivery_key: format!("event:{event_id}"),
                    payload,
                })
            })
            .collect::<Result<Vec<_>, AdapterError>>()?;
        // Raw provider records that are intentionally outside this Adapter's
        // evidence contract are terminally skipped only after their complete
        // supported v1 shape is validated. Unknown or partially-upgraded
        // records still fail closed rather than silently advancing the cursor.
        records = records
            .into_iter()
            .map(|record| {
                if terminally_skipped_v1_record(&record, &self.subject_ref)? {
                    Ok(None)
                } else {
                    Ok(Some(record))
                }
            })
            .collect::<Result<Vec<_>, AdapterError>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(SourcePage {
            records,
            next_cursor: Some(page.page.next_cursor.to_string()),
            has_more: page.page.has_more,
        })
    }
}

#[derive(Clone)]
pub struct StudyBuddyNormalizer {
    config: StudyBuddyAdapterConfig,
}

impl StudyBuddyNormalizer {
    pub fn new(config: StudyBuddyAdapterConfig) -> Self {
        Self { config }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyBuddyEvent {
    event_id: Uuid,
    event_type: String,
    event_schema_version: u16,
    occurred_at: DateTime<Utc>,
    subject_ref: String,
    source_identity: StudyBuddySourceIdentity,
    payload: Option<Value>,
    withdrawal_reason: Option<StudyBuddyWithdrawalReason>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyBuddySourceIdentity {
    source_product: String,
    source_installation_id: Uuid,
    record_type: String,
    record_id: String,
    revision: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyBuddyAttempt {
    kind: String,
    subject_ref: String,
    attempt_role: StudyBuddyAttemptRole,
    subject: String,
    problem: String,
    submitted_answer: String,
    expected_answer: String,
    mistake_type: Option<String>,
    is_correct: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StudyBuddyLegacyAttempt {
    kind: String,
    subject_ref: String,
    subject: String,
    problem: String,
    submitted_answer: String,
    expected_answer: Option<String>,
    mistake_type: Option<String>,
    source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StudyBuddyChatReference {
    kind: String,
    subject_ref: String,
    session_ref: String,
    turn_ref: String,
    role: String,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StudyBuddyLegacySession {
    Study(StudyBuddyLegacyStudySession),
    Game(StudyBuddyLegacyGameSession),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StudyBuddyLegacyStudySession {
    kind: String,
    subject_ref: String,
    session_kind: String,
    subject: Option<String>,
    started_at: f64,
    ended_at: f64,
    duration_minutes: f64,
    average_focus_score: f64,
    posture_warning_count: u32,
    off_topic_count: u32,
    off_topic_recovered: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StudyBuddyLegacyGameSession {
    kind: String,
    subject_ref: String,
    session_kind: String,
    app_id: String,
    started_at: f64,
    ended_at: f64,
    duration_minutes: f64,
    total_questions: u32,
    correct_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudyBuddySession {
    kind: String,
    subject_ref: String,
    session_kind: String,
    attempt_role: StudyBuddyAttemptRole,
    started_at: Value,
    ended_at: Value,
    activity_count: u32,
    successful_activity_count: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StudyBuddyAttemptRole {
    Original,
    Correction,
    Reinforcement,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StudyBuddyWithdrawalReason {
    CorrectedAtSource,
    DeletedAtSource,
    ConsentWithdrawn,
}

#[async_trait]
impl Normalizer for StudyBuddyNormalizer {
    async fn normalize(
        &self,
        record: SourceRecord,
    ) -> Result<NormalizedGatewayRequest, AdapterError> {
        let event: StudyBuddyEvent =
            serde_json::from_value(record.payload).map_err(|_| AdapterError::Normalization)?;
        if !valid_event_envelope(&event, &record.delivery_key)
            || !matches!(
                event.source_identity.record_type.as_str(),
                "learning_attempt" | "learning_session"
            )
        {
            return Err(AdapterError::Normalization);
        }

        let payload_subject_ref = event
            .payload
            .as_ref()
            .and_then(|payload| payload.get("subjectRef"))
            .and_then(Value::as_str);
        if payload_subject_ref
            .is_some_and(|payload_subject| event.subject_ref.as_str() != payload_subject)
        {
            return Err(AdapterError::Normalization);
        }
        let subject_ref = event.subject_ref;
        if !self.config.allowed_subject_refs.contains(&subject_ref) {
            return Err(AdapterError::Normalization);
        }

        let identity = json!({
            "source_product": event.source_identity.source_product,
            "source_installation_id": event.source_identity.source_installation_id,
            "record_type": event.source_identity.record_type,
            "record_id": event.source_identity.record_id,
            "revision": event.source_identity.revision,
        });
        let (evidence, tombstone) = match (
            event.source_identity.record_type.as_str(),
            event.event_type.as_str(),
        ) {
            ("learning_attempt", "learning_attempt_recorded" | "source_record_corrected") => {
                if !valid_content_revision(
                    &event.event_type,
                    event.source_identity.revision,
                    "learning_attempt_recorded",
                ) {
                    return Err(AdapterError::Normalization);
                }
                let attempt: StudyBuddyAttempt =
                    serde_json::from_value(event.payload.ok_or(AdapterError::Normalization)?)
                        .map_err(|_| AdapterError::Normalization)?;
                if attempt.kind != "learning_attempt"
                    || attempt.subject_ref != subject_ref
                    || !bounded(&attempt.subject, 80)
                    || !bounded(&attempt.problem, 200)
                    || !bounded(&attempt.submitted_answer, 120)
                    || !bounded(&attempt.expected_answer, 120)
                    || attempt
                        .mistake_type
                        .as_ref()
                        .is_some_and(|value| !bounded(value, 120))
                    || attempt.is_correct == attempt.mistake_type.is_some()
                {
                    return Err(AdapterError::Normalization);
                }
                let summary = match attempt.attempt_role {
                    StudyBuddyAttemptRole::Original => "Initial foundation attempt recorded",
                    StudyBuddyAttemptRole::Correction => "Foundation correction attempt recorded",
                    StudyBuddyAttemptRole::Reinforcement => {
                        "Foundation reinforcement attempt recorded"
                    }
                };
                let goal = attempt_role_goal(attempt.attempt_role);
                let mistake = (!attempt.is_correct)
                    .then(|| {
                        let mistake_type = normalize_study_buddy_mistake_type(
                            attempt.mistake_type.as_deref().expect("validated above"),
                        )
                        .ok_or(AdapterError::Normalization)?;
                        Ok::<_, AdapterError>(json!({
                            "expected_text": attempt.expected_answer,
                            "actual_text": attempt.submitted_answer,
                            "mistake_type": mistake_type,
                        }))
                    })
                    .transpose()?;
                (
                    Some(json!({
                        "kind": "learning_attempt",
                        "goal": goal,
                        "task": attempt.problem,
                        "summary": summary,
                        "mistake": mistake,
                        "input_source": null,
                        "input_confirmation": null,
                    })),
                    None,
                )
            }
            ("learning_session", "learning_session_completed" | "source_record_corrected") => {
                if !valid_content_revision(
                    &event.event_type,
                    event.source_identity.revision,
                    "learning_session_completed",
                ) {
                    return Err(AdapterError::Normalization);
                }
                let session: StudyBuddySession =
                    serde_json::from_value(event.payload.ok_or(AdapterError::Normalization)?)
                        .map_err(|_| AdapterError::Normalization)?;
                if session.kind != "learning_session"
                    || session.subject_ref != subject_ref
                    || !matches!(session.session_kind.as_str(), "study" | "game" | "practice")
                {
                    return Err(AdapterError::Normalization);
                }
                let started_at = provider_datetime(&session.started_at)?;
                let ended_at = provider_datetime(&session.ended_at)?;
                if started_at > ended_at || ended_at > event.occurred_at {
                    return Err(AdapterError::Normalization);
                }
                if session.successful_activity_count > session.activity_count {
                    return Err(AdapterError::Normalization);
                }
                let goal = attempt_role_goal(session.attempt_role);
                (
                    Some(json!({
                        "kind": "learning_session",
                        "goal": goal,
                        "task": "Complete a bounded foundation learning session",
                        "started_at": started_at,
                        "ended_at": ended_at,
                        "activity_count": session.activity_count,
                        "successful_activity_count": session.successful_activity_count,
                    })),
                    None,
                )
            }
            ("learning_attempt" | "learning_session", "source_record_withdrawn") => {
                if event.payload.is_some() || event.source_identity.revision < 2 {
                    return Err(AdapterError::Normalization);
                }
                let reason = event
                    .withdrawal_reason
                    .unwrap_or(StudyBuddyWithdrawalReason::DeletedAtSource);
                (
                    None,
                    Some(json!({
                        "withdrawn_at": event.occurred_at,
                        "reason": match reason {
                            StudyBuddyWithdrawalReason::CorrectedAtSource => "corrected_at_source",
                            StudyBuddyWithdrawalReason::DeletedAtSource => "deleted_at_source",
                            StudyBuddyWithdrawalReason::ConsentWithdrawn => "consent_withdrawn",
                        }
                    })),
                )
            }
            _ => return Err(AdapterError::Normalization),
        };

        let normalized = json!({
            "namespace": FOUNDATION_NAMESPACE,
            "surface": "performance",
            "action": "submit_attempt",
            "actor": self.config.actor_id,
            "adapter": "mcp",
            "payload": {
                "space_id": self.config.space_id,
                "source_evidence": {
                    "contract_version": 1,
                    "source_identity": identity,
                    "occurred_at": event.occurred_at,
                    "observed_at": event.occurred_at,
                    "evidence_trust": "contract_trusted",
                    "provenance": {
                        "adapter_id": "study_buddy_reference_adapter",
                        "adapter_version": self.config.adapter_version,
                        "source_schema_version": event.event_schema_version.to_string(),
                    },
                    "evidence": evidence,
                    "tombstone": tombstone,
                }
            },
            "context": {
                "mode": "fast",
                "locale": null,
                "device": null,
                "runtime_preference": "deterministic",
            }
        });
        serde_json::from_value(normalized).map_err(|_| AdapterError::Normalization)
    }
}

fn terminally_skipped_v1_record(
    record: &SourceRecord,
    allowed_subject_ref: &str,
) -> Result<bool, AdapterError> {
    let event: StudyBuddyEvent = serde_json::from_value(record.payload.clone()).map_err(|_| {
        AdapterError::InvalidData("Study Buddy event envelope is invalid".to_string())
    })?;
    if !valid_event_envelope(&event, &record.delivery_key) {
        return Err(AdapterError::InvalidData(
            "Study Buddy event envelope is invalid".to_string(),
        ));
    }
    if event.subject_ref != allowed_subject_ref {
        return Err(AdapterError::InvalidData(
            "Study Buddy event subject is not authorized".to_string(),
        ));
    }
    let payload = event.payload.as_ref();
    match event.source_identity.record_type.as_str() {
        "chat_turn" => {
            if event.event_type == "source_record_withdrawn"
                && event.source_identity.revision >= 2
                && payload.is_none()
                && valid_chat_turn_ref(&event.source_identity.record_id)
            {
                return Ok(true);
            }
            if event.event_type != "chat_turn_recorded" || event.source_identity.revision != 1 {
                return Err(AdapterError::InvalidData(
                    "Study Buddy chat event semantics are invalid".to_string(),
                ));
            }
            let chat: StudyBuddyChatReference =
                serde_json::from_value(payload.cloned().ok_or_else(|| {
                    AdapterError::InvalidData("Study Buddy chat reference is missing".to_string())
                })?)
                .map_err(|_| {
                    AdapterError::InvalidData("Study Buddy chat reference is invalid".to_string())
                })?;
            if chat.kind != "chat_turn_reference"
                || chat.subject_ref != allowed_subject_ref
                || !valid_prefixed_ref(&chat.session_ref, "session:")
                || chat.turn_ref != event.source_identity.record_id
                || !valid_chat_turn_ref(&chat.turn_ref)
                || !matches!(chat.role.as_str(), "child" | "agent")
                || chat.occurred_at != event.occurred_at
            {
                return Err(AdapterError::InvalidData(
                    "Study Buddy chat event semantics are invalid".to_string(),
                ));
            }
            Ok(true)
        }
        "learning_attempt"
            if payload.is_some_and(|value| {
                value.get("attemptRole").is_none() && value.get("isCorrect").is_none()
            }) =>
        {
            if event.event_type != "learning_attempt_recorded"
                || event.source_identity.revision != 1
            {
                return Err(AdapterError::InvalidData(
                    "Study Buddy legacy attempt semantics are invalid".to_string(),
                ));
            }
            let attempt: StudyBuddyLegacyAttempt =
                serde_json::from_value(payload.cloned().expect("matched present payload"))
                    .map_err(|_| {
                        AdapterError::InvalidData(
                            "Study Buddy legacy attempt is invalid".to_string(),
                        )
                    })?;
            if attempt.kind != "learning_attempt"
                || attempt.subject_ref != allowed_subject_ref
                || !bounded(&attempt.subject, 80)
                || !bounded(&attempt.problem, 200)
                || !bounded_allow_empty(&attempt.submitted_answer, 120)
                || attempt
                    .expected_answer
                    .as_ref()
                    .is_some_and(|value| !bounded_allow_empty(value, 120))
                || attempt
                    .mistake_type
                    .as_ref()
                    .is_some_and(|value| !bounded_allow_empty(value, 120))
                || !bounded(&attempt.source, 80)
            {
                return Err(AdapterError::InvalidData(
                    "Study Buddy legacy attempt semantics are invalid".to_string(),
                ));
            }
            Ok(true)
        }
        "learning_session"
            if payload.is_some_and(|value| {
                value.get("attemptRole").is_none()
                    && value.get("activityCount").is_none()
                    && value.get("successfulActivityCount").is_none()
            }) =>
        {
            let valid_revision = match event.event_type.as_str() {
                "learning_session_completed" => event.source_identity.revision == 1,
                "source_record_corrected" => event.source_identity.revision >= 2,
                _ => false,
            };
            if !valid_revision
                || !payload
                    .is_some_and(|value| valid_legacy_session_payload(value, allowed_subject_ref))
            {
                return Err(AdapterError::InvalidData(
                    "Study Buddy legacy session semantics are invalid".to_string(),
                ));
            }
            Ok(true)
        }
        "learning_attempt" | "learning_session" => Ok(false),
        _ => Err(AdapterError::InvalidData(
            "Study Buddy record type is unsupported".to_string(),
        )),
    }
}

fn valid_event_envelope(event: &StudyBuddyEvent, delivery_key: &str) -> bool {
    delivery_key == format!("event:{}", event.event_id)
        && event.event_schema_version == SOURCE_SCHEMA_VERSION
        && event.source_identity.source_product == SOURCE_PRODUCT
        && !event.source_identity.source_installation_id.is_nil()
        && matches!(
            event.source_identity.record_type.as_str(),
            "learning_attempt" | "learning_session" | "chat_turn"
        )
        && Uuid::parse_str(&event.subject_ref).is_ok()
        && bounded(&event.source_identity.record_id, 160)
        && event.source_identity.revision >= 1
}

fn valid_content_revision(event_type: &str, revision: i64, recorded_event_type: &str) -> bool {
    (event_type == recorded_event_type && revision == 1)
        || (event_type == "source_record_corrected" && revision >= 2)
}

fn valid_legacy_session_payload(payload: &Value, allowed_subject_ref: &str) -> bool {
    let Ok(session) = serde_json::from_value::<StudyBuddyLegacySession>(payload.clone()) else {
        return false;
    };
    match session {
        StudyBuddyLegacySession::Study(session) => {
            session.kind == "learning_session"
                && session.session_kind == "study"
                && session.subject_ref == allowed_subject_ref
                && session
                    .subject
                    .as_deref()
                    .map_or(true, |value| bounded(value, 80))
                && valid_nonnegative_f64(session.started_at)
                && valid_nonnegative_f64(session.ended_at)
                && session.ended_at >= session.started_at
                && valid_nonnegative_f64(session.duration_minutes)
                && valid_nonnegative_f64(session.average_focus_score)
                && session.posture_warning_count <= 1_000_000
                && session.off_topic_count <= 1_000_000
                && session.off_topic_recovered <= session.off_topic_count
        }
        StudyBuddyLegacySession::Game(session) => {
            session.kind == "learning_session"
                && session.session_kind == "game"
                && session.subject_ref == allowed_subject_ref
                && bounded(&session.app_id, 80)
                && valid_nonnegative_f64(session.started_at)
                && valid_nonnegative_f64(session.ended_at)
                && session.ended_at >= session.started_at
                && valid_nonnegative_f64(session.duration_minutes)
                && session.total_questions > 0
                && session.correct_count <= session.total_questions
        }
    }
}

fn valid_nonnegative_f64(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_prefixed_ref(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix.len() <= 128
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    })
}

fn valid_chat_turn_ref(value: &str) -> bool {
    value.strip_prefix("chat_turn:").is_some_and(|suffix| {
        !suffix.starts_with('0')
            && suffix.bytes().all(|byte| byte.is_ascii_digit())
            && suffix
                .parse::<u64>()
                .is_ok_and(|value| value <= 9_007_199_254_740_991)
    })
}

fn bounded(value: &str, max_len: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_len && !value.chars().any(char::is_control)
}

fn bounded_allow_empty(value: &str, max_len: usize) -> bool {
    value.len() <= max_len && !value.chars().any(char::is_control)
}

fn attempt_role_goal(role: StudyBuddyAttemptRole) -> &'static str {
    match role {
        StudyBuddyAttemptRole::Original => FOUNDATION_INITIAL_GOAL,
        StudyBuddyAttemptRole::Correction => FOUNDATION_CORRECTION_GOAL,
        StudyBuddyAttemptRole::Reinforcement => FOUNDATION_REINFORCEMENT_GOAL,
    }
}

fn normalize_study_buddy_mistake_type(value: &str) -> Option<FoundationMistakeType> {
    match value {
        "compute" => Some(FoundationMistakeType::ArithmeticComputation),
        "carry" => Some(FoundationMistakeType::PlaceValueCarry),
        "borrow" => Some(FoundationMistakeType::PlaceValueBorrow),
        "multiply" => Some(FoundationMistakeType::MultiplicationFact),
        "sign" => Some(FoundationMistakeType::OperationSign),
        "审题" => Some(FoundationMistakeType::TaskInterpretation),
        "钟表" => Some(FoundationMistakeType::TimeReading),
        "应用题" => Some(FoundationMistakeType::WordProblemModeling),
        "confirmed" => Some(FoundationMistakeType::UnclassifiedConfirmedError),
        _ => None,
    }
}

fn provider_datetime(value: &Value) -> Result<DateTime<Utc>, AdapterError> {
    if let Some(milliseconds) = value.as_i64() {
        return DateTime::from_timestamp_millis(milliseconds).ok_or(AdapterError::Normalization);
    }
    value
        .as_str()
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|value| value.with_timezone(&Utc))
        .ok_or(AdapterError::Normalization)
}
