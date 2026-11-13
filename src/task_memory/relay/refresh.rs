//! One explicit refresh freezes each stream's target and records evidence at both ends.
use super::*;

pub(super) const ROUNDS: &str = "CREATE TABLE task_relay_rounds (id INTEGER PRIMARY KEY, task_id TEXT NOT NULL, epoch TEXT NOT NULL, round TEXT NOT NULL, target INTEGER NOT NULL, peer_target INTEGER, received INTEGER NOT NULL, confirmed INTEGER NOT NULL, receipt TEXT, UNIQUE(task_id,epoch,round), FOREIGN KEY(task_id,epoch) REFERENCES task_relay_pairs(task_id,epoch))";

pub(super) const ENQUEUED: &str = "CREATE TABLE task_relay_enqueued (record_id TEXT PRIMARY KEY NOT NULL REFERENCES task_relay_changes(record_id), queued_at_ms INTEGER NOT NULL)";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StatusOptions {
    #[serde(default)]
    pub include_batch_age: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Start {
    round: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    round: String,
    after: Option<i64>,
    limit: Option<i64>,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Target {
    task_id: String,
    epoch: String,
    origin: String,
    destination: String,
    round: String,
    upper: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proof<T> {
    pub(super) body: T,
    mac: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    target: Target,
    through: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Page {
    offer: Proof<Target>,
    batch: Option<Signed>,
}

fn tag<T: Serialize>(key: &str, domain: &[u8], body: &T) -> Result<Hmac<Sha256>, Error> {
    let mut h = Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|_| Error::invalid())?;
    h.update(domain);
    h.update(&serde_json::to_vec(body).map_err(Error::storage)?);
    Ok(h)
}
const OFFER: &[u8] = b"memorynexus/relay-target/v1\0";
const ACK: &[u8] = b"memorynexus/relay-receipt/v1\0";
impl<T: Serialize> Proof<T> {
    pub(super) fn sign(body: T, key: &str, domain: &[u8]) -> Result<Self, Error> {
        let mac = format!("{:x}", tag(key, domain, &body)?.finalize().into_bytes());
        Ok(Self { body, mac })
    }
    pub(super) fn verify(&self, key: &str, domain: &[u8]) -> Result<(), Error> {
        let bytes = decode_tag(&self.mac)?;
        tag(key, domain, &self.body)?
            .verify_slice(&bytes)
            .map_err(|_| Error::conflict())
    }
}

async fn latest(tx: &mut SqliteConnection, pair: &SqliteRow) -> Result<Option<SqliteRow>, Error> {
    if !matches!(recovery::schema_version(tx).await?, 5..=14) {
        return Ok(None);
    }
    sqlx::query(
        "SELECT * FROM task_relay_rounds WHERE task_id=? AND epoch=? ORDER BY id DESC LIMIT 1",
    )
    .bind(pair.get::<&str, _>("task_id"))
    .bind(pair.get::<&str, _>("epoch"))
    .fetch_optional(tx)
    .await
    .map_err(Error::storage)
}

pub(super) async fn last_positions(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
) -> Result<(i64, i64), Error> {
    let row = latest(tx, pair).await?;
    Ok((
        row.as_ref().map_or(0, |r| r.get("confirmed")),
        row.as_ref().map_or(0, |r| r.get("received")),
    ))
}

async fn current(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
    round: &str,
) -> Result<SqliteRow, Error> {
    latest(tx, pair)
        .await?
        .filter(|r| r.get::<&str, _>("round") == round)
        .ok_or_else(Error::conflict)
}
fn target(pair: &SqliteRow, round: &str, upper: i64, outgoing: bool) -> Target {
    let (origin, destination) = if outgoing {
        ("device", "peer")
    } else {
        ("peer", "device")
    };
    Target {
        task_id: pair.get("task_id"),
        epoch: pair.get("epoch"),
        origin: pair.get(origin),
        destination: pair.get(destination),
        round: round.to_owned(),
        upper,
    }
}
fn offer(pair: &SqliteRow, row: &SqliteRow, key: &str) -> Result<Proof<Target>, Error> {
    Proof::sign(
        target(pair, row.get("round"), row.get("target"), true),
        key,
        OFFER,
    )
}

// Count the uninterrupted prefix, including already stored records beyond a hole.
async fn contiguous(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
    previous: i64,
) -> Result<i64, Error> {
    sqlx::query_scalar("WITH stream AS (SELECT seq FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=? AND seq>?), positions AS (SELECT ? AS seq UNION ALL SELECT seq FROM stream) SELECT MIN(p.seq) FROM positions p WHERE NOT EXISTS (SELECT 1 FROM stream s WHERE s.seq=p.seq+1)")
        .bind(pair.get::<&str,_>("task_id"))
        .bind(pair.get::<&str,_>("epoch"))
        .bind(pair.get::<&str,_>("peer"))
        .bind(previous)
        .bind(previous)
        .fetch_one(tx)
        .await.map_err(Error::storage)
}

async fn local_upper(tx: &mut SqliteConnection, pair: &SqliteRow) -> Result<i64, Error> {
    sqlx::query_scalar("SELECT COALESCE(MAX(seq),0) FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=?")
        .bind(pair.get::<&str, _>("task_id"))
        .bind(pair.get::<&str, _>("epoch"))
        .bind(pair.get::<&str, _>("device"))
        .fetch_one(tx)
        .await
        .map_err(Error::storage)
}

pub(super) async fn install(tx: &mut SqliteConnection) -> Result<(), Error> {
    let version = recovery::schema_version(tx).await?;
    let marker = match version {
        3 | 5 => "PRAGMA user_version=7",
        4 | 6 => "PRAGMA user_version=8",
        7..=14 => return Ok(()),
        _ => return Err(Error::version()),
    };
    if matches!(version, 3 | 4) {
        sqlx::query(ROUNDS)
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
    }
    sqlx::query(ENQUEUED)
        .execute(&mut *tx)
        .await
        .map_err(Error::storage)?;
    sqlx::query(marker)
        .execute(tx)
        .await
        .map_err(Error::storage)?;
    Ok(())
}

pub(super) async fn start(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
    input: Start,
    key: &str,
) -> Result<Value, Error> {
    let round = input.round.unwrap_or_else(|| Uuid::new_v4().to_string());
    canonical_uuid(&round)?;
    let prev = latest(tx, pair).await?;
    if let Some(ref row) = prev {
        if row.get::<&str, _>("round") == round {
            return Ok(json!(offer(pair, row, key)?));
        }
    }
    install(tx).await?;
    let confirmed = prev.as_ref().map_or(0, |r| r.get("confirmed"));
    let received = contiguous(tx, pair, prev.as_ref().map_or(0, |r| r.get("received"))).await?;
    let task = pair.get::<&str, _>("task_id");
    let epoch = pair.get::<&str, _>("epoch");
    let upper = local_upper(tx, pair).await?;
    let inserted = sqlx::query("INSERT INTO task_relay_rounds(task_id,epoch,round,target,received,confirmed) VALUES(?,?,?,?,?,?) ON CONFLICT DO NOTHING")
       .bind(task)
        .bind(epoch)
        .bind(&round)
        .bind(upper)
        .bind(received)
        .bind(confirmed)
        .execute(tx)
        .await.map_err(Error::storage)?;
    if inserted.rows_affected() != 1 {
        return Err(Error::conflict());
    }
    Ok(json!(Proof::sign(
        target(pair, &round, upper, true),
        key,
        OFFER
    )?))
}

pub(super) async fn page(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
    sel: Selection,
    key: &str,
) -> Result<Value, Error> {
    let row = current(tx, pair, &sel.round).await?;
    let upper = row.get("target");
    let after = sel.after.unwrap_or_else(|| row.get("confirmed"));
    let limit = sel.limit.unwrap_or(20);
    if !(0..=upper).contains(&after) || !(1..=20).contains(&limit) {
        return Err(Error::invalid());
    }
    let task = pair.get::<&str, _>("task_id");
    let ids: Vec<String> = sqlx::query_scalar("SELECT record_id FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=? AND seq>? AND seq<=? ORDER BY seq LIMIT ?")
      .bind(task)
        .bind(pair.get::<&str,_>("epoch"))
        .bind(pair.get::<&str,_>("device"))
        .bind(after)
        .bind(upper)
        .bind(limit)
      .fetch_all(&mut *tx)
        .await.map_err(Error::storage)?;
    let mut page = json!({"offer":offer(pair, &row, key)?});
    if !ids.is_empty() {
        page["batch"] = export(
            tx,
            task,
            pair,
            key,
            super::Selection { record_ids: ids },
            true,
        )
        .await?;
    }
    Ok(page)
}

pub(super) async fn accept(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
    page: Page,
    key: &str,
) -> Result<Value, Error> {
    page.offer.verify(key, OFFER)?;
    let t = page.offer.body;
    if t.upper < 0 || t != target(pair, &t.round, t.upper, false) {
        return Err(Error::scope());
    }
    let row = current(tx, pair, &t.round).await?;
    if row
        .get::<Option<i64>, _>("peer_target")
        .is_some_and(|v| v != t.upper)
    {
        return Err(Error::conflict());
    }
    let changes = if let Some(signed) = page.batch {
        if signed.batch.changes.iter().any(|c| c.seq > t.upper) {
            return Err(Error::conflict());
        }
        Some(receive(tx, &t.task_id, pair, key, signed).await?)
    } else {
        None
    };
    let received = contiguous(tx, pair, row.get("received")).await?;
    sqlx::query("UPDATE task_relay_rounds SET peer_target=?,received=? WHERE task_id=? AND epoch=? AND round=?")
        .bind(t.upper)
        .bind(received)
        .bind(&t.task_id)
        .bind(&t.epoch)
        .bind(&t.round)
        .execute(tx)
        .await.map_err(Error::storage)?;
    let through = received.min(t.upper);
    let mut result = json!({"receipt":Proof::sign(Receipt{target:t,through},key,ACK)?});
    if let Some(changes) = changes {
        result["changes"] = changes;
    }
    Ok(result)
}

pub(super) async fn acknowledge(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
    proof: Proof<Receipt>,
    key: &str,
) -> Result<Value, Error> {
    proof.verify(key, ACK)?;
    let t = &proof.body.target;
    let row = current(tx, pair, &t.round).await?;
    if *t != target(pair, &t.round, row.get("target"), true) {
        return Err(Error::scope());
    }
    let through = proof.body.through;
    if !(0..=t.upper).contains(&through) {
        return Err(Error::invalid());
    }
    sqlx::query("UPDATE task_relay_rounds SET confirmed=?,receipt=? WHERE task_id=? AND epoch=? AND round=? AND confirmed<=?")
      .bind(through)
        .bind(json!(proof).to_string())
        .bind(&t.task_id)
        .bind(&t.epoch)
        .bind(&t.round)
        .bind(through)
        .execute(&mut *tx)
        .await.map_err(Error::storage)?;
    status(tx, pair, false).await
}

fn direction(position: i64, upper: Option<i64>, proved: bool) -> Value {
    let mut result =
        json!({"through":position,"covered":proved && upper.is_some_and(|v|position>=v)});
    if let Some(upper) = upper {
        result["target"] = json!(upper);
        if upper > position {
            result["unconfirmed_range"] = json!([position + 1, upper]);
        }
    }
    result
}
pub(in crate::task_memory) async fn status(
    tx: &mut SqliteConnection,
    pair: &SqliteRow,
    include_batch_age: bool,
) -> Result<Value, Error> {
    let row = latest(tx, pair).await?;
    let round = row.as_ref().map(|r| r.get::<&str, _>("round"));
    let received = match row.as_ref() {
        Some(r) => r.get("received"),
        None => contiguous(tx, pair, 0).await?,
    };
    let incoming_target = row
        .as_ref()
        .and_then(|r| r.get::<Option<i64>, _>("peer_target"));
    let receipt = row
        .as_ref()
        .and_then(|r| r.get::<Option<String>, _>("receipt"));
    let confirmed = row.as_ref().map_or(0, |r| r.get("confirmed"));
    let mut outgoing = direction(
        confirmed,
        row.as_ref().map(|r| r.get("target")),
        receipt.is_some(),
    );
    let pending = local_upper(tx, pair).await? - confirmed;
    outgoing["pending_count"] = json!(pending);
    if include_batch_age {
        let queued_at: Option<i64> = if pending > 0 && recovery::schema_version(tx).await? >= 7 {
            sqlx::query_scalar("SELECT e.queued_at_ms FROM task_relay_changes c JOIN task_relay_enqueued e USING(record_id) WHERE c.task_id=? AND c.epoch=? AND c.origin=? AND c.seq=?")
                .bind(pair.get::<&str, _>("task_id"))
                .bind(pair.get::<&str, _>("epoch"))
                .bind(pair.get::<&str, _>("device"))
                .bind(confirmed + 1)
                .fetch_optional(&mut *tx)
                .await.map_err(Error::storage)?
        } else {
            None
        };
        outgoing["oldest_age_ms"] =
            json!(queued_at.map(|ms| (chrono::Utc::now().timestamp_millis() - ms).max(0)));
    }
    let mut result = json!({"task_id":pair.get::<&str,_>("task_id"),"epoch":pair.get::<&str,_>("epoch"),"device":pair.get::<&str,_>("device"),"peer":pair.get::<&str,_>("peer"),"outgoing":outgoing,"incoming":direction(received,incoming_target,incoming_target.is_some()),"subsequent_peer_tail":"unknown"});
    if let Some(round) = round {
        result["round"] = json!(round);
    }
    Ok(result)
}
