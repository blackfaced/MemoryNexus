//! Task-only inspection and SQLite recovery use cases.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportInput {}

pub(super) async fn export(path: &Path, _: ExportInput) -> Result<Value, Error> {
    let mut conn = open(path, true).await?;
    let mut tx = conn.begin().await.map_err(Error::storage)?;
    validate_snapshot(&mut tx).await?;
    let task_rows = sqlx::query("SELECT id, display_name, execution_capture, review_capture, revision FROM tasks ORDER BY id")
        .fetch_all(&mut *tx).await.map_err(Error::storage)?;
    let revision: i64 = task_rows
        .iter()
        .map(|row| row.get::<i64, _>("revision"))
        .sum();
    let tasks = task_rows.into_iter().map(|row| {
        let mut policy = Vec::new();
        if row.get::<bool, _>("execution_capture") { policy.push("execution"); }
        if row.get::<bool, _>("review_capture") { policy.push("review"); }
        json!({"id":row.get::<String,_>("id"),"display_name":row.get::<String,_>("display_name"),"automatic_capture":policy,"revision":row.get::<i64,_>("revision")})
    }).collect::<Vec<_>>();
    let rows = sqlx::query("SELECT commit_sequence, id, task_id, kind, trust, text, occurred_at, run_id, client, evidence_refs, state, revision FROM task_records ORDER BY commit_sequence, id")
        .fetch_all(&mut *tx).await.map_err(Error::storage)?;
    let mut records = Vec::new();
    for row in rows {
        let state: String = row.get("state");
        let mut record = json!({"commit_sequence":row.get::<i64,_>("commit_sequence"),"id":row.get::<String,_>("id"),"task_id":row.get::<String,_>("task_id"),"kind":row.get::<String,_>("kind"),"trust":row.get::<String,_>("trust"),"occurred_at":row.get::<String,_>("occurred_at"),"run_id":row.get::<String,_>("run_id"),"client":row.get::<String,_>("client"),"state":state,"revision":row.get::<i64,_>("revision")});
        if state != "retracted" {
            record["text"] = json!(row.get::<String, _>("text"));
            record["evidence_refs"] = serde_json::from_str(&row.get::<String, _>("evidence_refs"))
                .map_err(Error::storage)?;
        }
        records.push(record);
    }
    let sources = sqlx::query("SELECT record_id, source_id, position FROM task_record_sources ORDER BY record_id, position")
        .fetch_all(&mut *tx).await.map_err(Error::storage)?.into_iter()
        .map(|row| json!({"record_id":row.get::<String,_>("record_id"),"source_id":row.get::<String,_>("source_id"),"position":row.get::<i64,_>("position")})).collect::<Vec<_>>();
    let revisions = sqlx::query("SELECT record_id, supersedes_id FROM task_record_revisions ORDER BY record_id")
        .fetch_all(&mut *tx).await.map_err(Error::storage)?.into_iter()
        .map(|row| json!({"record_id":row.get::<String,_>("record_id"),"supersedes_id":row.get::<String,_>("supersedes_id")})).collect::<Vec<_>>();
    let receipts = sqlx::query("SELECT task_id, idempotency_key, action, digest, record_id, revision FROM task_receipts ORDER BY task_id, revision, idempotency_key")
        .fetch_all(&mut *tx).await.map_err(Error::storage)?.into_iter()
        .map(|row| json!({"task_id":row.get::<String,_>("task_id"),"idempotency_key":row.get::<String,_>("idempotency_key"),"action":row.get::<String,_>("action"),"digest":row.get::<String,_>("digest"),"record_id":row.get::<String,_>("record_id"),"revision":row.get::<i64,_>("revision")})).collect::<Vec<_>>();
    let init_receipts = sqlx::query("SELECT idempotency_key, digest, task_id, revision FROM task_init_receipts ORDER BY idempotency_key")
        .fetch_all(&mut *tx).await.map_err(Error::storage)?.into_iter()
        .map(|row| json!({"idempotency_key":row.get::<String,_>("idempotency_key"),"digest":row.get::<String,_>("digest"),"task_id":row.get::<String,_>("task_id"),"revision":row.get::<i64,_>("revision")})).collect::<Vec<_>>();
    let sequence: Option<i64> =
        sqlx::query_scalar("SELECT seq FROM sqlite_sequence WHERE name='task_records'")
            .fetch_optional(&mut *tx)
            .await
            .map_err(Error::storage)?;
    Ok(
        json!({"schema_version":1,"status":"ok","revision":revision,"result":{"format":"memorynexus-task-ledger","version":1,"tasks":tasks,"records":records,"record_sources":sources,"record_revisions":revisions,"receipts":receipts,"init_receipts":init_receipts,"commit_sequence":sequence.unwrap_or(0)}}),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PathInput {
    path: PathBuf,
}

pub(super) async fn backup(ledger: &Path, input: PathInput) -> Result<Value, Error> {
    snapshot(ledger, &input.path).await
}

pub(super) async fn restore(ledger: &Path, input: PathInput) -> Result<Value, Error> {
    snapshot(&input.path, ledger).await
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn require_new_destination(path: &Path) -> Result<(), Error> {
    for candidate in [
        path.to_owned(),
        sidecar(path, "-wal"),
        sidecar(path, "-shm"),
        sidecar(path, "-journal"),
    ] {
        match fs::symlink_metadata(candidate) {
            Ok(_) => return Err(Error::conflict()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::storage(error)),
        }
    }
    Ok(())
}

// Only this operation's randomly named sibling may be removed. Existing owner
// files, including orphan WAL/SHM files at the requested destination, are refused.
struct Snapshot {
    file: TemporaryFile,
}
impl Drop for Snapshot {
    fn drop(&mut self) {
        for suffix in ["-wal", "-shm", "-journal"] {
            let _ = fs::remove_file(sidecar(&self.file.0, suffix));
        }
    }
}

async fn validate_snapshot(conn: &mut SqliteConnection) -> Result<i64, Error> {
    validate_format(conn).await?;
    // The fixed v1 schema includes its constraints and indexes, not just table
    // names. Do not adopt an experiment DB, partial schema, or future extension.
    let mut actual: Vec<String> = sqlx::query_scalar(
        "SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::storage)?;
    let mut expected: Vec<String> = SCHEMA
        .iter()
        .filter(|statement| statement.starts_with("CREATE "))
        .map(|statement| statement.to_string())
        .collect();
    actual.sort();
    expected.sort();
    if actual != expected {
        return Err(Error::version());
    }
    let integrity: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_all(&mut *conn)
        .await
        .map_err(Error::storage)?;
    if integrity != ["ok"] {
        return Err(Error::storage("invalid integrity"));
    }
    if !sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&mut *conn)
        .await
        .map_err(Error::storage)?
        .is_empty()
    {
        return Err(Error::storage("invalid links"));
    }
    sqlx::query_scalar("SELECT coalesce(sum(revision),0) FROM tasks")
        .fetch_one(&mut *conn)
        .await
        .map_err(Error::storage)
}

async fn snapshot(source: &Path, destination: &Path) -> Result<Value, Error> {
    if !source.is_absolute() || !destination.is_absolute() {
        return Err(Error::invalid());
    }
    require_new_destination(destination)?;
    let mut source_conn = open(source, true).await?;
    validate_snapshot(&mut source_conn).await?;
    let parent = destination.parent().ok_or_else(Error::invalid)?;
    let (temporary, file) = temporary_file(parent)?;
    let temporary = Snapshot { file: temporary };
    drop(file);
    // SQLite materializes the live source and its WAL in one consistent snapshot.
    // Plain fs::copy(source, ...) would lose committed WAL-only changes.
    sqlx::query("VACUUM INTO ?")
        .bind(temporary.file.0.to_str().ok_or_else(Error::invalid)?)
        .execute(&mut source_conn)
        .await
        .map_err(Error::storage)?;
    source_conn.close().await.map_err(Error::storage)?;
    let mut restored = connection(&temporary.file.0, false).await?;
    let revision = validate_snapshot(&mut restored).await?;
    sqlx::query("PRAGMA journal_mode=DELETE")
        .execute(&mut restored)
        .await
        .map_err(Error::storage)?;
    restored.close().await.map_err(Error::storage)?;
    fs::File::open(&temporary.file.0)
        .and_then(|file| file.sync_all())
        .map_err(Error::storage)?;
    require_new_destination(destination)?;
    fs::hard_link(&temporary.file.0, destination).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Error::conflict()
        } else {
            Error::storage(error)
        }
    })?;
    // Publish only a complete closed SQLite file. If the process fails after the
    // link, an intact destination remains for inspection; never overwrite it.
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(Error::storage)?;
    Ok(
        json!({"schema_version":1,"status":"ok","revision":revision,"result":{"path":destination,"format":"memorynexus-task-ledger","version":1}}),
    )
}
