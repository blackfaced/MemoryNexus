use serde_json::{json, Value};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    thread,
};
use tempfile::TempDir;

fn cli(ledger: &Path, command: &str, input: Option<Value>) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_memorynexus-feedback-cli"))
        .args(["--ledger", ledger.to_str().unwrap(), command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
    }

    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

fn initial(statement: &str, key: &str) -> Value {
    json!({
        "confirmation": "confirmed",
        "kind": "initial",
        "statement": statement,
        "occurred_at": "2026-08-29T08:00:00+08:00",
        "source": "owner_report",
        "idempotency_key": key
    })
}

#[test]
fn confirmed_observation_survives_a_separate_cli_process() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("nested/ledger.sqlite");

    let accepted = cli(
        &ledger,
        "observe",
        Some(initial("午后精力比昨天稳定", "observe-1")),
    );
    assert_eq!(accepted["status"], "accepted");
    assert_eq!(accepted["observation"]["state"], "current");

    let history = cli(&ledger, "observation-history", None);
    assert_eq!(history["status"], "ok");
    assert_eq!(history["observations"].as_array().unwrap().len(), 1);
    assert_eq!(
        history["observations"][0]["statement"],
        "午后精力比昨天稳定"
    );
}

#[test]
fn correction_is_append_only_and_retraction_keeps_audit_provenance() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let first = cli(
        &ledger,
        "observe",
        Some(initial("昨晚十点半睡着", "observe-1")),
    );
    let first_id = first["observation"]["id"].as_str().unwrap();

    let correction = cli(
        &ledger,
        "observe",
        Some(json!({
            "confirmation": "confirmed",
            "kind": "correction",
            "statement": "昨晚十一点睡着",
            "occurred_at": "2026-08-29T08:00:00+08:00",
            "source": "owner_report",
            "supersedes_observation_id": first_id,
            "idempotency_key": "observe-2"
        })),
    );
    let correction_id = correction["observation"]["id"].as_str().unwrap();

    let retracted = cli(
        &ledger,
        "retract",
        Some(json!({
            "confirmation": "confirmed",
            "observation_id": correction_id,
            "reason": "我不想保留这条估计",
            "idempotency_key": "retract-1"
        })),
    );
    assert_eq!(retracted["status"], "accepted");
    assert_eq!(retracted["retraction"]["observation_id"], correction_id);

    let history = cli(&ledger, "observation-history", None);
    assert_eq!(history["observations"].as_array().unwrap().len(), 2);
    assert_eq!(history["observations"][0]["id"], first_id);
    assert_eq!(history["observations"][0]["state"], "superseded");
    assert_eq!(history["observations"][1]["id"], correction_id);
    assert_eq!(history["observations"][1]["state"], "retracted");
}

#[test]
fn invalid_or_unconfirmed_input_cannot_create_an_observation() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let unconfirmed = cli(
        &ledger,
        "observe",
        Some(json!({
            "confirmation": "draft",
            "kind": "initial",
            "statement": "还没有确认",
            "occurred_at": "2026-08-29T08:00:00+08:00",
            "source": "owner_report",
            "idempotency_key": "draft-1"
        })),
    );
    assert_eq!(unconfirmed["status"], "rejected");
    assert_eq!(unconfirmed["code"], "confirmation_required");

    let sensitive_payload = cli(
        &ledger,
        "observe",
        Some(json!({
            "confirmation": "confirmed",
            "kind": "initial",
            "statement": "摘要",
            "occurred_at": "2026-08-29T08:00:00+08:00",
            "source": "owner_report",
            "idempotency_key": "sensitive-1",
            "raw_medical_report": "not a supported ledger field"
        })),
    );
    assert_eq!(sensitive_payload["status"], "rejected");
    assert_eq!(sensitive_payload["code"], "invalid_input");

    assert!(!ledger.exists());
}

#[test]
fn retry_and_low_frequency_concurrency_do_not_duplicate_the_observation() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let first = cli(&ledger, "observe", Some(initial("上午有点困", "observe-1")));
    let replay = cli(&ledger, "observe", Some(initial("上午有点困", "observe-1")));
    assert_eq!(replay["status"], "idempotent_replay");
    assert_eq!(replay["observation"]["id"], first["observation"]["id"]);

    let concurrent_ledger = ledger.clone();
    let first_child = thread::spawn(move || {
        cli(
            &concurrent_ledger,
            "observe",
            Some(initial("下午状态平稳", "observe-2")),
        )
    });
    let concurrent_ledger = ledger.clone();
    let second_child = thread::spawn(move || {
        cli(
            &concurrent_ledger,
            "observe",
            Some(initial("下午状态平稳", "observe-2")),
        )
    });
    let statuses = [
        first_child.join().unwrap()["status"].clone(),
        second_child.join().unwrap()["status"].clone(),
    ];
    assert!(statuses.contains(&json!("accepted")));
    assert!(statuses.contains(&json!("idempotent_replay")));

    let history = cli(&ledger, "observation-history", None);
    assert_eq!(history["observations"].as_array().unwrap().len(), 2);
}
