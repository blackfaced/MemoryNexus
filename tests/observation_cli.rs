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

#[test]
fn recommendation_and_experiment_keep_one_active_action_and_historical_review() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let recommendation = cli(
        &ledger,
        "add-recommendation",
        Some(json!({
            "confirmation":"confirmed", "summary":"连续七天把睡前屏幕时间缩短三十分钟", "source":"ant_afu", "idempotency_key":"recommendation-1"
        })),
    );
    assert_eq!(recommendation["status"], "accepted");
    let recommendation_id = recommendation["recommendation"]["id"].as_str().unwrap();
    let experiment = json!({
        "confirmation":"confirmed", "recommendation_id":recommendation_id, "action":"晚上十点后不看手机", "starts_at":"2026-08-29T22:00:00+08:00", "ends_at":"2026-09-05T22:00:00+08:00", "expected_signal":"入睡时间更稳定", "idempotency_key":"experiment-1"
    });
    let started = cli(&ledger, "start-experiment", Some(experiment));
    assert_eq!(started["status"], "accepted");
    let second = cli(
        &ledger,
        "start-experiment",
        Some(json!({
            "confirmation":"confirmed", "recommendation_id":recommendation_id, "action":"改为早起", "starts_at":"2026-08-30T08:00:00+08:00", "ends_at":"2026-09-06T08:00:00+08:00", "expected_signal":"精力更稳定", "idempotency_key":"experiment-2"
        })),
    );
    assert_eq!(second["code"], "conflict");
    let experiment_id = started["experiment"]["id"].as_str().unwrap();
    assert_eq!(
        cli(
            &ledger,
            "end-experiment",
            Some(
                json!({"confirmation":"confirmed","experiment_id":experiment_id,"state":"completed","idempotency_key":"end-1"})
            )
        )["status"],
        "accepted"
    );
    let review = cli(&ledger, "review", None);
    assert_eq!(review["experiments"][0]["state"], "completed");
    assert_eq!(
        review["experiments"][0]["recommendation"]["source"],
        "ant_afu"
    );
}

#[test]
fn confirmed_outcomes_are_append_only_and_review_keeps_evidence_gaps_explicit() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let recommendation = cli(
        &ledger,
        "add-recommendation",
        Some(
            json!({"confirmation":"confirmed","summary":"睡前少看屏幕","source":"owner","idempotency_key":"rec-outcome"}),
        ),
    );
    let experiment = cli(
        &ledger,
        "start-experiment",
        Some(
            json!({"confirmation":"confirmed","recommendation_id":recommendation["recommendation"]["id"],"action":"十点后不看手机","starts_at":"2026-08-29T22:00:00+08:00","ends_at":"2026-09-05T22:00:00+08:00","expected_signal":"入睡稳定","idempotency_key":"exp-outcome"}),
        ),
    );
    let first = cli(
        &ledger,
        "record-outcome",
        Some(
            json!({"confirmation":"confirmed","experiment_id":experiment["experiment"]["id"],"occurred_at":"2026-08-30T08:00:00+08:00","execution_state":"skipped","evaluation":"unclear","note":"昨晚没有做到","idempotency_key":"outcome-1"}),
        ),
    );
    assert_eq!(first["status"], "accepted");
    let correction = cli(
        &ledger,
        "record-outcome",
        Some(
            json!({"confirmation":"confirmed","experiment_id":experiment["experiment"]["id"],"occurred_at":"2026-08-30T08:00:00+08:00","execution_state":"performed","evaluation":"improved","note":"实际做到了，入睡更快","supersedes_outcome_id":first["outcome"]["id"],"idempotency_key":"outcome-2"}),
        ),
    );
    assert_eq!(correction["status"], "accepted");
    let review = cli(&ledger, "review", None);
    assert_eq!(review["outcomes"].as_array().unwrap().len(), 2);
    assert_eq!(review["outcomes"][1]["execution_state"], "performed");
    assert!(review["evidence_gaps"].as_array().unwrap().is_empty());
}

#[test]
fn due_is_clock_controlled_read_only_and_follows_active_experiment() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let now = json!({"now":"2026-08-30T08:00:00+08:00"});
    let first = cli(
        &ledger,
        "observe",
        Some(initial("今天状态平稳", "due-observation")),
    );
    assert_eq!(first["status"], "accepted");
    let daily = cli(&ledger, "due", Some(now.clone()));
    assert_eq!(daily["status"], "daily_observation_check_in");
    let history_before = cli(&ledger, "observation-history", None);
    let history_after = cli(&ledger, "due", Some(now.clone()));
    assert_eq!(history_after["read_only"], true);
    assert_eq!(cli(&ledger, "observation-history", None), history_before);
    let recommendation = cli(
        &ledger,
        "add-recommendation",
        Some(
            json!({"confirmation":"confirmed","summary":"睡前少看屏幕","source":"owner","idempotency_key":"due-rec"}),
        ),
    );
    let experiment = cli(
        &ledger,
        "start-experiment",
        Some(
            json!({"confirmation":"confirmed","recommendation_id":recommendation["recommendation"]["id"],"action":"十点后不看手机","starts_at":"2026-08-29T22:00:00+08:00","ends_at":"2026-09-05T22:00:00+08:00","expected_signal":"入睡稳定","idempotency_key":"due-exp"}),
        ),
    );
    let follow_up = cli(&ledger, "due", Some(now));
    assert_eq!(follow_up["status"], "active_experiment_follow_up");
    assert_eq!(follow_up["experiment_id"], experiment["experiment"]["id"]);
}

#[test]
fn due_distinguishes_no_check_in_and_completed_experiment_review() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let now = json!({"now":"2026-08-30T08:00:00+08:00"});

    let observed_today = cli(
        &ledger,
        "observe",
        Some(json!({
            "confirmation": "confirmed",
            "kind": "initial",
            "statement": "今天状态平稳",
            "occurred_at": "2026-08-30T07:00:00+08:00",
            "source": "owner_report",
            "idempotency_key": "due-today-observation"
        })),
    );
    assert_eq!(observed_today["status"], "accepted");
    assert_eq!(
        cli(&ledger, "due", Some(now.clone()))["status"],
        "no_check_in_due"
    );

    let recommendation = cli(
        &ledger,
        "add-recommendation",
        Some(json!({
            "confirmation":"confirmed", "summary":"睡前少看屏幕", "source":"owner", "idempotency_key":"due-review-rec"
        })),
    );
    let experiment = cli(
        &ledger,
        "start-experiment",
        Some(json!({
            "confirmation":"confirmed", "recommendation_id":recommendation["recommendation"]["id"], "action":"十点后不看手机", "starts_at":"2026-08-29T22:00:00+08:00", "ends_at":"2026-08-30T07:00:00+08:00", "expected_signal":"入睡稳定", "idempotency_key":"due-review-exp"
        })),
    );
    assert_eq!(
        cli(
            &ledger,
            "end-experiment",
            Some(
                json!({"confirmation":"confirmed","experiment_id":experiment["experiment"]["id"],"state":"completed","idempotency_key":"due-review-end"})
            )
        )["status"],
        "accepted"
    );

    let review_due = cli(&ledger, "due", Some(now));
    assert_eq!(review_due["status"], "review_due");
    assert_eq!(review_due["read_only"], true);
}

#[test]
fn export_backup_and_restore_preserve_public_ledger_state() {
    let directory = TempDir::new().unwrap();
    let ledger = directory.path().join("ledger.sqlite");
    let accepted = cli(
        &ledger,
        "observe",
        Some(initial("午后精力稳定", "recovery-1")),
    );
    assert_eq!(accepted["status"], "accepted");
    let retracted = cli(
        &ledger,
        "retract",
        Some(json!({
            "confirmation":"confirmed", "observation_id":accepted["observation"]["id"],
            "reason":"测试导出撤回审计", "idempotency_key":"recovery-retract"
        })),
    );
    assert_eq!(retracted["status"], "accepted");
    let exported = cli(&ledger, "export", None);
    assert_eq!(exported["format"], "memorynexus-feedback-ledger");
    assert_eq!(exported["version"], 1);
    assert_eq!(exported["observations"][0]["statement"], "午后精力稳定");
    assert_eq!(
        exported["observation_retractions"][0]["observation_id"],
        accepted["observation"]["id"]
    );
    assert!(exported["observations"][0].get("request_json").is_none());
    let backup = directory.path().join("backup.sqlite");
    assert_eq!(
        cli(&ledger, "backup", Some(json!({"path":backup})))["status"],
        "ok"
    );
    let restored = directory.path().join("restored.sqlite");
    assert_eq!(
        cli(&restored, "restore", Some(json!({"path":backup})))["status"],
        "ok"
    );
    assert_eq!(
        cli(&restored, "observation-history", None),
        cli(&ledger, "observation-history", None)
    );
}
