//! Withdrawal events occupy their own position in the paired stream. Neither
//! the reason nor the withdrawn text is part of the wire representation.
use super::*;

pub(super) fn changes_schema() -> String {
    CHANGES
        .replace("UNIQUE REFERENCES task_records(id)", "UNIQUE")
        .replace(
            "PRIMARY KEY(task_id,epoch,origin,seq)",
            "subject_id TEXT REFERENCES task_records(id), PRIMARY KEY(task_id,epoch,origin,seq)",
        )
}

pub(super) async fn install(tx: &mut SqliteConnection) -> Result<(), Error> {
    if recovery::schema_version(tx).await? >= 11 {
        return Ok(());
    }
    revisions::allow_forks(tx).await?;
    let version = recovery::schema_version(tx).await?;
    // Keep all outstanding positions and their original batch timers while
    // replacing the creation-only log. This runs inside the write transaction.
    let schema = changes_schema();
    for sql in [
        "CREATE TEMP TABLE saved_relay_times AS SELECT * FROM task_relay_enqueued",
        "DROP TABLE task_relay_enqueued",
        "ALTER TABLE task_relay_changes RENAME TO task_relay_creations",
        &schema,
        "INSERT INTO task_relay_changes SELECT *,record_id FROM task_relay_creations",
        "DROP TABLE task_relay_creations",
        refresh::ENQUEUED,
        "INSERT INTO task_relay_enqueued SELECT * FROM saved_relay_times",
        "DROP TABLE saved_relay_times",
        if version == 9 {
            "PRAGMA user_version=11"
        } else {
            "PRAGMA user_version=12"
        },
    ] {
        sqlx::query(sql)
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
    }
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Withdrawal {
    target_id: String,
    confirmation: Confirmation,
}

pub(in crate::task_memory) async fn queue_withdrawal(
    tx: &mut SqliteConnection,
    task: &str,
    target: &str,
    revision: i64,
) -> Result<(), Error> {
    let Some(pair) = active(tx, task).await? else {
        return Ok(());
    };
    install(tx).await?;
    let input = Withdrawal {
        target_id: target.to_owned(),
        confirmation: Confirmation::Confirmed,
    };
    enqueue(
        tx,
        task,
        &pair,
        Uuid::new_v4().to_string(),
        revision,
        Vec::new(),
        json!({"action":"withdraw","input":input}),
    )
    .await
}

pub(super) async fn receive_withdrawal(
    tx: &mut SqliteConnection,
    task: &str,
    change: &Change,
) -> Result<(), Error> {
    if !change.parents.is_empty() {
        return Err(Error::invalid());
    }
    let input: Withdrawal =
        serde_json::from_value(change.mutation["input"].clone()).map_err(|_| Error::invalid())?;
    canonical_uuid(&input.target_id)?;
    lifecycle::scoped_record(tx, task, &input.target_id).await?;
    install(tx).await?;
    let revision = registered_task(tx, task).await?.get::<i64, _>("revision") + 1;
    lifecycle::withdraw_chain(tx, task, &input.target_id, revision).await?;
    sqlx::query("UPDATE tasks SET revision=? WHERE id=?")
        .bind(revision)
        .bind(task)
        .execute(tx)
        .await
        .map_err(Error::storage)?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Redaction {
    pub(super) digest: String,
    kind: String,
    trust: String,
    state: String,
    occurred_at: String,
    run_id: String,
    client: String,
    parents: Vec<String>,
    action: String,
    key: String,
    request_digest: String,
}

pub(super) async fn redacted(
    tx: &mut SqliteConnection,
    task: &str,
    rec: &SqliteRow,
    digest: &str,
) -> Result<Value, Error> {
    let id: &str = rec.get("id");
    let parents = sqlx::query_scalar(
        "SELECT supersedes_id FROM task_record_revisions WHERE record_id=? ORDER BY supersedes_id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(Error::storage)?;
    let receipt = sqlx::query("SELECT action,idempotency_key,digest FROM task_receipts WHERE task_id=? AND record_id=? AND action IN ('record','review','correct')")
        .bind(task).bind(id).fetch_one(tx).await.map_err(Error::storage)?;

    let input = Redaction {
        digest: digest.to_owned(),
        kind: rec.get("kind"),
        trust: rec.get("trust"),
        state: rec.get("state"),
        occurred_at: rec.get("occurred_at"),
        run_id: rec.get("run_id"),
        client: rec.get("client"),
        parents,
        action: receipt.get("action"),
        key: receipt.get("idempotency_key"),
        request_digest: receipt.get("digest"),
    };
    Ok(json!({"action":"redacted","input":input}))
}

pub(super) fn parse_redaction(c: &Change) -> Result<Option<Redaction>, Error> {
    if c.mutation["action"] != "redacted" {
        return Ok(None);
    }
    let input: Redaction =
        serde_json::from_value(c.mutation["input"].clone()).map_err(|_| Error::invalid())?;
    decode_tag(&input.digest)?;
    decode_tag(&input.request_digest)?;
    bounded(&input.key, 128)?;
    if input.parents.len() > 20
        || !matches!(input.action.as_str(), "correct" | "review" | "record")
        || (input.action == "correct") == input.parents.is_empty()
    {
        return Err(Error::invalid());
    }
    for parent in &input.parents {
        canonical_uuid(parent)?;
    }

    canonical_uuid(&input.run_id)?;
    bounded(&input.client, 64)?;
    chrono::DateTime::parse_from_rfc3339(&input.occurred_at).map_err(|_| Error::invalid())?;
    if !matches!(input.state.as_str(), "retracted" | "stale")
        || !matches!(
            input.kind.as_str(),
            "checkpoint" | "instruction" | "execution" | "lesson"
        )
        || !matches!(
            input.trust.as_str(),
            "execution_reported" | "owner_confirmed" | "review_candidate"
        )
    {
        return Err(Error::invalid());
    }
    Ok(Some(input))
}

pub(super) async fn receive_redaction(
    tx: &mut SqliteConnection,
    task: &str,
    c: &Change,
    input: Redaction,
) -> Result<(), Error> {
    install(tx).await?;
    // An unacknowledged predecessor may already have been scrubbed at the
    // origin. Persist only its identity and digest so its old signed batch
    // can subsequently be acknowledged without reintroducing its body.
    let revision = registered_task(tx, task).await?.get::<i64, _>("revision") + 1;
    sqlx::query("INSERT INTO task_records (id,task_id,kind,trust,text,occurred_at,run_id,client,evidence_refs,state,revision) VALUES (?,?,?,?,'',?,?,?,'[]',?,?)")
        .bind(&c.record_id).bind(task).bind(input.kind).bind(input.trust).bind(input.occurred_at).bind(input.run_id).bind(input.client).bind(input.state).bind(revision).execute(&mut *tx).await.map_err(Error::storage)?;
    for parent in &input.parents {
        lifecycle::scoped_record(tx, task, parent).await?;
        sqlx::query("INSERT INTO task_record_revisions VALUES (?,?)")
            .bind(&c.record_id)
            .bind(parent)
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
    }
    lifecycle::save_receipt(
        tx,
        task,
        &input.key,
        &input.action,
        &input.request_digest,
        &c.record_id,
        revision,
    )
    .await
}
