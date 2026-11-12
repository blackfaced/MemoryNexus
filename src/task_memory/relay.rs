//! Explicit two-device batches for one locally paired task.
use super::*;
use hmac::{Hmac, Mac};
use sqlx::sqlite::SqliteRow;

const PAIR: &str = "CREATE TABLE task_relay_pairs (task_id TEXT NOT NULL REFERENCES tasks(id), epoch TEXT NOT NULL, device TEXT NOT NULL, peer TEXT NOT NULL, key_hash TEXT NOT NULL, active INTEGER NOT NULL, PRIMARY KEY(task_id,epoch))";
const CHANGES: &str = "CREATE TABLE task_relay_changes (task_id TEXT NOT NULL, epoch TEXT NOT NULL, origin TEXT NOT NULL, seq INTEGER NOT NULL, record_id TEXT NOT NULL UNIQUE REFERENCES task_records(id), source_revision INTEGER NOT NULL, parents TEXT NOT NULL, envelope TEXT NOT NULL, digest TEXT NOT NULL, PRIMARY KEY(task_id,epoch,origin,seq), FOREIGN KEY(task_id,epoch) REFERENCES task_relay_pairs(task_id,epoch))";
const MAX_BATCH: u64 = 1024 * 1024;

pub(super) fn include_schema(version: i64, expected: &mut Vec<String>) {
    if matches!(version, 3 | 4) {
        expected.extend([PAIR.to_owned(), CHANGES.to_owned()]);
    }
}

async fn installed(c: &mut SqliteConnection) -> Result<bool, Error> {
    Ok(matches!(recovery::schema_version(c).await?, 3 | 4))
}

pub(super) async fn disable_restored_pairs(c: &mut SqliteConnection) -> Result<(), Error> {
    if installed(c).await? {
        sqlx::query("UPDATE task_relay_pairs SET active=0")
            .execute(c)
            .await
            .map_err(Error::storage)?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "action",
    content = "input",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(super) enum Mutation {
    Record(RecordInput),
    Review(lifecycle::ReviewInput),
    Correct(lifecycle::CorrectInput),
}

impl Mutation {
    fn checkpoint(&self) -> bool {
        matches!(self, Self::Record(input) if input.kind == RecordKind::Checkpoint)
    }

    async fn apply(
        self,
        tx: &mut SqliteConnection,
        task: &str,
        id: &str,
        parents: &[String],
    ) -> Result<Value, Error> {
        match self {
            Self::Record(input) => record(tx, task, id, parents, input).await,
            Self::Review(input) => lifecycle::review(tx, task, id, input).await,
            Self::Correct(input) => lifecycle::correct(tx, task, id, input).await,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairInput {
    device: String,
    peer: String,
    epoch: String,
    confirmation: Confirmation,
}

fn pair_key() -> Result<String, Error> {
    let key = env::var("MEMORYNEXUS_TASK_PAIR_KEY").map_err(|_| Error::scope())?;
    if key.len() != 64 || !key.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::invalid());
    }
    Ok(key)
}

pub(super) async fn active(
    c: &mut SqliteConnection,
    task: &str,
) -> Result<Option<SqliteRow>, Error> {
    if !installed(c).await? {
        return Ok(None);
    }
    sqlx::query("SELECT * FROM task_relay_pairs WHERE task_id=? AND active=1")
        .bind(task)
        .fetch_optional(c)
        .await
        .map_err(Error::storage)
}

async fn pair(
    tx: &mut SqliteConnection,
    task: &str,
    input: PairInput,
    key: &str,
) -> Result<Value, Error> {
    let Confirmation::Confirmed = input.confirmation;
    canonical_uuid(&input.device)?;
    canonical_uuid(&input.peer)?;
    canonical_uuid(&input.epoch)?;
    if input.device == input.peer {
        return Err(Error::invalid());
    }
    if !installed(tx).await? {
        let marker = match recovery::schema_version(tx).await? {
            1 => "PRAGMA user_version=3",
            2 => "PRAGMA user_version=4",
            _ => return Err(Error::version()),
        };
        sqlx::query(marker)
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
        for ddl in [PAIR, CHANGES] {
            sqlx::query(ddl)
                .execute(&mut *tx)
                .await
                .map_err(Error::storage)?;
        }
    }
    let hash = format!("{:x}", Sha256::digest(key.as_bytes()));
    if let Some(row) = active(tx, task).await? {
        if row.get::<String, _>("device") != input.device
            || row.get::<String, _>("peer") != input.peer
            || row.get::<String, _>("epoch") != input.epoch
            || row.get::<String, _>("key_hash") != hash
        {
            return Err(Error::conflict());
        }
    } else {
        let r =
            sqlx::query("INSERT INTO task_relay_pairs VALUES (?,?,?,?,?,1) ON CONFLICT DO NOTHING")
                .bind(task)
                .bind(&input.epoch)
                .bind(&input.device)
                .bind(&input.peer)
                .bind(hash)
                .execute(tx)
                .await
                .map_err(Error::storage)?;
        if r.rows_affected() != 1 {
            return Err(Error::conflict());
        }
    }
    Ok(json!({"task_id":task,"epoch":input.epoch,"device":input.device,"peer":input.peer}))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Change {
    seq: i64,
    record_id: String,
    source_revision: i64,
    parents: Vec<String>,
    mutation: Value,
}

async fn remember(
    tx: &mut SqliteConnection,
    task: &str,
    epoch: &str,
    origin: &str,
    change: &Change,
    hash: &str,
) -> Result<(), Error> {
    // Only a reconstruction recipe is retained. Retraction erases the single authoritative body.
    let mut envelope = change.mutation.clone();
    let input = envelope["input"]
        .as_object_mut()
        .ok_or_else(Error::invalid)?;
    input.remove("text");
    if let Some(refs) = input.get_mut("evidence_refs") {
        *refs = json!([]);
    }
    sqlx::query("INSERT INTO task_relay_changes VALUES (?,?,?,?,?,?,?,?,?)")
        .bind(task)
        .bind(epoch)
        .bind(origin)
        .bind(change.seq)
        .bind(&change.record_id)
        .bind(change.source_revision)
        .bind(json!(change.parents).to_string())
        .bind(envelope.to_string())
        .bind(hash)
        .execute(tx)
        .await
        .map_err(Error::storage)?;
    Ok(())
}

pub(super) async fn write_local(path: &Path, mutation: Mutation) -> Result<Value, Error> {
    let task = read_config(path)?.task_id;
    let mut conn = open(&ledger_path()?, false).await?;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(Error::storage)?;
    let pair = active(&mut tx, &task).await?;
    let parents = if mutation.checkpoint() {
        sqlx::query_scalar("SELECT id FROM task_records WHERE task_id=? AND kind='checkpoint' AND state='current' ORDER BY commit_sequence")
            .bind(&task).fetch_all(&mut *tx).await.map_err(Error::storage)?
    } else {
        Vec::new()
    };
    let wire = pair
        .as_ref()
        .map(|_| serde_json::to_value(&mutation))
        .transpose()
        .map_err(Error::storage)?;
    let id = Uuid::new_v4().to_string();
    let response = mutation.apply(&mut tx, &task, &id, &parents).await?;
    if let (Some(row), Some(mutation)) = (pair, wire) {
        if response["status"] == "accepted" {
            let epoch: String = row.get("epoch");
            let device: String = row.get("device");
            let seq=sqlx::query_scalar("SELECT COALESCE(MAX(seq),0)+1 FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=?")
                .bind(&task).bind(&epoch).bind(&device).fetch_one(&mut *tx).await.map_err(Error::storage)?;
            let change = Change {
                seq,
                record_id: id,
                source_revision: response["revision"].as_i64().ok_or_else(Error::invalid)?,
                parents,
                mutation,
            };
            let hash = digest("task-relay-v1", &change)?;
            remember(&mut tx, &task, &epoch, &device, &change, &hash).await?;
        }
    }
    tx.commit().await.map_err(Error::storage)?;
    Ok(response)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    record_ids: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    version: u8,
    task_id: String,
    epoch: String,
    origin: String,
    destination: String,
    changes: Vec<Change>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed {
    batch: Batch,
    mac: String,
}

fn authenticator(key: &str, batch: &Batch) -> Result<Hmac<Sha256>, Error> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|_| Error::invalid())?;
    mac.update(b"memorynexus/task-relay/v1\0");
    mac.update(&serde_json::to_vec(batch).map_err(|_| Error::invalid())?);
    Ok(mac)
}

async fn export(
    tx: &mut SqliteConnection,
    task: &str,
    pair: &SqliteRow,
    key: &str,
    sel: Selection,
) -> Result<Value, Error> {
    if sel.record_ids.is_empty() || sel.record_ids.len() > 20 {
        return Err(Error::invalid());
    }
    let epoch: String = pair.get("epoch");
    let origin: String = pair.get("device");
    let mut changes = Vec::new();
    for id in sel.record_ids {
        canonical_uuid(&id)?;
        let rec = lifecycle::scoped_record(tx, task, &id).await?;
        if matches!(
            rec.get::<String, _>("state").as_str(),
            "retracted" | "stale"
        ) {
            return Err(Error::conflict());
        }
        let row=sqlx::query("SELECT seq,source_revision,parents,envelope,digest FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=? AND record_id=?")
            .bind(task).bind(&epoch).bind(&origin).bind(&id).fetch_optional(&mut *tx).await.map_err(Error::storage)?.ok_or_else(Error::not_found)?;
        let mut mutation: Value =
            serde_json::from_str(&row.get::<String, _>("envelope")).map_err(Error::storage)?;
        mutation["input"]["text"] = json!(rec.get::<String, _>("text"));
        if mutation["input"].get("evidence_refs").is_some() {
            mutation["input"]["evidence_refs"] =
                serde_json::from_str(&rec.get::<String, _>("evidence_refs"))
                    .map_err(Error::storage)?;
        }
        let change = Change {
            seq: row.get("seq"),
            source_revision: row.get("source_revision"),
            parents: serde_json::from_str(&row.get::<String, _>("parents"))
                .map_err(Error::storage)?,
            record_id: id,
            mutation,
        };
        if digest("task-relay-v1", &change)? != row.get::<String, _>("digest") {
            return Err(Error::conflict());
        }
        changes.push(change);
    }
    changes.sort_by_key(|c| c.seq);
    if changes.windows(2).any(|p| p[0].seq == p[1].seq) {
        return Err(Error::invalid());
    }
    let batch = Batch {
        version: 1,
        task_id: task.to_owned(),
        epoch,
        origin,
        destination: pair.get("peer"),
        changes,
    };
    let mac = format!("{:x}", authenticator(key, &batch)?.finalize().into_bytes());
    let signed = json!(Signed { batch, mac });
    if serde_json::to_vec(&signed).map_err(Error::storage)?.len() as u64 > MAX_BATCH {
        return Err(Error::invalid());
    }
    Ok(signed)
}

async fn receive(
    tx: &mut SqliteConnection,
    task: &str,
    pair: &SqliteRow,
    key: &str,
    signed: Signed,
) -> Result<Value, Error> {
    let b = signed.batch;
    if b.version != 1 {
        return Err(Error::version());
    }
    if b.task_id != task
        || b.epoch != pair.get::<String, _>("epoch")
        || b.origin != pair.get::<String, _>("peer")
        || b.destination != pair.get::<String, _>("device")
    {
        return Err(Error::scope());
    }
    if signed.mac.len() != 64 || !signed.mac.is_ascii() {
        return Err(Error::invalid());
    }
    let bytes = (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&signed.mac[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::invalid())?;
    authenticator(key, &b)?
        .verify_slice(&bytes)
        .map_err(|_| Error::conflict())?;
    if b.changes.is_empty() || b.changes.len() > 20 {
        return Err(Error::invalid());
    }
    let mut accepted = Vec::new();
    let mut replayed = Vec::new();
    for c in b.changes {
        canonical_uuid(&c.record_id)?;
        if c.seq < 1 || c.source_revision < 2 {
            return Err(Error::invalid());
        }
        let hash = digest("task-relay-v1", &c)?;
        let seen:Option<String>=sqlx::query_scalar("SELECT digest FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=? AND seq=?")
            .bind(task).bind(&b.epoch).bind(&b.origin).bind(c.seq).fetch_optional(&mut *tx).await.map_err(Error::storage)?;
        if let Some(seen) = seen {
            if seen != hash {
                return Err(Error::conflict());
            }
            replayed.push(c.record_id);
            continue;
        }
        let used: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_records WHERE id=?)")
            .bind(&c.record_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(Error::storage)?;
        if used {
            return Err(Error::conflict());
        }
        let mutation: Mutation =
            serde_json::from_value(c.mutation.clone()).map_err(|_| Error::invalid())?;
        if !mutation.checkpoint() && !c.parents.is_empty() {
            return Err(Error::invalid());
        }
        let response = mutation.apply(tx, task, &c.record_id, &c.parents).await?;
        if response["status"] != "accepted" {
            return Err(Error::conflict());
        }
        remember(tx, task, &b.epoch, &b.origin, &c, &hash).await?;
        accepted.push(c.record_id);
    }
    let accepted_change_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_relay_changes WHERE task_id=? AND epoch=? AND origin=?",
    )
    .bind(task)
    .bind(&b.epoch)
    .bind(&b.origin)
    .fetch_one(tx)
    .await
    .map_err(Error::storage)?;
    Ok(
        json!({"accepted":accepted,"replayed":replayed,"accepted_change_count":accepted_change_count}),
    )
}

enum Request {
    Pair(PairInput),
    Export(Selection),
    Receive(Signed),
}

pub(super) async fn run(path: &Path, action: &str) -> Result<Value, Error> {
    let request = match action {
        "pair" => Request::Pair(read_input(false)?),
        "export" => Request::Export(read_input(false)?),
        "receive" => Request::Receive(read_bounded_input(false, MAX_BATCH)?),
        _ => return Err(Error::invalid()),
    };
    let task = read_config(path)?.task_id;
    let key = pair_key()?;
    let read_only = matches!(request, Request::Export(_));
    let mut conn = open(&ledger_path()?, read_only).await?;
    let mut tx = conn
        .begin_with(if read_only {
            "BEGIN"
        } else {
            "BEGIN IMMEDIATE"
        })
        .await
        .map_err(Error::storage)?;
    registered_task(&mut tx, &task).await?;
    let result = match request {
        Request::Pair(input) => pair(&mut tx, &task, input, &key).await?,
        Request::Export(selection) => {
            let binding = authorized(&mut tx, &task, &key).await?;
            export(&mut tx, &task, &binding, &key, selection).await?
        }
        Request::Receive(signed) => {
            let binding = authorized(&mut tx, &task, &key).await?;
            receive(&mut tx, &task, &binding, &key, signed).await?
        }
    };
    let revision: i64 = registered_task(&mut tx, &task).await?.get("revision");
    tx.commit().await.map_err(Error::storage)?;
    Ok(json!({"schema_version":1,"status":"ok","task_id":task,"revision":revision,"result":result}))
}

async fn authorized(tx: &mut SqliteConnection, task: &str, key: &str) -> Result<SqliteRow, Error> {
    let binding = active(tx, task).await?.ok_or_else(Error::scope)?;
    if binding.get::<String, _>("key_hash") != format!("{:x}", Sha256::digest(key.as_bytes())) {
        return Err(Error::scope());
    }
    Ok(binding)
}
