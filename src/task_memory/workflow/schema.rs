//! Exact task-ledger v2 schema. Migration is the only caller of `upgrade`.
use super::*;

pub(super) const RECEIPTS: &str = "CREATE TABLE task_receipts (task_id TEXT NOT NULL REFERENCES tasks(id), idempotency_key TEXT NOT NULL, action TEXT NOT NULL, digest TEXT NOT NULL, record_id TEXT REFERENCES task_records(id), revision INTEGER NOT NULL, result_type TEXT NOT NULL DEFAULT 'record' CHECK(result_type IN ('record','source_sync','capture','decision','approval','publication','workflow')), result_id TEXT, result_json TEXT, PRIMARY KEY(task_id,idempotency_key), CHECK(result_type != 'record' OR record_id IS NOT NULL), CHECK(result_type = 'record' OR (result_id IS NOT NULL AND result_json IS NOT NULL)))";
pub(super) const SCHEMA_V2: &[&str] = &[
 "CREATE UNIQUE INDEX task_records_task_id ON task_records(task_id,id)",
 "CREATE TABLE task_ledger_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), incarnation TEXT NOT NULL, workflow_sequence INTEGER NOT NULL CHECK(workflow_sequence>=0))",
 "CREATE TABLE task_workflows (task_id TEXT NOT NULL REFERENCES tasks(id), workflow_id TEXT NOT NULL, profile_revision INTEGER NOT NULL, profile TEXT NOT NULL, profile_sha256 TEXT NOT NULL, binding_sha256 TEXT NOT NULL, incarnation TEXT NOT NULL, state TEXT NOT NULL CHECK(state IN ('active','recovery_required')), PRIMARY KEY(task_id,workflow_id))",
 "CREATE TABLE task_source_collections (task_id TEXT NOT NULL, workflow_id TEXT NOT NULL, collection_id TEXT NOT NULL, definition TEXT NOT NULL, data TEXT NOT NULL, observed_definition_sha256 TEXT, workflow_sequence INTEGER NOT NULL UNIQUE, PRIMARY KEY(task_id,workflow_id,collection_id), UNIQUE(task_id,collection_id), FOREIGN KEY(task_id,workflow_id) REFERENCES task_workflows(task_id,workflow_id))",
 "CREATE TABLE task_sources (task_id TEXT NOT NULL, source_id TEXT NOT NULL, workflow_id TEXT NOT NULL, collection_id TEXT NOT NULL, root_record_id TEXT, current_revision INTEGER NOT NULL, blocked INTEGER NOT NULL CHECK(blocked IN (0,1)), start INTEGER, end INTEGER, workflow_sequence INTEGER NOT NULL UNIQUE, PRIMARY KEY(task_id,source_id), UNIQUE(source_id), FOREIGN KEY(task_id,workflow_id,collection_id) REFERENCES task_source_collections(task_id,workflow_id,collection_id), FOREIGN KEY(task_id,root_record_id) REFERENCES task_records(task_id,id))",
 "CREATE TABLE task_source_versions (task_id TEXT NOT NULL, source_id TEXT NOT NULL, revision INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(task_id,source_id,revision), FOREIGN KEY(task_id,source_id) REFERENCES task_sources(task_id,source_id))",
 "CREATE TABLE task_source_extractions (task_id TEXT NOT NULL, source_id TEXT NOT NULL, source_revision INTEGER NOT NULL, extractor_version TEXT NOT NULL, record_id TEXT NOT NULL UNIQUE, PRIMARY KEY(source_id,source_revision,extractor_version), FOREIGN KEY(task_id,source_id,source_revision) REFERENCES task_source_versions(task_id,source_id,revision), FOREIGN KEY(task_id,record_id) REFERENCES task_records(task_id,id))",
 "CREATE TABLE task_consumer_decisions (decision_id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL, workflow_id TEXT NOT NULL, consumer_id TEXT NOT NULL, root_record_id TEXT NOT NULL, generation INTEGER NOT NULL, input_record_id TEXT NOT NULL, data TEXT NOT NULL, workflow_sequence INTEGER NOT NULL UNIQUE, UNIQUE(task_id,decision_id), UNIQUE(task_id,workflow_id,consumer_id,root_record_id,generation), FOREIGN KEY(task_id,workflow_id) REFERENCES task_workflows(task_id,workflow_id), FOREIGN KEY(task_id,root_record_id) REFERENCES task_records(task_id,id), FOREIGN KEY(task_id,input_record_id) REFERENCES task_records(task_id,id))",
 "CREATE TABLE task_publications (approval_id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL, decision_id TEXT NOT NULL, data TEXT NOT NULL, workflow_sequence INTEGER NOT NULL UNIQUE, UNIQUE(task_id,approval_id), FOREIGN KEY(task_id,decision_id) REFERENCES task_consumer_decisions(task_id,decision_id))",
 "CREATE TABLE task_publication_attempts (attempt_id TEXT PRIMARY KEY NOT NULL, task_id TEXT NOT NULL, approval_id TEXT NOT NULL, data TEXT NOT NULL, workflow_sequence INTEGER NOT NULL UNIQUE, FOREIGN KEY(task_id,approval_id) REFERENCES task_publications(task_id,approval_id))",
];

pub(in crate::task_memory) async fn upgrade(conn: &mut SqliteConnection) -> Result<(), Error> {
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *conn)
        .await
        .map_err(Error::storage)?;
    if version != 1 {
        return Err(Error::version());
    }
    sqlx::query("ALTER TABLE task_receipts RENAME TO old_task_receipts")
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    sqlx::query(RECEIPTS)
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    sqlx::query("INSERT INTO task_receipts (task_id,idempotency_key,action,digest,record_id,revision,result_type,result_id) SELECT task_id,idempotency_key,action,digest,record_id,revision,'record',record_id FROM old_task_receipts").execute(&mut *conn).await.map_err(Error::storage)?;
    sqlx::query("DROP TABLE old_task_receipts")
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    for statement in SCHEMA_V2 {
        sqlx::query(statement)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
    }
    sqlx::query("INSERT INTO task_ledger_meta VALUES (1,?,0)")
        .bind(Uuid::new_v4().to_string())
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    sqlx::query("PRAGMA user_version=2")
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    Ok(())
}

pub(in crate::task_memory) async fn validate_schema(
    conn: &mut SqliteConnection,
) -> Result<(), Error> {
    let mut actual: Vec<String> = sqlx::query_scalar(
        "SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::storage)?;
    let mut expected: Vec<String> = SCHEMA
        .iter()
        .filter(|s| s.starts_with("CREATE ") && !s.starts_with("CREATE TABLE task_receipts "))
        .map(|s| s.to_string())
        .chain(SCHEMA_V2.iter().map(|s| s.to_string()))
        .chain(std::iter::once(RECEIPTS.to_string()))
        .collect();
    actual.sort();
    expected.sort();
    if actual != expected {
        return Err(Error::version());
    }
    Ok(())
}

pub(in crate::task_memory) async fn export_tables(
    conn: &mut SqliteConnection,
) -> Result<Value, Error> {
    use sqlx::{Column, TypeInfo, ValueRef};
    let mut result = json!({});
    for name in [
        "ledger_meta",
        "workflows",
        "source_collections",
        "sources",
        "source_versions",
        "source_extractions",
        "consumer_decisions",
        "publications",
        "publication_attempts",
    ] {
        let rows = sqlx::query(&format!("SELECT * FROM task_{name} ORDER BY rowid"))
            .fetch_all(&mut *conn)
            .await
            .map_err(Error::storage)?;
        let mut values = Vec::new();
        for row in rows {
            let mut v = json!({});
            for col in row.columns() {
                let key = col.name();
                let raw = row.try_get_raw(key).map_err(Error::storage)?;
                if raw.is_null() {
                    continue;
                }
                let val = if raw.type_info().name() == "INTEGER" {
                    json!(row.get::<i64, _>(key))
                } else {
                    let s: String = row.get(key);
                    if matches!(key, "profile" | "definition" | "data") {
                        serde_json::from_str(&s).map_err(Error::storage)?
                    } else {
                        json!(s)
                    }
                };
                v[key] = val;
            }
            values.push(v);
        }
        result[name] = json!(values);
    }
    result["workflow_sequence"] = json!(sqlx::query_scalar::<_, i64>(
        "SELECT workflow_sequence FROM task_ledger_meta WHERE singleton=1"
    )
    .fetch_one(&mut *conn)
    .await
    .map_err(Error::storage)?);
    Ok(result)
}

pub(in crate::task_memory) async fn restore_gate(conn: &mut SqliteConnection) -> Result<(), Error> {
    sqlx::query("UPDATE task_ledger_meta SET incarnation=? WHERE singleton=1")
        .bind(Uuid::new_v4().to_string())
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    sqlx::query("UPDATE task_workflows SET state='recovery_required'")
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    for (table, key) in [
        ("task_consumer_decisions", "decision_id"),
        ("task_publications", "approval_id"),
    ] {
        let rows = sqlx::query(&format!("SELECT {key},data FROM {table}"))
            .fetch_all(&mut *conn)
            .await
            .map_err(Error::storage)?;
        for r in rows {
            let mut d: Value =
                serde_json::from_str(&r.get::<String, _>("data")).map_err(Error::storage)?;
            if d["validity"] == "current" {
                d["validity"] = json!("stale");
            }
            sqlx::query(&format!("UPDATE {table} SET data=? WHERE {key}=?"))
                .bind(d.to_string())
                .bind(r.get::<String, _>(key))
                .execute(&mut *conn)
                .await
                .map_err(Error::storage)?;
        }
    }
    Ok(())
}
