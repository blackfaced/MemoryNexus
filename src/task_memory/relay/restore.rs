//! A restored device has no send authority until its old peer has reconciled
//! the task. Both endpoints start the new epoch with empty receipt positions.
use super::*;

pub(super) const TABLE: &str = "CREATE TABLE task_relay_recovery (task_id TEXT PRIMARY KEY, epoch TEXT NOT NULL, challenge TEXT NOT NULL, finalized TEXT, FOREIGN KEY(task_id,epoch) REFERENCES task_relay_pairs(task_id,epoch))";
const INVENTORY: &[u8] = b"memorynexus/task-recovery/inventory/v1";
const READY: &[u8] = b"memorynexus/task-recovery/ready/v1";
const COMPLETE: &[u8] = b"memorynexus/task-recovery/complete/v1";

pub(in crate::task_memory) fn required() -> Error {
    Error::new(
        "recovery_required",
        "restored task requires reconciliation; inspect relay recovery-status",
    )
}

// Older restores only disabled their pairs. Pick the most recent pairing for
// those copies, and refuse ordinary operations until restored by a current CLI.
pub(super) async fn pending(
    c: &mut SqliteConnection,
    task: &str,
) -> Result<Option<SqliteRow>, Error> {
    if !installed(c).await? {
        return Ok(None);
    }
    sqlx::query("SELECT * FROM task_relay_pairs p WHERE task_id=? AND NOT EXISTS(SELECT 1 FROM task_relay_pairs WHERE task_id=p.task_id AND active=1) ORDER BY rowid DESC LIMIT 1")
        .bind(task).fetch_optional(c).await.map_err(Error::storage)
}

async fn ensure_schema(c: &mut SqliteConnection) -> Result<(), Error> {
    withdrawal::install(c).await?;
    let version = recovery::schema_version(c).await?;
    if version < 13 {
        sqlx::query(TABLE)
            .execute(&mut *c)
            .await
            .map_err(Error::storage)?;
        sqlx::query(if version == 11 {
            "PRAGMA user_version=13"
        } else {
            "PRAGMA user_version=14"
        })
        .execute(&mut *c)
        .await
        .map_err(Error::storage)?;
    }
    Ok(())
}
pub(in crate::task_memory) async fn quarantine(c: &mut SqliteConnection) -> Result<(), Error> {
    if !installed(c).await? {
        return Ok(());
    }
    ensure_schema(c).await?;
    let pairs=sqlx::query("SELECT task_id,epoch FROM task_relay_pairs WHERE rowid IN (SELECT MAX(rowid) FROM task_relay_pairs GROUP BY task_id)").fetch_all(&mut *c).await.map_err(Error::storage)?;
    for pair in pairs {
        sqlx::query("INSERT INTO task_relay_recovery VALUES (?,?,?,NULL) ON CONFLICT(task_id) DO UPDATE SET epoch=excluded.epoch,challenge=excluded.challenge,finalized=NULL")
            .bind(pair.get::<&str,_>("task_id")).bind(pair.get::<&str,_>("epoch")).bind(Uuid::new_v4().to_string()).execute(&mut *c).await.map_err(Error::storage)?;
    }
    sqlx::query("UPDATE task_relay_pairs SET active=0")
        .execute(c)
        .await
        .map_err(Error::storage)?;
    Ok(())
}

pub(in crate::task_memory) async fn require_ready(
    c: &mut SqliteConnection,
    task: &str,
) -> Result<(), Error> {
    if pending(c, task).await?.is_some() {
        return Err(required());
    }
    Ok(())
}
pub(in crate::task_memory) async fn any_pending(c: &mut SqliteConnection) -> Result<bool, Error> {
    if !installed(c).await? {
        return Ok(false);
    }
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT task_id FROM task_relay_pairs GROUP BY task_id HAVING MAX(active)=0)",
    )
    .fetch_one(c)
    .await
    .map_err(Error::storage)
}

#[derive(Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Challenge {
    task_id: String,
    epoch: String,
    device: String,
    peer: String,
    next_epoch: String,
}
impl Challenge {
    fn check(&self, pair: &SqliteRow, restored: bool) -> Result<(), Error> {
        canonical_uuid(&self.next_epoch)?;
        if self.next_epoch == self.epoch
            || self.task_id != pair.get::<&str, _>("task_id")
            || self.device != pair.get::<&str, _>(if restored { "device" } else { "peer" })
            || self.peer != pair.get::<&str, _>(if restored { "peer" } else { "device" })
        {
            return Err(Error::scope());
        }
        Ok(())
    }
}
async fn challenge(c: &mut SqliteConnection, pair: &SqliteRow) -> Result<Challenge, Error> {
    let task_id = pair.get::<String, _>("task_id");
    let epoch = pair.get::<String, _>("epoch");
    if recovery::schema_version(c).await? < 13 {
        return Err(required());
    }
    let next_epoch =
        sqlx::query_scalar("SELECT challenge FROM task_relay_recovery WHERE task_id=? AND epoch=?")
            .bind(&task_id)
            .bind(&epoch)
            .fetch_optional(c)
            .await
            .map_err(Error::storage)?
            .ok_or_else(required)?;
    Ok(Challenge {
        task_id,
        epoch,
        device: pair.get("device"),
        peer: pair.get("peer"),
        next_epoch,
    })
}
pub(super) async fn status(c: &mut SqliteConnection, pair: &SqliteRow) -> Result<Value, Error> {
    let progress = refresh::status(c, pair, false).await?;
    let mut result = json!({"state":"recovery_required","task_id":progress["task_id"],"epoch":progress["epoch"],"device":progress["device"],"peer":progress["peer"],"last_confirmed_outgoing":progress["outgoing"]["through"],"last_received_incoming":progress["incoming"]["through"],"subsequent_peer_tail":"unknown"});
    if recovery::schema_version(c).await? >= 13 {
        result["challenge"] = json!(challenge(c, pair).await?);
    } else {
        result["next_action"] = json!("restore_to_new_path");
    }
    Ok(result)
}
pub(super) async fn local_status(path: &Path) -> Result<Value, Error> {
    let task = read_config(path)?.task_id;
    let mut c = open(&ledger_path()?, true).await?;
    let mut tx = c.begin().await.map_err(Error::storage)?;
    let revision: i64 = registered_task(&mut tx, &task).await?.get("revision");
    let result = if let Some(row) = pending(&mut tx, &task).await? {
        status(&mut tx, &row).await?
    } else {
        json!({"state":"ready"})
    };
    Ok(json!({"schema_version":1,"status":"ok","revision":revision,"task_id":task,"result":result}))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory {
    challenge: Challenge,
    // Ordered by the restored device's stream, then the surviving peer's.
    positions: [i64; 2],
    peer_confirmed: i64,
    peer_received: i64,
    sha256: String,
}
async fn inventory(
    c: &mut SqliteConnection,
    request: Challenge,
    peer_confirmed: i64,
    peer_received: i64,
) -> Result<Inventory, Error> {
    let task = &request.task_id;
    let mut hash = Sha256::new();
    let mut positions = [0i64; 2];
    for (index, origin) in [&request.device, &request.peer].into_iter().enumerate() {
        let rows=sqlx::query("SELECT seq,record_id,digest FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=? ORDER BY seq")
            .bind(task).bind(&request.epoch).bind(origin).fetch_all(&mut *c).await.map_err(Error::storage)?;
        for row in rows {
            positions[index] += 1;
            if row.get::<i64, _>("seq") != positions[index] {
                return Err(Error::conflict());
            }
            hash.update(
                serde_json::to_vec(&(
                    origin,
                    positions[index],
                    row.get::<&str, _>("record_id"),
                    row.get::<&str, _>("digest"),
                ))
                .map_err(Error::storage)?,
            );
        }
    }
    // Compare effective content, including pre-pair baseline and withdrawals.
    // Local revision and commit order can legitimately differ across replicas.
    let rows=sqlx::query("SELECT id,kind,trust,state,text,occurred_at,run_id,client,evidence_refs FROM task_records WHERE task_id=? ORDER BY id").bind(task).fetch_all(&mut *c).await.map_err(Error::storage)?;
    for row in rows {
        let id = row.get::<&str, _>("id");
        let state = row.get::<&str, _>("state");
        let mut record = [
            id,
            state,
            row.get::<&str, _>("kind"),
            row.get::<&str, _>("trust"),
            row.get::<&str, _>("occurred_at"),
            row.get::<&str, _>("run_id"),
            row.get::<&str, _>("client"),
        ]
        .into_iter()
        .map(|s| json!(s))
        .collect::<Vec<_>>();
        if !matches!(state, "stale" | "retracted") {
            let sources = lifecycle::source_ids(c, id).await?;
            record.extend([
                json!(row.get::<&str, _>("text")),
                serde_json::from_str(row.get("evidence_refs")).map_err(Error::storage)?,
                json!(sources),
            ]);
        }
        let parents:Vec<String>=sqlx::query_scalar("SELECT supersedes_id FROM task_record_revisions WHERE record_id=? ORDER BY supersedes_id").bind(id).fetch_all(&mut *c).await.map_err(Error::storage)?;
        hash.update(serde_json::to_vec(&(record, parents)).map_err(Error::storage)?);
    }
    if peer_received > positions[0] || peer_confirmed > positions[1] {
        return Err(Error::conflict());
    }
    Ok(Inventory {
        peer_confirmed,
        peer_received,
        challenge: request,
        positions,
        sha256: format!("{:x}", hash.finalize()),
    })
}

pub(super) async fn proof(
    c: &mut SqliteConnection,
    pair: &SqliteRow,
    request: Challenge,
    key: &str,
) -> Result<Value, Error> {
    request.check(pair, false)?;
    if request.epoch != pair.get::<&str, _>("epoch") {
        return Err(Error::scope());
    }
    let (confirmed, received) = refresh::last_positions(c, pair).await?;
    Ok(json!(refresh::Proof::sign(
        inventory(c, request, confirmed, received).await?,
        key,
        INVENTORY
    )?))
}

// Only the already-authorized pair can relay retained changes from either
// origin back into the quarantined copy. Rebuild the normal signed batch.
async fn route(
    c: &mut SqliteConnection,
    pair: &SqliteRow,
    origin: &str,
    outbound: bool,
) -> Result<SqliteRow, Error> {
    let device = pair.get::<&str, _>("device");
    let peer = pair.get::<&str, _>("peer");
    if origin != device && origin != peer {
        return Err(Error::scope());
    }
    let destination = if origin == device { peer } else { device };
    let (local, other) = if outbound {
        (origin, destination)
    } else {
        (destination, origin)
    };
    sqlx::query("SELECT task_id,epoch,? AS device,? AS peer FROM task_relay_pairs WHERE task_id=? AND epoch=?")
        .bind(local).bind(other).bind(pair.get::<&str,_>("task_id")).bind(pair.get::<&str,_>("epoch")).fetch_one(c).await.map_err(Error::storage)
}
pub(super) async fn incoming(
    c: &mut SqliteConnection,
    pair: &SqliteRow,
    origin: &str,
) -> Result<SqliteRow, Error> {
    route(c, pair, origin, false).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageRequest {
    challenge: Challenge,
    origin: String,
    after: i64,
}
pub(super) async fn page(
    c: &mut SqliteConnection,
    pair: &SqliteRow,
    req: PageRequest,
    key: &str,
) -> Result<Value, Error> {
    req.challenge.check(pair, false)?;
    if req.challenge.epoch != pair.get::<&str, _>("epoch") || req.after < 0 {
        return Err(Error::scope());
    }
    let stream = route(c, pair, &req.origin, true).await?;
    let rows=sqlx::query("SELECT seq,record_id FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=? AND seq>? ORDER BY seq LIMIT 20")
        .bind(&req.challenge.task_id).bind(&req.challenge.epoch).bind(&req.origin).bind(req.after).fetch_all(&mut *c).await.map_err(Error::storage)?;
    let through = rows.last().map_or(req.after, |r| r.get("seq"));
    let ids = rows
        .iter()
        .map(|r| r.get("record_id"))
        .collect::<Vec<String>>();
    let mut result = json!({"through":through});
    if !ids.is_empty() {
        result["batch"] = export(
            c,
            &req.challenge.task_id,
            &stream,
            key,
            Selection { record_ids: ids },
            true,
        )
        .await?;
    }
    Ok(result)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Approval {
    proof: refresh::Proof<Inventory>,
    confirmation: Confirmation,
}
pub(super) async fn reconcile(
    c: &mut SqliteConnection,
    pair: &SqliteRow,
    approval: Approval,
    key: &str,
    restored: bool,
) -> Result<Value, Error> {
    let Confirmation::Confirmed = approval.confirmation;
    let proof = approval.proof;
    let req = &proof.body.challenge;
    let local_device = pair.get::<&str, _>("device");
    let from_backup = local_device == req.device;
    req.check(pair, from_backup)?;
    let epoch = pair.get::<&str, _>("epoch");
    if !restored && epoch == req.next_epoch {
        // Only return the saved decision for this exact transcript. No state
        // comparison can be repeated after new-epoch writes have begun.
        proof.verify(key, if from_backup { COMPLETE } else { READY })?;
        let saved:Option<String>=sqlx::query_scalar("SELECT finalized FROM task_relay_recovery WHERE task_id=? AND epoch=? AND challenge=? AND finalized IS NOT NULL")
            .bind(&req.task_id).bind(&req.epoch).bind(&req.next_epoch).fetch_optional(&mut *c).await.map_err(Error::storage)?;
        let complete = refresh::Proof::sign(proof.body, key, COMPLETE)?;
        if Some(json!(complete).to_string()) != saved {
            return Err(Error::conflict());
        }
        return Ok(json!(complete));
    }
    if epoch != req.epoch || restored != from_backup {
        return Err(Error::scope());
    }
    let preview = restored && proof.verify(key, INVENTORY).is_ok();
    if !preview {
        proof.verify(key, if restored { COMPLETE } else { READY })?;
    }
    if restored && challenge(c, pair).await? != *req {
        return Err(Error::conflict());
    }
    let expected_positions = proof.body.positions;
    let expected_hash = proof.body.sha256;
    let (confirmed, received) = refresh::last_positions(c, pair).await?;
    if restored {
        // A receipt already retained by the backup cannot be ahead of
        // what the surviving device can still demonstrate for that stream.
        if confirmed > proof.body.positions[0] || received > proof.body.positions[1] {
            return Err(Error::conflict());
        }
    } else if (confirmed, received) != (proof.body.peer_confirmed, proof.body.peer_received) {
        return Err(Error::conflict());
    }
    let current = inventory(
        c,
        proof.body.challenge,
        proof.body.peer_confirmed,
        proof.body.peer_received,
    )
    .await?;
    if current.positions != expected_positions || current.sha256 != expected_hash {
        return Err(Error::conflict());
    }
    if preview {
        // Remain gated until the survivor has accepted this same snapshot.
        return Ok(json!(refresh::Proof::sign(current, key, READY)?));
    }
    ensure_schema(c).await?;
    sqlx::query("UPDATE task_relay_pairs SET active=0 WHERE task_id=? AND epoch=?")
        .bind(&current.challenge.task_id)
        .bind(epoch)
        .execute(&mut *c)
        .await
        .map_err(Error::storage)?;
    pair_new_epoch(c, pair, &current.challenge.next_epoch, key).await?;
    let complete = refresh::Proof::sign(current, key, COMPLETE)?;
    let req = &complete.body.challenge;
    sqlx::query("INSERT INTO task_relay_recovery VALUES (?,?,?,?) ON CONFLICT(task_id) DO UPDATE SET epoch=excluded.epoch,challenge=excluded.challenge,finalized=excluded.finalized")
        .bind(&req.task_id).bind(&req.epoch).bind(&req.next_epoch).bind(json!(complete).to_string()).execute(&mut *c).await.map_err(Error::storage)?;
    Ok(json!(complete))
}
async fn pair_new_epoch(
    c: &mut SqliteConnection,
    old: &SqliteRow,
    epoch: &str,
    key: &str,
) -> Result<(), Error> {
    super::pair(
        c,
        old.get("task_id"),
        PairInput {
            device: old.get("device"),
            peer: old.get("peer"),
            epoch: epoch.to_owned(),
            confirmation: Confirmation::Confirmed,
        },
        key,
    )
    .await?;
    Ok(())
}
