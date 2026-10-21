use serde_json::{json, Value};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn invoke(args: &[&str], ledger: Option<&Path>, input: &str) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_memorynexus-feedback-cli"));
    command.args(args).env_remove("MEMORYNEXUS_TASK_LEDGER");
    if let Some(ledger) = ledger {
        command.env("MEMORYNEXUS_TASK_LEDGER", ledger);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Err(error) = child.stdin.take().unwrap().write_all(input.as_bytes()) {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    let output = child.wait_with_output().unwrap();
    let response: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("non-JSON output: {:?}", output));
    assert_eq!(
        output.status.code(),
        Some(if response["status"] == "error" { 2 } else { 0 }),
        "{response}"
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected diagnostic: {:?}",
        output.stderr
    );
    response
}

fn task(config: &Path, ledger: &Path, action: &str, input: Value) -> Value {
    invoke(
        &["task", "--config", config.to_str().unwrap(), action],
        Some(ledger),
        &input.to_string(),
    )
}

#[test]
fn help_identifies_available_task_commands_without_a_ledger() {
    let response = invoke(&["task", "--help"], None, "");
    assert_eq!(response["status"], "ok");
    assert_eq!(response["schema_version"], 1);
    assert_eq!(response["result"]["command"], "task");
    assert_eq!(
        response["result"]["commands"],
        json!(["init", "record", "context", "review", "correct", "retract"])
    );
}

fn init_input(key: &str) -> Value {
    json!({"idempotency_key":key,"display_name":"视频任务","confirmation":"confirmed","automatic_capture":["execution","review"]})
}

#[test]
fn initialization_reuses_confirmed_identity_and_survives_directory_moves() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join(".memorynexus.json");
    let first = task(&config, &ledger, "init", init_input("init-a"));
    assert_eq!(first["status"], "accepted");
    let binding: Value = serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    assert_eq!(
        binding,
        json!({"schema_version":1,"task_id":first["task_id"],"ledger_env":"MEMORYNEXUS_TASK_LEDGER"})
    );
    let replay = task(&config, &ledger, "init", init_input("init-a"));
    assert_eq!(replay["status"], "replayed");
    assert_eq!(replay["task_id"], first["task_id"]);
    let moved = temp.path().join("moved.json");
    std::fs::rename(&config, &moved).unwrap();
    let same = task(&moved, &ledger, "init", init_input("init-a"));
    assert_eq!(same["task_id"], first["task_id"]);
    let other = task(&config, &ledger, "init", init_input("init-b"));
    assert_eq!(other["status"], "accepted");
    assert_ne!(other["task_id"], first["task_id"]);
}

fn record_input(key: &str, kind: &str, text: &str) -> Value {
    json!({"idempotency_key":key,"run_id":"ab11b084-e0c6-4d9e-a2d2-815f004a1d99","client":"minimax","kind":kind,"text":text,"occurred_at":"2026-10-21T10:00:00+08:00"})
}

fn review_input(key: &str, sources: Value, text: &str) -> Value {
    json!({"idempotency_key":key,"run_id":"ab11b084-e0c6-4d9e-a2d2-815f004a1d99","client":"minimax","text":text,"source_ids":sources,"occurred_at":"2026-10-21T10:00:00+08:00"})
}

#[test]
fn sourced_review_is_a_separate_candidate_even_when_retention_is_confirmed() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("init"));
    let source = task(
        &config,
        &ledger,
        "record",
        record_input("source", "execution", "拍摄前检查电量"),
    );
    let mut request = review_input(
        "review",
        json!([source["result"]["record_id"]]),
        "下次拍摄前确认电量",
    );
    request["confirmation"] = json!("confirmed");
    let review = task(&config, &ledger, "review", request.clone());
    assert_eq!(review["status"], "accepted", "{review}");
    let replay = task(&config, &ledger, "review", request.clone());
    assert_eq!(replay["status"], "replayed");
    assert_eq!(replay["result"]["record_id"], review["result"]["record_id"]);
    let context = task(&config, &ledger, "context", json!({}));
    assert_eq!(context["revision"], review["revision"]);
    assert_eq!(context["result"]["entries"].as_array().unwrap().len(), 1);
    let candidates = context["result"]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(context["result"]["candidate_count"], 1);
    assert_eq!(candidates[0]["kind"], "lesson");
    assert_eq!(candidates[0]["trust"], "review_candidate");
    assert_eq!(candidates[0]["text"], request["text"]);
    assert_eq!(candidates[0]["source_ids"], request["source_ids"]);
    assert_eq!(candidates[0]["id"], review["result"]["record_id"]);
    assert_eq!(task(&config, &ledger, "context", json!({})), context);
}

fn correct_input(key: &str, target: &Value, text: &str) -> Value {
    json!({"idempotency_key":key,"run_id":"ab11b084-e0c6-4d9e-a2d2-815f004a1d99","client":"codex","target_id":target,"text":text,"occurred_at":"2026-10-21T10:30:00+08:00","confirmation":"confirmed"})
}

#[test]
fn owner_correction_replaces_source_and_invalidates_candidates_in_fresh_context() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("init"));
    let source_request = record_input("source", "execution", "拍摄完成");
    let source = task(&config, &ledger, "record", source_request.clone());
    let source_id = &source["result"]["record_id"];
    let review_request = review_input("review", json!([source_id]), "现在可以剪辑");
    let review = task(&config, &ledger, "review", review_request.clone());
    let review_id = &review["result"]["record_id"];
    let request = correct_input("correct", source_id, "仅拍摄完成第一段");
    let corrected = task(&config, &ledger, "correct", request.clone());
    assert_eq!(corrected["status"], "accepted", "{corrected}");
    assert_ne!(corrected["result"]["record_id"], *source_id);
    let context = task(&config, &ledger, "context", json!({}));
    assert_eq!(context["revision"], corrected["revision"]);
    assert_eq!(context["result"]["candidates"], json!([]));
    let entries = context["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["text"], request["text"]);
    assert_eq!(entries[0]["kind"], "execution");
    assert_eq!(entries[0]["trust"], "owner_confirmed");
    assert_eq!(entries[0]["supersedes_id"], *source_id);
    let old = task(
        &config,
        &ledger,
        "context",
        json!({"record_ids":[source_id, review_id]}),
    );
    assert_eq!(old["result"]["entries"][0]["state"], "superseded");
    assert_eq!(old["result"]["candidates"][0]["state"], "stale");
    assert!(old["result"]["candidates"][0].get("text").is_none());
    let replay = task(&config, &ledger, "correct", request);
    assert_eq!(replay["status"], "replayed");
    assert_eq!(
        replay["result"]["record_id"],
        corrected["result"]["record_id"]
    );
    let review_replay = task(&config, &ledger, "review", review_request);
    assert_eq!(review_replay["status"], "replayed");
    assert_eq!(review_replay["result"]["state"], "stale");
    assert!(review_replay["result"].get("text").is_none());
    let competing = task(
        &config,
        &ledger,
        "correct",
        correct_input("competing", source_id, "全部拍完"),
    );
    assert_eq!(competing["code"], "conflict");
    assert_eq!(task(&config, &ledger, "context", json!({})), context);
    let fresh = task(
        &config,
        &ledger,
        "review",
        review_input(
            "fresh",
            json!([corrected["result"]["record_id"]]),
            "先补拍剩余片段",
        ),
    );
    assert_eq!(fresh["status"], "accepted");
    assert_eq!(
        task(&config, &ledger, "context", json!({}))["result"]["candidates"][0]["id"],
        fresh["result"]["record_id"]
    );
}

#[test]
fn a_new_checkpoint_invalidates_both_candidates_and_owner_endorsed_lessons() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("init"));
    let source = task(
        &config,
        &ledger,
        "record",
        record_input("source", "checkpoint", "目前完成两段"),
    );
    let source_id = &source["result"]["record_id"];
    let candidate = task(
        &config,
        &ledger,
        "review",
        review_input("candidate", json!([source_id]), "还需要补拍"),
    );
    let reviewed = task(
        &config,
        &ledger,
        "review",
        review_input("review", json!([source_id]), "完成前保留原片"),
    );
    let endorsement = correct_input(
        "endorse",
        &reviewed["result"]["record_id"],
        "完成前保留原片",
    );
    let endorsed = task(&config, &ledger, "correct", endorsement.clone());
    assert_eq!(endorsed["status"], "accepted", "{endorsed}");
    let before = task(&config, &ledger, "context", json!({}));
    assert_eq!(
        before["result"]["entries"][0]["id"],
        endorsed["result"]["record_id"]
    );
    assert_eq!(before["result"]["entries"][0]["kind"], "lesson");
    assert_eq!(before["result"]["entries"][0]["trust"], "owner_confirmed");
    assert_eq!(
        before["result"]["entries"][0]["source_ids"],
        json!([source_id])
    );
    assert_eq!(before["result"]["candidate_count"], 1);
    let next = task(
        &config,
        &ledger,
        "record",
        record_input("next", "checkpoint", "全部片段已完成"),
    );
    let after = task(&config, &ledger, "context", json!({}));
    assert_eq!(after["revision"], next["revision"]);
    assert_eq!(after["result"]["candidates"], json!([]));
    assert_eq!(after["result"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        after["result"]["entries"][0]["id"],
        next["result"]["record_id"]
    );
    let stale = task(
        &config,
        &ledger,
        "context",
        json!({"record_ids":[candidate["result"]["record_id"],endorsed["result"]["record_id"]]}),
    );
    assert_eq!(stale["result"]["candidates"][0]["state"], "stale");
    assert_eq!(stale["result"]["entries"][0]["state"], "stale");
    assert!(stale["result"]["entries"][0].get("text").is_none());
    let replay = task(&config, &ledger, "correct", endorsement);
    assert_eq!(replay["status"], "replayed");
    assert_eq!(replay["result"]["state"], "stale");
    let mut cannot_revive = correct_input("revive", &endorsed["result"]["record_id"], "保留原片");
    cannot_revive["source_ids"] = json!([next["result"]["record_id"]]);
    assert_eq!(
        task(&config, &ledger, "correct", cannot_revive)["code"],
        "conflict"
    );
}

fn retract_input(key: &str, target: &Value) -> Value {
    json!({"idempotency_key":key,"run_id":"ab11b084-e0c6-4d9e-a2d2-815f004a1d99","client":"codex","target_id":target,"reason":"owner requests withdrawal","confirmation":"confirmed"})
}

#[test]
fn retracting_an_older_revision_removes_the_entire_chain_and_replays_only_identity() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("init"));
    let mut original_request = record_input("original", "execution", "original withdrawn text");
    original_request["evidence_refs"] = json!([{"path":"notes/withdrawn-original.txt"}]);
    let original = task(&config, &ledger, "record", original_request.clone());
    let original_id = &original["result"]["record_id"];
    let correction_request = correct_input("correct", original_id, "corrected withdrawn text");
    let corrected = task(&config, &ledger, "correct", correction_request.clone());
    let corrected_id = &corrected["result"]["record_id"];
    let mut last_request = correct_input("correct-again", corrected_id, "latest withdrawn text");
    last_request["evidence_refs"] = json!([{"path":"notes/withdrawn-latest.txt"}]);
    let latest = task(&config, &ledger, "correct", last_request.clone());
    let latest_id = &latest["result"]["record_id"];
    let review_request = review_input(
        "review",
        json!([latest_id]),
        "candidate from latest evidence",
    );
    let candidate = task(&config, &ledger, "review", review_request.clone());
    let request = retract_input("withdraw", original_id);
    let withdrawn = task(&config, &ledger, "retract", request.clone());
    assert_eq!(withdrawn["status"], "accepted", "{withdrawn}");
    assert_eq!(withdrawn["result"]["state"], "retracted");
    let current = task(&config, &ledger, "context", json!({}));
    assert_eq!(current["revision"], withdrawn["revision"]);
    assert_eq!(current["result"]["entries"], json!([]));
    assert_eq!(current["result"]["candidates"], json!([]));
    let chain = task(
        &config,
        &ledger,
        "context",
        json!({"record_ids":[original_id,corrected_id,latest_id,candidate["result"]["record_id"]]}),
    );
    let entries = chain["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    for entry in entries {
        assert_eq!(entry["state"], "retracted");
        assert!(entry.get("text").is_none());
        assert!(entry.get("evidence_refs").is_none());
    }
    assert_eq!(chain["result"]["candidates"][0]["state"], "stale");
    assert!(chain["result"]["candidates"][0].get("text").is_none());
    for (action, envelope, expected_id, state) in [
        ("record", original_request, original_id, "retracted"),
        ("correct", correction_request, corrected_id, "retracted"),
        ("correct", last_request, latest_id, "retracted"),
        (
            "review",
            review_request,
            &candidate["result"]["record_id"],
            "stale",
        ),
        ("retract", request, original_id, "retracted"),
    ] {
        let replay = task(&config, &ledger, action, envelope);
        assert_eq!(replay["status"], "replayed", "{replay}");
        assert_eq!(replay["result"]["record_id"], *expected_id);
        assert_eq!(replay["result"]["state"], state);
        assert!(!replay.to_string().contains("withdrawn"));
        assert!(replay["result"].get("text").is_none());
    }
    assert_eq!(task(&config, &ledger, "context", json!({})), current);
}

#[test]
fn review_requires_authorization_and_current_same_task_non_lesson_sources() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    let foreign_config = temp.path().join("foreign.json");
    let mut policy = init_input("init");
    policy["automatic_capture"] = json!(["execution"]);
    task(&config, &ledger, "init", policy);
    task(&foreign_config, &ledger, "init", init_input("foreign"));
    let source = task(
        &config,
        &ledger,
        "record",
        record_input("source", "execution", "结果记录"),
    );
    let source_id = &source["result"]["record_id"];
    let mut request = review_input("review", json!([source_id]), "候选经验");
    let before = task(&config, &ledger, "context", json!({}));
    assert_eq!(
        task(&config, &ledger, "review", request.clone())["code"],
        "invalid_input"
    );
    assert_eq!(task(&config, &ledger, "context", json!({})), before);
    request["confirmation"] = json!("confirmed");
    let candidate = task(&config, &ledger, "review", request.clone());
    assert_eq!(candidate["status"], "accepted");
    assert_eq!(
        task(&config, &ledger, "context", json!({}))["result"]["candidates"][0]["trust"],
        "review_candidate"
    );
    let foreign = task(
        &foreign_config,
        &ledger,
        "record",
        record_input("source", "execution", "其他任务记录"),
    );
    let old_checkpoint = task(
        &config,
        &ledger,
        "record",
        record_input("checkpoint", "checkpoint", "旧进度"),
    );
    task(
        &config,
        &ledger,
        "record",
        record_input("new-checkpoint", "checkpoint", "新进度"),
    );
    let old_execution = task(
        &config,
        &ledger,
        "record",
        record_input("old-execution", "execution", "错误报告"),
    );
    task(
        &config,
        &ledger,
        "correct",
        correct_input("replace", &old_execution["result"]["record_id"], "正确报告"),
    );
    let withdrawn = task(
        &config,
        &ledger,
        "record",
        record_input("withdrawn", "execution", "撤回报告"),
    );
    task(
        &config,
        &ledger,
        "retract",
        retract_input("withdraw", &withdrawn["result"]["record_id"]),
    );
    let before = task(&config, &ledger, "context", json!({}));
    for (sources, code) in [
        (json!([]), "invalid_input"),
        (json!([source_id, source_id]), "invalid_input"),
        (json!(["not-a-uuid"]), "invalid_input"),
        (
            json!((0..21)
                .map(|_| uuid::Uuid::new_v4().to_string())
                .collect::<Vec<_>>()),
            "invalid_input",
        ),
        (json!([uuid::Uuid::new_v4().to_string()]), "not_found"),
        (json!([foreign["result"]["record_id"]]), "scope_mismatch"),
        (json!([candidate["result"]["record_id"]]), "invalid_input"),
        (json!([old_checkpoint["result"]["record_id"]]), "conflict"),
        (json!([old_execution["result"]["record_id"]]), "conflict"),
        (json!([withdrawn["result"]["record_id"]]), "conflict"),
    ] {
        let mut invalid = request.clone();
        invalid["idempotency_key"] = json!("reusable-failed-key");
        invalid["source_ids"] = sources;
        let rejected = task(&config, &ledger, "review", invalid);
        assert_eq!(rejected["code"], code, "{rejected}");
        assert_eq!(task(&config, &ledger, "context", json!({})), before);
    }
    request["idempotency_key"] = json!("reusable-failed-key");
    assert_eq!(
        task(&config, &ledger, "review", request)["status"],
        "accepted"
    );
}

#[test]
fn correcting_a_lesson_requires_valid_evidence_and_preserves_replacement_dependencies() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    let foreign_config = temp.path().join("foreign.json");
    task(&config, &ledger, "init", init_input("init"));
    task(&foreign_config, &ledger, "init", init_input("foreign"));
    let source = task(
        &config,
        &ledger,
        "record",
        record_input("source", "execution", "原始报告"),
    );
    let replacement = task(
        &config,
        &ledger,
        "record",
        record_input("replacement", "execution", "替代依据"),
    );
    let foreign = task(
        &foreign_config,
        &ledger,
        "record",
        record_input("foreign", "execution", "另一任务"),
    );
    let historical = task(
        &config,
        &ledger,
        "record",
        record_input("historical", "checkpoint", "旧进度"),
    );
    task(
        &config,
        &ledger,
        "record",
        record_input("next", "checkpoint", "新进度"),
    );
    let candidate = task(
        &config,
        &ledger,
        "review",
        review_input("review", json!([source["result"]["record_id"]]), "候选建议"),
    );
    let candidate_id = &candidate["result"]["record_id"];
    let before = task(&config, &ledger, "context", json!({}));
    let mut request = correct_input("correct", candidate_id, "owner 认可的经验");
    for (sources, code) in [
        (json!([]), "invalid_input"),
        (json!([candidate_id]), "invalid_input"),
        (json!([foreign["result"]["record_id"]]), "scope_mismatch"),
        (json!([historical["result"]["record_id"]]), "conflict"),
        (json!([uuid::Uuid::new_v4().to_string()]), "not_found"),
    ] {
        request["source_ids"] = sources;
        assert_eq!(
            task(&config, &ledger, "correct", request.clone())["code"],
            code
        );
        assert_eq!(task(&config, &ledger, "context", json!({})), before);
    }
    request["source_ids"] = json!([replacement["result"]["record_id"]]);
    request["evidence_refs"] = json!([{"path":"notes/owner-confirmed.txt"}]);
    let endorsed = task(&config, &ledger, "correct", request.clone());
    assert_eq!(endorsed["status"], "accepted", "{endorsed}");
    let context = task(&config, &ledger, "context", json!({}));
    assert_eq!(
        context["result"]["entries"][0]["id"],
        endorsed["result"]["record_id"]
    );
    assert_eq!(
        context["result"]["entries"][0]["source_ids"],
        request["source_ids"]
    );
    assert_eq!(
        context["result"]["entries"][0]["evidence_refs"],
        request["evidence_refs"]
    );
    let mut base_correction = correct_input("base", &source["result"]["record_id"], "报告订正");
    base_correction["source_ids"] = request["source_ids"].clone();
    assert_eq!(
        task(&config, &ledger, "correct", base_correction.clone())["code"],
        "invalid_input"
    );
    base_correction["source_ids"] = json!([]);
    assert_eq!(
        task(&config, &ledger, "correct", base_correction)["status"],
        "accepted"
    );
    assert_eq!(
        task(&config, &ledger, "context", json!({}))["result"]["entries"][0]["id"],
        endorsed["result"]["record_id"]
    );
    task(
        &config,
        &ledger,
        "retract",
        retract_input("retract-replacement", &replacement["result"]["record_id"]),
    );
    let stale = task(
        &config,
        &ledger,
        "context",
        json!({"record_ids":[endorsed["result"]["record_id"]]}),
    );
    assert_eq!(stale["result"]["entries"][0]["state"], "stale");
    assert!(stale["result"]["entries"][0].get("text").is_none());
    assert_eq!(
        task(&config, &ledger, "correct", request)["status"],
        "replayed"
    );
}

#[test]
fn concurrent_reviews_replay_once_and_competing_corrections_have_one_current_winner() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("init"));
    let source = task(
        &config,
        &ledger,
        "record",
        record_input("source", "execution", "初始执行报告"),
    );
    let source_id = &source["result"]["record_id"];
    let request = review_input("review-race", json!([source_id]), "待确认经验");
    let barrier = std::sync::Barrier::new(2);
    let reviews = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    task(&config, &ledger, "review", request.clone())
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        reviews
            .iter()
            .filter(|response| response["status"] == "accepted")
            .count(),
        1
    );
    assert_eq!(
        reviews
            .iter()
            .filter(|response| response["status"] == "replayed")
            .count(),
        1
    );
    assert_eq!(
        reviews[0]["result"]["record_id"],
        reviews[1]["result"]["record_id"]
    );
    let before = task(&config, &ledger, "context", json!({}));
    for (action, envelope) in [
        (
            "review",
            review_input("source", json!([source_id]), "跨动作不能复用 key"),
        ),
        ("correct", correct_input("source", source_id, "修改报告")),
        ("retract", retract_input("source", source_id)),
    ] {
        assert_eq!(task(&config, &ledger, action, envelope)["code"], "conflict");
        assert_eq!(task(&config, &ledger, "context", json!({})), before);
    }
    let mut changed_envelope = request.clone();
    changed_envelope["client"] = json!("codex");
    assert_eq!(
        task(&config, &ledger, "review", changed_envelope)["code"],
        "conflict"
    );
    let correction_requests = [
        correct_input("correction-a", source_id, "owner 订正 A"),
        correct_input("correction-b", source_id, "owner 订正 B"),
    ];
    let corrections = std::thread::scope(|scope| {
        let handles: Vec<_> = correction_requests
            .iter()
            .map(|request| {
                scope.spawn(|| {
                    barrier.wait();
                    task(&config, &ledger, "correct", request.clone())
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        corrections
            .iter()
            .filter(|response| response["status"] == "accepted")
            .count(),
        1
    );
    assert_eq!(
        corrections
            .iter()
            .filter(|response| response["code"] == "conflict")
            .count(),
        1
    );
    let winner = corrections
        .iter()
        .position(|response| response["status"] == "accepted")
        .unwrap();
    let current = task(&config, &ledger, "context", json!({}));
    assert_eq!(current["result"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        current["result"]["entries"][0]["text"],
        correction_requests[winner]["text"]
    );
    assert_eq!(current["result"]["candidates"], json!([]));
    assert_eq!(
        current["revision"].as_i64().unwrap(),
        before["revision"].as_i64().unwrap() + 1
    );
    let replay = task(
        &config,
        &ledger,
        "correct",
        correction_requests[winner].clone(),
    );
    assert_eq!(replay["status"], "replayed");
    assert_eq!(
        replay["result"]["record_id"],
        corrections[winner]["result"]["record_id"]
    );
    assert_eq!(
        task(&config, &ledger, "review", request)["result"]["state"],
        "stale"
    );
    assert_eq!(task(&config, &ledger, "context", json!({})), current);
}

#[test]
fn lifecycle_rejects_unconfirmed_malformed_or_foreign_mutations_without_changing_context() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    let other_config = temp.path().join("other.json");
    task(&config, &ledger, "init", init_input("init"));
    task(&other_config, &ledger, "init", init_input("other"));
    let source = task(
        &config,
        &ledger,
        "record",
        record_input("source", "execution", "valid source"),
    );
    let foreign = task(
        &other_config,
        &ledger,
        "record",
        record_input("foreign", "execution", "foreign source"),
    );
    let id = &source["result"]["record_id"];
    let before = task(&config, &ledger, "context", json!({}));
    for (action, request) in [
        (
            "review",
            review_input("review", json!([id]), "sensitive-request-sentinel"),
        ),
        (
            "correct",
            correct_input("correct", id, "sensitive-request-sentinel"),
        ),
        ("retract", retract_input("retract", id)),
    ] {
        for (field, value) in [
            ("unrecognized", json!("sensitive-request-sentinel")),
            ("idempotency_key", json!("")),
            ("idempotency_key", json!("k".repeat(129))),
            ("run_id", json!("AB11B084-E0C6-4D9E-A2D2-815F004A1D99")),
            ("client", json!("c".repeat(65))),
            ("confirmation", json!("automatic")),
            ("confirmation", Value::Null),
        ] {
            let mut invalid = request.clone();
            invalid[field] = value;
            let rejected = task(&config, &ledger, action, invalid);
            assert_eq!(rejected["code"], "invalid_input", "{rejected}");
            assert!(!rejected.to_string().contains("sensitive-request-sentinel"));
            assert_eq!(task(&config, &ledger, "context", json!({})), before);
        }
        if action != "review" {
            let mut no_confirmation = request.clone();
            no_confirmation
                .as_object_mut()
                .unwrap()
                .remove("confirmation");
            assert_eq!(
                task(&config, &ledger, action, no_confirmation)["code"],
                "invalid_input"
            );
            for (target, code) in [
                (foreign["result"]["record_id"].clone(), "scope_mismatch"),
                (json!(uuid::Uuid::new_v4().to_string()), "not_found"),
            ] {
                let mut wrong_target = request.clone();
                wrong_target["target_id"] = target;
                assert_eq!(task(&config, &ledger, action, wrong_target)["code"], code);
                assert_eq!(task(&config, &ledger, "context", json!({})), before);
            }
        }
        let text_field = if action == "retract" {
            "reason"
        } else {
            "text"
        };
        let max = if action == "retract" { 240 } else { 1000 };
        for text in [String::new(), " ".to_string(), "字".repeat(max + 1)] {
            let mut invalid = request.clone();
            invalid[text_field] = json!(text);
            assert_eq!(
                task(&config, &ledger, action, invalid)["code"],
                "invalid_input"
            );
            assert_eq!(task(&config, &ledger, "context", json!({})), before);
        }
    }
    let mut correction = correct_input("correct", id, "replacement");
    correction["evidence_refs"] = json!([{"path":"../private.txt"}]);
    assert_eq!(
        task(&config, &ledger, "correct", correction)["code"],
        "invalid_input"
    );
    for (action, mut request) in [
        ("review", review_input("review", json!([id]), "candidate")),
        ("correct", correct_input("correct", id, "replacement")),
    ] {
        request["occurred_at"] = json!("2026-10-21T10:00:00");
        assert_eq!(
            task(&config, &ledger, action, request)["code"],
            "invalid_input"
        );
    }
    assert_eq!(task(&config, &ledger, "context", json!({})), before);
    let mut valid = correct_input("correct", id, &"字".repeat(1000));
    valid["evidence_refs"] = json!([]);
    assert_eq!(
        task(&config, &ledger, "correct", valid)["status"],
        "accepted"
    );
}

#[test]
fn candidate_context_shares_entry_and_unicode_budgets_with_evidence() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("init"));
    let source = task(
        &config,
        &ledger,
        "record",
        record_input("source", "execution", "reported execution"),
    );
    let mut candidate_ids = Vec::new();
    for index in 0..23 {
        let mut request = review_input(
            &format!("review-{index}"),
            json!([source["result"]["record_id"]]),
            &format!("候选 {index}"),
        );
        request["occurred_at"] = json!("2020-01-01T00:00:00Z");
        let response = task(&config, &ledger, "review", request);
        assert_eq!(response["status"], "accepted");
        candidate_ids.push(response["result"]["record_id"].clone());
    }
    let context = task(&config, &ledger, "context", json!({}));
    assert_eq!(context["result"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        context["result"]["candidates"].as_array().unwrap().len(),
        19
    );
    assert_eq!(context["result"]["candidates"][0]["id"], candidate_ids[22]);
    assert_eq!(context["result"]["candidate_count"], 23);
    assert_eq!(context["result"]["omitted_count"], 4);
    assert_eq!(context["result"]["truncated"], true);
    let limited = task(&config, &ledger, "context", json!({"limit":2}));
    assert_eq!(limited["result"]["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(limited["result"]["candidate_count"], 23);
    assert_eq!(limited["result"]["omitted_count"], 22);
    assert_eq!(task(&config, &ledger, "context", json!({})), context);

    let wide_config = temp.path().join("wide.json");
    let mut init = init_input("wide");
    init["display_name"] = json!("名".repeat(120));
    task(&wide_config, &ledger, "init", init);
    let source = task(
        &wide_config,
        &ledger,
        "record",
        record_input("source", "execution", &"证".repeat(1000)),
    );
    for index in 0..12 {
        let response = task(
            &wide_config,
            &ledger,
            "review",
            review_input(
                &format!("review-{index}"),
                json!([source["result"]["record_id"]]),
                &"候".repeat(1000),
            ),
        );
        assert_eq!(response["status"], "accepted");
    }
    let wide = task(&wide_config, &ledger, "context", json!({}));
    assert_eq!(wide["result"]["entries"].as_array().unwrap().len(), 1);
    let candidates = wide["result"]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 10);
    assert!(candidates
        .iter()
        .all(|entry| entry["text"].as_str().unwrap().chars().count() == 1000));
    assert_eq!(wide["result"]["candidate_count"], 12);
    assert_eq!(wide["result"]["omitted_count"], 2);
}

#[test]
fn separate_clients_read_shared_bounded_records_with_reported_provenance() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join(".memorynexus.json");
    let initialized = task(&config, &ledger, "init", init_input("init-a"));
    assert_eq!(initialized["status"], "accepted");
    let report = task(
        &config,
        &ledger,
        "record",
        record_input("capture-1", "checkpoint", "已拍两段，待选片"),
    );
    assert_eq!(report["status"], "accepted", "{report}");
    let mut second_input = record_input("capture-2", "execution", "完成选片");
    second_input["client"] = json!("codex");
    second_input["evidence_refs"] = json!([{"path":"notes/selection.txt"}]);
    let second = task(&config, &ledger, "record", second_input);
    assert_eq!(second["status"], "accepted");
    let context = task(&config, &ledger, "context", json!({}));
    assert_eq!(context["status"], "ok", "{context}");
    assert_eq!(context["revision"], second["revision"]);
    let entries = context["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["id"], report["result"]["record_id"]);
    assert_eq!(entries[0]["text"], "已拍两段，待选片");
    assert_eq!(entries[0]["trust"], "execution_reported");
    assert_eq!(entries[0]["client"], "minimax");
    assert_eq!(entries[1]["client"], "codex");
    assert_eq!(
        entries[1]["evidence_refs"],
        json!([{"path":"notes/selection.txt"}])
    );
    assert_eq!(entries[1]["source_ids"], json!([]));
    assert_eq!(entries[1]["state"], "current");
    let reread = task(&config, &ledger, "context", json!({}));
    assert_eq!(reread, context);
    let moved = temp.path().join("renamed.json");
    std::fs::rename(&config, &moved).unwrap();
    assert_eq!(task(&moved, &ledger, "context", json!({})), context);
}

#[test]
fn init_recovers_committed_identity_after_config_publication_failure() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("missing-directory/.memorynexus.json");
    let failed = task(&config, &ledger, "init", init_input("recover-init"));
    assert_eq!(failed["code"], "storage_unavailable");
    assert!(!config.exists());
    let recovered_config = temp.path().join("recovered.json");
    let recovered = task(
        &recovered_config,
        &ledger,
        "init",
        init_input("recover-init"),
    );
    assert_eq!(recovered["status"], "replayed");
    std::fs::create_dir(config.parent().unwrap()).unwrap();
    let retried = task(&config, &ledger, "init", init_input("recover-init"));
    assert_eq!(retried["task_id"], recovered["task_id"]);
    assert_eq!(retried["status"], "replayed");
    let original = std::fs::read(&config).unwrap();
    let mut changed = init_input("recover-init");
    changed["automatic_capture"] = json!([]);
    assert_eq!(task(&config, &ledger, "init", changed)["code"], "conflict");
    assert_eq!(
        task(&config, &ledger, "init", init_input("new-task"))["code"],
        "conflict"
    );
    assert_eq!(std::fs::read(&config).unwrap(), original);
}

#[test]
fn parallel_writes_and_parallel_init_share_receipts_without_duplicate_records() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    let initializers: Vec<_> = (0..2)
        .map(|_| {
            let ledger = ledger.clone();
            let config = config.clone();
            std::thread::spawn(move || task(&config, &ledger, "init", init_input("init-race")))
        })
        .collect();
    let inits: Vec<_> = initializers
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect();
    assert_eq!(
        inits.iter().filter(|r| r["status"] == "accepted").count(),
        1,
        "{inits:?}"
    );
    assert_eq!(
        inits.iter().filter(|r| r["status"] == "replayed").count(),
        1,
        "{inits:?}"
    );
    assert_eq!(inits[0]["task_id"], inits[1]["task_id"]);
    let writers: Vec<_> = (0..2)
        .map(|_| {
            let ledger = ledger.clone();
            let config = config.clone();
            std::thread::spawn(move || {
                task(
                    &config,
                    &ledger,
                    "record",
                    record_input("same-key", "execution", "只记录一次"),
                )
            })
        })
        .collect();
    let records: Vec<_> = writers.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(
        records.iter().filter(|r| r["status"] == "accepted").count(),
        1,
        "{records:?}"
    );
    assert_eq!(
        records.iter().filter(|r| r["status"] == "replayed").count(),
        1,
        "{records:?}"
    );
    assert_eq!(
        records[0]["result"]["record_id"],
        records[1]["result"]["record_id"]
    );
    let context = task(&config, &ledger, "context", json!({}));
    assert_eq!(context["result"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(context["revision"], 2);
    let mut changed = record_input("same-key", "execution", "改变内容");
    assert_eq!(
        task(&config, &ledger, "record", changed.clone())["code"],
        "conflict"
    );
    changed["text"] = json!("只记录一次");
    changed["client"] = json!("codex");
    assert_eq!(
        task(&config, &ledger, "record", changed)["code"],
        "conflict"
    );
    assert_eq!(task(&config, &ledger, "context", json!({})), context);
}

#[test]
fn task_partition_rejects_foreign_and_missing_evidence_ids() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let a = temp.path().join("a.json");
    let b = temp.path().join("b.json");
    task(&a, &ledger, "init", init_input("task-a"));
    task(&b, &ledger, "init", init_input("task-b"));
    let record = task(
        &a,
        &ledger,
        "record",
        record_input("same-local-key", "execution", "A only"),
    );
    let b_context = task(&b, &ledger, "context", json!({}));
    assert_eq!(b_context["result"]["entries"], json!([]));
    let foreign = task(
        &b,
        &ledger,
        "context",
        json!({"record_ids":[record["result"]["record_id"]]}),
    );
    assert_eq!(foreign["code"], "scope_mismatch");
    assert!(!foreign.to_string().contains("A only"));
    assert_eq!(
        task(
            &a,
            &ledger,
            "context",
            json!({"record_ids":["c08a25a2-68bc-4114-976c-596f9939656a"]})
        )["code"],
        "not_found"
    );
    let b_record = task(
        &b,
        &ledger,
        "record",
        record_input("same-local-key", "execution", "B only"),
    );
    assert_eq!(b_record["status"], "accepted");
    assert_ne!(
        b_record["result"]["record_id"],
        record["result"]["record_id"]
    );
}

#[test]
fn capture_policy_is_persisted_and_confirmation_determines_record_trust() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    let mut setup = init_input("manual");
    setup["automatic_capture"] = json!([]);
    task(&config, &ledger, "init", setup);
    for kind in ["checkpoint", "execution", "instruction"] {
        let mut input = record_input(kind, kind, "明确确认后保留");
        assert_eq!(
            task(&config, &ledger, "record", input.clone())["code"],
            "invalid_input"
        );
        input["confirmation"] = json!("confirmed");
        assert_eq!(
            task(&config, &ledger, "record", input)["status"],
            "accepted"
        );
    }
    let context = task(&config, &ledger, "context", json!({}));
    for entry in context["result"]["entries"].as_array().unwrap() {
        assert_eq!(entry["trust"], "owner_confirmed");
    }
    let automatic = temp.path().join("automatic.json");
    task(&automatic, &ledger, "init", init_input("automatic"));
    assert_eq!(
        task(
            &automatic,
            &ledger,
            "record",
            record_input("instruction", "instruction", "不能自动授权")
        )["code"],
        "invalid_input"
    );
    let mut forged: Value = serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    forged["automatic_capture"] = json!(["execution"]);
    std::fs::write(&config, forged.to_string()).unwrap();
    assert_eq!(
        task(
            &config,
            &ledger,
            "record",
            record_input("forged", "execution", "不能通过配置提权")
        )["code"],
        "invalid_input"
    );
}

#[test]
fn context_orders_confirmed_instructions_and_only_latest_checkpoint_by_commit() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("context"));
    let old = task(
        &config,
        &ledger,
        "record",
        record_input("old", "checkpoint", "先前进度"),
    );
    let mut newest = record_input("new", "checkpoint", "当前进度");
    newest["occurred_at"] = json!("2020-01-01T00:00:00Z");
    let latest = task(&config, &ledger, "record", newest);
    for (key, text) in [("instruction-a", "用横屏"), ("instruction-b", "用竖屏")] {
        let mut input = record_input(key, "instruction", text);
        input["confirmation"] = json!("confirmed");
        assert_eq!(
            task(&config, &ledger, "record", input)["status"],
            "accepted"
        );
    }
    for index in 0..22 {
        assert_eq!(
            task(
                &config,
                &ledger,
                "record",
                record_input(
                    &format!("execution-{index}"),
                    "execution",
                    &format!("执行 {index}")
                )
            )["status"],
            "accepted"
        );
    }
    let context = task(&config, &ledger, "context", json!({}));
    let entries = context["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 20);
    assert_eq!(entries[0]["text"], "用竖屏");
    assert_eq!(entries[1]["text"], "用横屏");
    assert_eq!(entries[2]["id"], latest["result"]["record_id"]);
    assert_eq!(entries[3]["text"], "执行 21");
    assert_eq!(context["result"]["omitted_count"], 5);
    assert_eq!(context["result"]["truncated"], true);
    assert_eq!(context["result"]["candidate_count"], 0);
    let limited = task(&config, &ledger, "context", json!({"limit":1}));
    assert_eq!(limited["result"]["entries"].as_array().unwrap().len(), 1);
    assert_eq!(limited["result"]["omitted_count"], 24);
    let history = task(
        &config,
        &ledger,
        "context",
        json!({"record_ids":[old["result"]["record_id"]]}),
    );
    assert_eq!(history["result"]["entries"][0]["state"], "historical");
    assert_eq!(task(&config, &ledger, "context", json!({})), context);
}

#[test]
fn unicode_context_budget_counts_display_name_and_never_slices_entry_text() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    let mut setup = init_input("budget");
    setup["display_name"] = json!("名".repeat(120));
    task(&config, &ledger, "init", setup);
    let text = "拍".repeat(1000);
    for index in 0..13 {
        task(
            &config,
            &ledger,
            "record",
            record_input(&format!("entry-{index}"), "execution", &text),
        );
    }
    let context = task(&config, &ledger, "context", json!({}));
    let entries = context["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 11);
    assert_eq!(context["result"]["omitted_count"], 2);
    assert_eq!(context["result"]["truncated"], true);
    for entry in entries {
        assert_eq!(entry["text"], text);
    }
}

#[test]
fn invalid_envelopes_do_not_create_records_or_echo_request_content() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("validation"));
    let baseline = task(&config, &ledger, "context", json!({}));
    for (field, value) in [
        ("unexpected", json!("PRIVATE_PAYLOAD")),
        ("kind", json!("lesson")),
        ("trust", json!("owner_confirmed")),
        ("text", json!("")),
        ("text", json!("字".repeat(1001))),
        ("client", json!("c".repeat(65))),
        ("idempotency_key", json!("k".repeat(129))),
        ("run_id", json!("AB11B084-E0C6-4D9E-A2D2-815F004A1D99")),
        ("occurred_at", json!("2026-10-21T10:00:00")),
        ("confirmation", json!("unconfirmed")),
        ("confirmation", Value::Null),
        (
            "evidence_refs",
            json!([{"path":"note.txt","extra":"PRIVATE_PAYLOAD"}]),
        ),
        (
            "evidence_refs",
            json!([{"path":"note.txt","sha256":"A".repeat(64)}]),
        ),
        ("evidence_refs", json!(vec![json!({"path":"note.txt"}); 6])),
    ] {
        let mut input = record_input("invalid", "execution", "PRIVATE_PAYLOAD");
        input[field] = value;
        let response = task(&config, &ledger, "record", input);
        assert_eq!(
            response["code"], "invalid_input",
            "field {field}: {response}"
        );
        assert!(!response.to_string().contains("PRIVATE_PAYLOAD"));
    }
    for path in [
        "",
        "/absolute/file",
        "../private.txt",
        "notes/../private.txt",
        "https://example.com/artifact",
        "C:\\file.txt",
        "\\\\server\\file",
        "note\0.txt",
    ] {
        let mut input = record_input("invalid-ref", "execution", "PRIVATE_PAYLOAD");
        input["evidence_refs"] = json!([{"path":path}]);
        assert_eq!(
            task(&config, &ledger, "record", input)["code"],
            "invalid_input"
        );
    }
    for input in [
        json!({"limit":0}),
        json!({"limit":21}),
        json!({"limit":null}),
        json!({"record_ids":["invalid"]}),
        json!({"unknown":true}),
    ] {
        assert_eq!(
            task(&config, &ledger, "context", input)["code"],
            "invalid_input"
        );
    }
    let oversize = format!(
        "{}{}",
        record_input("oversize", "execution", "PRIVATE_PAYLOAD"),
        " ".repeat(33000)
    );
    assert_eq!(
        invoke(
            &["task", "--config", config.to_str().unwrap(), "record"],
            Some(&ledger),
            &oversize
        )["code"],
        "invalid_input"
    );
    assert_eq!(task(&config, &ledger, "context", json!({})), baseline);
}

#[test]
fn failed_resolution_never_creates_a_substitute_ledger_or_task() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    assert_eq!(
        task(&config, &ledger, "context", json!({}))["code"],
        "not_found"
    );
    assert!(!ledger.exists());
    assert_eq!(
        invoke(
            &["task", "--config", config.to_str().unwrap(), "init"],
            None,
            &init_input("missing-env").to_string()
        )["code"],
        "invalid_input"
    );
    assert!(!ledger.exists());
    assert!(!config.exists());
    task(&config, &ledger, "init", init_input("stable"));
    let missing = temp.path().join("wrong-directory/ledger.sqlite");
    assert_eq!(
        task(&config, &missing, "context", json!({}))["code"],
        "not_found"
    );
    let absent = task(&config, &missing, "init", init_input("stable"));
    assert_eq!(absent["code"], "not_found", "{absent}");
    assert!(!missing.exists());
    assert!(!missing.parent().unwrap().exists());
    let other_ledger = temp.path().join("other.sqlite");
    let other_config = temp.path().join("other.json");
    task(&other_config, &other_ledger, "init", init_input("other"));
    assert_eq!(
        task(&config, &other_ledger, "context", json!({}))["code"],
        "not_found"
    );
    assert_eq!(
        invoke(
            &["task", "--config", "relative.json", "context"],
            Some(&ledger),
            "{}"
        )["code"],
        "invalid_input"
    );
    let mut binding: Value = serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    binding["schema_version"] = json!(2);
    std::fs::write(&config, binding.to_string()).unwrap();
    assert_eq!(
        task(&config, &ledger, "context", json!({}))["code"],
        "unsupported_version"
    );
}

#[test]
fn task_commands_reject_experiment_and_unknown_database_formats_without_mutation() {
    let temp = tempfile::TempDir::new().unwrap();
    let experiment = temp.path().join("experiment.sqlite");
    let config = temp.path().join("task.json");
    let initial = json!({"confirmation":"confirmed","kind":"initial","statement":"原实验记录","occurred_at":"2026-10-21T10:00:00+08:00","source":"owner_report","idempotency_key":"observe"});
    assert_eq!(
        invoke(
            &["--ledger", experiment.to_str().unwrap(), "observe"],
            None,
            &initial.to_string()
        )["status"],
        "accepted"
    );
    let before = invoke(
        &[
            "--ledger",
            experiment.to_str().unwrap(),
            "observation-history",
        ],
        None,
        "",
    );
    assert_eq!(
        task(&config, &experiment, "init", init_input("cannot-adopt"))["code"],
        "scope_mismatch"
    );
    assert!(!config.exists());
    assert_eq!(
        invoke(
            &[
                "--ledger",
                experiment.to_str().unwrap(),
                "observation-history"
            ],
            None,
            ""
        ),
        before
    );
    let empty = temp.path().join("unknown.sqlite");
    std::fs::write(&empty, []).unwrap();
    assert_eq!(
        task(&config, &empty, "init", init_input("cannot-adopt"))["code"],
        "scope_mismatch"
    );
    assert_eq!(std::fs::read(&empty).unwrap(), Vec::<u8>::new());
    let ledger = temp.path().join("task.sqlite");
    task(&config, &ledger, "init", init_input("task"));
    // SQLite documents user_version in its public file header, offset 60.
    // Simulate a newer file format without querying any private task tables.
    let bytes = std::fs::read(&ledger).unwrap();
    let mut future = bytes.clone();
    future[60..64].copy_from_slice(&2_u32.to_be_bytes());
    std::fs::write(&ledger, &future).unwrap();
    assert_eq!(
        task(&config, &ledger, "context", json!({}))["code"],
        "unsupported_version"
    );
    assert_eq!(
        task(&config, &ledger, "init", init_input("task"))["code"],
        "unsupported_version"
    );
    assert_eq!(std::fs::read(&ledger).unwrap(), future);
    std::fs::write(&ledger, bytes).unwrap();
    assert_eq!(task(&config, &ledger, "context", json!({}))["status"], "ok");
}

#[test]
fn context_revision_and_entries_come_from_one_snapshot_during_writes() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("snapshots"));
    let writer = {
        let ledger = ledger.clone();
        let config = config.clone();
        std::thread::spawn(move || {
            for index in 0..10 {
                assert_eq!(
                    task(
                        &config,
                        &ledger,
                        "record",
                        record_input(&format!("write-{index}"), "execution", "进展")
                    )["status"],
                    "accepted"
                );
            }
        })
    };
    for _ in 0..10 {
        let context = task(&config, &ledger, "context", json!({}));
        assert_eq!(
            context["revision"].as_u64().unwrap(),
            context["result"]["entries"].as_array().unwrap().len() as u64 + 1
        );
    }
    writer.join().unwrap();
    assert_eq!(task(&config, &ledger, "context", json!({}))["revision"], 11);
}

#[test]
fn evidence_references_are_relative_descriptors_without_filesystem_reads() {
    let temp = tempfile::TempDir::new().unwrap();
    let ledger = temp.path().join("ledger.sqlite");
    let config = temp.path().join("task.json");
    task(&config, &ledger, "init", init_input("references"));
    let mut input = record_input("descriptor", "execution", "保存引用即可");
    let references =
        json!([{"path":"notes/day:1.txt","sha256":"a".repeat(64)},{"path":"notes\\result.txt"}]);
    input["evidence_refs"] = references.clone();
    assert_eq!(
        task(&config, &ledger, "record", input)["status"],
        "accepted"
    );
    assert_eq!(
        task(&config, &ledger, "context", json!({}))["result"]["entries"][0]["evidence_refs"],
        references
    );
    let mut traversal = record_input("traversal", "execution", "不能保留上级路径");
    traversal["evidence_refs"] = json!([{"path":"notes\\..\\private.txt"}]);
    assert_eq!(
        task(&config, &ledger, "record", traversal)["code"],
        "invalid_input"
    );
}
