use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn invoke(flag: &str, path: &Path, ledger: Option<&Path>, action: &str, input: Value) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_memorynexus-feedback-cli"));
    command
        .args(["task", flag, path.to_str().unwrap(), action])
        .env_remove("MEMORYNEXUS_TASK_LEDGER");
    if let Some(ledger) = ledger {
        command.env("MEMORYNEXUS_TASK_LEDGER", ledger);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes());
    let output = child.wait_with_output().unwrap();
    let response: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("non-JSON output: {output:?}"));
    assert_eq!(
        output.status.code(),
        Some(if response["status"] == "error" { 2 } else { 0 }),
        "{response}"
    );
    assert!(output.stderr.is_empty(), "{output:?}");
    response
}
fn recovery(ledger: &Path, action: &str, input: Value) -> Value {
    invoke("--ledger", ledger, None, action, input)
}
fn task(config: &Path, ledger: &Path, action: &str, input: Value) -> Value {
    invoke("--config", config, Some(ledger), action, input)
}
fn initialize(config: &Path, ledger: &Path) -> Value {
    let result = task(
        config,
        ledger,
        "init",
        json!({"idempotency_key":"init", "display_name":"Recovery fixture", "confirmation":"confirmed", "automatic_capture":["execution","review"]}),
    );
    assert_eq!(result["status"], "accepted", "{result}");
    result
}
fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
fn migration(destination: &Path, backup: &Path) -> Value {
    json!({"path":destination,"backup_path":backup,"backup_sha256":sha(backup),"expected_version":1,"target_version":2,"confirmation":"confirmed"})
}

#[test]
fn explicit_migration_preserves_legacy_memory_and_retry_identity_without_changing_source() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.sqlite");
    let config = temp.path().join("task.json");
    initialize(&config, &source);
    let input = json!({"idempotency_key":"keep", "run_id":"10000000-0000-4000-8000-000000000001", "client":"codex", "kind":"execution", "text":"Observed result with bounded conditions", "occurred_at":"2026-10-23T00:00:00Z"});
    let accepted = task(&config, &source, "record", input.clone());
    assert_eq!(accepted["status"], "accepted", "{accepted}");
    let before = recovery(&source, "export", json!({}));
    let backup = temp.path().join("backup.sqlite");
    assert_eq!(
        recovery(&source, "backup", json!({"path":backup}))["status"],
        "ok"
    );
    let source_hash = sha(&source);
    let backup_hash = sha(&backup);
    let destination = temp.path().join("v2.sqlite");
    let migrated = recovery(&source, "migrate", migration(&destination, &backup));
    assert_eq!(migrated["status"], "ok", "{migrated}");
    assert_eq!(migrated["schema_version"], 1);
    assert_eq!(migrated["workflow_contract"], 1);
    assert_eq!(migrated["result"]["path"], destination.to_str().unwrap());
    assert_eq!(migrated["result"]["version"], 2);
    assert_eq!(migrated["revision"], before["revision"]);
    assert!(uuid::Uuid::parse_str(migrated["result"]["incarnation"].as_str().unwrap()).is_ok());
    assert_eq!(sha(&source), source_hash);
    assert_eq!(sha(&backup), backup_hash);
    assert_eq!(recovery(&source, "export", json!({})), before);
    let after = recovery(&destination, "export", json!({}));
    assert_eq!(after["result"]["version"], 2, "{after}");
    for field in [
        "tasks",
        "records",
        "record_sources",
        "record_revisions",
        "init_receipts",
        "commit_sequence",
    ] {
        assert_eq!(after["result"][field], before["result"][field], "{field}");
    }
    for field in [
        "task_id",
        "idempotency_key",
        "action",
        "digest",
        "record_id",
        "revision",
    ] {
        assert_eq!(
            after["result"]["receipts"][0][field], before["result"]["receipts"][0][field],
            "{field}"
        );
    }
    assert_eq!(after["result"]["receipts"][0]["result_type"], "record");
    assert_eq!(
        after["result"]["receipts"][0]["result_id"],
        before["result"]["receipts"][0]["record_id"]
    );
    let replay = task(&config, &destination, "record", input);
    assert_eq!(replay["status"], "replayed", "{replay}");
    assert_eq!(
        replay["result"]["record_id"],
        accepted["result"]["record_id"]
    );
    assert_eq!(
        recovery(&source, "migrate", migration(&destination, &backup))["code"],
        "conflict"
    );
}

#[test]
fn migration_requires_the_exact_complete_backup_and_new_destination() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.sqlite");
    let config = temp.path().join("task.json");
    initialize(&config, &source);
    let backup = temp.path().join("backup.sqlite");
    assert_eq!(
        recovery(&source, "backup", json!({"path":backup}))["status"],
        "ok"
    );
    let destination = temp.path().join("destination.sqlite");
    let original = migration(&destination, &backup);
    for field in ["expected_version", "target_version"] {
        let mut input = original.clone();
        input[field] = json!(88);
        assert_eq!(
            recovery(&source, "migrate", input)["code"],
            "unsupported_version"
        );
        assert!(!destination.exists());
    }
    for field in ["confirmation", "backup_sha256", "backup_path", "path"] {
        let mut input = original.clone();
        input.as_object_mut().unwrap().remove(field);
        assert_eq!(recovery(&source, "migrate", input)["code"], "invalid_input");
        assert!(!destination.exists());
    }
    for (field, value) in [
        ("unknown", json!(true)),
        ("backup_sha256", json!("A".repeat(64))),
        ("path", json!("relative.sqlite")),
        ("confirmation", json!("unconfirmed")),
    ] {
        let mut input = original.clone();
        input[field] = value;
        assert_eq!(recovery(&source, "migrate", input)["code"], "invalid_input");
        assert!(!destination.exists());
    }
    let mut wrong_hash = original.clone();
    wrong_hash["backup_sha256"] = json!("0".repeat(64));
    assert_eq!(recovery(&source, "migrate", wrong_hash)["code"], "conflict");
    assert!(!destination.exists());
    let mut same_file = original.clone();
    same_file["backup_path"] = json!(source);
    same_file["backup_sha256"] = json!(sha(&source));
    assert_eq!(recovery(&source, "migrate", same_file)["code"], "conflict");
    assert!(!destination.exists());
    // A matching byte hash is insufficient: this backup has different committed state.
    let input = json!({"idempotency_key":"later", "run_id":"10000000-0000-4000-8000-000000000001", "client":"codex", "kind":"execution", "text":"Observed after the verified backup", "occurred_at":"2026-10-23T00:00:00Z"});
    assert_eq!(
        task(&config, &source, "record", input)["status"],
        "accepted"
    );
    let before = recovery(&source, "export", json!({}));
    let source_hash = sha(&source);
    let backup_hash = sha(&backup);
    assert_eq!(
        recovery(&source, "migrate", original.clone())["code"],
        "conflict"
    );
    assert!(!destination.exists());
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let existing = temp.path().join(format!("destination.sqlite{suffix}"));
        std::fs::write(&existing, b"owner content").unwrap();
        assert_eq!(
            recovery(&source, "migrate", original.clone())["code"],
            "conflict"
        );
        assert_eq!(std::fs::read(&existing).unwrap(), b"owner content");
        std::fs::remove_file(existing).unwrap();
    }
    assert_eq!(sha(&source), source_hash);
    assert_eq!(sha(&backup), backup_hash);
    assert_eq!(recovery(&source, "export", json!({})), before);
    assert_eq!(
        std::fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".memorynexus-"))
            .count(),
        0
    );
}

#[test]
fn backup_retains_v2_incarnation_and_restore_renews_it_without_losing_legacy_tombstones() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("v1.sqlite");
    let config = temp.path().join("task.json");
    initialize(&config, &source);
    let original = json!({"idempotency_key":"original", "run_id":"10000000-0000-4000-8000-000000000001", "client":"codex", "kind":"execution", "text":"Withdraw this bounded report", "occurred_at":"2026-10-23T00:00:00Z"});
    let accepted = task(&config, &source, "record", original.clone());
    assert_eq!(accepted["status"], "accepted");
    let retract = json!({"idempotency_key":"withdraw", "run_id":"10000000-0000-4000-8000-000000000002", "client":"codex", "target_id":accepted["result"]["record_id"], "reason":"Owner requests withdrawal", "confirmation":"confirmed"});
    let retracted = task(&config, &source, "retract", retract.clone());
    assert_eq!(retracted["status"], "accepted", "{retracted}");
    let v1_before = recovery(&source, "export", json!({}));
    let v1_backup = temp.path().join("v1-backup.sqlite");
    assert_eq!(
        recovery(&source, "backup", json!({"path":v1_backup}))["result"]["version"],
        1
    );
    let v1_restored = temp.path().join("v1-restored.sqlite");
    assert_eq!(
        recovery(&v1_restored, "restore", json!({"path":v1_backup}))["result"]["version"],
        1
    );
    assert_eq!(recovery(&v1_restored, "export", json!({})), v1_before);
    let ledger = temp.path().join("v2.sqlite");
    assert_eq!(
        recovery(&source, "migrate", migration(&ledger, &v1_backup))["status"],
        "ok"
    );
    let before = recovery(&ledger, "export", json!({}));
    let backup = temp.path().join("v2-backup.sqlite");
    let backed = recovery(&ledger, "backup", json!({"path":backup}));
    assert_eq!(backed["status"], "ok", "{backed}");
    assert_eq!(backed["result"]["version"], 2);
    assert_eq!(recovery(&backup, "export", json!({})), before);
    let invalid_migration = temp.path().join("wrong-backup-migration.sqlite");
    assert_eq!(
        recovery(&source, "migrate", migration(&invalid_migration, &backup))["code"],
        "conflict"
    );
    assert!(!invalid_migration.exists());
    let restored = temp.path().join("v2-restored.sqlite");
    let response = recovery(&restored, "restore", json!({"path":backup}));
    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(response["result"]["version"], 2);
    assert_ne!(
        response["result"]["incarnation"],
        backed["result"]["incarnation"]
    );
    let mut after = recovery(&restored, "export", json!({}));
    assert_eq!(
        after["result"]["ledger_meta"][0]["incarnation"],
        response["result"]["incarnation"]
    );
    after["result"]["ledger_meta"][0]["incarnation"] =
        before["result"]["ledger_meta"][0]["incarnation"].clone();
    assert_eq!(after, before);
    for (action, input) in [("record", original), ("retract", retract)] {
        let replay = task(&config, &restored, action, input);
        assert_eq!(replay["status"], "replayed", "{replay}");
        assert_eq!(replay["result"]["state"], "retracted");
    }
    assert!(!after.to_string().contains("Withdraw this bounded report"));
    let unsupported_target = temp.path().join("not-created.sqlite");
    assert_eq!(
        recovery(
            &ledger,
            "migrate",
            migration(&unsupported_target, &v1_backup)
        )["code"],
        "unsupported_version"
    );
    assert!(!unsupported_target.exists());
}

#[test]
fn unknown_version_and_corrupt_backup_are_refused_without_mutation_or_publication() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.sqlite");
    let config = temp.path().join("task.json");
    initialize(&config, &source);
    let backup = temp.path().join("backup.sqlite");
    assert_eq!(
        recovery(&source, "backup", json!({"path":backup}))["status"],
        "ok"
    );
    let pristine = std::fs::read(&backup).unwrap();
    let future = temp.path().join("future.sqlite");
    let mut bytes = pristine.clone();
    bytes[60..64].copy_from_slice(&99_u32.to_be_bytes());
    std::fs::write(&future, bytes).unwrap();
    let future_before = sha(&future);
    let corrupt = temp.path().join("corrupt.sqlite");
    std::fs::write(&corrupt, &pristine[..400]).unwrap();
    let corrupt_before = sha(&corrupt);
    for (name, candidate, code) in [
        ("future", &future, "unsupported_version"),
        ("corrupt", &corrupt, "storage_unavailable"),
    ] {
        let destination = temp.path().join(format!("{name}-target.sqlite"));
        assert_eq!(recovery(candidate, "export", json!({}))["code"], code);
        assert_eq!(
            recovery(candidate, "backup", json!({"path":destination}))["code"],
            code
        );
        assert_eq!(
            recovery(&destination, "restore", json!({"path":candidate}))["code"],
            code
        );
        assert_eq!(
            recovery(&source, "migrate", migration(&destination, candidate))["code"],
            code
        );
        assert!(!destination.exists());
    }
    assert_eq!(sha(&future), future_before);
    assert_eq!(sha(&corrupt), corrupt_before);
}

#[test]
fn restored_workflow_requires_new_exact_confirmation_and_keeps_prior_receipt_identity() {
    let temp = tempfile::tempdir().unwrap();
    let v1 = temp.path().join("v1.sqlite");
    let config = temp.path().join("task.json");
    initialize(&config, &v1);
    let v1_backup = temp.path().join("v1-backup.sqlite");
    assert_eq!(
        recovery(&v1, "backup", json!({"path":v1_backup}))["status"],
        "ok"
    );
    let ledger = temp.path().join("v2.sqlite");
    let migration = recovery(&v1, "migrate", migration(&ledger, &v1_backup));
    assert_eq!(migration["status"], "ok");
    let fixture: Value = serde_json::from_str(include_str!(
        "../docs/repository-memory/examples/sequence-v1.json"
    ))
    .unwrap();
    let profile = fixture["profile"].clone();
    let request = json!({"idempotency_key":"bind", "run_id":"60000000-0000-4000-8000-000000000001", "client":"codex", "workflow_id":profile["workflow_id"], "profile_revision":1, "incarnation":migration["result"]["incarnation"], "expected_profile_revision":0, "profile_sha256":format!("{:x}", Sha256::digest(profile.to_string().as_bytes())), "binding_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "profile":profile,"confirmation":"confirmed"});
    let bound = task(&config, &ledger, "workflow-bind", request.clone());
    assert_eq!(bound["status"], "accepted", "{bound}");
    let before = recovery(&ledger, "export", json!({}));
    let backup = temp.path().join("v2-backup.sqlite");
    assert_eq!(
        recovery(&ledger, "backup", json!({"path":backup}))["status"],
        "ok"
    );
    assert_eq!(recovery(&backup, "export", json!({})), before);
    let restored = temp.path().join("restored.sqlite");
    let restored_result = recovery(&restored, "restore", json!({"path":backup}));
    assert_eq!(restored_result["status"], "ok");
    let after = recovery(&restored, "export", json!({}));
    assert_eq!(after["result"]["receipts"], before["result"]["receipts"]);
    assert_eq!(
        after["result"]["workflows"][0]["state"],
        "recovery_required"
    );
    assert_eq!(
        after["result"]["source_collections"],
        before["result"]["source_collections"]
    );
    assert_ne!(
        after["result"]["ledger_meta"][0]["incarnation"],
        before["result"]["ledger_meta"][0]["incarnation"]
    );
    let replay = task(&config, &restored, "workflow-bind", request.clone());
    assert_eq!(replay["status"], "replayed", "{replay}");
    assert_eq!(recovery(&restored, "export", json!({})), after);
    let mut renewed = request;
    renewed["idempotency_key"] = json!("renewed");
    renewed["incarnation"] = restored_result["result"]["incarnation"].clone();
    renewed["expected_profile_revision"] = json!(1);
    renewed["profile_revision"] = json!(2);
    renewed["profile"]["revision"] = json!(2);
    renewed["profile_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(renewed["profile"].to_string().as_bytes())
    ));
    assert_eq!(
        task(&config, &restored, "workflow-bind", renewed.clone())["code"],
        "recovery_required"
    );
    assert_eq!(recovery(&restored, "export", json!({})), after);
    renewed["recovery_acknowledged"] = json!(true);
    let accepted = task(&config, &restored, "workflow-bind", renewed);
    assert_eq!(accepted["status"], "accepted", "{accepted}");
    let status = task(
        &config,
        &restored,
        "workflow-status",
        json!({"workflow_id":profile["workflow_id"],"view":"summary"}),
    );
    assert_eq!(status["result"]["items"][0]["state"], "active", "{status}");
}

#[test]
fn restored_publication_history_is_readable_but_cannot_authorize_a_new_effect() {
    let temp = tempfile::tempdir().unwrap();
    let v1 = temp.path().join("v1.sqlite");
    let config = temp.path().join("task.json");
    initialize(&config, &v1);
    let v1_backup = temp.path().join("v1-backup.sqlite");
    assert_eq!(
        recovery(&v1, "backup", json!({"path":v1_backup}))["status"],
        "ok"
    );
    let ledger = temp.path().join("v2.sqlite");
    let migrated = recovery(&v1, "migrate", migration(&ledger, &v1_backup));
    assert_eq!(migrated["status"], "ok");
    let fixture: Value = serde_json::from_str(include_str!(
        "../docs/repository-memory/examples/sequence-v1.json"
    ))
    .unwrap();
    let request = |name: &str| {
        let mut input = fixture["requests"][name]["input"].clone();
        input["incarnation"] = migrated["result"]["incarnation"].clone();
        input
    };
    let bound = task(&config, &ledger, "workflow-bind", request("bind"));
    assert_eq!(bound["status"], "accepted", "{bound}");
    for name in [
        "observe_incident",
        "observe_mechanism",
        "observe_correction",
    ] {
        let response = task(&config, &ledger, "source-sync", request(name));
        assert_eq!(response["status"], "accepted", "{name}: {response}");
    }
    let capture = task(&config, &ledger, "source-capture", request("capture"));
    assert_eq!(capture["status"], "accepted", "{capture}");
    let mut decision = Value::Null;
    for name in ["review_a", "review_b"] {
        let mut input = request(name);
        input["input_record_id"] = capture["result"]["record_id"].clone();
        for dep in input["dependencies"].as_array_mut().unwrap() {
            if dep["type"] == "record" {
                dep["id"] = capture["result"]["record_id"].clone();
            }
        }
        let response = task(&config, &ledger, "consumer-review", input);
        assert_eq!(response["status"], "accepted", "{name}: {response}");
        if name == "review_a" {
            decision = response;
        }
    }
    let mut approve = request("approve_a");
    approve["decision_id"] = decision["result"]["decision_id"].clone();
    approve["decision_sha256"] = decision["result"]["decision_sha256"].clone();
    approve["proposal"]["block_id"] = decision["result"]["block_id"].clone();
    let approved = task(&config, &ledger, "consumer-approve", approve.clone());
    assert_eq!(approved["status"], "accepted", "{approved}");
    let mut publish = request("publish_a");
    publish["approval_id"] = approved["result"]["approval_id"].clone();
    let published = task(&config, &ledger, "publication-result", publish.clone());
    assert_eq!(published["status"], "accepted", "{published}");
    let before = recovery(&ledger, "export", json!({}));
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
        assert!(
            !before["result"][name].as_array().unwrap().is_empty(),
            "{name}"
        );
    }
    let backup = temp.path().join("complete-backup.sqlite");
    assert_eq!(
        recovery(&ledger, "backup", json!({"path":backup}))["status"],
        "ok"
    );
    assert_eq!(recovery(&backup, "export", json!({})), before);
    let restored = temp.path().join("restored.sqlite");
    let restored_result = recovery(&restored, "restore", json!({"path":backup}));
    assert_eq!(restored_result["status"], "ok", "{restored_result}");
    let after = recovery(&restored, "export", json!({}));
    assert_eq!(after["result"]["receipts"], before["result"]["receipts"]);
    assert_eq!(
        after["result"]["publication_attempts"],
        before["result"]["publication_attempts"]
    );
    let status = task(
        &config,
        &restored,
        "workflow-status",
        json!({"workflow_id":fixture["profile"]["workflow_id"],"view":"publications"}),
    );
    assert_eq!(
        status["result"]["items"][0]["validity"], "stale",
        "{status}"
    );
    assert_eq!(status["result"]["items"][0]["publication"], "applied");
    let replay = task(&config, &restored, "publication-result", publish.clone());
    assert_eq!(replay["status"], "replayed", "{replay}");
    assert_eq!(replay["result"]["validity"], "stale", "{replay}");
    let mut retry = publish;
    retry["idempotency_key"] = json!("new-effect-after-restore");
    assert_eq!(
        task(&config, &restored, "publication-result", retry.clone())["code"],
        "recovery_required"
    );
    retry["incarnation"] = restored_result["result"]["incarnation"].clone();
    assert_eq!(
        task(&config, &restored, "publication-result", retry)["code"],
        "recovery_required"
    );
    let replayed_approval = task(&config, &restored, "consumer-approve", approve);
    assert_eq!(
        replayed_approval["status"], "replayed",
        "{replayed_approval}"
    );
    assert_eq!(
        replayed_approval["result"]["validity"], "stale",
        "{replayed_approval}"
    );
    assert_eq!(recovery(&restored, "export", json!({})), after);
    let mut normalized = after;
    normalized["result"]["ledger_meta"][0]["incarnation"] =
        before["result"]["ledger_meta"][0]["incarnation"].clone();
    for workflow in normalized["result"]["workflows"].as_array_mut().unwrap() {
        workflow["state"] = json!("active");
    }
    for table in ["consumer_decisions", "publications"] {
        for row in normalized["result"][table].as_array_mut().unwrap() {
            row["data"]["validity"] = json!("current");
        }
    }
    assert_eq!(normalized, before);
    // Renewing the workflow does not renew an old publication authorization.
    let mut renewal = request("bind");
    renewal["idempotency_key"] = json!("recovery-bind");
    renewal["incarnation"] = restored_result["result"]["incarnation"].clone();
    renewal["expected_profile_revision"] = json!(1);
    renewal["profile_revision"] = json!(2);
    renewal["profile"]["revision"] = json!(2);
    renewal["profile_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(renewal["profile"].to_string().as_bytes())
    ));
    renewal["recovery_acknowledged"] = json!(true);
    let rebound = task(&config, &restored, "workflow-bind", renewal);
    assert_eq!(rebound["status"], "accepted", "{rebound}");
    let after_rebind = recovery(&restored, "export", json!({}));
    let mut old_approval_effect = request("publish_a");
    old_approval_effect["idempotency_key"] = json!("old-approval-after-rebind");
    old_approval_effect["approval_id"] = approved["result"]["approval_id"].clone();
    old_approval_effect["incarnation"] = restored_result["result"]["incarnation"].clone();
    let rejected = task(
        &config,
        &restored,
        "publication-result",
        old_approval_effect,
    );
    assert_eq!(rejected["code"], "recovery_required", "{rejected}");
    assert_eq!(recovery(&restored, "export", json!({})), after_rebind);
    let mut neutral = request("approve_a");
    neutral["idempotency_key"] = json!("fresh-neutral-after-recovery");
    neutral["incarnation"] = restored_result["result"]["incarnation"].clone();
    neutral["profile_revision"] = json!(2);
    neutral["decision_id"] = decision["result"]["decision_id"].clone();
    neutral["decision_sha256"] = decision["result"]["decision_sha256"].clone();
    neutral.as_object_mut().unwrap().remove("confirmation");
    neutral["maintenance"] = json!("neutralize");
    neutral["proposal"]["block_id"] = decision["result"]["block_id"].clone();
    neutral["proposal"]["base_exists"] = json!(true);
    neutral["proposal"]["base_sha256"] = approved["result"]["proposal"]["after_sha256"].clone();
    neutral["proposal"]["output_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(b"MemoryNexus: source evidence changed; review required.\n")
    ));
    neutral["proposal"]["after_sha256"] =
        json!(format!("{:x}", Sha256::digest(b"rendered neutral block")));
    let fresh = task(&config, &restored, "consumer-approve", neutral.clone());
    assert_eq!(fresh["status"], "accepted", "{fresh}");
    assert_ne!(
        fresh["result"]["approval_id"],
        approved["result"]["approval_id"]
    );
    let mut effect = request("publish_a");
    effect["idempotency_key"] = json!("fresh-neutral-effect");
    effect["incarnation"] = restored_result["result"]["incarnation"].clone();
    effect["profile_revision"] = json!(2);
    effect["approval_id"] = fresh["result"]["approval_id"].clone();
    effect["observed_sha256"] = neutral["proposal"]["after_sha256"].clone();
    let result = task(&config, &restored, "publication-result", effect);
    assert_eq!(result["status"], "accepted", "{result}");
    assert_eq!(result["result"]["validity"], "stale");
}
