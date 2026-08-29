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
const MAX_ACTION_LENGTH: usize = 300;

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
        "add-recommendation" => add_recommendation(&ledger_path, read_json_input()?).await,
        "start-experiment" => start_experiment(&ledger_path, read_json_input()?).await,
        "end-experiment" => end_experiment(&ledger_path, read_json_input()?).await,
        "record-outcome" => record_outcome(&ledger_path, read_json_input()?).await,
        "review" => review(&ledger_path).await,
        _ => Err(CliError::usage("unknown command")),
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

async fn add_recommendation(ledger_path: &Path, value: Value) -> Result<Value, CliError> {
    let input: RecommendationInput = serde_json::from_value(value).map_err(|_| {
        CliError::invalid_input("recommendation input does not match the supported contract")
    })?;
    input.validate()?;
    let request = serde_json::to_string(&input)
        .map_err(|_| CliError::invalid_input("recommendation input could not be normalized"))?;
    let pool = open_ledger(ledger_path, true).await?;
    if let Some(row) =
        sqlx::query("SELECT id, request_json FROM recommendations WHERE idempotency_key = ?")
            .bind(&input.idempotency_key)
            .fetch_optional(&pool)
            .await
            .map_err(CliError::storage)?
    {
        if row
            .try_get::<String, _>("request_json")
            .map_err(CliError::storage)?
            == request
        {
            return Ok(
                json!({"status":"idempotent_replay","recommendation_id":row.try_get::<String,_>("id").map_err(CliError::storage)?}),
            );
        }
        return Err(CliError::conflict(
            "idempotency_key was already used for another recommendation",
        ));
    }
    for observation_id in &input.observation_ids {
        if sqlx::query("SELECT 1 FROM observations WHERE id = ?")
            .bind(observation_id)
            .fetch_optional(&pool)
            .await
            .map_err(CliError::storage)?
            .is_none()
        {
            return Err(CliError::not_found(
                "observation_ids must identify confirmed Observations",
            ));
        }
    }
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO recommendations (id, summary, source, created_at, idempotency_key, request_json) VALUES (?, ?, ?, ?, ?, ?)").bind(&id).bind(input.summary.trim()).bind(input.source.as_str()).bind(&created_at).bind(&input.idempotency_key).bind(&request).execute(&pool).await.map_err(CliError::storage)?;
    for observation_id in &input.observation_ids {
        sqlx::query("INSERT INTO recommendation_observations (recommendation_id, observation_id) VALUES (?, ?)").bind(&id).bind(observation_id).execute(&pool).await.map_err(CliError::storage)?;
    }
    Ok(
        json!({"status":"accepted","recommendation":{"id":id,"summary":input.summary.trim(),"source":input.source,"created_at":created_at,"observation_ids":input.observation_ids}}),
    )
}

async fn start_experiment(ledger_path: &Path, value: Value) -> Result<Value, CliError> {
    let input: StartExperimentInput = serde_json::from_value(value).map_err(|_| {
        CliError::invalid_input("start-experiment input does not match the supported contract")
    })?;
    input.validate()?;
    let request = serde_json::to_string(&input)
        .map_err(|_| CliError::invalid_input("experiment input could not be normalized"))?;
    let pool = open_ledger(ledger_path, true).await?;
    if let Some(row) =
        sqlx::query("SELECT id, request_json FROM experiments WHERE idempotency_key = ?")
            .bind(&input.idempotency_key)
            .fetch_optional(&pool)
            .await
            .map_err(CliError::storage)?
    {
        if row
            .try_get::<String, _>("request_json")
            .map_err(CliError::storage)?
            == request
        {
            return Ok(
                json!({"status":"idempotent_replay","experiment_id":row.try_get::<String,_>("id").map_err(CliError::storage)?}),
            );
        }
        return Err(CliError::conflict(
            "idempotency_key was already used for another experiment",
        ));
    }
    if sqlx::query("SELECT 1 FROM recommendations WHERE id = ?")
        .bind(&input.recommendation_id)
        .fetch_optional(&pool)
        .await
        .map_err(CliError::storage)?
        .is_none()
    {
        return Err(CliError::not_found(
            "recommendation_id does not identify a Recommendation",
        ));
    }
    if sqlx::query("SELECT 1 FROM experiments WHERE state = 'active'")
        .fetch_optional(&pool)
        .await
        .map_err(CliError::storage)?
        .is_some()
    {
        return Err(CliError::conflict("only one active Experiment is allowed"));
    }
    let id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO experiments (id, recommendation_id, action, starts_at, ends_at, expected_signal, state, created_at, idempotency_key, request_json) VALUES (?, ?, ?, ?, ?, ?, 'active', ?, ?, ?)").bind(&id).bind(&input.recommendation_id).bind(input.action.trim()).bind(&input.starts_at).bind(&input.ends_at).bind(input.expected_signal.trim()).bind(&started_at).bind(&input.idempotency_key).bind(&request).execute(&pool).await.map_err(CliError::storage)?;
    Ok(
        json!({"status":"accepted","experiment":{"id":id,"recommendation_id":input.recommendation_id,"action":input.action.trim(),"starts_at":input.starts_at,"ends_at":input.ends_at,"expected_signal":input.expected_signal.trim(),"state":"active"}}),
    )
}

async fn end_experiment(ledger_path: &Path, value: Value) -> Result<Value, CliError> {
    let input: EndExperimentInput = serde_json::from_value(value).map_err(|_| {
        CliError::invalid_input("end-experiment input does not match the supported contract")
    })?;
    input.validate()?;
    let pool = open_ledger(ledger_path, true).await?;
    let result = sqlx::query(
        "UPDATE experiments SET state = ?, ended_at = ? WHERE id = ? AND state = 'active'",
    )
    .bind(input.state.as_str())
    .bind(Utc::now().to_rfc3339())
    .bind(&input.experiment_id)
    .execute(&pool)
    .await
    .map_err(CliError::storage)?;
    if result.rows_affected() == 0 {
        return Err(CliError::conflict("only an active Experiment can be ended"));
    }
    Ok(json!({"status":"accepted","experiment_id":input.experiment_id,"state":input.state}))
}

async fn record_outcome(ledger_path: &Path, value: Value) -> Result<Value, CliError> {
    let input: OutcomeInput = serde_json::from_value(value).map_err(|_| {
        CliError::invalid_input("record-outcome input does not match the supported contract")
    })?;
    input.validate()?;
    let request = serde_json::to_string(&input)
        .map_err(|_| CliError::invalid_input("outcome input could not be normalized"))?;
    let pool = open_ledger(ledger_path, true).await?;
    if let Some(row) =
        sqlx::query("SELECT id, request_json FROM outcomes WHERE idempotency_key = ?")
            .bind(&input.idempotency_key)
            .fetch_optional(&pool)
            .await
            .map_err(CliError::storage)?
    {
        if row
            .try_get::<String, _>("request_json")
            .map_err(CliError::storage)?
            == request
        {
            return Ok(
                json!({"status":"idempotent_replay","outcome_id":row.try_get::<String,_>("id").map_err(CliError::storage)?}),
            );
        }
        return Err(CliError::conflict(
            "idempotency_key was already used for another outcome",
        ));
    }
    if sqlx::query("SELECT 1 FROM experiments WHERE id = ?")
        .bind(&input.experiment_id)
        .fetch_optional(&pool)
        .await
        .map_err(CliError::storage)?
        .is_none()
    {
        return Err(CliError::not_found(
            "experiment_id does not identify an Experiment",
        ));
    }
    if let Some(previous) = &input.supersedes_outcome_id {
        let prior = sqlx::query("SELECT experiment_id FROM outcomes WHERE id = ?")
            .bind(previous)
            .fetch_optional(&pool)
            .await
            .map_err(CliError::storage)?
            .ok_or_else(|| {
                CliError::not_found("supersedes_outcome_id does not identify an Outcome")
            })?;
        if prior.get::<String, _>("experiment_id") != input.experiment_id {
            return Err(CliError::conflict(
                "an Outcome correction must keep the same Experiment",
            ));
        }
    }
    let id = Uuid::new_v4().to_string();
    let confirmed_at = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO outcomes (id, experiment_id, occurred_at, execution_state, evaluation, note, supersedes_outcome_id, confirmed_at, idempotency_key, request_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)").bind(&id).bind(&input.experiment_id).bind(&input.occurred_at).bind(input.execution_state.as_str()).bind(input.evaluation.as_str()).bind(input.note.trim()).bind(&input.supersedes_outcome_id).bind(&confirmed_at).bind(&input.idempotency_key).bind(&request).execute(&pool).await.map_err(CliError::storage)?;
    Ok(
        json!({"status":"accepted","outcome":{"id":id,"experiment_id":input.experiment_id,"occurred_at":input.occurred_at,"execution_state":input.execution_state,"evaluation":input.evaluation,"note":input.note.trim(),"supersedes_outcome_id":input.supersedes_outcome_id,"confirmed_at":confirmed_at}}),
    )
}

async fn review(ledger_path: &Path) -> Result<Value, CliError> {
    let pool = open_ledger(ledger_path, false).await?;
    let rows = sqlx::query("SELECT e.id, e.action, e.state, e.starts_at, e.ends_at, e.expected_signal, r.id AS recommendation_id, r.summary, r.source FROM experiments e JOIN recommendations r ON r.id = e.recommendation_id ORDER BY e.created_at ASC").fetch_all(&pool).await.map_err(CliError::storage)?;
    let experiments = rows.into_iter().map(|r| json!({"id":r.get::<String,_>("id"),"action":r.get::<String,_>("action"),"state":r.get::<String,_>("state"),"starts_at":r.get::<String,_>("starts_at"),"ends_at":r.get::<String,_>("ends_at"),"expected_signal":r.get::<String,_>("expected_signal"),"recommendation":{"id":r.get::<String,_>("recommendation_id"),"summary":r.get::<String,_>("summary"),"source":r.get::<String,_>("source")}})).collect::<Vec<_>>();
    let outcomes = sqlx::query("SELECT id, experiment_id, occurred_at, execution_state, evaluation, note, supersedes_outcome_id FROM outcomes ORDER BY confirmed_at ASC")
        .fetch_all(&pool).await.map_err(CliError::storage)?
        .into_iter().map(|row| json!({"id":row.get::<String,_>("id"),"experiment_id":row.get::<String,_>("experiment_id"),"occurred_at":row.get::<String,_>("occurred_at"),"execution_state":row.get::<String,_>("execution_state"),"evaluation":row.get::<String,_>("evaluation"),"note":row.get::<String,_>("note"),"supersedes_outcome_id":row.get::<Option<String>,_>("supersedes_outcome_id")})).collect::<Vec<_>>();
    let evidence_gaps = if experiments.is_empty() {
        vec!["no_experiment"]
    } else if outcomes.is_empty() {
        vec!["no_confirmed_outcome"]
    } else {
        Vec::new()
    };
    Ok(
        json!({"status":"ok","experiments":experiments,"outcomes":outcomes,"evidence_gaps":evidence_gaps}),
    )
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
        "CREATE TABLE IF NOT EXISTS recommendations (id TEXT PRIMARY KEY NOT NULL, summary TEXT NOT NULL, source TEXT NOT NULL CHECK (source IN ('owner','ant_afu','agent_candidate')), created_at TEXT NOT NULL, idempotency_key TEXT NOT NULL UNIQUE, request_json TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS recommendation_observations (recommendation_id TEXT NOT NULL REFERENCES recommendations(id), observation_id TEXT NOT NULL REFERENCES observations(id), PRIMARY KEY (recommendation_id, observation_id))",
        "CREATE TABLE IF NOT EXISTS experiments (id TEXT PRIMARY KEY NOT NULL, recommendation_id TEXT NOT NULL REFERENCES recommendations(id), action TEXT NOT NULL, starts_at TEXT NOT NULL, ends_at TEXT NOT NULL, expected_signal TEXT NOT NULL, state TEXT NOT NULL CHECK (state IN ('active','completed','cancelled')), created_at TEXT NOT NULL, ended_at TEXT, idempotency_key TEXT NOT NULL UNIQUE, request_json TEXT NOT NULL)",
        "CREATE UNIQUE INDEX IF NOT EXISTS experiments_one_active ON experiments(state) WHERE state = 'active'",
        "CREATE TABLE IF NOT EXISTS outcomes (id TEXT PRIMARY KEY NOT NULL, experiment_id TEXT NOT NULL REFERENCES experiments(id), occurred_at TEXT NOT NULL, execution_state TEXT NOT NULL CHECK (execution_state IN ('performed','skipped','not_evaluable')), evaluation TEXT NOT NULL CHECK (evaluation IN ('improved','unchanged','worse','unclear')), note TEXT NOT NULL, supersedes_outcome_id TEXT REFERENCES outcomes(id), confirmed_at TEXT NOT NULL, idempotency_key TEXT NOT NULL UNIQUE, request_json TEXT NOT NULL)",
        "CREATE UNIQUE INDEX IF NOT EXISTS outcomes_one_successor ON outcomes(supersedes_outcome_id) WHERE supersedes_outcome_id IS NOT NULL",
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum RecommendationSource {
    Owner,
    AntAfu,
    AgentCandidate,
}
impl RecommendationSource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::AntAfu => "ant_afu",
            Self::AgentCandidate => "agent_candidate",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RecommendationInput {
    confirmation: Confirmation,
    summary: String,
    source: RecommendationSource,
    #[serde(default)]
    observation_ids: Vec<String>,
    idempotency_key: String,
}
impl RecommendationInput {
    fn validate(&self) -> Result<(), CliError> {
        require_confirmed(self.confirmation)?;
        validate_bounded("summary", &self.summary, MAX_STATEMENT_LENGTH)?;
        validate_idempotency_key(&self.idempotency_key)?;
        if self.observation_ids.iter().any(|id| !valid_id(id)) {
            return Err(CliError::invalid_input(
                "observation_ids must be non-empty identifiers",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StartExperimentInput {
    confirmation: Confirmation,
    recommendation_id: String,
    action: String,
    starts_at: String,
    ends_at: String,
    expected_signal: String,
    idempotency_key: String,
}
impl StartExperimentInput {
    fn validate(&self) -> Result<(), CliError> {
        require_confirmed(self.confirmation)?;
        if !valid_id(&self.recommendation_id) {
            return Err(CliError::invalid_input(
                "recommendation_id must be a non-empty identifier",
            ));
        }
        validate_bounded("action", &self.action, MAX_ACTION_LENGTH)?;
        validate_bounded("expected_signal", &self.expected_signal, MAX_ACTION_LENGTH)?;
        let start = DateTime::parse_from_rfc3339(&self.starts_at)
            .map_err(|_| CliError::invalid_input("starts_at must be RFC3339"))?;
        let end = DateTime::parse_from_rfc3339(&self.ends_at)
            .map_err(|_| CliError::invalid_input("ends_at must be RFC3339"))?;
        if end <= start {
            return Err(CliError::invalid_input("ends_at must be after starts_at"));
        }
        validate_idempotency_key(&self.idempotency_key)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndExperimentInput {
    confirmation: Confirmation,
    experiment_id: String,
    state: EndExperimentState,
    idempotency_key: String,
}
impl EndExperimentInput {
    fn validate(&self) -> Result<(), CliError> {
        require_confirmed(self.confirmation)?;
        if !valid_id(&self.experiment_id) {
            return Err(CliError::invalid_input(
                "experiment_id must be a non-empty identifier",
            ));
        }
        validate_idempotency_key(&self.idempotency_key)
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum EndExperimentState {
    Completed,
    Cancelled,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OutcomeInput {
    confirmation: Confirmation,
    experiment_id: String,
    occurred_at: String,
    execution_state: ExecutionState,
    evaluation: Evaluation,
    note: String,
    #[serde(default)]
    supersedes_outcome_id: Option<String>,
    idempotency_key: String,
}
impl OutcomeInput {
    fn validate(&self) -> Result<(), CliError> {
        require_confirmed(self.confirmation)?;
        if !valid_id(&self.experiment_id) {
            return Err(CliError::invalid_input(
                "experiment_id must be a non-empty identifier",
            ));
        }
        if self
            .supersedes_outcome_id
            .as_deref()
            .is_some_and(|id| !valid_id(id))
        {
            return Err(CliError::invalid_input(
                "supersedes_outcome_id must be a non-empty identifier",
            ));
        }
        DateTime::parse_from_rfc3339(&self.occurred_at)
            .map_err(|_| CliError::invalid_input("occurred_at must be RFC3339"))?;
        validate_bounded("note", &self.note, MAX_STATEMENT_LENGTH)?;
        validate_idempotency_key(&self.idempotency_key)
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ExecutionState {
    Performed,
    Skipped,
    NotEvaluable,
}
impl ExecutionState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Performed => "performed",
            Self::Skipped => "skipped",
            Self::NotEvaluable => "not_evaluable",
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Evaluation {
    Improved,
    Unchanged,
    Worse,
    Unclear,
}
impl Evaluation {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Improved => "improved",
            Self::Unchanged => "unchanged",
            Self::Worse => "worse",
            Self::Unclear => "unclear",
        }
    }
}
impl EndExperimentState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
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
