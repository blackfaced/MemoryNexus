//! Source observations and bounded extraction; never reads repository files.
use super::*;
pub(super) fn validate(action: &str, v: &Value) -> Result<(), Error> {
    if action == "source-sync" {
        envelope(
            v,
            &[
                "collection_id",
                "expected_scan",
                "observed_at",
                "availability",
                "units",
            ],
            &["cursor", "reason"],
        )?;
        canonical_uuid(string(v, "collection_id")?)?;
        number(v, "expected_scan")?;
        time(v, "observed_at")?;
        array(v, "units", 20)?;
        match string(v, "availability")? {
            "available" => {
                if v.get("reason").is_some() {
                    return Err(Error::invalid());
                }
            }
            "unavailable" => {
                if !["missing", "unreadable", "identity_uncertain"].contains(&string(v, "reason")?)
                    || !v["units"].as_array().unwrap().is_empty()
                    || v.get("cursor").is_some()
                {
                    return Err(Error::invalid());
                }
            }
            _ => return Err(Error::invalid()),
        }
        if let Some(c) = v.get("cursor") {
            object(c, &["offset", "prefix_sha256"], &[])?;
            number(c, "offset")?;
            hash(string(c, "prefix_sha256")?)?;
        }
        for u in v["units"].as_array().unwrap() {
            object(u, &["sha256"], &["start", "end"])?;
            hash(string(u, "sha256")?)?;
            if u.get("start").is_some() {
                number(u, "start")?;
                number(u, "end")?;
            } else if u.get("end").is_some() {
                return Err(Error::invalid());
            }
        }
    } else {
        envelope(
            v,
            &[
                "source_id",
                "source_revision",
                "extractor_version",
                "text",
                "occurred_at",
            ],
            &["confirmation"],
        )?;
        canonical_uuid(string(v, "source_id")?)?;
        if number(v, "source_revision")? == 0 {
            return Err(Error::invalid());
        }
        name(string(v, "extractor_version")?)?;
        bounded(string(v, "text")?, 1000)?;
        time(v, "occurred_at")?;
        optional_confirm(v)?;
    }
    Ok(())
}
pub(super) async fn execute(
    conn: &mut SqliteConnection,
    task: &str,
    action: &str,
    v: &Value,
    profile: &Value,
    revision: i64,
) -> Result<Operation, Error> {
    if action == "source-sync" {
        sync(conn, task, v, revision).await
    } else {
        capture(conn, task, v, profile, revision).await
    }
}
async fn sync(
    conn: &mut SqliteConnection,
    task: &str,
    v: &Value,
    revision: i64,
) -> Result<Operation, Error> {
    let wid = string(v, "workflow_id")?;
    let cid = string(v, "collection_id")?;
    let row=sqlx::query("SELECT * FROM task_source_collections WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(task).bind(wid).bind(cid).fetch_optional(&mut *conn).await.map_err(Error::storage)?.ok_or_else(Error::not_found)?;
    let def: Value =
        serde_json::from_str(&row.get::<String, _>("definition")).map_err(Error::storage)?;
    let mut c: Value =
        serde_json::from_str(&row.get::<String, _>("data")).map_err(Error::storage)?;
    if c["active"] != true {
        return Err(stale());
    }
    let text = def["format"] == "text";
    let available = v["availability"] == "available";
    let units = v["units"].as_array().unwrap();
    if text {
        if v.get("cursor").is_some()
            || available && units.len() != 1
            || units.iter().any(|u| u.get("start").is_some())
        {
            return Err(Error::invalid());
        }
    } else if available
        && (v.get("cursor").is_none() || units.iter().any(|u| u.get("start").is_none()))
    {
        return Err(Error::invalid());
    }
    let definition_hash = hash_value(&def);
    let definition_changed = row
        .get::<Option<String>, _>("observed_definition_sha256")
        .as_deref()
        != Some(definition_hash.as_str());
    let known=sqlx::query("SELECT * FROM task_sources WHERE task_id=? AND workflow_id=? AND collection_id=? ORDER BY workflow_sequence").bind(task).bind(wid).bind(cid).fetch_all(&mut *conn).await.map_err(Error::storage)?;
    let mut requested = Vec::new();
    let mut same_units = true;
    for u in units {
        let sid = if text {
            cid.to_owned()
        } else {
            Uuid::new_v5(
                &Uuid::parse_str(cid).map_err(|_| Error::invalid())?,
                number(u, "start")?.to_string().as_bytes(),
            )
            .to_string()
        };
        let existing = known
            .iter()
            .find(|r| r.get::<String, _>("source_id") == sid);
        if let Some(r) = existing {
            let s = current_source(conn, task, &sid).await?;
            if s["sha256"] != u["sha256"] {
                if !text {
                    return Err(Error::conflict());
                }
                same_units = false;
            }
            if !text && (r.get::<Option<i64>, _>("end") != Some(number(u, "end")?)) {
                return Err(Error::conflict());
            }
            requested.push(s);
        } else {
            same_units = false;
            requested.push(json!({"source_id":sid}));
        }
    }
    let same_availability =
        c["availability"] == v["availability"] && (available || c.get("reason") == v.get("reason"));
    let cursor_same = text || !available || c["cursor"] == v["cursor"];
    if !definition_changed && same_availability && same_units && cursor_same {
        return Ok(sync_result(false, cid, &c, requested, &Counts::default()));
    }
    if number(v, "expected_scan")? != number(&c, "scan")? {
        return Err(Error::conflict());
    }
    if !text && available {
        let old_offset = number(&c["cursor"], "offset")?;
        let mut end = old_offset;
        for u in units {
            let start = number(u, "start")?;
            let next = number(u, "end")?;
            if start != end || next <= start || next - start > 32768 {
                return Err(Error::conflict());
            }
            end = next;
        }
        if number(&v["cursor"], "offset")? != end
            || end == old_offset && v["cursor"] != c["cursor"]
            || end > 64 * 1024 * 1024
        {
            return Err(Error::conflict());
        }
        if end == 0 && string(&v["cursor"], "prefix_sha256")? != hash_bytes(b"") {
            return Err(Error::invalid());
        }
    }
    let mut invalid = Counts::default();
    // Refresh all known stream occurrences on definition/availability transitions.
    for r in &known {
        let sid = r.get::<String, _>("source_id");
        let old = current_source(conn, task, &sid).await?;
        let unit = if text { units.first() } else { None };
        let changed = definition_changed
            || old["availability"] != v["availability"]
            || unit.is_some_and(|u| u["sha256"] != old["sha256"]);
        if !changed {
            continue;
        }
        let mut observation = old.clone();
        observation["source_revision"] = json!(r.get::<i64, _>("current_revision") + 1);
        observation["availability"] = v["availability"].clone();
        observation["observed_at"] = v["observed_at"].clone();
        observation["extractor_version"] = def["extractor_version"].clone();
        observation["collection_definition_sha256"] = json!(definition_hash);
        if let Some(u) = unit {
            observation["sha256"] = u["sha256"].clone();
        }
        observation["locator"] = if text {
            json!({"path":def["path"],"selector":def["selector"]})
        } else {
            json!({"path":def["path"],"start":r.get::<i64,_>("start"),"end":r.get::<i64,_>("end")})
        };
        store_observation(conn, task, &sid, &observation).await?;
        invalid.add(invalidate_source(conn, task, &sid, revision).await?);
    }
    for (index, u) in units.iter().enumerate() {
        let sid = string(&requested[index], "source_id")?.to_owned();
        if known.iter().any(|r| r.get::<String, _>("source_id") == sid) {
            continue;
        }
        let seq = next_sequence(conn).await?;
        sqlx::query("INSERT INTO task_sources VALUES (?,?,?,?,NULL,1,0,?,?,?)")
            .bind(task)
            .bind(&sid)
            .bind(wid)
            .bind(cid)
            .bind(if text {
                None
            } else {
                Some(number(u, "start")?)
            })
            .bind(if text { None } else { Some(number(u, "end")?) })
            .bind(seq)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
        let locator = if text {
            json!({"path":def["path"],"selector":def["selector"]})
        } else {
            json!({"path":def["path"],"start":u["start"],"end":u["end"]})
        };
        let observation = json!({"source_id":sid,"source_revision":1,"sha256":u["sha256"],"availability":"available","observed_at":v["observed_at"],"locator":locator,"extractor_version":def["extractor_version"],"collection_definition_sha256":definition_hash});
        store_observation(conn, task, &sid, &observation).await?;
    }
    c["scan"] = json!(number(&c, "scan")? + 1);
    c["availability"] = v["availability"].clone();
    c["observed_at"] = v["observed_at"].clone();
    if available {
        c.as_object_mut().unwrap().remove("reason");
        if !text {
            c["cursor"] = v["cursor"].clone();
        }
    } else {
        c["reason"] = v["reason"].clone();
    }
    sqlx::query("UPDATE task_source_collections SET data=?,observed_definition_sha256=? WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(c.to_string()).bind(definition_hash).bind(task).bind(wid).bind(cid).execute(&mut *conn).await.map_err(Error::storage)?;
    for r in &mut requested {
        *r = current_source(conn, task, string(r, "source_id")?).await?;
    }
    Ok(sync_result(true, cid, &c, requested, &invalid))
}
fn sync_result(
    changed: bool,
    cid: &str,
    c: &Value,
    sources: Vec<Value>,
    counts: &Counts,
) -> Operation {
    let minimal:Vec<Value>=sources.iter().map(|s|json!({"source_id":s["source_id"],"source_revision":s["source_revision"],"sha256":s["sha256"],"availability":s["availability"]})).collect();
    let mut result = json!({"collection_id":cid,"scan":c["scan"],"sources":minimal,"invalidated":{"records":counts.records,"decisions":counts.decisions,"approvals":counts.approvals}});
    if let Some(cursor) = c.get("cursor") {
        result["cursor"] = cursor.clone();
    }
    Operation {
        changed,
        result,
        kind: "source_sync",
        id: Uuid::new_v4().to_string(),
    }
}
async fn store_observation(
    conn: &mut SqliteConnection,
    task: &str,
    sid: &str,
    value: &Value,
) -> Result<(), Error> {
    let mut clean = json!({});
    for k in [
        "source_id",
        "source_revision",
        "sha256",
        "availability",
        "observed_at",
        "locator",
        "extractor_version",
        "collection_definition_sha256",
    ] {
        clean[k] = value[k].clone();
    }
    sqlx::query("INSERT INTO task_source_versions VALUES (?,?,?,?)")
        .bind(task)
        .bind(sid)
        .bind(number(value, "source_revision")?)
        .bind(clean.to_string())
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    sqlx::query("UPDATE task_sources SET current_revision=? WHERE task_id=? AND source_id=?")
        .bind(number(value, "source_revision")?)
        .bind(task)
        .bind(sid)
        .execute(conn)
        .await
        .map_err(Error::storage)?;
    Ok(())
}
async fn capture(
    conn: &mut SqliteConnection,
    task: &str,
    v: &Value,
    p: &Value,
    revision: i64,
) -> Result<Operation, Error> {
    let sid = string(v, "source_id")?;
    let src = scoped_source(conn, task, sid).await?;
    if src.get::<String, _>("workflow_id") != string(v, "workflow_id")? {
        return Err(Error::scope());
    }
    if src.get::<bool, _>("blocked") {
        return Err(Error::conflict());
    }
    let c=sqlx::query("SELECT definition,data FROM task_source_collections WHERE task_id=? AND workflow_id=? AND collection_id=?").bind(task).bind(string(v,"workflow_id")?).bind(src.get::<String,_>("collection_id")).fetch_one(&mut *conn).await.map_err(Error::storage)?;
    let collection: Value =
        serde_json::from_str(&c.get::<String, _>("data")).map_err(Error::storage)?;
    if collection["active"] != true {
        return Err(stale());
    }
    if p["capture"]["automatic_drafts"] != true && v["confirmation"] != "confirmed" {
        return Err(approval());
    }
    let existing:Option<String>=sqlx::query_scalar("SELECT record_id FROM task_source_extractions WHERE task_id=? AND source_id=? AND source_revision=? AND extractor_version=?").bind(task).bind(sid).bind(number(v,"source_revision")?).bind(string(v,"extractor_version")?).fetch_optional(&mut *conn).await.map_err(Error::storage)?;
    if let Some(id) = existing {
        let row = lifecycle::scoped_record(conn, task, &id).await?;
        if row.get::<String, _>("text") != string(v, "text")?
            || row.get::<String, _>("occurred_at") != string(v, "occurred_at")?
        {
            return Err(Error::conflict());
        }
        let result = capture_result(conn, task, &id).await?;
        return Ok(Operation {
            changed: false,
            result,
            kind: "capture",
            id,
        });
    }
    let observation = current_source(conn, task, sid).await?;
    let definition: Value =
        serde_json::from_str(&c.get::<String, _>("definition")).map_err(Error::storage)?;
    if observation["availability"] != "available"
        || observation["source_revision"] != v["source_revision"]
        || observation["extractor_version"] != v["extractor_version"]
        || hash_value(&definition) != string(&observation, "collection_definition_sha256")?
    {
        return Err(stale());
    }
    let id = Uuid::new_v4().to_string();
    let root = src.get::<Option<String>, _>("root_record_id");
    let refs = json!([{"path":observation["locator"]["path"],"sha256":observation["sha256"]}]);
    sqlx::query("INSERT INTO task_records (id,task_id,kind,trust,text,occurred_at,run_id,client,evidence_refs,state,revision) VALUES (?,?,'draft','source_reported',?,?,?,?,?,'current',?)").bind(&id).bind(task).bind(string(v,"text")?).bind(string(v,"occurred_at")?).bind(string(v,"run_id")?).bind(string(v,"client")?).bind(refs.to_string()).bind(revision).execute(&mut *conn).await.map_err(Error::storage)?;
    if let Some(root) = root {
        let head = chain_head(conn, task, &root).await?; // Newly inserted row is not yet linked.
        sqlx::query("INSERT INTO task_record_revisions VALUES (?,?)")
            .bind(&id)
            .bind(head)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
    } else {
        sqlx::query("UPDATE task_sources SET root_record_id=? WHERE task_id=? AND source_id=?")
            .bind(&id)
            .bind(task)
            .bind(sid)
            .execute(&mut *conn)
            .await
            .map_err(Error::storage)?;
    }
    sqlx::query("INSERT INTO task_source_extractions VALUES (?,?,?,?,?)")
        .bind(task)
        .bind(sid)
        .bind(number(v, "source_revision")?)
        .bind(string(v, "extractor_version")?)
        .bind(&id)
        .execute(&mut *conn)
        .await
        .map_err(Error::storage)?;
    Ok(Operation {
        changed: true,
        result: capture_result(conn, task, &id).await?,
        kind: "capture",
        id,
    })
}
pub(super) async fn capture_result(
    conn: &mut SqliteConnection,
    task: &str,
    id: &str,
) -> Result<Value, Error> {
    let row = lifecycle::scoped_record(conn, task, id).await?;
    let origin = record_origin(conn, task, id)
        .await?
        .ok_or_else(Error::not_found)?;
    Ok(
        json!({"record_id":id,"root_record_id":root_record(conn,task,id).await?,"source_id":origin["source_id"],"source_revision":origin["source_revision"],"state":row.get::<String,_>("state"),"kind":row.get::<String,_>("kind"),"trust":row.get::<String,_>("trust")}),
    )
}
