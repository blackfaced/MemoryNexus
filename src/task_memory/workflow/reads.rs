//! Bounded revision-pinned inspection. It never repairs or observes anything.
use super::*;
use std::collections::BTreeSet;
pub(super) async fn status(
    conn: &mut SqliteConnection,
    task: &str,
    v: &Value,
) -> Result<Value, Error> {
    object(
        v,
        &[],
        &[
            "workflow_id",
            "view",
            "limit",
            "at_revision",
            "after",
            "ids",
            "keys",
        ],
    )?;
    let view = if v.get("view").is_some() {
        string(v, "view")?
    } else {
        "summary"
    };
    if ![
        "summary",
        "collections",
        "sources",
        "inputs",
        "decisions",
        "publications",
        "receipts",
    ]
    .contains(&view)
    {
        return Err(Error::invalid());
    }
    let limit = if v.get("limit").is_some() {
        number(v, "limit")?
    } else {
        20
    };
    if !(1..=20).contains(&limit) {
        return Err(Error::invalid());
    }
    let after = if v.get("after").is_some() {
        number(v, "after")?
    } else {
        0
    };
    let ids = v.get("ids");
    let keys = v.get("keys");
    if ids.is_some() && keys.is_some()
        || (ids.is_some() || keys.is_some()) && v.get("after").is_some()
        || keys.is_some() && view != "receipts"
    {
        return Err(Error::invalid());
    }
    let filter = if ids.is_some() {
        Some(array(v, "ids", 20)?)
    } else if keys.is_some() {
        Some(array(v, "keys", 20)?)
    } else {
        None
    };
    if let Some(items) = filter {
        let mut unique = BTreeSet::new();
        for value in items {
            let s = value.as_str().ok_or_else(Error::invalid)?;
            if ids.is_some() {
                canonical_uuid(s)?;
            } else {
                bounded(s, 128)?;
            }
            if !unique.insert(s) {
                return Err(Error::invalid());
            }
        }
    }
    let mut tx = conn.begin().await.map_err(Error::storage)?;
    let revision = registered_task(&mut tx, task)
        .await?
        .get::<i64, _>("revision");
    if v.get("at_revision").is_some() && number(v, "at_revision")? != revision {
        return Err(Error::conflict());
    }
    let inc = incarnation(&mut tx).await?;
    let wid = if v.get("workflow_id").is_some() {
        let id = string(v, "workflow_id")?;
        canonical_uuid(id)?;
        Some(id)
    } else {
        None
    };
    let mut rows = Vec::<(i64, Value)>::new();
    if let Some(id) = wid {
        let w = workflow(&mut tx, task, id).await?;
        match view{
 "summary"=>{let source_count:i64=sqlx::query_scalar("SELECT count(*) FROM task_sources WHERE task_id=? AND workflow_id=?").bind(task).bind(id).fetch_one(&mut *tx).await.map_err(Error::storage)?;
 let inputs=input_rows(&mut tx,task,id,false).await?;let decisions=data_rows(&mut tx,"task_consumer_decisions",task,Some(id)).await?;let pubs=consumers::projection(&mut tx,task,id,"publications").await?;
 let current=decisions.iter().filter(|(_,d)|d["validity"]=="current").count();let stale=decisions.iter().filter(|(_,d)|d["validity"]=="stale").count();let pending=pubs.iter().filter(|(_,p)|p["publication"]!="applied"||p["validity"]!="current").count();
 rows.push((1,json!({"workflow_id":id,"profile_revision":w.get::<i64,_>("profile_revision"),"profile_sha256":w.get::<String,_>("profile_sha256"),"binding_sha256":w.get::<String,_>("binding_sha256"),"state":w.get::<String,_>("state"),"counts":{"sources":source_count,"inputs":inputs.len(),"current_decisions":current,"stale_decisions":stale,"pending_publications":pending}})));},
 "collections"=>rows=data_rows(&mut tx,"task_source_collections",task,Some(id)).await?,
 "sources"=>{for r in sqlx::query("SELECT source_id,workflow_sequence FROM task_sources WHERE task_id=? AND workflow_id=? ORDER BY workflow_sequence").bind(task).bind(id).fetch_all(&mut *tx).await.map_err(Error::storage)?{let mut s=current_source(&mut tx,task,&r.get::<String,_>("source_id")).await?;for k in ["workflow_id","collection_definition_sha256"]{s.as_object_mut().unwrap().remove(k);}rows.push((r.get("workflow_sequence"),s));}},
 "inputs"=>rows=input_rows(&mut tx,task,id,filter.is_some()).await?,
 "decisions"|"publications"=>rows=consumers::projection(&mut tx,task,id,view).await?,
 "receipts"=>{for r in sqlx::query("SELECT * FROM task_receipts WHERE task_id=? ORDER BY revision").bind(task).fetch_all(&mut *tx).await.map_err(Error::storage)?{let action=r.get::<String,_>("action");let rid=r.get::<Option<String>,_>("result_id").or_else(||r.get::<Option<String>,_>("record_id")).ok_or_else(Error::not_found)?;let mut result=if let Some(s)=r.get::<Option<String>,_>("result_json"){serde_json::from_str(&s).map_err(Error::storage)?}else{let row=lifecycle::scoped_record(&mut tx,task,&rid).await?;json!({"record_id":rid,"state":row.get::<String,_>("state")})};result=current_result(&mut tx,task,&action,result).await?;rows.push((r.get("revision"),json!({"idempotency_key":r.get::<String,_>("idempotency_key"),"action":action,"digest":r.get::<String,_>("digest"),"result_type":r.get::<String,_>("result_type"),"result_id":rid,"revision":r.get::<i64,_>("revision"),"result":result})));}},_=>unreachable!()}
    } else {
        if view != "summary" || filter.is_some() {
            return Err(Error::invalid());
        }
        let workflows: Vec<String> = sqlx::query_scalar(
            "SELECT workflow_id FROM task_workflows WHERE task_id=? ORDER BY workflow_id",
        )
        .bind(task)
        .fetch_all(&mut *tx)
        .await
        .map_err(Error::storage)?;
        rows.push((1,json!({"task_id":task,"incarnation":inc,"ledger_version":2,"workflow_contract":1,"workflows":workflows})));
    }
    let mut items = Vec::new();
    let mut more = false;
    let mut last = 0;
    if let Some(filter) = filter {
        let field = if keys.is_some() {
            "idempotency_key"
        } else {
            match view {
                "collections" => "collection_id",
                "sources" => "source_id",
                "inputs" => "id",
                "decisions" => "decision_id",
                "publications" => "approval_id",
                "receipts" => "result_id",
                _ => return Err(Error::invalid()),
            }
        };
        for wanted in filter {
            if let Some((_, row)) = rows.iter().find(|(_, r)| r[field] == *wanted) {
                items.push(row.clone());
            } else {
                let (table, col) = match view {
                    "collections" => ("task_source_collections", "collection_id"),
                    "sources" => ("task_sources", "source_id"),
                    "inputs" => ("task_records", "id"),
                    "decisions" => ("task_consumer_decisions", "decision_id"),
                    "publications" => ("task_publications", "approval_id"),
                    "receipts" => ("task_receipts", field),
                    _ => return Err(Error::not_found()),
                };
                let exists: bool = sqlx::query_scalar(&format!(
                    "SELECT EXISTS(SELECT 1 FROM {table} WHERE {col}=?)"
                ))
                .bind(wanted.as_str().unwrap())
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
        rows.sort_by_key(|r| r.0);
        for (seq, row) in rows {
            if seq <= after {
                continue;
            }
            if items.len() == limit as usize {
                more = true;
                break;
            }
            last = seq;
            items.push(row);
        }
    }
    let mut result = json!({"items":items,"complete":!more});
    if more {
        result["next_after"] = json!(last);
    }
    let out = response("ok", task, revision, &inc, result);
    if out.to_string().len() > 128 * 1024 {
        return Err(Error::invalid());
    }
    Ok(out)
}
async fn data_rows(
    conn: &mut SqliteConnection,
    table: &str,
    task: &str,
    wid: Option<&str>,
) -> Result<Vec<(i64, Value)>, Error> {
    let query = format!(
        "SELECT data,workflow_sequence FROM {table} WHERE task_id=?{} ORDER BY workflow_sequence",
        if wid.is_some() {
            " AND workflow_id=?"
        } else {
            ""
        }
    );
    let mut q = sqlx::query(&query).bind(task);
    if let Some(id) = wid {
        q = q.bind(id);
    }
    q.fetch_all(conn)
        .await
        .map_err(Error::storage)?
        .into_iter()
        .map(|r| {
            Ok((
                r.get("workflow_sequence"),
                serde_json::from_str(&r.get::<String, _>("data")).map_err(Error::storage)?,
            ))
        })
        .collect()
}
async fn input_rows(
    conn: &mut SqliteConnection,
    task: &str,
    wid: &str,
    all: bool,
) -> Result<Vec<(i64, Value)>, Error> {
    let mut rows = Vec::new();
    for r in sqlx::query(
        "SELECT * FROM task_records WHERE task_id=? AND kind!='lesson' ORDER BY commit_sequence",
    )
    .bind(task)
    .fetch_all(&mut *conn)
    .await
    .map_err(Error::storage)?
    {
        let id = r.get::<String, _>("id");
        if !all {
            let has_next: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM task_record_revisions WHERE supersedes_id=?)",
            )
            .bind(&id)
            .fetch_one(&mut *conn)
            .await
            .map_err(Error::storage)?;
            if has_next {
                continue;
            }
        }
        let origin = record_origin(conn, task, &id).await?;
        if let Some(o) = &origin {
            let source = scoped_source(conn, task, string(o, "source_id")?).await?;
            if source.get::<String, _>("workflow_id") != wid
                || !all && source.get::<bool, _>("blocked")
            {
                continue;
            }
        }
        let state = r.get::<String, _>("state");
        let mut item = json!({"id":id,"kind":r.get::<String,_>("kind"),"trust":r.get::<String,_>("trust"),"occurred_at":r.get::<String,_>("occurred_at"),"run_id":r.get::<String,_>("run_id"),"client":r.get::<String,_>("client"),"state":state,"revision":r.get::<i64,_>("revision"),"source_ids":lifecycle::source_ids(conn,&id).await?,"root_record_id":root_record(conn,task,&id).await?});
        if state == "current" {
            item["text"] = json!(r.get::<String, _>("text"));
            item["evidence_refs"] = serde_json::from_str(&r.get::<String, _>("evidence_refs"))
                .map_err(Error::storage)?;
        }
        if let Some(o) = origin {
            item["origin"] = o;
        }
        rows.push((r.get("commit_sequence"), item));
    }
    Ok(rows)
}
