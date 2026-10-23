//! Repository memory workflows over the existing task ledger.
use super::*;
mod consumers;
mod lifecycle_hooks;
mod reads;
mod schema;
mod sources;
mod validation;
use lifecycle_hooks::*;
pub(super) use lifecycle_hooks::{invalidate_record, retract_records};
struct Operation {
    changed: bool,
    result: Value,
    kind: &'static str,
    id: String,
}
pub(super) use schema::{export_tables, restore_gate, upgrade, validate_schema};
use validation::*;
pub(super) const ACTIONS: &[&str] = &[
    "workflow-bind",
    "source-sync",
    "source-capture",
    "consumer-review",
    "consumer-approve",
    "publication-result",
    "workflow-status",
];
fn stale() -> Error {
    Error::new("stale_snapshot", "workflow snapshot is no longer current")
}
fn approval() -> Error {
    Error::new("approval_required", "exact workflow approval is required")
}
fn recovery() -> Error {
    Error::new(
        "recovery_required",
        "restored workflow requires reconciliation and renewed approval",
    )
}
async fn incarnation(conn: &mut SqliteConnection) -> Result<String, Error> {
    sqlx::query_scalar("SELECT incarnation FROM task_ledger_meta WHERE singleton=1")
        .fetch_one(conn)
        .await
        .map_err(Error::storage)
}
async fn next_sequence(conn: &mut SqliteConnection) -> Result<i64, Error> {
    sqlx::query_scalar("UPDATE task_ledger_meta SET workflow_sequence=workflow_sequence+1 WHERE singleton=1 RETURNING workflow_sequence").fetch_one(conn).await.map_err(Error::storage)
}
fn response(status: &str, task: &str, rev: i64, inc: &str, result: Value) -> Value {
    json!({"schema_version":1,"workflow_contract":1,"status":status,"task_id":task,"revision":rev,"incarnation":inc,"result":result})
}
async fn workflow(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<sqlx::sqlite::SqliteRow, Error> {
    if let Some(row) = sqlx::query("SELECT * FROM task_workflows WHERE task_id=? AND workflow_id=?")
        .bind(task)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(Error::storage)?
    {
        return Ok(row);
    }
    let other: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_workflows WHERE workflow_id=?)")
            .bind(id)
            .fetch_one(conn)
            .await
            .map_err(Error::storage)?;
    Err(if other {
        Error::scope()
    } else {
        Error::not_found()
    })
}
async fn save_receipt(
    conn: &mut SqliteConnection,
    task: &str,
    action: &str,
    input: &Value,
    result: &Value,
    identity: (&str, &str),
    rev: i64,
) -> Result<(), Error> {
    let (kind, id) = identity;
    sqlx::query("INSERT INTO task_receipts (task_id,idempotency_key,action,digest,revision,result_type,result_id,result_json) VALUES (?,?,?,?,?,?,?,?)").bind(task).bind(string(input,"idempotency_key")?).bind(action).bind(digest(action,input)?).bind(rev).bind(kind).bind(id).bind(result.to_string()).execute(&mut *conn).await.map_err(Error::storage)?;
    sqlx::query("UPDATE tasks SET revision=? WHERE id=?")
        .bind(rev)
        .bind(task)
        .execute(conn)
        .await
        .map_err(Error::storage)?;
    Ok(())
}
pub(super) async fn run(path: &Path, action: &str, mut input: Value) -> Result<Value, Error> {
    let config = read_config(path)?;
    let mut conn = open(&ledger_path()?, action == "workflow-status").await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut conn)
        .await
        .map_err(Error::storage)?;
    if version != 2 {
        return Err(Error::version());
    }
    if action == "workflow-status" {
        return reads::status(&mut conn, &config.task_id, &input).await;
    }
    if action == "workflow-bind" {
        envelope(
            &input,
            &[
                "expected_profile_revision",
                "profile",
                "profile_sha256",
                "binding_sha256",
                "confirmation",
            ],
            &["recovery_acknowledged"],
        )?;
        profile(&input["profile"])?;
        number(&input, "expected_profile_revision")?;
        literal(&input, "confirmation", "confirmed")?;
        for k in ["profile_sha256", "binding_sha256"] {
            hash(string(&input, k)?)?;
        }
        if input.get("recovery_acknowledged").is_some() && input["recovery_acknowledged"] != true {
            return Err(Error::invalid());
        }
        if input["profile"]["workflow_id"] != input["workflow_id"]
            || input["profile"]["revision"] != input["profile_revision"]
            || hash_value(&input["profile"]) != string(&input, "profile_sha256")?
        {
            return Err(Error::invalid());
        }
    } else if ["source-sync", "source-capture"].contains(&action) {
        sources::validate(action, &input)?;
    } else {
        consumers::validate(action, &mut input)?;
    }
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    let task = registered_task(&mut tx, &config.task_id).await?;
    let rev = task.get::<i64, _>("revision");
    let inc = incarnation(&mut tx).await?;
    if let Some(r) = sqlx::query(
        "SELECT action,digest,result_json FROM task_receipts WHERE task_id=? AND idempotency_key=?",
    )
    .bind(&config.task_id)
    .bind(string(&input, "idempotency_key")?)
    .fetch_optional(&mut *tx)
    .await
    .map_err(Error::storage)?
    {
        if r.get::<String, _>("action") != action
            || r.get::<String, _>("digest") != digest(action, &input)?
        {
            return Err(Error::conflict());
        }
        let mut result: Value =
            serde_json::from_str(&r.get::<String, _>("result_json")).map_err(Error::storage)?;
        result = current_result(&mut tx, &config.task_id, action, result).await?;
        return Ok(response("replayed", &config.task_id, rev, &inc, result));
    }
    if string(&input, "incarnation")? != inc {
        return Err(if action == "publication-result" {
            recovery()
        } else {
            stale()
        });
    }
    let op = if action == "workflow-bind" {
        let (changed, result) = bind(&mut tx, &config.task_id, &input, &inc).await?;
        Operation {
            changed,
            result,
            kind: "workflow",
            id: string(&input, "workflow_id")?.to_owned(),
        }
    } else {
        let w = workflow(&mut tx, &config.task_id, string(&input, "workflow_id")?).await?;
        if w.get::<String, _>("state") != "active" {
            return Err(recovery());
        }
        if action != "publication-result"
            && w.get::<i64, _>("profile_revision") != number(&input, "profile_revision")?
        {
            return Err(stale());
        }
        let p: Value =
            serde_json::from_str(&w.get::<String, _>("profile")).map_err(Error::storage)?;
        if ["source-sync", "source-capture"].contains(&action) {
            sources::execute(&mut tx, &config.task_id, action, &input, &p, rev + 1).await?
        } else {
            consumers::execute(&mut tx, &config.task_id, action, &input, &p, rev + 1).await?
        }
    };
    if op.changed {
        save_receipt(
            &mut tx,
            &config.task_id,
            action,
            &input,
            &op.result,
            (op.kind, &op.id),
            rev + 1,
        )
        .await?;
        tx.commit().await.map_err(Error::storage)?;
    }
    Ok(response(
        if op.changed { "accepted" } else { "noop" },
        &config.task_id,
        rev + i64::from(op.changed),
        &inc,
        op.result,
    ))
}
async fn bind(
    conn: &mut SqliteConnection,
    task: &str,
    v: &Value,
    inc: &str,
) -> Result<(bool, Value), Error> {
    let id = string(v, "workflow_id")?;
    let old = sqlx::query("SELECT * FROM task_workflows WHERE task_id=? AND workflow_id=?")
        .bind(task)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(Error::storage)?;
    let p = &v["profile"];
    let result = json!({"workflow_id":id,"profile_revision":v["profile_revision"],"profile_sha256":v["profile_sha256"],"binding_sha256":v["binding_sha256"],"state":"active"});
    if let Some(old) = &old {
        if old.get::<String, _>("state") == "recovery_required"
            && v["recovery_acknowledged"] != true
        {
            return Err(recovery());
        }
        if old.get::<String, _>("state") == "active"
            && old.get::<String, _>("profile").as_str() == p.to_string().as_str()
            && old.get::<String, _>("profile_sha256") == string(v, "profile_sha256")?
            && old.get::<String, _>("binding_sha256") == string(v, "binding_sha256")?
            && old.get::<String, _>("incarnation") == inc
        {
            return Ok((false, result));
        }
    }
    let expected = old
        .as_ref()
        .map(|o| o.get::<i64, _>("profile_revision"))
        .unwrap_or(0);
    if number(v, "expected_profile_revision")? != expected
        || number(v, "profile_revision")? != expected + 1
    {
        return Err(Error::conflict());
    }
    if let Some(old) = &old {
        let prior: Value =
            serde_json::from_str(&old.get::<String, _>("profile")).map_err(Error::storage)?;
        for c in p["collections"].as_array().unwrap() {
            if let Some(o) = prior["collections"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["collection_id"] == c["collection_id"])
            {
                if (o["selector"] != c["selector"] || o["format"] != c["format"])
                    && o["extractor_version"] == c["extractor_version"]
                {
                    return Err(Error::invalid());
                }
                if o["format"] != c["format"] {
                    return Err(Error::conflict());
                }
            }
        }
    }
    if let Some(old) = &old {
        invalidate_workflow(conn, task, id).await?;
        let rev = registered_task(conn, task).await?.get::<i64, _>("revision") + 1;
        let prior: Value =
            serde_json::from_str(&old.get::<String, _>("profile")).map_err(Error::storage)?;
        for prior_c in prior["collections"].as_array().unwrap() {
            let new = p["collections"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["collection_id"] == prior_c["collection_id"]);
            if new.map_or(true, |c| {
                c["selector"] != prior_c["selector"]
                    || c["extractor_version"] != prior_c["extractor_version"]
            }) {
                let ids:Vec<String>=sqlx::query_scalar("SELECT source_id FROM task_sources WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(task).bind(id).bind(string(prior_c,"collection_id")?).fetch_all(&mut *conn).await.map_err(Error::storage)?;
                for sid in ids {
                    invalidate_source(conn, task, &sid, rev).await?;
                }
            }
        }
    }
    sqlx::query("INSERT INTO task_workflows VALUES (?,?,?,?,?,?,?,'active') ON CONFLICT(task_id,workflow_id) DO UPDATE SET profile_revision=excluded.profile_revision,profile=excluded.profile,profile_sha256=excluded.profile_sha256,binding_sha256=excluded.binding_sha256,incarnation=excluded.incarnation,state='active'").bind(task).bind(id).bind(number(v,"profile_revision")?).bind(p.to_string()).bind(string(v,"profile_sha256")?).bind(string(v,"binding_sha256")?).bind(inc).execute(&mut *conn).await.map_err(Error::storage)?;
    let existing = sqlx::query(
        "SELECT collection_id,data FROM task_source_collections WHERE task_id=? AND workflow_id=?",
    )
    .bind(task)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::storage)?;
    for row in existing {
        let mut d: Value =
            serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
        d["active"] = json!(false);
        sqlx::query("UPDATE task_source_collections SET data=? WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(d.to_string()).bind(task).bind(id).bind(row.get::<String,_>("collection_id")).execute(&mut *conn).await.map_err(Error::storage)?;
    }
    for c in p["collections"].as_array().unwrap() {
        let cid = string(c, "collection_id")?;
        let row=sqlx::query("SELECT data FROM task_source_collections WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(task).bind(id).bind(cid).fetch_optional(&mut *conn).await.map_err(Error::storage)?;
        if let Some(row) = row {
            let mut d: Value =
                serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
            d["active"] = json!(true);
            sqlx::query("UPDATE task_source_collections SET definition=?,data=? WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(c.to_string()).bind(d.to_string()).bind(task).bind(id).bind(cid).execute(&mut *conn).await.map_err(Error::storage)?;
        } else {
            let seq = next_sequence(conn).await?;
            let mut d = json!({"collection_id":cid,"format":c["format"],"scan":0,"availability":"unknown","active":true});
            if c["format"] == "append_jsonl" {
                d["cursor"] = json!({"offset":0,"prefix_sha256":hash_bytes(b"")});
            }
            sqlx::query("INSERT INTO task_source_collections VALUES (?,?,?,?,?,NULL,?)")
                .bind(task)
                .bind(id)
                .bind(cid)
                .bind(c.to_string())
                .bind(d.to_string())
                .bind(seq)
                .execute(&mut *conn)
                .await
                .map_err(Error::storage)?;
        }
    }
    Ok((true, result))
}

async fn current_result(
    conn: &mut SqliteConnection,
    task: &str,
    action: &str,
    mut result: Value,
) -> Result<Value, Error> {
    if action == "source-capture" {
        return sources::capture_result(conn, task, string(&result, "record_id")?).await;
    }
    if ["consumer-review", "consumer-approve", "publication-result"].contains(&action) {
        return consumers::current_result(
            conn,
            task,
            match action {
                "consumer-review" => "decision",
                "consumer-approve" => "approval",
                _ => "publication",
            },
            &result,
        )
        .await;
    }
    if let Some(id) = result.get("record_id").and_then(Value::as_str) {
        let r = lifecycle::scoped_record(conn, task, id).await?;
        result["state"] = json!(r.get::<String, _>("state"));
    }
    if action == "workflow-bind" {
        let w = workflow(conn, task, string(&result, "workflow_id")?).await?;
        result["state"] = json!(w.get::<String, _>("state"));
    }
    Ok(result)
}
