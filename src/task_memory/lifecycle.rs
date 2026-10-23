use super::*;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewInput {
    idempotency_key: String,
    run_id: String,
    client: String,
    text: String,
    source_ids: Vec<String>,
    occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    confirmation: Option<Confirmation>,
}

fn validate_envelope(key: &str, run_id: &str, client: &str) -> Result<(), Error> {
    bounded(key, 128)?;
    canonical_uuid(run_id)?;
    bounded(client, 64)
}

fn validate_sources(ids: &[String]) -> Result<(), Error> {
    if ids.is_empty() || ids.len() > 20 {
        return Err(Error::invalid());
    }
    for id in ids {
        canonical_uuid(id)?;
    }
    if ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len() {
        return Err(Error::invalid());
    }
    Ok(())
}

pub(super) async fn scoped_record(
    conn: &mut SqliteConnection,
    task_id: &str,
    id: &str,
) -> Result<sqlx::sqlite::SqliteRow, Error> {
    if let Some(row) = sqlx::query("SELECT * FROM task_records WHERE task_id=? AND id=?")
        .bind(task_id)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(Error::storage)?
    {
        return Ok(row);
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_records WHERE id=?)")
        .bind(id)
        .fetch_one(conn)
        .await
        .map_err(Error::storage)?;
    Err(if exists {
        Error::scope()
    } else {
        Error::not_found()
    })
}

async fn require_current_sources(
    conn: &mut SqliteConnection,
    task_id: &str,
    ids: &[String],
) -> Result<(), Error> {
    validate_sources(ids)?;
    for id in ids {
        let row = scoped_record(conn, task_id, id).await?;
        if row.get::<String, _>("kind") == "lesson" {
            return Err(Error::invalid());
        }
        if row.get::<String, _>("state") != "current" {
            return Err(Error::conflict());
        }
    }
    Ok(())
}

pub(super) async fn source_ids(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Vec<String>, Error> {
    sqlx::query_scalar(
        "SELECT source_id FROM task_record_sources WHERE record_id=? ORDER BY position",
    )
    .bind(id)
    .fetch_all(conn)
    .await
    .map_err(Error::storage)
}

async fn save_sources(
    conn: &mut SqliteConnection,
    id: &str,
    sources: &[String],
) -> Result<(), Error> {
    for (position, source) in sources.iter().enumerate() {
        sqlx::query("INSERT INTO task_record_sources VALUES (?,?,?)")
            .bind(id)
            .bind(source)
            .bind(position as i64)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
    }
    Ok(())
}

pub(super) async fn replay(
    conn: &mut SqliteConnection,
    task_id: &str,
    key: &str,
    action: &str,
    request_digest: &str,
    revision: i64,
) -> Result<Option<Value>, Error> {
    let receipt = sqlx::query(
        "SELECT action,digest,record_id FROM task_receipts WHERE task_id=? AND idempotency_key=?",
    )
    .bind(task_id)
    .bind(key)
    .fetch_optional(&mut *conn)
    .await
    .map_err(Error::storage)?;
    let Some(receipt) = receipt else {
        return Ok(None);
    };
    if receipt.get::<String, _>("action") != action
        || receipt.get::<String, _>("digest") != request_digest
    {
        return Err(Error::conflict());
    }
    let id: String = receipt.get("record_id");
    let record = scoped_record(conn, task_id, &id).await?;
    Ok(Some(write_response(
        "replayed",
        task_id,
        revision,
        &id,
        &record.get::<String, _>("state"),
    )))
}

pub(super) fn write_response(
    status: &str,
    task_id: &str,
    revision: i64,
    id: &str,
    state: &str,
) -> Value {
    json!({"schema_version":1,"status":status,"task_id":task_id,"revision":revision,"result":{"record_id":id,"state":state}})
}

pub(super) async fn save_receipt(
    conn: &mut SqliteConnection,
    task_id: &str,
    key: &str,
    action: &str,
    request_digest: &str,
    id: &str,
    revision: i64,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO task_receipts (task_id,idempotency_key,action,digest,record_id,revision) VALUES (?,?,?,?,?,?)")
        .bind(task_id)
        .bind(key)
        .bind(action)
        .bind(request_digest)
        .bind(id)
        .bind(revision)
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    sqlx::query("UPDATE tasks SET revision=? WHERE id=?")
        .bind(revision)
        .bind(task_id)
        .execute(conn)
        .await
        .map_err(Error::storage)?;
    Ok(())
}

pub(super) async fn review(path: &Path, input: ReviewInput) -> Result<Value, Error> {
    validate_envelope(&input.idempotency_key, &input.run_id, &input.client)?;
    bounded(&input.text, 1000)?;
    chrono::DateTime::parse_from_rfc3339(&input.occurred_at).map_err(|_| Error::invalid())?;
    validate_sources(&input.source_ids)?;
    let request_digest = digest("review", &input)?;
    let config = read_config(path)?;
    let mut conn = open(&ledger_path()?, false).await?;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    let task = registered_task(&mut tx, &config.task_id).await?;
    let previous_revision = task.get::<i64, _>("revision");
    if let Some(response) = replay(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "review",
        &request_digest,
        previous_revision,
    )
    .await?
    {
        return Ok(response);
    }
    if input.confirmation.is_none() && !task.get::<bool, _>("review_capture") {
        return Err(Error::invalid());
    }
    require_current_sources(&mut tx, &config.task_id, &input.source_ids).await?;
    let id = Uuid::new_v4().to_string();
    let revision = previous_revision + 1;
    sqlx::query("INSERT INTO task_records (id,task_id,kind,trust,text,occurred_at,run_id,client,evidence_refs,state,revision) VALUES (?,?,'lesson','review_candidate',?,?,?,?,'[]','current',?)")
        .bind(&id).bind(&config.task_id).bind(&input.text).bind(&input.occurred_at).bind(&input.run_id).bind(&input.client).bind(revision).execute(&mut *tx).await.map_err(Error::storage)?;
    save_sources(&mut tx, &id, &input.source_ids).await?;
    save_receipt(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "review",
        &request_digest,
        &id,
        revision,
    )
    .await?;
    tx.commit().await.map_err(Error::storage)?;
    Ok(write_response(
        "accepted",
        &config.task_id,
        revision,
        &id,
        "current",
    ))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CorrectInput {
    idempotency_key: String,
    run_id: String,
    client: String,
    target_id: String,
    text: String,
    occurred_at: String,
    confirmation: Confirmation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evidence_refs: Option<Vec<EvidenceRef>>,
}

pub(super) async fn invalidate_dependents(
    conn: &mut SqliteConnection,
    task_id: &str,
    source: &str,
    revision: i64,
) -> Result<(), Error> {
    workflow::invalidate_record(conn, task_id, source, revision).await?;
    sqlx::query("UPDATE task_records SET state='stale', revision=? WHERE task_id=? AND state='current' AND id IN (SELECT record_id FROM task_record_sources WHERE source_id=?)")
        .bind(revision).bind(task_id).bind(source).execute(conn).await.map_err(Error::storage)?;
    Ok(())
}

pub(super) async fn correct(path: &Path, input: CorrectInput) -> Result<Value, Error> {
    validate_envelope(&input.idempotency_key, &input.run_id, &input.client)?;
    canonical_uuid(&input.target_id)?;
    bounded(&input.text, 1000)?;
    chrono::DateTime::parse_from_rfc3339(&input.occurred_at).map_err(|_| Error::invalid())?;
    if let Some(refs) = &input.evidence_refs {
        validate_evidence_refs(refs)?;
    }
    if let Some(ids) = &input.source_ids {
        if !ids.is_empty() {
            validate_sources(ids)?;
        }
    }
    let request_digest = digest("correct", &input)?;
    let config = read_config(path)?;
    let mut conn = open(&ledger_path()?, false).await?;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    let task = registered_task(&mut tx, &config.task_id).await?;
    let previous_revision = task.get::<i64, _>("revision");
    if let Some(response) = replay(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "correct",
        &request_digest,
        previous_revision,
    )
    .await?
    {
        return Ok(response);
    }
    let target = scoped_record(&mut tx, &config.task_id, &input.target_id).await?;
    if target.get::<String, _>("state") != "current" {
        return Err(Error::conflict());
    }
    let sources = match &input.source_ids {
        Some(ids) => ids.clone(),
        None => source_ids(&mut tx, &input.target_id).await?,
    };
    let kind: String = target.get("kind");
    if kind == "lesson" {
        require_current_sources(&mut tx, &config.task_id, &sources).await?;
    } else if !sources.is_empty() {
        return Err(Error::invalid());
    }
    let refs = match &input.evidence_refs {
        Some(refs) => serde_json::to_string(refs).map_err(Error::storage)?,
        None => target.get("evidence_refs"),
    };
    let id = Uuid::new_v4().to_string();
    let revision = previous_revision + 1;
    sqlx::query("UPDATE task_records SET state='superseded',revision=? WHERE task_id=? AND id=?")
        .bind(revision)
        .bind(&config.task_id)
        .bind(&input.target_id)
        .execute(&mut *tx)
        .await
        .map_err(Error::storage)?;
    invalidate_dependents(&mut tx, &config.task_id, &input.target_id, revision).await?;
    sqlx::query("INSERT INTO task_records (id,task_id,kind,trust,text,occurred_at,run_id,client,evidence_refs,state,revision) VALUES (?,?,?,'owner_confirmed',?,?,?,?,?,'current',?)")
        .bind(&id).bind(&config.task_id).bind(&kind).bind(&input.text).bind(&input.occurred_at).bind(&input.run_id).bind(&input.client).bind(&refs).bind(revision).execute(&mut *tx).await.map_err(Error::storage)?;
    sqlx::query("INSERT INTO task_record_revisions VALUES (?,?)")
        .bind(&id)
        .bind(&input.target_id)
        .execute(&mut *tx)
        .await
        .map_err(Error::storage)?;
    save_sources(&mut tx, &id, &sources).await?;
    save_receipt(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "correct",
        &request_digest,
        &id,
        revision,
    )
    .await?;
    tx.commit().await.map_err(Error::storage)?;
    Ok(write_response(
        "accepted",
        &config.task_id,
        revision,
        &id,
        "current",
    ))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RetractInput {
    idempotency_key: String,
    run_id: String,
    client: String,
    target_id: String,
    reason: String,
    confirmation: Confirmation,
}

pub(super) async fn retract(path: &Path, input: RetractInput) -> Result<Value, Error> {
    validate_envelope(&input.idempotency_key, &input.run_id, &input.client)?;
    canonical_uuid(&input.target_id)?;
    bounded(&input.reason, 240)?;
    let request_digest = digest("retract", &input)?;
    let config = read_config(path)?;
    let mut conn = open(&ledger_path()?, false).await?;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    let task = registered_task(&mut tx, &config.task_id).await?;
    let previous_revision = task.get::<i64, _>("revision");
    if let Some(response) = replay(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "retract",
        &request_digest,
        previous_revision,
    )
    .await?
    {
        return Ok(response);
    }
    let target = scoped_record(&mut tx, &config.task_id, &input.target_id).await?;
    if target.get::<String, _>("state") == "retracted" {
        return Err(Error::conflict());
    }
    // Traverse both directions so withdrawing any revision removes the entire
    // retained chain, including successors written after the supplied ID.
    let chain: Vec<String> = sqlx::query_scalar("WITH RECURSIVE chain(id) AS (SELECT id FROM task_records WHERE task_id=? AND id=? UNION SELECT CASE WHEN r.record_id=c.id THEN r.supersedes_id ELSE r.record_id END FROM task_record_revisions r JOIN chain c ON r.record_id=c.id OR r.supersedes_id=c.id) SELECT id FROM chain")
        .bind(&config.task_id).bind(&input.target_id).fetch_all(&mut *tx).await.map_err(Error::storage)?;
    let revision = previous_revision + 1;
    workflow::retract_records(&mut tx, &config.task_id, &chain, revision).await?;
    for id in chain {
        invalidate_dependents(&mut tx, &config.task_id, &id, revision).await?;
        sqlx::query("UPDATE task_records SET state='retracted',text='',evidence_refs='[]',revision=? WHERE task_id=? AND id=?")
            .bind(revision).bind(&config.task_id).bind(&id).execute(&mut *tx).await.map_err(Error::storage)?;
    }
    // Only the digest is retained; the reason and request body are never copied.
    save_receipt(
        &mut tx,
        &config.task_id,
        &input.idempotency_key,
        "retract",
        &request_digest,
        &input.target_id,
        revision,
    )
    .await?;
    tx.commit().await.map_err(Error::storage)?;
    Ok(write_response(
        "accepted",
        &config.task_id,
        revision,
        &input.target_id,
        "retracted",
    ))
}
