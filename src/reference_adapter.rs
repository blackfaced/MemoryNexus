//! Crash-safe, provider-neutral reference Adapter runtime.
//!
//! The ledger contains normalized delivery jobs and safe acknowledgements only.
//! Source and Gateway credentials stay in their clients and are never persisted.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
    time::Duration as StdDuration,
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::surface::{Surface, SurfaceAction, SurfaceAdapter, SurfaceContext};

const EXTERNAL_CALL_TIMEOUT: StdDuration = StdDuration::from_secs(60);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NormalizedGatewayRequest {
    namespace: String,
    surface: Surface,
    action: SurfaceAction,
    actor: Uuid,
    adapter: SurfaceAdapter,
    payload: NormalizedGatewayPayload,
    context: SurfaceContext,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedGatewayPayload {
    space_id: Uuid,
    source_evidence: NormalizedSourceEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedSourceEvidence {
    contract_version: u16,
    source_identity: NormalizedSourceIdentity,
    occurred_at: DateTime<Utc>,
    observed_at: DateTime<Utc>,
    evidence_trust: NormalizedEvidenceTrust,
    provenance: NormalizedProvenance,
    evidence: Option<NormalizedEvidenceBody>,
    tombstone: Option<NormalizedTombstone>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct NormalizedSourceIdentity {
    source_product: String,
    source_installation_id: Uuid,
    record_type: String,
    record_id: String,
    revision: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum NormalizedEvidenceTrust {
    ContractTrusted,
    ModelDerivedUnreviewed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedProvenance {
    adapter_id: String,
    adapter_version: String,
    source_schema_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NormalizedEvidenceBody {
    LearningAttempt(NormalizedLearningAttempt),
    LearningSession(NormalizedLearningSession),
    LearnerJourneySummary(NormalizedLearnerJourneySummary),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedLearningAttempt {
    goal: Option<String>,
    task: String,
    summary: String,
    mistake: Option<NormalizedLearningMistake>,
    input_source: Option<String>,
    input_confirmation: Option<NormalizedInputConfirmation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedLearningMistake {
    expected_text: String,
    actual_text: String,
    mistake_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedInputConfirmation {
    status: NormalizedConfirmationStatus,
    method: NormalizedConfirmationMethod,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum NormalizedConfirmationStatus {
    Confirmed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum NormalizedConfirmationMethod {
    ExplicitAcceptance,
    ExplicitCorrection,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedLearningSession {
    goal: Option<String>,
    task: String,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
    activity_count: u32,
    successful_activity_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedLearnerJourneySummary {
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    summary: String,
    strengths: Vec<String>,
    next_steps: Vec<String>,
    source_refs: Vec<NormalizedSourceIdentity>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct NormalizedTombstone {
    withdrawn_at: DateTime<Utc>,
    reason: NormalizedWithdrawalReason,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum NormalizedWithdrawalReason {
    CorrectedAtSource,
    DeletedAtSource,
    ConsentWithdrawn,
}

impl NormalizedGatewayRequest {
    fn source_identity(&self) -> &NormalizedSourceIdentity {
        &self.payload.source_evidence.source_identity
    }

    fn validate_for_ledger(&self) -> Result<(), AdapterError> {
        let envelope = &self.payload.source_evidence;
        if self.surface != Surface::Performance
            || self.action != SurfaceAction::SubmitAttempt
            || self.action.surface() != self.surface
            || envelope.contract_version != 1
            || envelope.source_identity.revision < 1
            || envelope.source_identity.source_installation_id.is_nil()
            || self.actor.is_nil()
            || self.payload.space_id.is_nil()
            || envelope.occurred_at > envelope.observed_at
            || envelope.evidence.is_some() == envelope.tombstone.is_some()
            || !valid_identifier(&self.namespace, 128)
            || !valid_normalized_identity(&envelope.source_identity)
            || !valid_identifier(&envelope.provenance.adapter_id, 64)
            || !valid_identifier(&envelope.provenance.adapter_version, 64)
            || !valid_identifier(&envelope.provenance.source_schema_version, 64)
        {
            return Err(AdapterError::InvalidData(
                "normalized request violates the closed Source Evidence contract".to_string(),
            ));
        }
        match (&envelope.evidence, &envelope.tombstone) {
            (Some(NormalizedEvidenceBody::LearningAttempt(evidence)), None) => {
                if envelope.evidence_trust != NormalizedEvidenceTrust::ContractTrusted
                    || !bounded_text(&evidence.task, 200)
                    || !bounded_text(&evidence.summary, 400)
                    || evidence
                        .goal
                        .as_ref()
                        .is_some_and(|value| !bounded_text(value, 200))
                    || evidence.mistake.as_ref().is_some_and(|mistake| {
                        [
                            &mistake.expected_text,
                            &mistake.actual_text,
                            &mistake.mistake_type,
                        ]
                        .into_iter()
                        .any(|value| !bounded_text(value, 120))
                    })
                    || !valid_input_confirmation(
                        evidence.input_source.as_deref(),
                        evidence.input_confirmation.as_ref(),
                    )
                {
                    return Err(AdapterError::InvalidData(
                        "normalized Learning Attempt is invalid".to_string(),
                    ));
                }
            }
            (Some(NormalizedEvidenceBody::LearningSession(evidence)), None) => {
                if envelope.evidence_trust != NormalizedEvidenceTrust::ContractTrusted
                    || !bounded_text(&evidence.task, 200)
                    || evidence.started_at > evidence.ended_at
                    || evidence.successful_activity_count > evidence.activity_count
                    || evidence.activity_count > 10_000
                    || evidence
                        .goal
                        .as_ref()
                        .is_some_and(|value| !bounded_text(value, 200))
                {
                    return Err(AdapterError::InvalidData(
                        "normalized Learning Session is invalid".to_string(),
                    ));
                }
            }
            (Some(NormalizedEvidenceBody::LearnerJourneySummary(summary)), None) => {
                if envelope.evidence_trust != NormalizedEvidenceTrust::ModelDerivedUnreviewed
                    || summary.period_start > summary.period_end
                    || !bounded_text(&summary.summary, 1_000)
                    || summary.strengths.len() > 10
                    || summary.next_steps.len() > 10
                    || summary.source_refs.is_empty()
                    || summary.source_refs.len() > 100
                    || summary
                        .strengths
                        .iter()
                        .chain(&summary.next_steps)
                        .any(|value| !bounded_text(value, 200))
                    || summary
                        .source_refs
                        .iter()
                        .any(|source_ref| !valid_normalized_identity(source_ref))
                    || summary.source_refs.iter().collect::<HashSet<_>>().len()
                        != summary.source_refs.len()
                {
                    return Err(AdapterError::InvalidData(
                        "normalized Learner Journey Summary is invalid".to_string(),
                    ));
                }
            }
            (None, Some(tombstone)) => {
                if envelope.evidence_trust != NormalizedEvidenceTrust::ContractTrusted
                    || tombstone.withdrawn_at < envelope.occurred_at
                {
                    return Err(AdapterError::InvalidData(
                        "normalized Source Tombstone is invalid".to_string(),
                    ));
                }
            }
            _ => unreachable!("evidence/tombstone shape was validated"),
        }
        let serialized = serde_json::to_value(self)
            .map_err(|error| AdapterError::InvalidData(error.to_string()))?;
        if serde_json::to_vec(&serialized)
            .map_err(|error| AdapterError::InvalidData(error.to_string()))?
            .len()
            > 64 * 1024
            || contains_secret_value(&serialized)
        {
            return Err(AdapterError::InvalidData(
                "normalized request is oversized or contains secret-shaped text".to_string(),
            ));
        }
        Ok(())
    }
}

fn valid_normalized_identity(identity: &NormalizedSourceIdentity) -> bool {
    identity.revision > 0
        && !identity.source_installation_id.is_nil()
        && valid_identifier(&identity.source_product, 64)
        && valid_identifier(&identity.record_type, 64)
        && valid_identifier(&identity.record_id, 128)
}

fn valid_identifier(value: &str, max_len: usize) -> bool {
    let bytes = value.as_bytes();
    (1..=max_len).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn bounded_text(value: &str, max_len: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_len && !value.chars().any(char::is_control)
}

fn valid_input_confirmation(
    input_source: Option<&str>,
    confirmation: Option<&NormalizedInputConfirmation>,
) -> bool {
    matches!(
        (input_source, confirmation),
        (None | Some("typed" | "pasted"), None)
            | (
                Some("agent_ocr"),
                Some(NormalizedInputConfirmation {
                    status: NormalizedConfirmationStatus::Confirmed,
                    method: NormalizedConfirmationMethod::ExplicitAcceptance
                        | NormalizedConfirmationMethod::ExplicitCorrection,
                }),
            )
    )
}

fn contains_secret_value(value: &Value) -> bool {
    match value {
        Value::String(text) => is_secret_like(text),
        Value::Array(values) => values.iter().any(contains_secret_value),
        Value::Object(values) => values.values().any(contains_secret_value),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn is_secret_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("bearer ")
        || value.starts_with("-----BEGIN PRIVATE KEY-----")
        || value.starts_with("sk-")
        || value.starts_with("AKIA")
        || value.starts_with("AIza")
        || value.starts_with("ghp_")
        || lower.contains("x-amz-signature=")
        || lower.contains("x-amz-credential=")
        || lower.contains("sig=")
        || {
            let parts = value.split('.').collect::<Vec<_>>();
            parts.len() == 3
                && parts.iter().all(|part| {
                    part.len() >= 8
                        && part.chars().all(|character| {
                            character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                        })
                })
        }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    /// Stable provider-native identity including revision.
    pub delivery_key: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourcePage {
    pub records: Vec<SourceRecord>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayAcknowledgement {
    pub status: GatewayAcknowledgementStatus,
    pub source_identity: NormalizedSourceIdentity,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GatewayAcknowledgementStatus {
    Accepted,
    Replayed,
}

#[async_trait]
pub trait SourceClient: Send + Sync {
    async fn acquire_page(
        &self,
        after_cursor: Option<&str>,
        limit: usize,
    ) -> Result<SourcePage, AdapterError>;
}

#[async_trait]
pub trait Normalizer: Send + Sync {
    async fn normalize(
        &self,
        record: SourceRecord,
    ) -> Result<NormalizedGatewayRequest, AdapterError>;
}

#[async_trait]
pub trait GatewayClient: Send + Sync {
    async fn deliver(
        &self,
        payload: &NormalizedGatewayRequest,
    ) -> Result<GatewayAcknowledgement, AdapterError>;
}

pub trait AdapterClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl AdapterClock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("source acquisition failed")]
    Acquisition,
    #[error("source normalization failed")]
    Normalization,
    #[error("gateway delivery failed")]
    Delivery,
    #[error("adapter lease is held by another runner")]
    LeaseHeld,
    #[error("adapter ledger failed: {0}")]
    Ledger(#[from] sqlx::Error),
    #[error("adapter data is invalid: {0}")]
    InvalidData(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    pub acquired: usize,
    pub acknowledged: usize,
    pub cursor: Option<String>,
    pub has_more: bool,
}

pub struct ReferenceAdapter<S, N, G, C = SystemClock> {
    ledger: SqlitePool,
    source: Arc<S>,
    normalizer: Arc<N>,
    gateway: Arc<G>,
    clock: Arc<C>,
    page_limit: usize,
}

impl<S, N, G, C> ReferenceAdapter<S, N, G, C>
where
    S: SourceClient,
    N: Normalizer,
    G: GatewayClient,
    C: AdapterClock,
{
    pub async fn open(
        ledger_url: &str,
        source: Arc<S>,
        normalizer: Arc<N>,
        gateway: Arc<G>,
        clock: Arc<C>,
        page_limit: usize,
    ) -> Result<Self, AdapterError> {
        if page_limit == 0 || page_limit > 1_000 {
            return Err(AdapterError::InvalidData(
                "page_limit must be between 1 and 1000".to_string(),
            ));
        }
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(ledger_url)?
            .create_if_missing(true)
            .foreign_keys(true);
        let ledger = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        initialize_ledger(&ledger).await?;
        Ok(Self {
            ledger,
            source,
            normalizer,
            gateway,
            clock,
            page_limit,
        })
    }

    /// Processes one durable page. An unfinished ledger page is always resumed
    /// before the Source cursor is used to acquire more work.
    pub async fn run_one_page(&self) -> Result<RunSummary, AdapterError> {
        let lease_owner = Uuid::new_v4();
        self.acquire_lease(lease_owner).await?;
        let result = self.run_one_page_with_lease(lease_owner).await;
        let release = self.release_lease(lease_owner).await;
        match (result, release) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(summary), Ok(())) => Ok(summary),
        }
    }

    async fn run_one_page_with_lease(&self, lease_owner: Uuid) -> Result<RunSummary, AdapterError> {
        self.acquire_lease(lease_owner).await?;
        let cursor = self.cursor().await?;
        let existing_page = sqlx::query(
            "SELECT cursor_before, cursor_after, has_more FROM adapter_pages WHERE state != 'cursor_committed' ORDER BY acquired_at, cursor_before LIMIT 1",
        )
        .fetch_optional(&self.ledger)
        .await?;

        let (cursor_before, cursor_after, has_more, acquired, acquired_records) = if let Some(
            page,
        ) =
            existing_page
        {
            let cursor_before = page.get::<String, _>("cursor_before");
            let cursor_after = page.get::<Option<String>, _>("cursor_after");
            let has_more = page.get::<i64, _>("has_more") != 0;
            let normalization_pending: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM adapter_delivery_jobs WHERE cursor_before = ?1 AND state = 'normalization_pending'",
            )
            .bind(&cursor_before)
            .fetch_one(&self.ledger)
            .await?;
            let records = if normalization_pending > 0 {
                let resumed = self
                    .timed_source_page(empty_as_none(&cursor_before))
                    .await?;
                self.acquire_lease(lease_owner).await?;
                if resumed.records.len() > self.page_limit
                    || resumed.next_cursor != cursor_after
                    || resumed.has_more != has_more
                {
                    return Err(AdapterError::InvalidData(
                        "source page changed while normalization was pending".to_string(),
                    ));
                }
                let expected_keys = sqlx::query_scalar::<_, String>(
                    "SELECT delivery_key FROM adapter_page_manifest WHERE cursor_before = ?1 ORDER BY ordinal",
                )
                .bind(&cursor_before)
                .fetch_all(&self.ledger)
                .await?;
                let actual_keys = resumed
                    .records
                    .iter()
                    .map(|record| record.delivery_key.clone())
                    .collect::<Vec<_>>();
                let unique_keys = actual_keys.iter().collect::<HashSet<_>>();
                if actual_keys != expected_keys
                    || unique_keys.len() != actual_keys.len()
                    || actual_keys.iter().any(|key| !valid_delivery_key(key))
                {
                    return Err(AdapterError::InvalidData(
                        "resumed source page delivery keys changed".to_string(),
                    ));
                }
                Some(resumed.records)
            } else {
                None
            };
            (cursor_before, cursor_after, has_more, 0, records)
        } else {
            let page = self.timed_source_page(cursor.as_deref()).await?;
            self.acquire_lease(lease_owner).await?;
            if page.records.len() > self.page_limit {
                return Err(AdapterError::InvalidData(
                    "source returned an oversized page".to_string(),
                ));
            }
            let mut delivery_keys = HashSet::with_capacity(page.records.len());
            for record in &page.records {
                if !valid_delivery_key(&record.delivery_key)
                    || !delivery_keys.insert(record.delivery_key.clone())
                {
                    return Err(AdapterError::InvalidData(
                        "source page contains an invalid or duplicate delivery key".to_string(),
                    ));
                }
            }
            let cursor_before = cursor.clone().unwrap_or_default();
            let mut tx = self.ledger.begin().await?;
            self.fence_lease_in_tx(&mut tx, lease_owner).await?;
            sqlx::query(
                    "INSERT INTO adapter_pages (cursor_before, cursor_after, has_more, state, acquired_at) VALUES (?1,?2,?3,'acquired',?4)",
                )
                .bind(&cursor_before)
                .bind(&page.next_cursor)
                .bind(page.has_more)
                .bind(self.clock.now().to_rfc3339())
                .execute(&mut *tx)
                .await?;
            for (ordinal, record) in page.records.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO adapter_page_manifest (cursor_before, ordinal, delivery_key) VALUES (?1,?2,?3)",
                )
                .bind(&cursor_before)
                .bind(ordinal as i64)
                .bind(&record.delivery_key)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                        "INSERT INTO adapter_delivery_jobs (delivery_key, cursor_before, ordinal, normalized_payload, expected_source_identity, state) VALUES (?1,?2,?3,NULL,NULL,'normalization_pending') ON CONFLICT(delivery_key) DO NOTHING",
                    )
                    .bind(&record.delivery_key)
                    .bind(&cursor_before)
                    .bind(ordinal as i64)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            let acquired = page.records.len();
            (
                cursor_before,
                page.next_cursor,
                page.has_more,
                acquired,
                Some(page.records),
            )
        };

        if let Some(records) = acquired_records {
            self.normalize_pending_records(lease_owner, &cursor_before, records)
                .await?;
        }

        let jobs = sqlx::query(
            "SELECT delivery_key, normalized_payload, expected_source_identity FROM adapter_delivery_jobs WHERE cursor_before = ?1 AND state = 'pending' ORDER BY ordinal",
        )
        .bind(&cursor_before)
        .fetch_all(&self.ledger)
        .await?;
        let mut acknowledged = 0;
        for job in jobs {
            self.acquire_lease(lease_owner).await?;
            let delivery_key = job.get::<String, _>("delivery_key");
            let payload: NormalizedGatewayRequest =
                serde_json::from_str(job.get::<String, _>("normalized_payload").as_str())
                    .map_err(|error| AdapterError::InvalidData(error.to_string()))?;
            payload.validate_for_ledger()?;
            let expected_source_identity: NormalizedSourceIdentity =
                serde_json::from_str(job.get::<String, _>("expected_source_identity").as_str())
                    .map_err(|error| AdapterError::InvalidData(error.to_string()))?;
            sqlx::query(
                "INSERT INTO adapter_delivery_attempts (delivery_key, attempted_at, outcome) VALUES (?1,?2,'started')",
            )
            .bind(&delivery_key)
            .bind(self.clock.now().to_rfc3339())
            .execute(&self.ledger)
            .await?;
            let acknowledgement =
                tokio::time::timeout(EXTERNAL_CALL_TIMEOUT, self.gateway.deliver(&payload))
                    .await
                    .map_err(|_| AdapterError::Delivery)?
                    .map_err(|_| AdapterError::Delivery)?;
            self.acquire_lease(lease_owner).await?;
            if acknowledgement.source_identity != expected_source_identity {
                let mut tx = self.ledger.begin().await?;
                self.fence_lease_in_tx(&mut tx, lease_owner).await?;
                sqlx::query("UPDATE adapter_delivery_attempts SET outcome = 'identity_mismatch' WHERE id = (SELECT MAX(id) FROM adapter_delivery_attempts WHERE delivery_key = ?1)")
                    .bind(&delivery_key)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                return Err(AdapterError::InvalidData(
                    "Gateway acknowledgement Source Identity does not match the delivery job"
                        .to_string(),
                ));
            }
            let acknowledgement_json = serde_json::to_string(&acknowledgement)
                .map_err(|error| AdapterError::InvalidData(error.to_string()))?;
            let mut tx = self.ledger.begin().await?;
            self.fence_lease_in_tx(&mut tx, lease_owner).await?;
            sqlx::query(
                "INSERT INTO adapter_acknowledgements (delivery_key, acknowledgement, received_at) VALUES (?1,?2,?3) ON CONFLICT(delivery_key) DO NOTHING",
            )
            .bind(&delivery_key)
            .bind(acknowledgement_json)
            .bind(self.clock.now().to_rfc3339())
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE adapter_delivery_jobs SET state = 'acknowledged' WHERE delivery_key = ?1",
            )
            .bind(&delivery_key)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE adapter_delivery_attempts SET outcome = 'acknowledged' WHERE id = (SELECT MAX(id) FROM adapter_delivery_attempts WHERE delivery_key = ?1)")
                .bind(&delivery_key)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            acknowledged += 1;
        }

        let pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM adapter_delivery_jobs WHERE cursor_before = ?1 AND state != 'acknowledged'",
        )
        .bind(&cursor_before)
        .fetch_one(&self.ledger)
        .await?;
        if pending == 0 {
            self.acquire_lease(lease_owner).await?;
            let mut tx = self.ledger.begin().await?;
            self.fence_lease_in_tx(&mut tx, lease_owner).await?;
            sqlx::query("UPDATE adapter_state SET acknowledged_cursor = ?1 WHERE singleton = 1")
                .bind(&cursor_after)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE adapter_pages SET state = 'cursor_committed' WHERE cursor_before = ?1",
            )
            .bind(&cursor_before)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }

        Ok(RunSummary {
            acquired,
            acknowledged,
            cursor: self.cursor().await?,
            has_more,
        })
    }

    pub async fn run_bounded(&self, max_pages: usize) -> Result<RunSummary, AdapterError> {
        if max_pages == 0 {
            return Err(AdapterError::InvalidData(
                "max_pages must be positive".to_string(),
            ));
        }
        let lease_owner = Uuid::new_v4();
        self.acquire_lease(lease_owner).await?;
        let mut total = RunSummary {
            acquired: 0,
            acknowledged: 0,
            cursor: self.cursor().await?,
            has_more: true,
        };
        let outcome = async {
            for _ in 0..max_pages {
                let page = self.run_one_page_with_lease(lease_owner).await?;
                total.acquired += page.acquired;
                total.acknowledged += page.acknowledged;
                total.cursor = page.cursor;
                total.has_more = page.has_more;
                if !page.has_more {
                    break;
                }
            }
            Ok(total)
        }
        .await;
        let release = self.release_lease(lease_owner).await;
        match (outcome, release) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(summary), Ok(())) => Ok(summary),
        }
    }

    async fn normalize_pending_records(
        &self,
        lease_owner: Uuid,
        cursor_before: &str,
        records: Vec<SourceRecord>,
    ) -> Result<(), AdapterError> {
        let mut records = records
            .into_iter()
            .map(|record| (record.delivery_key.clone(), record))
            .collect::<HashMap<_, _>>();
        let pending_keys = sqlx::query_scalar::<_, String>(
            "SELECT delivery_key FROM adapter_delivery_jobs WHERE cursor_before = ?1 AND state = 'normalization_pending' ORDER BY ordinal",
        )
        .bind(cursor_before)
        .fetch_all(&self.ledger)
        .await?;
        for delivery_key in pending_keys {
            self.acquire_lease(lease_owner).await?;
            let record = records.remove(&delivery_key).ok_or_else(|| {
                AdapterError::InvalidData(
                    "pending normalization record is absent from the stable source page"
                        .to_string(),
                )
            })?;
            let payload =
                tokio::time::timeout(EXTERNAL_CALL_TIMEOUT, self.normalizer.normalize(record))
                    .await
                    .map_err(|_| AdapterError::Normalization)?
                    .map_err(|_| AdapterError::Normalization)?;
            self.acquire_lease(lease_owner).await?;
            payload.validate_for_ledger()?;
            let normalized_payload = serde_json::to_string(&payload)
                .map_err(|error| AdapterError::InvalidData(error.to_string()))?;
            let expected_source_identity = serde_json::to_string(payload.source_identity())
                .map_err(|error| AdapterError::InvalidData(error.to_string()))?;
            let mut tx = self.ledger.begin().await?;
            self.fence_lease_in_tx(&mut tx, lease_owner).await?;
            sqlx::query(
                "UPDATE adapter_delivery_jobs SET normalized_payload = ?2, expected_source_identity = ?3, state = 'pending' WHERE delivery_key = ?1 AND state = 'normalization_pending'",
            )
            .bind(&delivery_key)
            .bind(normalized_payload)
            .bind(expected_source_identity)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        Ok(())
    }

    async fn acquire_lease(&self, owner_id: Uuid) -> Result<(), AdapterError> {
        let now = self.clock.now();
        let expires_at = now + Duration::minutes(15);
        let result = sqlx::query(
            r#"
            UPDATE adapter_lease
            SET owner_id = ?1, expires_at = ?2
            WHERE singleton = 1
              AND (owner_id IS NULL OR expires_at <= ?3 OR owner_id = ?1)
            "#,
        )
        .bind(owner_id.to_string())
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.ledger)
        .await?;
        if result.rows_affected() != 1 {
            return Err(AdapterError::LeaseHeld);
        }
        Ok(())
    }

    async fn fence_lease_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        owner_id: Uuid,
    ) -> Result<(), AdapterError> {
        let now = self.clock.now();
        let expires_at = now + Duration::minutes(15);
        let result = sqlx::query(
            r#"
            UPDATE adapter_lease
            SET expires_at = ?1
            WHERE singleton = 1 AND owner_id = ?2 AND expires_at > ?3
            "#,
        )
        .bind(expires_at.to_rfc3339())
        .bind(owner_id.to_string())
        .bind(now.to_rfc3339())
        .execute(&mut **tx)
        .await?;
        if result.rows_affected() != 1 {
            return Err(AdapterError::LeaseHeld);
        }
        Ok(())
    }

    async fn timed_source_page(
        &self,
        after_cursor: Option<&str>,
    ) -> Result<SourcePage, AdapterError> {
        tokio::time::timeout(
            EXTERNAL_CALL_TIMEOUT,
            self.source.acquire_page(after_cursor, self.page_limit),
        )
        .await
        .map_err(|_| AdapterError::Acquisition)?
        .map_err(|_| AdapterError::Acquisition)
    }

    async fn release_lease(&self, owner_id: Uuid) -> Result<(), AdapterError> {
        let result = sqlx::query(
            "UPDATE adapter_lease SET owner_id = NULL, expires_at = NULL WHERE singleton = 1 AND owner_id = ?1",
        )
        .bind(owner_id.to_string())
        .execute(&self.ledger)
        .await?;
        if result.rows_affected() != 1 {
            return Err(AdapterError::LeaseHeld);
        }
        Ok(())
    }

    pub async fn cursor(&self) -> Result<Option<String>, AdapterError> {
        Ok(
            sqlx::query_scalar("SELECT acknowledged_cursor FROM adapter_state WHERE singleton = 1")
                .fetch_one(&self.ledger)
                .await?,
        )
    }
}

fn empty_as_none(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn valid_delivery_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
        && !is_secret_like(value)
}

async fn initialize_ledger(pool: &SqlitePool) -> Result<(), AdapterError> {
    for statement in [
        "CREATE TABLE IF NOT EXISTS adapter_state (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), acknowledged_cursor TEXT)",
        "INSERT INTO adapter_state (singleton, acknowledged_cursor) VALUES (1, NULL) ON CONFLICT(singleton) DO NOTHING",
        "CREATE TABLE IF NOT EXISTS adapter_lease (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), owner_id TEXT, expires_at TEXT)",
        "INSERT INTO adapter_lease (singleton, owner_id, expires_at) VALUES (1, NULL, NULL) ON CONFLICT(singleton) DO NOTHING",
        "CREATE TABLE IF NOT EXISTS adapter_pages (cursor_before TEXT PRIMARY KEY, cursor_after TEXT, has_more INTEGER NOT NULL, state TEXT NOT NULL CHECK(state IN ('acquired','cursor_committed')), acquired_at TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS adapter_page_manifest (cursor_before TEXT NOT NULL REFERENCES adapter_pages(cursor_before), ordinal INTEGER NOT NULL, delivery_key TEXT NOT NULL, PRIMARY KEY(cursor_before, ordinal))",
        "CREATE TABLE IF NOT EXISTS adapter_delivery_jobs (delivery_key TEXT PRIMARY KEY, cursor_before TEXT NOT NULL REFERENCES adapter_pages(cursor_before), ordinal INTEGER NOT NULL, normalized_payload TEXT, expected_source_identity TEXT, state TEXT NOT NULL CHECK(state IN ('normalization_pending','pending','acknowledged')))",
        "CREATE TABLE IF NOT EXISTS adapter_delivery_attempts (id INTEGER PRIMARY KEY AUTOINCREMENT, delivery_key TEXT NOT NULL REFERENCES adapter_delivery_jobs(delivery_key), attempted_at TEXT NOT NULL, outcome TEXT NOT NULL CHECK(outcome IN ('started','acknowledged','identity_mismatch')))",
        "CREATE TABLE IF NOT EXISTS adapter_acknowledgements (delivery_key TEXT PRIMARY KEY REFERENCES adapter_delivery_jobs(delivery_key), acknowledgement TEXT NOT NULL, received_at TEXT NOT NULL)",
    ] {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use chrono::TimeZone;

    use super::*;

    struct FakeClock;
    impl AdapterClock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 8, 11, 0, 0, 0).unwrap()
        }
    }

    struct FakeSource {
        pages: Mutex<VecDeque<SourcePage>>,
        calls: Mutex<Vec<Option<String>>>,
    }
    #[async_trait]
    impl SourceClient for FakeSource {
        async fn acquire_page(
            &self,
            after_cursor: Option<&str>,
            _limit: usize,
        ) -> Result<SourcePage, AdapterError> {
            self.calls
                .lock()
                .unwrap()
                .push(after_cursor.map(str::to_string));
            self.pages
                .lock()
                .unwrap()
                .pop_front()
                .ok_or(AdapterError::Acquisition)
        }
    }

    struct Passthrough;
    #[async_trait]
    impl Normalizer for Passthrough {
        async fn normalize(
            &self,
            record: SourceRecord,
        ) -> Result<NormalizedGatewayRequest, AdapterError> {
            serde_json::from_value(record.payload)
                .map_err(|error| AdapterError::InvalidData(error.to_string()))
        }
    }

    struct RejectingNormalizer;
    #[async_trait]
    impl Normalizer for RejectingNormalizer {
        async fn normalize(
            &self,
            _record: SourceRecord,
        ) -> Result<NormalizedGatewayRequest, AdapterError> {
            Err(AdapterError::Normalization)
        }
    }

    struct FakeGateway {
        responses: Mutex<VecDeque<Result<GatewayAcknowledgement, AdapterError>>>,
        deliveries: Mutex<Vec<NormalizedGatewayRequest>>,
    }
    #[async_trait]
    impl GatewayClient for FakeGateway {
        async fn deliver(
            &self,
            payload: &NormalizedGatewayRequest,
        ) -> Result<GatewayAcknowledgement, AdapterError> {
            self.deliveries.lock().unwrap().push(payload.clone());
            self.responses.lock().unwrap().pop_front().unwrap()
        }
    }

    fn page(key: &str, next: &str, has_more: bool) -> SourcePage {
        SourcePage {
            records: vec![SourceRecord {
                delivery_key: key.to_string(),
                payload: serde_json::to_value(normalized_request(key)).unwrap(),
            }],
            next_cursor: Some(next.to_string()),
            has_more,
        }
    }

    fn two_record_page() -> SourcePage {
        SourcePage {
            records: vec![
                SourceRecord {
                    delivery_key: "one".to_string(),
                    payload: serde_json::to_value(normalized_request("one")).unwrap(),
                },
                SourceRecord {
                    delivery_key: "two".to_string(),
                    payload: serde_json::to_value(normalized_request("two")).unwrap(),
                },
            ],
            next_cursor: Some("cursor-2".to_string()),
            has_more: false,
        }
    }

    fn acknowledgement(
        status: GatewayAcknowledgementStatus,
        record_id: &str,
    ) -> GatewayAcknowledgement {
        GatewayAcknowledgement {
            status,
            source_identity: normalized_request(record_id).source_identity().clone(),
        }
    }

    fn normalized_request(record_id: &str) -> NormalizedGatewayRequest {
        serde_json::from_value(serde_json::json!({
            "namespace": "learning.foundation",
            "surface": "performance",
            "action": "submit_attempt",
            "actor": "00000000-0000-4000-8000-000000000001",
            "adapter": "mcp",
            "payload": {
                "space_id": "00000000-0000-4000-8000-000000000002",
                "source_evidence": {
                    "contract_version": 1,
                    "source_identity": {
                        "source_product": "study_buddy",
                        "source_installation_id": "00000000-0000-4000-8000-000000000003",
                        "record_type": "learning_session",
                        "record_id": record_id,
                        "revision": 1
                    },
                    "occurred_at": "2026-08-10T08:00:00Z",
                    "observed_at": "2026-08-10T08:00:02Z",
                    "evidence_trust": "contract_trusted",
                    "provenance": {
                        "adapter_id": "reference_adapter",
                        "adapter_version": "0.1.0",
                        "source_schema_version": "1"
                    },
                    "evidence": {
                        "kind": "learning_session",
                        "goal": "Practice learning.foundation",
                        "task": "Complete a bounded session",
                        "started_at": "2026-08-10T08:00:00Z",
                        "ended_at": "2026-08-10T08:30:00Z",
                        "activity_count": 5,
                        "successful_activity_count": 4
                    },
                    "tombstone": null
                }
            },
            "context": {"mode": "fast", "locale": null, "device": null, "runtime_preference": "deterministic"}
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn accepted_page_advances_cursor_once() {
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([page("one", "cursor-1", false)])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::from([Ok(acknowledgement(
                GatewayAcknowledgementStatus::Accepted,
                "one",
            ))])),
            deliveries: Mutex::new(Vec::new()),
        });
        let adapter = ReferenceAdapter::open(
            "sqlite::memory:",
            source,
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        let result = adapter.run_one_page().await.unwrap();
        assert_eq!(result.cursor.as_deref(), Some("cursor-1"));
        assert_eq!(result.acknowledged, 1);
    }

    #[tokio::test]
    async fn normalization_failure_stays_durable_and_restart_resumes_pending_work() {
        let directory = tempfile::tempdir().unwrap();
        let ledger_url = format!("sqlite://{}", directory.path().join("ledger.db").display());
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([
                page("one", "cursor-1", false),
                page("one", "cursor-1", false),
            ])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::from([Ok(acknowledgement(
                GatewayAcknowledgementStatus::Accepted,
                "one",
            ))])),
            deliveries: Mutex::new(Vec::new()),
        });
        let adapter = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(RejectingNormalizer),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        assert!(matches!(
            adapter.run_one_page().await,
            Err(AdapterError::Normalization)
        ));
        assert_eq!(adapter.cursor().await.unwrap(), None);
        let page_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM adapter_pages")
            .fetch_one(&adapter.ledger)
            .await
            .unwrap();
        assert_eq!(page_count, 1);
        let state: String = sqlx::query_scalar(
            "SELECT state FROM adapter_delivery_jobs WHERE delivery_key = 'one'",
        )
        .fetch_one(&adapter.ledger)
        .await
        .unwrap();
        assert_eq!(state, "normalization_pending");
        drop(adapter);

        let restarted = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        let result = restarted.run_one_page().await.unwrap();
        assert_eq!(result.cursor.as_deref(), Some("cursor-1"));
        assert_eq!(source.calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn lost_ack_restart_resumes_ledger_and_consumes_replay() {
        let directory = tempfile::tempdir().unwrap();
        let ledger_url = format!("sqlite://{}", directory.path().join("ledger.db").display());
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([page("one", "cursor-1", false)])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::from([
                Err(AdapterError::Delivery),
                Ok(acknowledgement(
                    GatewayAcknowledgementStatus::Replayed,
                    "one",
                )),
            ])),
            deliveries: Mutex::new(Vec::new()),
        });
        let first = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(Passthrough),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        assert!(matches!(
            first.run_one_page().await,
            Err(AdapterError::Delivery)
        ));
        drop(first);
        let restarted = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(Passthrough),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        let result = restarted.run_one_page().await.unwrap();
        assert_eq!(result.cursor.as_deref(), Some("cursor-1"));
        assert_eq!(source.calls.lock().unwrap().len(), 1);
        assert_eq!(gateway.deliveries.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn bounded_multi_page_run_preserves_stable_cursor_order() {
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([
                page("one", "cursor-1", true),
                page("two", "cursor-2", false),
            ])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::from([
                Ok(acknowledgement(
                    GatewayAcknowledgementStatus::Accepted,
                    "one",
                )),
                Ok(acknowledgement(
                    GatewayAcknowledgementStatus::Accepted,
                    "two",
                )),
            ])),
            deliveries: Mutex::new(Vec::new()),
        });
        let adapter = ReferenceAdapter::open(
            "sqlite::memory:",
            source.clone(),
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        let result = adapter.run_bounded(2).await.unwrap();
        assert_eq!(result.cursor.as_deref(), Some("cursor-2"));
        assert_eq!(result.acknowledged, 2);
        assert_eq!(
            *source.calls.lock().unwrap(),
            vec![None, Some("cursor-1".to_string())]
        );
    }

    #[tokio::test]
    async fn partial_page_failure_keeps_cursor_and_resumes_only_pending_job() {
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([two_record_page()])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::from([
                Ok(acknowledgement(
                    GatewayAcknowledgementStatus::Accepted,
                    "one",
                )),
                Err(AdapterError::Delivery),
                Ok(acknowledgement(
                    GatewayAcknowledgementStatus::Replayed,
                    "two",
                )),
            ])),
            deliveries: Mutex::new(Vec::new()),
        });
        let adapter = ReferenceAdapter::open(
            "sqlite::memory:",
            source.clone(),
            Arc::new(Passthrough),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        assert!(matches!(
            adapter.run_one_page().await,
            Err(AdapterError::Delivery)
        ));
        assert_eq!(adapter.cursor().await.unwrap(), None);
        let resumed = adapter.run_one_page().await.unwrap();
        assert_eq!(resumed.cursor.as_deref(), Some("cursor-2"));
        assert_eq!(source.calls.lock().unwrap().len(), 1);
        let deliveries = gateway.deliveries.lock().unwrap();
        assert_eq!(deliveries.len(), 3);
        assert_ne!(deliveries[0], deliveries[1]);
        assert_eq!(deliveries[1], deliveries[2]);
    }

    #[tokio::test]
    async fn mismatched_ack_identity_keeps_job_pending_and_cursor_unchanged() {
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([page("one", "cursor-1", false)])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::from([Ok(acknowledgement(
                GatewayAcknowledgementStatus::Accepted,
                "different",
            ))])),
            deliveries: Mutex::new(Vec::new()),
        });
        let adapter = ReferenceAdapter::open(
            "sqlite::memory:",
            source,
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();

        assert!(matches!(
            adapter.run_one_page().await,
            Err(AdapterError::InvalidData(_))
        ));
        assert_eq!(adapter.cursor().await.unwrap(), None);
        let state: String = sqlx::query_scalar(
            "SELECT state FROM adapter_delivery_jobs WHERE delivery_key = 'one'",
        )
        .fetch_one(&adapter.ledger)
        .await
        .unwrap();
        assert_eq!(state, "pending");
    }

    #[tokio::test]
    async fn overlapping_runner_is_excluded_by_the_durable_lease() {
        let directory = tempfile::tempdir().unwrap();
        let ledger_url = format!("sqlite://{}", directory.path().join("ledger.db").display());
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([page("one", "cursor-1", false)])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::new()),
            deliveries: Mutex::new(Vec::new()),
        });
        let first = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(Passthrough),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        let second = ReferenceAdapter::open(
            &ledger_url,
            source,
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        let owner = Uuid::new_v4();
        first.acquire_lease(owner).await.unwrap();

        assert!(matches!(
            second.run_one_page().await,
            Err(AdapterError::LeaseHeld)
        ));
        first.release_lease(owner).await.unwrap();
    }

    #[tokio::test]
    async fn secret_shaped_normalized_request_never_enters_the_ledger_payload() {
        let mut request = serde_json::to_value(normalized_request("one")).unwrap();
        request["payload"]["source_evidence"]["source_identity"]["record_id"] =
            serde_json::json!("ghp_not_a_record_identifier");
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([SourcePage {
                records: vec![SourceRecord {
                    delivery_key: "one".to_string(),
                    payload: request,
                }],
                next_cursor: Some("cursor-1".to_string()),
                has_more: false,
            }])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::new()),
            deliveries: Mutex::new(Vec::new()),
        });
        let adapter = ReferenceAdapter::open(
            "sqlite::memory:",
            source,
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();

        assert!(matches!(
            adapter.run_one_page().await,
            Err(AdapterError::InvalidData(_))
        ));
        let persisted: Option<String> = sqlx::query_scalar(
            "SELECT normalized_payload FROM adapter_delivery_jobs WHERE delivery_key = 'one'",
        )
        .fetch_one(&adapter.ledger)
        .await
        .unwrap();
        assert!(persisted.is_none());
    }

    #[tokio::test]
    async fn changed_resumed_page_key_set_never_advances_the_cursor() {
        let directory = tempfile::tempdir().unwrap();
        let ledger_url = format!("sqlite://{}", directory.path().join("ledger.db").display());
        let mut changed_page = page("one", "cursor-1", false);
        changed_page.records.push(SourceRecord {
            delivery_key: "unexpected".to_string(),
            payload: serde_json::to_value(normalized_request("unexpected")).unwrap(),
        });
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([
                page("one", "cursor-1", false),
                changed_page,
            ])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::new()),
            deliveries: Mutex::new(Vec::new()),
        });
        let first = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(RejectingNormalizer),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        assert!(matches!(
            first.run_one_page().await,
            Err(AdapterError::Normalization)
        ));
        drop(first);
        let restarted = ReferenceAdapter::open(
            &ledger_url,
            source,
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();

        assert!(matches!(
            restarted.run_one_page().await,
            Err(AdapterError::InvalidData(_))
        ));
        assert_eq!(restarted.cursor().await.unwrap(), None);
    }

    #[tokio::test]
    async fn expired_runner_cannot_fence_or_release_a_taken_over_lease() {
        let adapter = ReferenceAdapter::open(
            "sqlite::memory:",
            Arc::new(FakeSource {
                pages: Mutex::new(VecDeque::new()),
                calls: Mutex::new(Vec::new()),
            }),
            Arc::new(Passthrough),
            Arc::new(FakeGateway {
                responses: Mutex::new(VecDeque::new()),
                deliveries: Mutex::new(Vec::new()),
            }),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        let expired_owner = Uuid::new_v4();
        let new_owner = Uuid::new_v4();
        adapter.acquire_lease(expired_owner).await.unwrap();
        sqlx::query("UPDATE adapter_lease SET owner_id = ?1, expires_at = ?2 WHERE singleton = 1")
            .bind(new_owner.to_string())
            .bind("2026-08-11T00:15:00+00:00")
            .execute(&adapter.ledger)
            .await
            .unwrap();

        let mut tx = adapter.ledger.begin().await.unwrap();
        assert!(matches!(
            adapter.fence_lease_in_tx(&mut tx, expired_owner).await,
            Err(AdapterError::LeaseHeld)
        ));
        tx.rollback().await.unwrap();
        assert!(matches!(
            adapter.release_lease(expired_owner).await,
            Err(AdapterError::LeaseHeld)
        ));
        let owner: String =
            sqlx::query_scalar("SELECT owner_id FROM adapter_lease WHERE singleton = 1")
                .fetch_one(&adapter.ledger)
                .await
                .unwrap();
        assert_eq!(owner, new_owner.to_string());
    }

    #[tokio::test]
    async fn page_manifest_preserves_prior_delivery_keys_across_normalization_restart() {
        let directory = tempfile::tempdir().unwrap();
        let ledger_url = format!("sqlite://{}", directory.path().join("ledger.db").display());
        let second_page = two_record_page();
        let source = Arc::new(FakeSource {
            pages: Mutex::new(VecDeque::from([
                page("one", "cursor-1", true),
                second_page.clone(),
                second_page,
            ])),
            calls: Mutex::new(Vec::new()),
        });
        let gateway = Arc::new(FakeGateway {
            responses: Mutex::new(VecDeque::from([
                Ok(acknowledgement(
                    GatewayAcknowledgementStatus::Accepted,
                    "one",
                )),
                Ok(acknowledgement(
                    GatewayAcknowledgementStatus::Accepted,
                    "two",
                )),
            ])),
            deliveries: Mutex::new(Vec::new()),
        });
        let first = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(Passthrough),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        assert_eq!(
            first.run_one_page().await.unwrap().cursor.as_deref(),
            Some("cursor-1")
        );
        drop(first);

        let failing = ReferenceAdapter::open(
            &ledger_url,
            source.clone(),
            Arc::new(RejectingNormalizer),
            gateway.clone(),
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        assert!(matches!(
            failing.run_one_page().await,
            Err(AdapterError::Normalization)
        ));
        drop(failing);

        let restarted = ReferenceAdapter::open(
            &ledger_url,
            source,
            Arc::new(Passthrough),
            gateway,
            Arc::new(FakeClock),
            10,
        )
        .await
        .unwrap();
        assert_eq!(
            restarted.run_one_page().await.unwrap().cursor.as_deref(),
            Some("cursor-2")
        );
        let manifest: Vec<String> = sqlx::query_scalar(
            "SELECT delivery_key FROM adapter_page_manifest WHERE cursor_before = 'cursor-1' ORDER BY ordinal",
        )
        .fetch_all(&restarted.ledger)
        .await
        .unwrap();
        assert_eq!(manifest, vec!["one", "two"]);
    }
}
