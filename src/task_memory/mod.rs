//! Local, partitioned task memory exposed through versioned CLI use cases.
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqliteConnectOptions, Connection, Row, SqliteConnection};
use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

mod lifecycle;
mod recovery;

const APPLICATION_ID: i64 = 0x4d4e5453;
const SCHEMA_VERSION: i64 = 1;
const LEDGER_ENV: &str = "MEMORYNEXUS_TASK_LEDGER";
const MAX_INPUT: u64 = 32 * 1024;

const SCHEMA: &[&str] = &[
        "PRAGMA application_id=1296979027",
        "PRAGMA user_version=1",
        "CREATE TABLE tasks (id TEXT PRIMARY KEY NOT NULL, display_name TEXT NOT NULL, execution_capture INTEGER NOT NULL, review_capture INTEGER NOT NULL, revision INTEGER NOT NULL)",
        "CREATE TABLE task_records (commit_sequence INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE, task_id TEXT NOT NULL REFERENCES tasks(id), kind TEXT NOT NULL, trust TEXT NOT NULL, text TEXT NOT NULL, occurred_at TEXT NOT NULL, run_id TEXT NOT NULL, client TEXT NOT NULL, evidence_refs TEXT NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL)",
        "CREATE INDEX task_records_scope ON task_records(task_id, state, commit_sequence)",
        "CREATE TABLE task_record_sources (record_id TEXT NOT NULL REFERENCES task_records(id), source_id TEXT NOT NULL REFERENCES task_records(id), position INTEGER NOT NULL, PRIMARY KEY(record_id, source_id), UNIQUE(record_id, position))",
        "CREATE INDEX task_record_sources_source ON task_record_sources(source_id)",
        "CREATE TABLE task_record_revisions (record_id TEXT PRIMARY KEY NOT NULL REFERENCES task_records(id), supersedes_id TEXT NOT NULL UNIQUE REFERENCES task_records(id))",
        "CREATE TABLE task_receipts (task_id TEXT NOT NULL REFERENCES tasks(id), idempotency_key TEXT NOT NULL, action TEXT NOT NULL, digest TEXT NOT NULL, record_id TEXT NOT NULL REFERENCES task_records(id), revision INTEGER NOT NULL, PRIMARY KEY(task_id,idempotency_key))",
        "CREATE TABLE task_init_receipts (idempotency_key TEXT PRIMARY KEY NOT NULL, digest TEXT NOT NULL, task_id TEXT NOT NULL REFERENCES tasks(id), revision INTEGER NOT NULL)",
];

pub async fn run(args: &[String]) -> Result<Value, Error> {
    if args == ["--help"] {
        return Ok(
            json!({"schema_version":1,"status":"ok","revision":0,"result":{"command":"task","commands":["init","record","context","review","correct","retract","export","backup","restore"]}}),
        );
    }
    let [flag, path, action] = args else {
        return Err(Error::invalid());
    };
    if !Path::new(path).is_absolute() {
        return Err(Error::invalid());
    }
    if flag == "--ledger" {
        return match action.as_str() {
            "export" => recovery::export(Path::new(path), read_input(true)?).await,
            "backup" => recovery::backup(Path::new(path), read_input(false)?).await,
            "restore" => recovery::restore(Path::new(path), read_input(false)?).await,
            _ => Err(Error::invalid()),
        };
    }
    if flag != "--config" {
        return Err(Error::invalid());
    }
    match action.as_str() {
        "init" => init(Path::new(path), read_input(false)?).await,
        "record" => record(Path::new(path), read_input(false)?).await,
        "context" => context(Path::new(path), read_input(true)?).await,
        "review" => lifecycle::review(Path::new(path), read_input(false)?).await,
        "correct" => lifecycle::correct(Path::new(path), read_input(false)?).await,
        "retract" => lifecycle::retract(Path::new(path), read_input(false)?).await,
        _ => Err(Error::invalid()),
    }
}

fn read_input<T: DeserializeOwned>(optional: bool) -> Result<T, Error> {
    let mut input = Vec::new();
    std::io::stdin()
        .take(MAX_INPUT + 1)
        .read_to_end(&mut input)
        .map_err(|_| Error::invalid())?;
    if input.len() as u64 > MAX_INPUT {
        return Err(Error::invalid());
    }
    if optional && input.iter().all(u8::is_ascii_whitespace) {
        input = b"{}".to_vec();
    }
    let value: Value = serde_json::from_slice(&input).map_err(|_| Error::invalid())?;
    if !value.is_object() || contains_null(&value) {
        return Err(Error::invalid());
    }
    serde_json::from_value(value).map_err(|_| Error::invalid())
}

fn contains_null(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(values) => values.iter().any(contains_null),
        Value::Object(values) => values.values().any(contains_null),
        _ => false,
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: i64,
    task_id: String,
    ledger_env: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Confirmation {
    Confirmed,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Capture {
    Execution,
    Review,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InitInput {
    idempotency_key: String,
    display_name: String,
    confirmation: Confirmation,
    automatic_capture: Vec<Capture>,
}

fn bounded(value: &str, max: usize) -> Result<(), Error> {
    if value.trim().is_empty() || value.chars().count() > max {
        Err(Error::invalid())
    } else {
        Ok(())
    }
}

fn canonical_uuid(value: &str) -> Result<(), Error> {
    if Uuid::parse_str(value)
        .map_err(|_| Error::invalid())?
        .to_string()
        != value
    {
        return Err(Error::invalid());
    }
    Ok(())
}

fn digest(action: &str, request: &impl Serialize) -> Result<String, Error> {
    let bytes = serde_json::to_vec(&json!({"action":action,"request":request}))
        .map_err(|_| Error::invalid())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn ledger_path() -> Result<PathBuf, Error> {
    let path = PathBuf::from(env::var_os(LEDGER_ENV).ok_or_else(Error::invalid)?);
    if !path.is_absolute() {
        return Err(Error::invalid());
    }
    Ok(path)
}

fn read_config(path: &Path) -> Result<Config, Error> {
    let file = fs::File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::not_found()
        } else {
            Error::storage(error)
        }
    })?;
    let mut bytes = Vec::new();
    file.take(MAX_INPUT + 1)
        .read_to_end(&mut bytes)
        .map_err(Error::storage)?;
    if bytes.len() as u64 > MAX_INPUT {
        return Err(Error::invalid());
    }
    let config: Config = serde_json::from_slice(&bytes).map_err(|_| Error::invalid())?;
    if config.schema_version != SCHEMA_VERSION {
        return Err(Error::version());
    }
    canonical_uuid(&config.task_id)?;
    if config.ledger_env != LEDGER_ENV {
        return Err(Error::scope());
    }
    Ok(config)
}

// A temporary sibling and a no-clobber hard link publish a complete file. A
// failed publication leaves the ledger receipt available for an exact retry.
struct TemporaryFile(PathBuf);
impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn temporary_file(parent: &Path) -> Result<(TemporaryFile, fs::File), Error> {
    let path = parent.join(format!(".memorynexus-{}.tmp", Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&path).map_err(Error::storage)?;
    Ok((TemporaryFile(path), file))
}

fn publish_config(path: &Path, config: &Config) -> Result<(), Error> {
    if fs::symlink_metadata(path).is_ok() {
        return if read_config(path)? == *config {
            Ok(())
        } else {
            Err(Error::conflict())
        };
    }
    let parent = path.parent().ok_or_else(Error::invalid)?;
    let (temporary, mut file) = temporary_file(parent)?;
    file.write_all(&serde_json::to_vec_pretty(config).map_err(Error::storage)?)
        .map_err(Error::storage)?;
    file.sync_all().map_err(Error::storage)?;
    match fs::hard_link(&temporary.0, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_config(path)? == *config {
                Ok(())
            } else {
                Err(Error::conflict())
            }
        }
        Err(error) => Err(Error::storage(error)),
    }
}

async fn connection(path: &Path, read_only: bool) -> Result<SqliteConnection, Error> {
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(path)
            .read_only(read_only)
            .create_if_missing(false)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5)),
    )
    .await
    .map_err(Error::storage)
}

/// Guard the legacy experiment entrypoint before any schema or restore mutation.
pub async fn is_task_ledger(path: &Path) -> Result<bool, Error> {
    if !path.is_file() {
        return Ok(false);
    }
    let mut conn = connection(path, true).await?;
    let application: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(&mut conn)
        .await
        .map_err(Error::storage)?;
    conn.close().await.map_err(Error::storage)?;
    Ok(application == APPLICATION_ID)
}

async fn validate_format(conn: &mut SqliteConnection) -> Result<(), Error> {
    let application: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(&mut *conn)
        .await
        .map_err(Error::storage)?;
    if application != APPLICATION_ID {
        return Err(Error::scope());
    }
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *conn)
        .await
        .map_err(Error::storage)?;
    if version != SCHEMA_VERSION {
        return Err(Error::version());
    }
    Ok(())
}

async fn open(path: &Path, read_only: bool) -> Result<SqliteConnection, Error> {
    if !path.is_file() {
        return Err(Error::not_found());
    }
    let setup = async {
        loop {
            let mut conn = connection(path, read_only).await?;
            validate_format(&mut conn).await?;
            if read_only {
                return Ok(conn);
            }
            match sqlx::query_scalar::<_, String>("PRAGMA journal_mode=WAL")
                .fetch_one(&mut conn)
                .await
            {
                Ok(mode) if mode == "wal" => return Ok(conn),
                Ok(_) => return Err(Error::storage("WAL activation failed")),
                Err(error)
                    if error
                        .as_database_error()
                        .and_then(|error| error.code())
                        .is_some_and(|code| {
                            code.parse::<i32>().is_ok_and(|code| code & 0xff == 5)
                        }) =>
                {
                    // Concurrent DELETE -> WAL upgrades may return SQLITE_BUSY
                    // without invoking busy_timeout. Release this connection's
                    // locks before retrying setup; no task transaction has begun.
                    conn.close().await.map_err(Error::storage)?;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(error) => return Err(Error::storage(error)),
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(5), setup)
        .await
        .map_err(Error::storage)?
}

async fn initialize_ledger(path: &Path) -> Result<(), Error> {
    if fs::symlink_metadata(path).is_ok() {
        let conn = open(path, true).await?;
        conn.close().await.map_err(Error::storage)?;
        return Ok(());
    }
    let parent = path.parent().ok_or_else(Error::invalid)?;
    fs::create_dir_all(parent).map_err(Error::storage)?;
    let (temporary, file) = temporary_file(parent)?;
    drop(file);
    let mut conn = connection(&temporary.0, false).await?;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    for statement in SCHEMA {
        sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
    }
    tx.commit().await.map_err(Error::storage)?;
    conn.close().await.map_err(Error::storage)?;
    fs::File::open(&temporary.0)
        .and_then(|file| file.sync_all())
        .map_err(Error::storage)?;
    match fs::hard_link(&temporary.0, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let conn = open(path, true).await?;
            conn.close().await.map_err(Error::storage)?;
            Ok(())
        }
        Err(error) => Err(Error::storage(error)),
    }
}

async fn init(path: &Path, mut input: InitInput) -> Result<Value, Error> {
    bounded(&input.idempotency_key, 128)?;
    bounded(&input.display_name, 120)?;
    input.automatic_capture.sort();
    if input
        .automatic_capture
        .windows(2)
        .any(|values| values[0] == values[1])
    {
        return Err(Error::invalid());
    }
    let request_digest = digest("init", &input)?;
    let existing_config = if fs::symlink_metadata(path).is_ok() {
        Some(read_config(path)?)
    } else {
        None
    };
    let ledger = ledger_path()?;
    if path == ledger {
        return Err(Error::scope());
    }
    if existing_config.is_some() && !ledger.is_file() {
        return Err(Error::not_found());
    }
    initialize_ledger(&ledger).await?;
    let mut conn = open(&ledger, false).await?;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    let receipt = sqlx::query(
        "SELECT digest, task_id, revision FROM task_init_receipts WHERE idempotency_key=?",
    )
    .bind(&input.idempotency_key)
    .fetch_optional(&mut *tx)
    .await
    .map_err(Error::storage)?;
    let (task_id, revision, status) = if let Some(receipt) = receipt {
        if receipt.get::<String, _>("digest") != request_digest {
            return Err(Error::conflict());
        }
        (
            receipt.get::<String, _>("task_id"),
            receipt.get::<i64, _>("revision"),
            "replayed",
        )
    } else {
        if existing_config.is_some() {
            return Err(Error::conflict());
        }
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO tasks VALUES (?,?,?,?,1)")
            .bind(&id)
            .bind(&input.display_name)
            .bind(input.automatic_capture.contains(&Capture::Execution))
            .bind(input.automatic_capture.contains(&Capture::Review))
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
        sqlx::query("INSERT INTO task_init_receipts VALUES (?,?,?,1)")
            .bind(&input.idempotency_key)
            .bind(&request_digest)
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
        (id, 1, "accepted")
    };
    let config = Config {
        schema_version: SCHEMA_VERSION,
        task_id,
        ledger_env: LEDGER_ENV.to_owned(),
    };
    if existing_config
        .as_ref()
        .is_some_and(|existing| *existing != config)
    {
        return Err(Error::conflict());
    }
    tx.commit().await.map_err(Error::storage)?;
    publish_config(path, &config)?;
    Ok(
        json!({"schema_version":1,"status":status,"task_id":config.task_id,"revision":revision,"result":{"display_name":input.display_name,"automatic_capture":input.automatic_capture}}),
    )
}

#[derive(Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RecordKind {
    Checkpoint,
    Execution,
    Instruction,
}
impl RecordKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Checkpoint => "checkpoint",
            Self::Execution => "execution",
            Self::Instruction => "instruction",
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EvidenceRef {
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RecordInput {
    idempotency_key: String,
    run_id: String,
    client: String,
    kind: RecordKind,
    text: String,
    occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    confirmation: Option<Confirmation>,
    #[serde(default)]
    evidence_refs: Vec<EvidenceRef>,
}

impl RecordInput {
    fn validate(&self) -> Result<(), Error> {
        bounded(&self.idempotency_key, 128)?;
        bounded(&self.client, 64)?;
        bounded(&self.text, 1000)?;
        canonical_uuid(&self.run_id)?;
        chrono::DateTime::parse_from_rfc3339(&self.occurred_at).map_err(|_| Error::invalid())?;
        validate_evidence_refs(&self.evidence_refs)
    }
}

fn validate_evidence_refs(refs: &[EvidenceRef]) -> Result<(), Error> {
    if refs.len() > 5 {
        return Err(Error::invalid());
    }
    for reference in refs {
        let uri_scheme = reference.path.split_once(':').is_some_and(|(prefix, _)| {
            prefix.starts_with(|character: char| character.is_ascii_alphabetic())
                && prefix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"+.-".contains(&byte))
        });
        if reference.path.trim().is_empty()
            || Path::new(&reference.path).is_absolute()
            || reference.path.starts_with('\\')
            || uri_scheme
            || reference.path.contains('\0')
            || reference.path.split(['/', '\\']).any(|part| part == "..")
        {
            return Err(Error::invalid());
        }
        if let Some(hash) = &reference.sha256 {
            if hash.len() != 64
                || !hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(Error::invalid());
            }
        }
    }
    Ok(())
}

async fn registered_task(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<sqlx::sqlite::SqliteRow, Error> {
    sqlx::query("SELECT * FROM tasks WHERE id=?")
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(Error::storage)?
        .ok_or_else(Error::not_found)
}

async fn record(path: &Path, input: RecordInput) -> Result<Value, Error> {
    input.validate()?;
    let request_digest = digest("record", &input)?;
    let config = read_config(path)?;
    let mut conn = open(&ledger_path()?, false).await?;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    let task = registered_task(&mut tx, &config.task_id).await?;
    let previous_revision = task.get::<i64, _>("revision");
    if let Some(response) = lifecycle::replay(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "record",
        &request_digest,
        previous_revision,
    )
    .await?
    {
        return Ok(response);
    }
    if input.confirmation.is_none()
        && (input.kind == RecordKind::Instruction || !task.get::<bool, _>("execution_capture"))
    {
        return Err(Error::invalid());
    }
    let trust = if input.confirmation.is_some() {
        "owner_confirmed"
    } else {
        "execution_reported"
    };
    let id = Uuid::new_v4().to_string();
    let revision = previous_revision + 1;
    if input.kind == RecordKind::Checkpoint {
        let previous: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM task_records WHERE task_id=? AND kind='checkpoint' AND state='current'",
        )
        .bind(&config.task_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(Error::storage)?;
        for id in previous {
            lifecycle::invalidate_dependents(&mut tx, &config.task_id, &id, revision).await?;
        }
        sqlx::query("UPDATE task_records SET state='historical', revision=? WHERE task_id=? AND kind='checkpoint' AND state='current'").bind(revision).bind(&config.task_id).execute(&mut *tx).await.map_err(Error::storage)?;
    }
    sqlx::query("INSERT INTO task_records (id,task_id,kind,trust,text,occurred_at,run_id,client,evidence_refs,state,revision) VALUES (?,?,?,?,?,?,?,?,?,'current',?)")
        .bind(&id).bind(&config.task_id).bind(input.kind.as_str()).bind(trust).bind(&input.text).bind(&input.occurred_at).bind(&input.run_id).bind(&input.client).bind(serde_json::to_string(&input.evidence_refs).map_err(Error::storage)?).bind(revision).execute(&mut *tx).await.map_err(Error::storage)?;
    lifecycle::save_receipt(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "record",
        &request_digest,
        &id,
        revision,
    )
    .await?;
    tx.commit().await.map_err(Error::storage)?;
    Ok(lifecycle::write_response(
        "accepted",
        &config.task_id,
        revision,
        &id,
        "current",
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextInput {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    record_ids: Option<Vec<String>>,
}
fn default_limit() -> usize {
    20
}

async fn context(path: &Path, input: ContextInput) -> Result<Value, Error> {
    if !(1..=20).contains(&input.limit) {
        return Err(Error::invalid());
    }
    if let Some(ids) = &input.record_ids {
        if ids.len() > 20 {
            return Err(Error::invalid());
        }
        for id in ids {
            canonical_uuid(id)?;
        }
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        if unique.len() != ids.len() {
            return Err(Error::invalid());
        }
    }
    let config = read_config(path)?;
    let mut conn = open(&ledger_path()?, true).await?;
    let mut tx = conn.begin().await.map_err(Error::storage)?;
    let task = registered_task(&mut tx, &config.task_id).await?;
    let mut rows = Vec::new();
    if let Some(ids) = &input.record_ids {
        for id in ids {
            let row = sqlx::query("SELECT * FROM task_records WHERE task_id=? AND id=?")
                .bind(&config.task_id)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(Error::storage)?;
            if let Some(row) = row {
                rows.push(row);
            } else {
                let exists: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_records WHERE id=?)")
                        .bind(id)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(Error::storage)?;
                return Err(if exists {
                    Error::scope()
                } else {
                    Error::not_found()
                });
            }
        }
    } else {
        rows = sqlx::query("SELECT * FROM task_records WHERE task_id=? AND state='current'")
            .bind(&config.task_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(Error::storage)?;
    }
    rows.sort_by(|a, b| {
        fn priority(row: &sqlx::sqlite::SqliteRow) -> u8 {
            match row.get::<String, _>("kind").as_str() {
                "instruction" => 0,
                "lesson" if row.get::<String, _>("trust") == "owner_confirmed" => 0,
                "lesson" => 3,
                "checkpoint" => 1,
                _ => 2,
            }
        }
        priority(a)
            .cmp(&priority(b))
            .then_with(|| {
                b.get::<i64, _>("commit_sequence")
                    .cmp(&a.get::<i64, _>("commit_sequence"))
            })
            .then_with(|| a.get::<String, _>("id").cmp(&b.get::<String, _>("id")))
    });
    let display_name: String = task.get("display_name");
    let mut characters = display_name.chars().count();
    let count = rows.len();
    let candidate_count = rows
        .iter()
        .filter(|row| {
            row.get::<String, _>("trust") == "review_candidate"
                && row.get::<String, _>("state") == "current"
        })
        .count();
    let mut entries = Vec::new();
    let mut candidates = Vec::new();
    for row in rows {
        let state: String = row.get("state");
        let visible = state != "retracted" && state != "stale";
        let text: String = if visible {
            row.get("text")
        } else {
            String::new()
        };
        if entries.len() + candidates.len() == input.limit
            || characters + text.chars().count() > 12000
        {
            continue;
        }
        characters += text.chars().count();
        let refs: Value =
            serde_json::from_str(&row.get::<String, _>("evidence_refs")).map_err(Error::storage)?;
        let id: String = row.get("id");
        let sources = lifecycle::source_ids(&mut tx, &id).await?;
        let supersedes: Option<String> =
            sqlx::query_scalar("SELECT supersedes_id FROM task_record_revisions WHERE record_id=?")
                .bind(&id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(Error::storage)?;
        let mut entry = json!({"id":id,"kind":row.get::<String,_>("kind"),"trust":row.get::<String,_>("trust"),"occurred_at":row.get::<String,_>("occurred_at"),"run_id":row.get::<String,_>("run_id"),"client":row.get::<String,_>("client"),"state":state,"revision":row.get::<i64,_>("revision"),"source_ids":sources});
        if visible {
            entry["text"] = json!(text);
            entry["evidence_refs"] = refs;
        }
        if let Some(supersedes) = supersedes {
            entry["supersedes_id"] = json!(supersedes);
        }
        if row.get::<String, _>("trust") == "review_candidate" {
            candidates.push(entry);
        } else {
            entries.push(entry);
        }
    }
    let omitted = count - entries.len() - candidates.len();
    Ok(
        json!({"schema_version":1,"status":"ok","task_id":config.task_id,"revision":task.get::<i64,_>("revision"),"result":{"display_name":display_name,"entries":entries,"candidates":candidates,"candidate_count":candidate_count,"omitted_count":omitted,"truncated":omitted>0}}),
    )
}

pub struct Error {
    code: &'static str,
    message: &'static str,
}
impl Error {
    fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
    fn invalid() -> Self {
        Self::new(
            "invalid_input",
            "request does not match the task memory contract",
        )
    }
    fn scope() -> Self {
        Self::new(
            "scope_mismatch",
            "the requested ledger or identity does not match this task",
        )
    }
    fn version() -> Self {
        Self::new(
            "unsupported_version",
            "task memory schema version is not supported",
        )
    }
    fn conflict() -> Self {
        Self::new("conflict", "request conflicts with retained task state")
    }
    fn not_found() -> Self {
        Self::new("not_found", "the requested task resource does not exist")
    }
    fn storage<E>(_: E) -> Self {
        Self::new(
            "storage_unavailable",
            "the local task ledger could not complete this request",
        )
    }
    pub fn response(self) -> Value {
        json!({"schema_version":1,"status":"error","code":self.code,"message":self.message})
    }
}
