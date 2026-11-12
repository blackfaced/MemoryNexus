//! A signed peer can have corrected a revision before learning about its other successor.
use super::*;

pub(super) const EDGES: &str = "CREATE TABLE task_record_revisions (record_id TEXT NOT NULL REFERENCES task_records(id), supersedes_id TEXT NOT NULL REFERENCES task_records(id), PRIMARY KEY(record_id,supersedes_id))";
pub(super) const BY_PARENT: &str =
    "CREATE INDEX task_revision_children ON task_record_revisions(supersedes_id)";

async fn allow_forks(tx: &mut SqliteConnection) -> Result<(), Error> {
    if recovery::schema_version(tx).await? >= 9 {
        return Ok(());
    }
    refresh::install(tx).await?;
    let version = recovery::schema_version(tx).await?;
    for statement in [
        "ALTER TABLE task_record_revisions RENAME TO task_linear_revisions",
        EDGES,
        "INSERT INTO task_record_revisions SELECT record_id,supersedes_id FROM task_linear_revisions",
        "DROP TABLE task_linear_revisions",
        BY_PARENT,
        if version == 7 { "PRAGMA user_version=9" } else { "PRAGMA user_version=10" },
    ] {
        sqlx::query(statement).execute(&mut *tx).await.map_err(Error::storage)?;
    }
    Ok(())
}

// The local writer still rejects a stale CAS. Only a verified incoming change may
// fork a parent already corrected by the opposite origin in this pairing.
async fn competing_child(
    tx: &mut SqliteConnection,
    task: &str,
    parent: &str,
    pair: &SqliteRow,
) -> Result<bool, Error> {
    sqlx::query_scalar("SELECT COUNT(*) > 0 AND COUNT(*) = SUM(c.origin=?) FROM task_record_revisions r LEFT JOIN task_relay_changes c ON c.record_id=r.record_id AND c.task_id=? AND c.epoch=? WHERE r.supersedes_id=?")
        .bind(pair.get::<&str,_>("device")).bind(task).bind(pair.get::<&str,_>("epoch")).bind(parent)
        .fetch_one(tx).await.map_err(Error::storage)
}

pub(in crate::task_memory) async fn prepare(
    tx: &mut SqliteConnection,
    task: &str,
    target: &SqliteRow,
    resolve_with: Option<&[String]>,
    peer: Option<&SqliteRow>,
    revision: i64,
) -> Result<(Vec<String>, bool), Error> {
    let id = target.get::<String, _>("id");
    let state = target.get::<&str, _>("state");
    if state == "current" && resolve_with.is_none() {
        return Ok((vec![id], false));
    }
    if peer.is_none() && (state != "pending_confirmation" || resolve_with.is_none()) {
        return Err(Error::conflict());
    }
    let mut parents = vec![id.clone()];
    if let Some(others) = resolve_with {
        if others.is_empty() || others.len() >= 20 {
            return Err(Error::invalid());
        }
        for other in others {
            canonical_uuid(other)?;
            if parents.contains(other) {
                return Err(Error::invalid());
            }
            parents.push(other.clone());
        }
    }
    let rows = sqlx::query("WITH RECURSIVE chain(id) AS (SELECT ? UNION SELECT CASE WHEN r.record_id=c.id THEN r.supersedes_id ELSE r.record_id END FROM task_record_revisions r JOIN chain c ON r.record_id=c.id OR r.supersedes_id=c.id) SELECT t.id,t.state FROM chain JOIN task_records t ON t.id=chain.id WHERE t.task_id=?")
        .bind(&id).bind(task).fetch_all(&mut *tx).await.map_err(Error::storage)?;
    for parent in &parents {
        let row = rows
            .iter()
            .find(|r| r.get::<&str, _>("id") == parent)
            .ok_or_else(Error::conflict)?;
        match (row.get::<&str, _>("state"), peer) {
            ("pending_confirmation", _) => (),
            ("superseded", Some(pair)) if competing_child(tx, task, parent, pair).await? => (),
            _ => return Err(Error::conflict()),
        }
    }
    let heads: Vec<_> = rows
        .iter()
        .filter(|r| {
            matches!(
                r.get::<&str, _>("state"),
                "current" | "pending_confirmation"
            )
        })
        .collect();
    if heads.is_empty() {
        return Err(Error::conflict());
    }
    let remaining: Vec<_> = heads
        .into_iter()
        .filter(|r| !parents.iter().any(|p| p == r.get::<&str, _>("id")))
        .collect();
    if peer.is_none() && !remaining.is_empty() {
        return Err(Error::conflict());
    }
    // An old peer edit cannot revive a fully retired branch.
    if remaining.is_empty() && state == "superseded" && resolve_with.is_none() {
        return Err(Error::conflict());
    }
    allow_forks(tx).await?;
    for row in &remaining {
        if row.get::<&str, _>("state") == "current" {
            let branch = row.get::<&str, _>("id");
            sqlx::query(
                "UPDATE task_records SET state='pending_confirmation',revision=? WHERE id=?",
            )
            .bind(revision)
            .bind(branch)
            .execute(&mut *tx)
            .await
            .map_err(Error::storage)?;
            lifecycle::invalidate_dependents(tx, task, branch, revision).await?;
        }
    }
    let pending =
        !remaining.is_empty() || (state == "pending_confirmation" && resolve_with.is_none());
    Ok((parents, pending))
}
