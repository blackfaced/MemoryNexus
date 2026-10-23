//! Shared origin traversal and atomic invalidation of derived uses.
use super::*;
use std::collections::BTreeSet;
#[derive(Default)]
pub(super) struct Counts {
    pub records: u64,
    pub decisions: u64,
    pub approvals: u64,
}
impl Counts {
    pub fn add(&mut self, c: Self) {
        self.records += c.records;
        self.decisions += c.decisions;
        self.approvals += c.approvals;
    }
}
pub(super) async fn root_record(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<String, Error> {
    lifecycle::scoped_record(conn, task, id).await?;
    let mut current = id.to_owned();
    loop {
        let previous: Option<String> =
            sqlx::query_scalar("SELECT supersedes_id FROM task_record_revisions WHERE record_id=?")
                .bind(&current)
                .fetch_optional(&mut *conn)
                .await
                .map_err(Error::storage)?;
        match previous {
            Some(p) => current = p,
            None => return Ok(current),
        }
    }
}
pub(super) async fn chain_head(
    conn: &mut SqliteConnection,
    task: &str,
    root: &str,
) -> Result<String, Error> {
    lifecycle::scoped_record(conn, task, root).await?;
    let mut current = root.to_owned();
    loop {
        let next: Option<String> =
            sqlx::query_scalar("SELECT record_id FROM task_record_revisions WHERE supersedes_id=?")
                .bind(&current)
                .fetch_optional(&mut *conn)
                .await
                .map_err(Error::storage)?;
        match next {
            Some(n) => current = n,
            None => return Ok(current),
        }
    }
}
pub(super) async fn record_origin(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<Option<Value>, Error> {
    let mut current = id.to_owned();
    loop {
        if let Some(row)=sqlx::query("SELECT source_id,source_revision,extractor_version FROM task_source_extractions WHERE task_id=? AND record_id=?").bind(task).bind(&current).fetch_optional(&mut *conn).await.map_err(Error::storage)?{return Ok(Some(json!({"source_id":row.get::<String,_>("source_id"),"source_revision":row.get::<i64,_>("source_revision"),"extractor_version":row.get::<String,_>("extractor_version")})));}
        let previous: Option<String> =
            sqlx::query_scalar("SELECT supersedes_id FROM task_record_revisions WHERE record_id=?")
                .bind(current)
                .fetch_optional(&mut *conn)
                .await
                .map_err(Error::storage)?;
        match previous {
            Some(p) => current = p,
            None => return Ok(None),
        }
    }
}
pub(super) async fn scoped_source(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<sqlx::sqlite::SqliteRow, Error> {
    if let Some(row) = sqlx::query("SELECT * FROM task_sources WHERE task_id=? AND source_id=?")
        .bind(task)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(Error::storage)?
    {
        return Ok(row);
    }
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_sources WHERE source_id=?)")
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
pub(super) async fn current_source(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<Value, Error> {
    let s = scoped_source(conn, task, id).await?;
    let data: String = sqlx::query_scalar(
        "SELECT data FROM task_source_versions WHERE task_id=? AND source_id=? AND revision=?",
    )
    .bind(task)
    .bind(id)
    .bind(s.get::<i64, _>("current_revision"))
    .fetch_one(conn)
    .await
    .map_err(Error::storage)?;
    let mut v: Value = serde_json::from_str(&data).map_err(Error::storage)?;
    v["collection_id"] = json!(s.get::<String, _>("collection_id"));
    v["workflow_id"] = json!(s.get::<String, _>("workflow_id"));
    v["blocked"] = json!(s.get::<bool, _>("blocked"));
    if let Some(root) = s.get::<Option<String>, _>("root_record_id") {
        v["root_record_id"] = json!(root);
    }
    Ok(v)
}
async fn derived_records(
    conn: &mut SqliteConnection,
    task: &str,
    starts: &[String],
) -> Result<BTreeSet<String>, Error> {
    let mut ids: BTreeSet<String> = starts.iter().cloned().collect();
    let mut queue = starts.to_vec();
    while let Some(id) = queue.pop() {
        let related:Vec<String>=sqlx::query_scalar("SELECT r.record_id FROM task_record_sources r JOIN task_records t ON t.id=r.record_id WHERE t.task_id=? AND r.source_id=? UNION SELECT r.record_id FROM task_record_revisions r JOIN task_records t ON t.id=r.record_id WHERE t.task_id=? AND r.supersedes_id=? UNION SELECT r.supersedes_id FROM task_record_revisions r JOIN task_records t ON t.id=r.record_id WHERE t.task_id=? AND r.record_id=?").bind(task).bind(&id).bind(task).bind(&id).bind(task).bind(&id).fetch_all(&mut *conn).await.map_err(Error::storage)?;
        for r in related {
            if ids.insert(r.clone()) {
                queue.push(r);
            }
        }
    }
    Ok(ids)
}
pub(super) async fn invalidate_source(
    conn: &mut SqliteConnection,
    task: &str,
    source: &str,
    revision: i64,
) -> Result<Counts, Error> {
    let s = scoped_source(conn, task, source).await?;
    let roots = s
        .get::<Option<String>, _>("root_record_id")
        .into_iter()
        .collect::<Vec<_>>();
    let records = derived_records(conn, task, &roots).await?;
    invalidate(
        conn,
        task,
        &records,
        &[source.to_owned()],
        revision,
        false,
        true,
    )
    .await
}
pub(in crate::task_memory) async fn invalidate_record(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
    revision: i64,
) -> Result<(), Error> {
    if !is_v2(conn).await? {
        return Ok(());
    }
    let records = derived_records(conn, task, &[id.to_owned()]).await?;
    invalidate(conn, task, &records, &[], revision, false, false).await?;
    Ok(())
}
pub(in crate::task_memory) async fn retract_records(
    conn: &mut SqliteConnection,
    task: &str,
    chain: &[String],
    revision: i64,
) -> Result<(), Error> {
    if !is_v2(conn).await? {
        return Ok(());
    }
    let records = derived_records(conn, task, chain).await?;
    let mut sources = BTreeSet::new();
    for id in chain {
        if let Some(o) = record_origin(conn, task, id).await? {
            sources.insert(string(&o, "source_id")?.to_owned());
        }
    }
    for id in &sources {
        sqlx::query("UPDATE task_sources SET blocked=1 WHERE task_id=? AND source_id=?")
            .bind(task)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
    }
    invalidate(
        conn,
        task,
        &records,
        &sources.into_iter().collect::<Vec<_>>(),
        revision,
        true,
        false,
    )
    .await?;
    Ok(())
}
async fn is_v2(conn: &mut SqliteConnection) -> Result<bool, Error> {
    Ok(sqlx::query_scalar::<_, i64>("PRAGMA user_version")
        .fetch_one(conn)
        .await
        .map_err(Error::storage)?
        == 2)
}
async fn invalidate(
    conn: &mut SqliteConnection,
    task: &str,
    records: &BTreeSet<String>,
    sources: &[String],
    revision: i64,
    scrub: bool,
    include_base: bool,
) -> Result<Counts, Error> {
    let mut counts = Counts::default();
    for id in records {
        let row = lifecycle::scoped_record(conn, task, id).await?;
        let lesson = row.get::<String, _>("kind") == "lesson";
        if (include_base || lesson) && row.get::<String, _>("state") == "current" {
            sqlx::query(
                "UPDATE task_records SET state='stale',revision=? WHERE task_id=? AND id=?",
            )
            .bind(revision)
            .bind(task)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
            counts.records += 1;
        }
        if scrub && lesson {
            sqlx::query("UPDATE task_records SET text='',evidence_refs='[]',revision=? WHERE task_id=? AND id=?").bind(revision).bind(task).bind(id).execute(&mut *conn).await.map_err(Error::storage)?;
        }
    }
    let decisions =
        sqlx::query("SELECT decision_id,data FROM task_consumer_decisions WHERE task_id=?")
            .bind(task)
            .fetch_all(&mut *conn)
            .await
            .map_err(Error::storage)?;
    for row in decisions {
        let mut d: Value =
            serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
        let affected = d["dependencies"].as_array().is_some_and(|deps| {
            deps.iter().any(|x| {
                x["type"] == "record" && x["id"].as_str().is_some_and(|id| records.contains(id))
                    || x["type"] == "source"
                        && x["id"]
                            .as_str()
                            .is_some_and(|id| sources.iter().any(|s| s == id))
            })
        });
        if !affected {
            continue;
        }
        if d["validity"] == "current" || scrub && d["validity"] != "retracted" {
            counts.decisions += 1;
            d["validity"] = json!(if scrub { "retracted" } else { "stale" });
        }
        if scrub {
            d.as_object_mut().unwrap().remove("reason");
        }
        let id = row.get::<String, _>("decision_id");
        sqlx::query("UPDATE task_consumer_decisions SET data=? WHERE decision_id=?")
            .bind(d.to_string())
            .bind(&id)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
        counts.approvals += invalidate_approvals(conn, task, &id, scrub).await?;
    }
    Ok(counts)
}
pub(super) async fn invalidate_approvals(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
    scrub: bool,
) -> Result<u64, Error> {
    let rows = sqlx::query(
        "SELECT approval_id,data FROM task_publications WHERE task_id=? AND decision_id=?",
    )
    .bind(task)
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::storage)?;
    let mut count = 0;
    for row in rows {
        let mut p: Value =
            serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
        if p["validity"] == "current" || scrub && p["validity"] != "retracted" {
            p["validity"] = json!(if scrub { "retracted" } else { "stale" });
            count += 1;
            sqlx::query("UPDATE task_publications SET data=? WHERE approval_id=?")
                .bind(p.to_string())
                .bind(row.get::<String, _>("approval_id"))
                .execute(&mut *conn)
                .await
                .map_err(Error::storage)?;
        }
    }
    Ok(count)
}
pub(super) async fn invalidate_workflow(
    conn: &mut SqliteConnection,
    task: &str,
    wid: &str,
) -> Result<(), Error> {
    let rows = sqlx::query(
        "SELECT decision_id,data FROM task_consumer_decisions WHERE task_id=? AND workflow_id=?",
    )
    .bind(task)
    .bind(wid)
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::storage)?;
    for row in rows {
        let mut d: Value =
            serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
        if d["validity"] == "current" {
            d["validity"] = json!("stale");
        }
        let id = row.get::<String, _>("decision_id");
        sqlx::query("UPDATE task_consumer_decisions SET data=? WHERE decision_id=?")
            .bind(d.to_string())
            .bind(&id)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
        invalidate_approvals(conn, task, &id, false).await?;
    }
    Ok(())
}
