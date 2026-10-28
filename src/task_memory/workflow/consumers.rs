//! Independent consumer decisions and exact publication authority.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(action: &str, v: &mut Value) -> Result<(), Error> {
    match action {
        "consumer-review" => {
            envelope(
                v,
                &[
                    "consumer_id",
                    "input_record_id",
                    "decision",
                    "reason",
                    "dependencies",
                ],
                &[
                    "output_sha256",
                    "previous_decision_id",
                    "reconsider",
                    "confirmation",
                ],
            )?;
            name(string(v, "consumer_id")?)?;
            canonical_uuid(string(v, "input_record_id")?)?;
            bounded(string(v, "reason")?, 240)?;
            optional_confirm(v)?;
            match string(v, "decision")? {
                "proposed" => hash(string(v, "output_sha256")?)?,
                "skipped" | "deferred" if v.get("output_sha256").is_none() => {}
                _ => return Err(Error::invalid()),
            }
            if let Some(id) = v.get("previous_decision_id") {
                canonical_uuid(id.as_str().ok_or_else(Error::invalid)?)?;
            }
            if v.get("reconsider").is_some() && v["reconsider"] != true {
                return Err(Error::invalid());
            }
            let deps = array(v, "dependencies", 20)?;
            if deps.is_empty() {
                return Err(Error::invalid());
            }
            let mut seen = BTreeSet::new();
            for dep in deps {
                match string(dep, "type")? {
                    "record" => object(dep, &["type", "id", "sha256"], &[])?,
                    "source" => {
                        object(dep, &["type", "id", "revision", "sha256"], &[])?;
                        if number(dep, "revision")? == 0 {
                            return Err(Error::invalid());
                        }
                    }
                    _ => return Err(Error::invalid()),
                }
                canonical_uuid(string(dep, "id")?)?;
                hash(string(dep, "sha256")?)?;
                if !seen.insert((
                    string(dep, "type")?.to_owned(),
                    string(dep, "id")?.to_owned(),
                )) {
                    return Err(Error::invalid());
                }
            }
            v["dependencies"]
                .as_array_mut()
                .unwrap()
                .sort_by_key(dep_key);
        }
        "publication-result" => {
            envelope(
                v,
                &["approval_id", "outcome", "observed_at"],
                &["observed_sha256", "error_code"],
            )?;
            canonical_uuid(string(v, "approval_id")?)?;
            time(v, "observed_at")?;
            if v.get("observed_sha256").is_some() {
                hash(string(v, "observed_sha256")?)?;
            }
            match string(v, "outcome")? {
                "applied"
                    if v.get("observed_sha256").is_some() && v.get("error_code").is_none() => {}
                "conflict"
                    if v.get("observed_sha256").is_some()
                        && matches!(
                            v["error_code"].as_str(),
                            Some("baseline_changed" | "managed_region_changed")
                        ) => {}
                "failed"
                    if matches!(
                        v["error_code"].as_str(),
                        Some("io_error" | "source_unavailable" | "plan_missing")
                    ) => {}
                _ => return Err(Error::invalid()),
            }
        }
        "consumer-approve" => {
            envelope(
                v,
                &["decision_id", "decision_sha256", "proposal"],
                &[
                    "confirmation",
                    "maintenance",
                    "previous_approval_id",
                    "reason",
                    "prior_block_sha256",
                ],
            )?;
            canonical_uuid(string(v, "decision_id")?)?;
            hash(string(v, "decision_sha256")?)?;
            let p = &v["proposal"];
            object(
                p,
                &[
                    "target_id",
                    "block_id",
                    "base_exists",
                    "base_sha256",
                    "after_sha256",
                    "output_sha256",
                    "plan_sha256",
                ],
                &[],
            )?;
            name(string(p, "target_id")?)?;
            boolean(p, "base_exists")?;
            for k in [
                "block_id",
                "base_sha256",
                "after_sha256",
                "output_sha256",
                "plan_sha256",
            ] {
                hash(string(p, k)?)?;
            }
            if p["base_exists"] == false && p["base_sha256"] != hash_bytes(b"") {
                return Err(Error::invalid());
            }
            if v["maintenance"] == "remove" {
                literal(v, "confirmation", "confirmed")?;
                canonical_uuid(string(v, "previous_approval_id")?)?;
                bounded(string(v, "reason")?, 240)?;
                hash(string(v, "prior_block_sha256")?)?;
                if p["base_exists"] != true || p["output_sha256"] != hash_bytes(b"") {
                    return Err(Error::invalid());
                }
            } else if v.get("maintenance").is_some() {
                literal(v, "maintenance", "neutralize")?;
                if v.get("confirmation").is_some() {
                    return Err(Error::invalid());
                }
            } else {
                if v.get("confirmation").is_none() {
                    return Err(approval());
                }
                literal(v, "confirmation", "confirmed")?;
            }
            if v["maintenance"] != "remove"
                && ["previous_approval_id", "reason", "prior_block_sha256"]
                    .iter()
                    .any(|k| v.get(*k).is_some())
            {
                return Err(Error::invalid());
            }
        }
        _ => return Err(Error::invalid()),
    }
    Ok(())
}
fn dep_key(v: &Value) -> (String, String, i64, String) {
    (
        v["type"].as_str().unwrap().into(),
        v["id"].as_str().unwrap().into(),
        v["revision"].as_i64().unwrap_or(0),
        v["sha256"].as_str().unwrap().into(),
    )
}
fn put_dependency(deps: &mut BTreeMap<(String, String), Value>, v: Value) -> Result<(), Error> {
    let key = (string(&v, "type")?.to_owned(), string(&v, "id")?.to_owned());
    if let Some(old) = deps.get(&key) {
        if old != &v {
            return Err(stale());
        }
    } else {
        deps.insert(key, v);
    }
    if deps.len() > 20 {
        return Err(Error::invalid());
    }
    Ok(())
}
async fn dependencies(
    conn: &mut SqliteConnection,
    task: &str,
    wid: &str,
    input: &str,
    declared: &Value,
) -> Result<Value, Error> {
    let mut deps = BTreeMap::new();
    let mut pending = Vec::new();
    let mut seen = BTreeSet::new();
    for d in declared.as_array().unwrap() {
        put_dependency(&mut deps, d.clone())?;
        if d["type"] == "record" {
            pending.push(string(d, "id")?.to_owned());
        }
    }
    pending.push(input.to_owned());
    while let Some(id) = pending.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let record = lifecycle::scoped_record(conn, task, &id).await?;
        if let Some(origin) = record_origin(conn, task, &id).await? {
            if current_source(conn, task, string(&origin, "source_id")?).await?["blocked"] == true {
                return Err(Error::conflict());
            }
        }
        if record.get::<String, _>("state") != "current" {
            return Err(stale());
        }
        if id == input && record.get::<String, _>("kind") == "lesson" {
            return Err(Error::invalid());
        }
        put_dependency(
            &mut deps,
            json!({"type":"record","id":id,"sha256":hash_bytes(record.get::<String,_>("text").as_bytes())}),
        )?;
        if let Some(origin) = record_origin(conn, task, &id).await? {
            let s = current_source(conn, task, string(&origin, "source_id")?).await?;
            if s["blocked"] == true {
                return Err(Error::conflict());
            }
            if s["source_revision"] != origin["source_revision"] {
                return Err(stale());
            }
            put_dependency(
                &mut deps,
                json!({"type":"source","id":origin["source_id"],"revision":origin["source_revision"],"sha256":s["sha256"]}),
            )?;
        }
        pending.extend(lifecycle::source_ids(conn, &id).await?);
    }
    for d in deps.values().filter(|d| d["type"] == "source") {
        let s = current_source(conn, task, string(d, "id")?).await?;
        if s["workflow_id"] != wid {
            return Err(Error::scope());
        }
        if s["blocked"] == true {
            return Err(Error::conflict());
        }
        if s["source_revision"] != d["revision"]
            || s["sha256"] != d["sha256"]
            || s["availability"] != "available"
        {
            return Err(stale());
        }
        let collection=sqlx::query("SELECT definition,data FROM task_source_collections WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(task).bind(wid).bind(string(&s,"collection_id")?).fetch_one(&mut *conn).await.map_err(Error::storage)?;
        let active: Value =
            serde_json::from_str(&collection.get::<String, _>("data")).map_err(Error::storage)?;
        let definition: Value = serde_json::from_str(&collection.get::<String, _>("definition"))
            .map_err(Error::storage)?;
        if active["active"] != true || s["collection_definition_sha256"] != hash_value(&definition)
        {
            return Err(stale());
        }
    }
    let mut sorted: Vec<Value> = deps.into_values().collect();
    sorted.sort_by_key(dep_key);
    Ok(json!(sorted))
}
async fn scoped_decision(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<Value, Error> {
    if let Some(row) =
        sqlx::query("SELECT data FROM task_consumer_decisions WHERE task_id=? AND decision_id=?")
            .bind(task)
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(Error::storage)?
    {
        return serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage);
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM task_consumer_decisions WHERE decision_id=?)",
    )
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
fn select_fields(v: &Value, fields: &[&str]) -> Value {
    let mut r = json!({});
    for k in fields {
        if let Some(value) = v.get(*k) {
            r[*k] = value.clone();
        }
    }
    r
}
fn decision_result(d: &Value) -> Value {
    select_fields(
        d,
        &[
            "decision_id",
            "generation",
            "root_record_id",
            "decision",
            "validity",
            "dependency_sha256",
            "decision_sha256",
            "block_id",
        ],
    )
}
fn consumer<'a>(profile: &'a Value, id: &str) -> Result<&'a Value, Error> {
    profile["consumers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["consumer_id"] == id)
        .ok_or_else(Error::scope)
}
pub(super) async fn execute(
    conn: &mut SqliteConnection,
    task: &str,
    action: &str,
    v: &Value,
    p: &Value,
    _revision: i64,
) -> Result<Operation, Error> {
    match action {
        "consumer-review" => review(conn, task, v, p).await,
        "consumer-approve" => approve(conn, task, v, p).await,
        "publication-result" => publication(conn, task, v).await,
        _ => Err(Error::invalid()),
    }
}
async fn review(
    conn: &mut SqliteConnection,
    task: &str,
    v: &Value,
    p: &Value,
) -> Result<Operation, Error> {
    if p["capture"]["automatic_reviews"] != true && v["confirmation"] != "confirmed" {
        return Err(approval());
    }
    let c = consumer(p, string(v, "consumer_id")?)?;
    let input = string(v, "input_record_id")?;
    let wid = string(v, "workflow_id")?;
    let deps = dependencies(conn, task, wid, input, &v["dependencies"]).await?;
    let root = root_record(conn, task, input).await?;
    let existing=sqlx::query("SELECT data FROM task_consumer_decisions WHERE task_id=? AND workflow_id=? AND consumer_id=? AND root_record_id=? ORDER BY generation DESC LIMIT 1").bind(task).bind(wid).bind(string(v,"consumer_id")?).bind(&root).fetch_optional(&mut *conn).await.map_err(Error::storage)?;
    let old = existing
        .map(|r| serde_json::from_str::<Value>(&r.get::<String, _>("data")))
        .transpose()
        .map_err(Error::storage)?;
    let mut d = json!({"task_id":task,"workflow_id":wid,"consumer_id":v["consumer_id"],"root_record_id":root,"input_record_id":input,"proposed_decision":v["decision"],"reason":v["reason"],"dependencies":deps,"profile_revision":v["profile_revision"],"incarnation":v["incarnation"],"lens_version":c["lens_version"],"rules_sha256":c["rules_sha256"],"target_id":c["target_id"]});
    if let Some(h) = v.get("output_sha256") {
        d["output_sha256"] = h.clone();
    }
    let equality_fields = [
        "input_record_id",
        "proposed_decision",
        "reason",
        "dependencies",
        "profile_revision",
        "incarnation",
        "lens_version",
        "rules_sha256",
        "target_id",
        "output_sha256",
    ];
    if let Some(old) = &old {
        let explicit_reconsider =
            v["reconsider"] == true && v["previous_decision_id"] == old["decision_id"];
        if old["validity"] == "current"
            && !explicit_reconsider
            && select_fields(old, &equality_fields) == select_fields(&d, &equality_fields)
        {
            return Ok(Operation {
                changed: false,
                result: decision_result(old),
                kind: "decision",
                id: string(old, "decision_id")?.into(),
            });
        }
        if v["previous_decision_id"] != old["decision_id"] {
            return Err(Error::conflict());
        }
        if old["validity"] == "retracted" {
            return Err(Error::conflict());
        }
        if old["validity"] == "current" && v["reconsider"] != true {
            return Err(Error::conflict());
        }
    } else if v.get("previous_decision_id").is_some() || v.get("reconsider").is_some() {
        return Err(Error::conflict());
    }
    let generation = old
        .as_ref()
        .map(|o| o["generation"].as_i64().unwrap() + 1)
        .unwrap_or(1);
    d["generation"] = json!(generation);
    let digest = hash_value(&d);
    let block = hash_value(
        &json!({"task_id":task,"workflow_id":wid,"consumer_id":v["consumer_id"],"root_record_id":root}),
    );
    let id = Uuid::new_v4().to_string();
    d["decision_id"] = json!(id);
    d["decision_sha256"] = json!(digest);
    d["dependency_sha256"] = json!(hash_value(&d["dependencies"]));
    d["block_id"] = json!(block);
    d["decision"] = v["decision"].clone();
    d["validity"] = json!("current");
    if let Some(mut old) = old {
        if old["validity"] == "current" {
            old["validity"] = json!("stale");
            sqlx::query("UPDATE task_consumer_decisions SET data=? WHERE decision_id=?")
                .bind(old.to_string())
                .bind(string(&old, "decision_id")?)
                .execute(&mut *conn)
                .await
                .map_err(Error::storage)?;
        }
        invalidate_approvals(conn, task, string(&old, "decision_id")?, false).await?;
    }
    let seq = next_sequence(conn).await?;
    sqlx::query("INSERT INTO task_consumer_decisions VALUES (?,?,?,?,?,?,?,?,?)")
        .bind(&id)
        .bind(task)
        .bind(wid)
        .bind(string(v, "consumer_id")?)
        .bind(root)
        .bind(generation)
        .bind(input)
        .bind(d.to_string())
        .bind(seq)
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    Ok(Operation {
        changed: true,
        result: decision_result(&d),
        kind: "decision",
        id,
    })
}
async fn publication_data(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<Value, Error> {
    if let Some(row) =
        sqlx::query("SELECT data FROM task_publications WHERE task_id=? AND approval_id=?")
            .bind(task)
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(Error::storage)?
    {
        return serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage);
    }
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_publications WHERE approval_id=?)")
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
async fn latest_attempt(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<Option<Value>, Error> {
    let data:Option<String>=sqlx::query_scalar("SELECT data FROM task_publication_attempts WHERE task_id=? AND approval_id=? ORDER BY workflow_sequence DESC LIMIT 1").bind(task).bind(id).fetch_optional(conn).await.map_err(Error::storage)?;
    data.map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(Error::storage)
}
async fn approval_result(
    conn: &mut SqliteConnection,
    task: &str,
    p: &Value,
) -> Result<Value, Error> {
    let d = scoped_decision(conn, task, string(p, "decision_id")?).await?;
    let latest = latest_attempt(conn, task, string(p, "approval_id")?).await?;
    let mut r = select_fields(p, &["approval_id", "decision_id", "validity", "proposal"]);
    r["decision"] = d["decision"].clone();
    r["publication"] = latest
        .map(|x| x["outcome"].clone())
        .unwrap_or(json!("not_applied"));
    Ok(r)
}
async fn approve(
    conn: &mut SqliteConnection,
    task: &str,
    v: &Value,
    profile: &Value,
) -> Result<Operation, Error> {
    let mut d = scoped_decision(conn, task, string(v, "decision_id")?).await?;
    if d["workflow_id"] != v["workflow_id"] {
        return Err(Error::scope());
    }
    if d["decision_sha256"] != v["decision_sha256"] {
        return Err(Error::conflict());
    }
    let proposal = &v["proposal"];
    let maintenance = v.get("maintenance").is_some();
    if proposal["target_id"] != d["target_id"] || proposal["block_id"] != d["block_id"] {
        return Err(Error::conflict());
    }
    if !maintenance && d["incarnation"] != v["incarnation"] {
        return Err(recovery());
    }
    let c = consumer(profile, string(&d, "consumer_id")?)?;
    if v["maintenance"] == "remove" {
        let prior = publication_data(conn, task, string(v, "previous_approval_id")?).await?;
        if prior["decision_id"] != d["decision_id"]
            || prior.get("maintenance").is_some()
            || prior["proposal"]["target_id"] != proposal["target_id"]
            || prior["proposal"]["block_id"] != proposal["block_id"]
            || prior["proposal"]["output_sha256"] != v["prior_block_sha256"]
            || c["target_id"] != d["target_id"]
            || latest_attempt(conn, task, string(v, "previous_approval_id")?)
                .await?
                .map_or(true, |a| a["outcome"] != "applied")
        {
            return Err(approval());
        }
        let newer: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM task_consumer_decisions d
             JOIN task_publications p ON p.decision_id=d.decision_id AND p.task_id=d.task_id
             JOIN task_publication_attempts a ON a.approval_id=p.approval_id
             WHERE d.task_id=? AND d.workflow_id=? AND json_extract(d.data,'$.block_id')=?
             AND d.generation>? AND json_extract(a.data,'$.outcome')='applied')",
        )
        .bind(task)
        .bind(string(&d, "workflow_id")?)
        .bind(string(&d, "block_id")?)
        .bind(d["generation"].as_i64().ok_or_else(Error::invalid)?)
        .fetch_one(&mut *conn)
        .await
        .map_err(Error::storage)?;
        if newer {
            return Err(approval());
        }
    } else if maintenance {
        if !matches!(d["validity"].as_str(), Some("stale" | "retracted"))
            || c["neutralize_stale"] != true
            || c["target_id"] != d["target_id"]
        {
            return Err(approval());
        }
        if proposal["base_exists"] != true
            || proposal["output_sha256"]
                != hash_bytes(b"MemoryNexus: source evidence changed; review required.\n")
        {
            return Err(approval());
        }
        let published:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_publication_attempts a JOIN task_publications p ON p.approval_id=a.approval_id WHERE p.task_id=? AND p.decision_id=? AND json_extract(a.data,'$.outcome')='applied' AND json_extract(p.data,'$.maintenance') IS NULL)").bind(task).bind(string(&d,"decision_id")?).fetch_one(&mut *conn).await.map_err(Error::storage)?;
        if !published {
            return Err(approval());
        }
        // A neutralisation writes over the block region, so it may only be
        // approved while this decision still owns that block.  A newer
        // generation of the same block that is current, publishable and
        // already published has taken the block over; neutralising now would
        // replace that newer generation's body with the review-required text.
        //
        // The stale validity of this decision does not protect the block: it
        // says the decision is outdated, not that it still owns the region.
        let superseded: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM task_consumer_decisions n
             JOIN task_publications p ON p.decision_id=n.decision_id AND p.task_id=n.task_id
             JOIN task_publication_attempts a ON a.approval_id=p.approval_id AND a.task_id=p.task_id
             WHERE n.task_id=? AND n.workflow_id=?
             AND json_extract(n.data,'$.block_id')=? AND n.generation>?
             AND json_extract(n.data,'$.validity')='current'
             AND json_extract(n.data,'$.proposed_decision')='proposed'
             AND json_extract(a.data,'$.outcome')='applied'
             AND json_extract(p.data,'$.maintenance') IS NULL)",
        )
        .bind(task)
        .bind(string(&d, "workflow_id")?)
        .bind(string(&d, "block_id")?)
        .bind(d["generation"].as_i64().ok_or_else(Error::invalid)?)
        .fetch_one(&mut *conn)
        .await
        .map_err(Error::storage)?;
        if superseded {
            return Err(approval());
        }
    } else {
        if d["validity"] != "current"
            || d["proposed_decision"] != "proposed"
            || d["profile_revision"] != v["profile_revision"]
            || d["lens_version"] != c["lens_version"]
            || d["rules_sha256"] != c["rules_sha256"]
            || d["target_id"] != c["target_id"]
        {
            return Err(approval());
        }
        dependencies(
            conn,
            task,
            string(v, "workflow_id")?,
            string(&d, "input_record_id")?,
            &d["dependencies"],
        )
        .await?;
        if proposal["output_sha256"] != d["output_sha256"] {
            return Err(Error::conflict());
        }
    }
    let existing=sqlx::query("SELECT data FROM task_publications WHERE task_id=? AND decision_id=? ORDER BY workflow_sequence").bind(task).bind(string(&d,"decision_id")?).fetch_all(&mut *conn).await.map_err(Error::storage)?;
    for r in existing {
        let p: Value = serde_json::from_str(&r.get::<String, _>("data")).map_err(Error::storage)?;
        if p.get("maintenance") != v.get("maintenance")
            || p["incarnation"] != v["incarnation"]
            || p["profile_revision"] != v["profile_revision"]
        {
            continue;
        }
        if p["proposal"] == *proposal
            && ["previous_approval_id", "reason", "prior_block_sha256"]
                .iter()
                .all(|k| p.get(*k) == v.get(*k))
            && p["incarnation"] == v["incarnation"]
            && p["profile_revision"] == v["profile_revision"]
        {
            return Ok(Operation {
                changed: false,
                result: approval_result(conn, task, &p).await?,
                kind: "approval",
                id: string(&p, "approval_id")?.into(),
            });
        }
        return Err(Error::conflict());
    }
    let id = Uuid::new_v4().to_string();
    let mut p = json!({"approval_id":id,"decision_id":d["decision_id"],"incarnation":v["incarnation"],"profile_revision":v["profile_revision"],"proposal":proposal,"validity":d["validity"]});
    if maintenance {
        p["maintenance"] = v["maintenance"].clone();
        if v["maintenance"] == "remove" {
            for k in ["previous_approval_id", "reason", "prior_block_sha256"] {
                p[k] = v[k].clone();
            }
        }
    } else {
        d["decision"] = json!("approved");
        sqlx::query("UPDATE task_consumer_decisions SET data=? WHERE decision_id=?")
            .bind(d.to_string())
            .bind(string(&d, "decision_id")?)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
    }
    let seq = next_sequence(conn).await?;
    sqlx::query("INSERT INTO task_publications VALUES (?,?,?,?,?)")
        .bind(&id)
        .bind(task)
        .bind(string(&d, "decision_id")?)
        .bind(p.to_string())
        .bind(seq)
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    Ok(Operation {
        changed: true,
        result: approval_result(conn, task, &p).await?,
        kind: "approval",
        id,
    })
}
async fn attempt_result(
    conn: &mut SqliteConnection,
    task: &str,
    p: &Value,
    a: &Value,
) -> Result<Value, Error> {
    let latest = latest_attempt(conn, task, string(p, "approval_id")?).await?;
    Ok(
        json!({"approval_id":p["approval_id"],"attempt_id":a["attempt_id"],"outcome":a["outcome"],"validity":p["validity"],"publication":latest.map(|a|a["outcome"].clone()).unwrap_or(json!("not_applied"))}),
    )
}
async fn publication(
    conn: &mut SqliteConnection,
    task: &str,
    v: &Value,
) -> Result<Operation, Error> {
    let p = publication_data(conn, task, string(v, "approval_id")?).await?;
    let d = scoped_decision(conn, task, string(&p, "decision_id")?).await?;
    if d["workflow_id"] != v["workflow_id"] {
        return Err(Error::scope());
    }
    if p["incarnation"] != v["incarnation"] || p["incarnation"] != incarnation(conn).await? {
        return Err(recovery());
    }
    if p["profile_revision"] != v["profile_revision"] {
        return Err(stale());
    }
    if v["outcome"] == "applied" && v["observed_sha256"] != p["proposal"]["after_sha256"] {
        return Err(Error::conflict());
    }
    let mut a = select_fields(
        v,
        &["outcome", "observed_at", "observed_sha256", "error_code"],
    );
    let previous = latest_attempt(conn, task, string(&p, "approval_id")?).await?;
    if let Some(previous) = previous {
        if previous["outcome"] == "applied" {
            if select_fields(
                &previous,
                &["outcome", "observed_at", "observed_sha256", "error_code"],
            ) == a
            {
                return Ok(Operation {
                    changed: false,
                    result: attempt_result(conn, task, &p, &previous).await?,
                    kind: "publication",
                    id: string(&previous, "attempt_id")?.into(),
                });
            }
            return Err(Error::conflict());
        }
        // Reporting an applied effect must survive a concurrent invalidation.
        // A stale intent never authorizes another ordinary failed/conflict retry.
        if p["validity"] != "current" && v["outcome"] != "applied" && p.get("maintenance").is_none()
        {
            return Err(stale());
        }
    }
    let id = Uuid::new_v4().to_string();
    a["attempt_id"] = json!(id);
    let seq = next_sequence(conn).await?;
    sqlx::query("INSERT INTO task_publication_attempts VALUES (?,?,?,?,?)")
        .bind(&id)
        .bind(task)
        .bind(string(&p, "approval_id")?)
        .bind(a.to_string())
        .bind(seq)
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    Ok(Operation {
        changed: true,
        result: attempt_result(conn, task, &p, &a).await?,
        kind: "publication",
        id,
    })
}
pub(super) async fn current_result(
    conn: &mut SqliteConnection,
    task: &str,
    kind: &str,
    result: &Value,
) -> Result<Value, Error> {
    match kind {
        "decision" => Ok(decision_result(
            &scoped_decision(conn, task, string(result, "decision_id")?).await?,
        )),
        "approval" => {
            let p = publication_data(conn, task, string(result, "approval_id")?).await?;
            approval_result(conn, task, &p).await
        }
        "publication" => {
            let p = publication_data(conn, task, string(result, "approval_id")?).await?;
            let raw:String=sqlx::query_scalar("SELECT data FROM task_publication_attempts WHERE task_id=? AND approval_id=? AND attempt_id=?").bind(task).bind(string(&p,"approval_id")?).bind(string(result,"attempt_id")?).fetch_one(&mut *conn).await.map_err(Error::storage)?;
            let a = serde_json::from_str(&raw).map_err(Error::storage)?;
            attempt_result(conn, task, &p, &a).await
        }
        _ => Err(Error::invalid()),
    }
}
pub(super) async fn projection(
    conn: &mut SqliteConnection,
    task: &str,
    wid: &str,
    view: &str,
) -> Result<Vec<(i64, Value)>, Error> {
    let mut out = Vec::new();
    if view == "decisions" {
        let rows=sqlx::query("SELECT workflow_sequence,data FROM task_consumer_decisions WHERE task_id=? AND workflow_id=? ORDER BY workflow_sequence").bind(task).bind(wid).fetch_all(&mut *conn).await.map_err(Error::storage)?;
        for row in rows {
            let d: Value =
                serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
            out.push((
                row.get("workflow_sequence"),
                select_fields(
                    &d,
                    &[
                        "decision_id",
                        "consumer_id",
                        "root_record_id",
                        "generation",
                        "input_record_id",
                        "proposed_decision",
                        "decision",
                        "validity",
                        "dependencies",
                        "dependency_sha256",
                        "decision_sha256",
                        "output_sha256",
                        "block_id",
                        "reason",
                        "profile_revision",
                    ],
                ),
            ));
        }
    } else if view == "publications" {
        let rows=sqlx::query("SELECT p.workflow_sequence,p.data FROM task_publications p JOIN task_consumer_decisions d ON d.decision_id=p.decision_id WHERE p.task_id=? AND d.workflow_id=? ORDER BY p.workflow_sequence").bind(task).bind(wid).fetch_all(&mut *conn).await.map_err(Error::storage)?;
        for row in rows {
            let mut p: Value =
                serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
            let latest = latest_attempt(conn, task, string(&p, "approval_id")?).await?;
            p["publication"] = latest
                .as_ref()
                .map(|a| a["outcome"].clone())
                .unwrap_or(json!("not_applied"));
            p["attempts"] = json!(latest.into_iter().collect::<Vec<_>>());
            out.push((row.get("workflow_sequence"), p));
        }
    } else {
        return Err(Error::invalid());
    }
    Ok(out)
}
