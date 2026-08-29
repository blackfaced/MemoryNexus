//! Local, owner-confirmed Observation lifecycle CLI for the feedback kernel.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};
use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

const MAX_STATEMENT_LENGTH: usize = 500;
const MAX_RETRACTION_REASON_LENGTH: usize = 240;
const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 128;

#[tokio::main]
async fn main() {
    match run(env::args().skip(1).collect()).await {
        Ok(response) => println!("{response}"),
        Err(error) => {
            println!("{}", error.response());
            std::process::exit(2);
        }
    }
}

async fn run(args: Vec<String>) -> Result<Value, CliError> {
    let (ledger_path, command) = parse_args(args)?;

    match command.as_str() {
        "observe" => observe(&ledger_path, read_json_input()?).await,
        "observation-history" => observation_history(&ledger_path).await,
        "retract" => retract(&ledger_path, read_json_input()?).await,
        _ => Err(CliError::usage(
            "unknown command; use observe, observation-history, or retract",
        )),
    }
}

fn parse_args(args: Vec<String>) -> Result<(PathBuf, String), CliError> {
    let [flag, ledger_path, command] = args.as_slice() else {
        return Err(CliError::usage(
            "usage: memorynexus-cli --ledger <sqlite-file> <observe|observation-history|retract>",
        ));
    };

    if flag != "--ledger" {
        return Err(CliError::usage(
            "the ledger path must be supplied with --ledger",
        ));
    }

    if ledger_path.trim().is_empty() {
        return Err(CliError::usage("the ledger path cannot be empty"));
    }

    Ok((PathBuf::from(ledger_path), command.to_owned()))
}

fn read_json_input() -> Result<Value, CliError> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|_| CliError::invalid_input("could not read JSON input"))?;

    serde_json::from_str(&input)
        .map_err(|_| CliError::invalid_input("input must be one valid JSON object"))
}

async fn observe(ledger_path: &Path, value: Value) -> Result<Value, CliError> {
    let input = serde_json::from_value::<ObserveInput>(value).map_err(|_| {
        CliError::invalid_input("observe input does not match the supported contract")
    })?;
    let occurred_at = input.validate()?;
    let request_json = serde_json::to_string(&input)
        .map_err(|_| CliError::invalid_input("observe input could not be normalized"))?;
    let pool = open_ledger(ledger_path, true).await?;

    if let Some(existing) = find_observation_by_idempotency(&pool, &input.idempotency_key).await? {
        return idempotency_response(existing, &request_json, "observe");
    }

    if let Some(target_id) = input.supersedes_observation_id.as_deref() {
        validate_correction_target(&pool, target_id).await?;
    }

    let record = ObservationRecord {
        id: Uuid::new_v4().to_string(),
        statement: input.statement.trim().to_owned(),
        occurred_at,
        source: input.source,
        confirmed_at: Utc::now().to_rfc3339(),
        kind: input.kind,
        supersedes_observation_id: input.supersedes_observation_id,
    };

    let result = sqlx::query(
        "INSERT OR IGNORE INTO observations \
         (id, statement, occurred_at, source, confirmed_at, kind, supersedes_observation_id, idempotency_key, request_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&record.id)
    .bind(&record.statement)
    .bind(&record.occurred_at)
    .bind(record.source.as_str())
    .bind(&record.confirmed_at)
    .bind(record.kind.as_str())
    .bind(&record.supersedes_observation_id)
    .bind(&input.idempotency_key)
    .bind(&request_json)
    .execute(&pool)
    .await
    .map_err(CliError::storage)?;

    if result.rows_affected() == 0 {
        let existing = find_observation_by_idempotency(&pool, &input.idempotency_key)
            .await?
            .ok_or_else(|| CliError::conflict("observation could not be written"))?;
        return idempotency_response(existing, &request_json, "observe");
    }

    Ok(json!({
        "status": "accepted",
        "observation": record.with_state("current", true),
    }))
}

async fn observation_history(ledger_path: &Path) -> Result<Value, CliError> {
    let pool = open_ledger(ledger_path, false).await?;
    let rows = sqlx::query(
        "SELECT o.id, o.statement, o.occurred_at, o.source, o.confirmed_at, o.kind, \
                o.supersedes_observation_id, r.id AS retraction_id, \
                EXISTS(SELECT 1 FROM observations successor WHERE successor.supersedes_observation_id = o.id) AS has_successor \
         FROM observations o \
         LEFT JOIN observation_retractions r ON r.observation_id = o.id \
         ORDER BY o.confirmed_at ASC, o.id ASC",
    )
    .fetch_all(&pool)
    .await
    .map_err(CliError::storage)?;

    let observations = rows
        .iter()
        .map(observation_from_history_row)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(json!({"status": "ok", "observations": observations}))
}

async fn retract(ledger_path: &Path, value: Value) -> Result<Value, CliError> {
    let input = serde_json::from_value::<RetractInput>(value).map_err(|_| {
        CliError::invalid_input("retract input does not match the supported contract")
    })?;
    input.validate()?;
    let request_json = serde_json::to_string(&input)
        .map_err(|_| CliError::invalid_input("retract input could not be normalized"))?;
    let pool = open_ledger(ledger_path, true).await?;

    if let Some(existing) = find_retraction_by_idempotency(&pool, &input.idempotency_key).await? {
        if existing.request_json == request_json {
            return Ok(json!({
                "status": "idempotent_replay",
                "retraction": existing.into_response(),
            }));
        }
        return Err(CliError::conflict(
            "idempotency_key was already used for another retract request",
        ));
    }

    let observation_exists = sqlx::query("SELECT 1 FROM observations WHERE id = ?")
        .bind(&input.observation_id)
        .fetch_optional(&pool)
        .await
        .map_err(CliError::storage)?
        .is_some();
    if !observation_exists {
        return Err(CliError::not_found(
            "observation_id does not identify a confirmed Observation",
        ));
    }

    let retraction = RetractionRecord {
        id: Uuid::new_v4().to_string(),
        observation_id: input.observation_id,
        reason: input.reason.trim().to_owned(),
        confirmed_at: Utc::now().to_rfc3339(),
        request_json: request_json.clone(),
    };
    let result = sqlx::query(
        "INSERT OR IGNORE INTO observation_retractions \
         (id, observation_id, reason, confirmed_at, idempotency_key, request_json) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&retraction.id)
    .bind(&retraction.observation_id)
    .bind(&retraction.reason)
    .bind(&retraction.confirmed_at)
    .bind(&input.idempotency_key)
    .bind(&retraction.request_json)
    .execute(&pool)
    .await
    .map_err(CliError::storage)?;

    if result.rows_affected() == 0 {
        let existing = find_retraction_by_idempotency(&pool, &input.idempotency_key)
            .await?
            .ok_or_else(|| CliError::conflict("Observation was already retracted"))?;
        if existing.request_json == request_json {
            return Ok(json!({
                "status": "idempotent_replay",
                "retraction": existing.into_response(),
            }));
        }
        return Err(CliError::conflict("Observation was already retracted"));
    }

    Ok(json!({"status": "accepted", "retraction": retraction.into_response()}))
}

async fn open_ledger(ledger_path: &Path, create_if_missing: bool) -> Result<SqlitePool, CliError> {
    if create_if_missing {
        if let Some(parent) = ledger_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .map_err(|_| CliError::storage("could not create ledger directory"))?;
        }
    } else if !ledger_path.is_file() {
        return Err(CliError::not_found(
            "the requested SQLite ledger does not exist",
        ));
    }

    let options = SqliteConnectOptions::new()
        .filename(ledger_path)
        .create_if_missing(create_if_missing)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(CliError::storage)?;

    if create_if_missing {
        initialize_schema(&pool).await?;
    }
    Ok(pool)
}

async fn initialize_schema(pool: &SqlitePool) -> Result<(), CliError> {
    for statement in [
        "CREATE TABLE IF NOT EXISTS observations (\
            id TEXT PRIMARY KEY NOT NULL,\
            statement TEXT NOT NULL,\
            occurred_at TEXT NOT NULL,\
            source TEXT NOT NULL CHECK (source = 'owner_report'),\
            confirmed_at TEXT NOT NULL,\
            kind TEXT NOT NULL CHECK (kind IN ('initial', 'correction')),\
            supersedes_observation_id TEXT REFERENCES observations(id),\
            idempotency_key TEXT NOT NULL UNIQUE,\
            request_json TEXT NOT NULL\
        )",
        "CREATE UNIQUE INDEX IF NOT EXISTS observation_one_successor \
         ON observations(supersedes_observation_id) WHERE supersedes_observation_id IS NOT NULL",
        "CREATE TABLE IF NOT EXISTS observation_retractions (\
            id TEXT PRIMARY KEY NOT NULL,\
            observation_id TEXT NOT NULL UNIQUE REFERENCES observations(id),\
            reason TEXT NOT NULL,\
            confirmed_at TEXT NOT NULL,\
            idempotency_key TEXT NOT NULL UNIQUE,\
            request_json TEXT NOT NULL\
        )",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .map_err(CliError::storage)?;
    }
    Ok(())
}

async fn find_observation_by_idempotency(
    pool: &SqlitePool,
    idempotency_key: &str,
) -> Result<Option<StoredObservation>, CliError> {
    sqlx::query(
        "SELECT id, statement, occurred_at, source, confirmed_at, kind, supersedes_observation_id, request_json \
         FROM observations WHERE idempotency_key = ?",
    )
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(CliError::storage)?
    .map(stored_observation_from_row)
    .transpose()
}

async fn validate_correction_target(pool: &SqlitePool, target_id: &str) -> Result<(), CliError> {
    let target = sqlx::query(
        "SELECT o.id, \
                EXISTS(SELECT 1 FROM observations successor WHERE successor.supersedes_observation_id = o.id) AS has_successor, \
                EXISTS(SELECT 1 FROM observation_retractions r WHERE r.observation_id = o.id) AS is_retracted \
         FROM observations o WHERE o.id = ?",
    )
    .bind(target_id)
    .fetch_optional(pool)
    .await
    .map_err(CliError::storage)?
    .ok_or_else(|| CliError::not_found("supersedes_observation_id does not identify a confirmed Observation"))?;

    if target
        .try_get::<i64, _>("has_successor")
        .map_err(CliError::storage)?
        != 0
    {
        return Err(CliError::conflict(
            "a correction must supersede the current Observation",
        ));
    }
    if target
        .try_get::<i64, _>("is_retracted")
        .map_err(CliError::storage)?
        != 0
    {
        return Err(CliError::conflict(
            "a retracted Observation cannot be corrected",
        ));
    }
    Ok(())
}

async fn find_retraction_by_idempotency(
    pool: &SqlitePool,
    idempotency_key: &str,
) -> Result<Option<StoredRetraction>, CliError> {
    sqlx::query(
        "SELECT id, observation_id, reason, confirmed_at, request_json \
         FROM observation_retractions WHERE idempotency_key = ?",
    )
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(CliError::storage)?
    .map(stored_retraction_from_row)
    .transpose()
}

fn idempotency_response(
    existing: StoredObservation,
    request_json: &str,
    action: &str,
) -> Result<Value, CliError> {
    if existing.request_json != request_json {
        return Err(CliError::conflict(format!(
            "idempotency_key was already used for another {action} request"
        )));
    }

    Ok(json!({
        "status": "idempotent_replay",
        "observation": existing.record.with_state("current", true),
    }))
}

fn observation_from_history_row(row: &SqliteRow) -> Result<Value, CliError> {
    let record = observation_record_from_row(row)?;
    let state = if row
        .try_get::<Option<String>, _>("retraction_id")
        .map_err(CliError::storage)?
        .is_some()
    {
        "retracted"
    } else if row
        .try_get::<i64, _>("has_successor")
        .map_err(CliError::storage)?
        != 0
    {
        "superseded"
    } else {
        "current"
    };
    Ok(record.with_state(state, state == "current"))
}

fn observation_record_from_row(row: &SqliteRow) -> Result<ObservationRecord, CliError> {
    Ok(ObservationRecord {
        id: row.try_get("id").map_err(CliError::storage)?,
        statement: row.try_get("statement").map_err(CliError::storage)?,
        occurred_at: row.try_get("occurred_at").map_err(CliError::storage)?,
        source: ObservationSource::from_db(
            &row.try_get::<String, _>("source")
                .map_err(CliError::storage)?,
        )?,
        confirmed_at: row.try_get("confirmed_at").map_err(CliError::storage)?,
        kind: ObservationKind::from_db(
            &row.try_get::<String, _>("kind")
                .map_err(CliError::storage)?,
        )?,
        supersedes_observation_id: row
            .try_get("supersedes_observation_id")
            .map_err(CliError::storage)?,
    })
}

fn stored_observation_from_row(row: SqliteRow) -> Result<StoredObservation, CliError> {
    Ok(StoredObservation {
        record: observation_record_from_row(&row)?,
        request_json: row.try_get("request_json").map_err(CliError::storage)?,
    })
}

fn stored_retraction_from_row(row: SqliteRow) -> Result<StoredRetraction, CliError> {
    Ok(StoredRetraction {
        id: row.try_get("id").map_err(CliError::storage)?,
        observation_id: row.try_get("observation_id").map_err(CliError::storage)?,
        reason: row.try_get("reason").map_err(CliError::storage)?,
        confirmed_at: row.try_get("confirmed_at").map_err(CliError::storage)?,
        request_json: row.try_get("request_json").map_err(CliError::storage)?,
    })
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObserveInput {
    confirmation: Confirmation,
    kind: ObservationKind,
    statement: String,
    occurred_at: String,
    source: ObservationSource,
    idempotency_key: String,
    #[serde(default)]
    supersedes_observation_id: Option<String>,
}

impl ObserveInput {
    fn validate(&self) -> Result<String, CliError> {
        require_confirmed(self.confirmation)?;
        validate_bounded("statement", &self.statement, MAX_STATEMENT_LENGTH)?;
        validate_idempotency_key(&self.idempotency_key)?;
        let occurred_at = DateTime::parse_from_rfc3339(&self.occurred_at)
            .map_err(|_| CliError::invalid_input("occurred_at must be an RFC3339 timestamp"))?
            .with_timezone(&Utc)
            .to_rfc3339();

        match (self.kind, self.supersedes_observation_id.as_deref()) {
            (ObservationKind::Initial, None) => Ok(occurred_at),
            (ObservationKind::Initial, Some(_)) => Err(CliError::invalid_input(
                "an initial Observation must not include supersedes_observation_id",
            )),
            (ObservationKind::Correction, Some(id)) if valid_id(id) => Ok(occurred_at),
            (ObservationKind::Correction, Some(_)) => Err(CliError::invalid_input(
                "a correction requires a non-empty supersedes_observation_id",
            )),
            (ObservationKind::Correction, None) => Err(CliError::invalid_input(
                "a correction requires supersedes_observation_id",
            )),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RetractInput {
    confirmation: Confirmation,
    observation_id: String,
    reason: String,
    idempotency_key: String,
}

impl RetractInput {
    fn validate(&self) -> Result<(), CliError> {
        require_confirmed(self.confirmation)?;
        if !valid_id(&self.observation_id) {
            return Err(CliError::invalid_input(
                "observation_id must be a non-empty identifier",
            ));
        }
        validate_bounded("reason", &self.reason, MAX_RETRACTION_REASON_LENGTH)?;
        validate_idempotency_key(&self.idempotency_key)
    }
}

fn require_confirmed(confirmation: Confirmation) -> Result<(), CliError> {
    if confirmation == Confirmation::Confirmed {
        Ok(())
    } else {
        Err(CliError::confirmation_required(
            "confirmation must be exactly confirmed before an authoritative write",
        ))
    }
}

fn validate_bounded(field: &str, value: &str, max_length: usize) -> Result<(), CliError> {
    let length = value.trim().chars().count();
    if length == 0 || length > max_length {
        return Err(CliError::invalid_input(format!(
            "{field} must contain between 1 and {max_length} characters"
        )));
    }
    Ok(())
}

fn validate_idempotency_key(value: &str) -> Result<(), CliError> {
    if !valid_id(value) || value.chars().count() > MAX_IDEMPOTENCY_KEY_LENGTH {
        return Err(CliError::invalid_input(format!(
            "idempotency_key must contain between 1 and {MAX_IDEMPOTENCY_KEY_LENGTH} characters"
        )));
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.trim() == value
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Confirmation {
    Confirmed,
    Draft,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ObservationKind {
    Initial,
    Correction,
}

impl ObservationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Correction => "correction",
        }
    }

    fn from_db(value: &str) -> Result<Self, CliError> {
        match value {
            "initial" => Ok(Self::Initial),
            "correction" => Ok(Self::Correction),
            _ => Err(CliError::storage(
                "ledger contains an unsupported Observation kind",
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ObservationSource {
    OwnerReport,
}

impl ObservationSource {
    fn as_str(&self) -> &'static str {
        "owner_report"
    }

    fn from_db(value: &str) -> Result<Self, CliError> {
        match value {
            "owner_report" => Ok(Self::OwnerReport),
            _ => Err(CliError::storage(
                "ledger contains an unsupported Observation source",
            )),
        }
    }
}

#[derive(Debug, Serialize)]
struct ObservationRecord {
    id: String,
    statement: String,
    occurred_at: String,
    source: ObservationSource,
    confirmed_at: String,
    kind: ObservationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    supersedes_observation_id: Option<String>,
}

impl ObservationRecord {
    fn with_state(self, state: &str, is_current: bool) -> Value {
        let mut value = serde_json::to_value(self).expect("ObservationRecord serializes");
        let object = value
            .as_object_mut()
            .expect("ObservationRecord serializes to an object");
        object.insert("state".to_owned(), Value::String(state.to_owned()));
        object.insert("is_current".to_owned(), Value::Bool(is_current));
        value
    }
}

struct StoredObservation {
    record: ObservationRecord,
    request_json: String,
}

struct RetractionRecord {
    id: String,
    observation_id: String,
    reason: String,
    confirmed_at: String,
    request_json: String,
}

impl RetractionRecord {
    fn into_response(self) -> Value {
        json!({
            "id": self.id,
            "observation_id": self.observation_id,
            "reason": self.reason,
            "confirmed_at": self.confirmed_at,
            "state": "retracted",
        })
    }
}

struct StoredRetraction {
    id: String,
    observation_id: String,
    reason: String,
    confirmed_at: String,
    request_json: String,
}

impl StoredRetraction {
    fn into_response(self) -> Value {
        json!({
            "id": self.id,
            "observation_id": self.observation_id,
            "reason": self.reason,
            "confirmed_at": self.confirmed_at,
            "state": "retracted",
        })
    }
}

struct CliError {
    code: &'static str,
    message: String,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: "usage",
            message: message.into(),
        }
    }

    fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_input",
            message: message.into(),
        }
    }

    fn confirmation_required(message: impl Into<String>) -> Self {
        Self {
            code: "confirmation_required",
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found",
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "conflict",
            message: message.into(),
        }
    }

    fn storage<E>(_error: E) -> Self {
        Self {
            code: "storage_error",
            message: "the local SQLite ledger could not complete this request".to_owned(),
        }
    }

    fn response(self) -> Value {
        json!({"status": "rejected", "code": self.code, "message": self.message})
    }
}
