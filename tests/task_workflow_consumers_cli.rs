use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
const WORKFLOW: &str = "40000000-0000-4000-8000-000000000001";
const RUN: &str = "60000000-0000-4000-8000-000000000001";
fn sha(v: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(v.as_ref()))
}
fn invoke(args: &[&str], ledger: Option<&Path>, input: &Value) -> Value {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_memorynexus-feedback-cli"));
    cmd.args(args).env_remove("MEMORYNEXUS_TASK_LEDGER");
    if let Some(path) = ledger {
        cmd.env("MEMORYNEXUS_TASK_LEDGER", path);
    }
    let mut child = cmd
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
    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{out:?}"));
    assert_eq!(
        out.status.code(),
        Some(if v["status"] == "error" { 2 } else { 0 }),
        "{v}"
    );
    assert!(out.stderr.is_empty(), "{:?}", out.stderr);
    v
}
struct Harness {
    _dir: tempfile::TempDir,
    config: PathBuf,
    ledger: PathBuf,
    incarnation: String,
}
impl Harness {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("config.json");
        let v1 = dir.path().join("v1.sqlite");
        let ledger = dir.path().join("v2.sqlite");
        let backup = dir.path().join("backup.sqlite");
        let init = invoke(
            &["task", "--config", config.to_str().unwrap(), "init"],
            Some(&v1),
            &json!({"idempotency_key":"init","display_name":"consumer fixture","automatic_capture":["execution","review"],"confirmation":"confirmed"}),
        );
        assert_eq!(init["status"], "accepted", "{init}");
        let b = invoke(
            &["task", "--ledger", v1.to_str().unwrap(), "backup"],
            None,
            &json!({"path":backup}),
        );
        assert_eq!(b["status"], "ok", "{b}");
        let migrated = invoke(
            &["task", "--ledger", v1.to_str().unwrap(), "migrate"],
            None,
            &json!({"path":ledger,"backup_path":backup,"backup_sha256":sha(std::fs::read(&backup).unwrap()),"expected_version":1,"target_version":2,"confirmation":"confirmed"}),
        );
        assert_eq!(migrated["status"], "ok", "{migrated}");
        let h = Self {
            _dir: dir,
            config,
            ledger,
            incarnation: migrated["result"]["incarnation"].as_str().unwrap().into(),
        };
        let profile = Self::profile();
        let bound = h.call("workflow-bind", h.req("bind", json!({"profile_sha256":sha(profile.to_string()),"binding_sha256":sha("synthetic binding"),"profile":profile,"expected_profile_revision":0,"confirmation":"confirmed"})));
        assert_eq!(bound["status"], "accepted", "{bound}");
        h
    }
    fn profile() -> Value {
        serde_json::from_str::<Value>(include_str!(
            "../docs/repository-memory/examples/sequence-v1.json"
        ))
        .unwrap()["profile"]
            .clone()
    }
    fn call(&self, action: &str, input: Value) -> Value {
        invoke(
            &["task", "--config", self.config.to_str().unwrap(), action],
            Some(&self.ledger),
            &input,
        )
    }
    fn req(&self, key: &str, fields: Value) -> Value {
        let mut v = json!({"idempotency_key":key,"run_id":RUN,"client":"consumer-test","workflow_id":WORKFLOW,"profile_revision":1,"incarnation":self.incarnation});
        for (k, value) in fields.as_object().unwrap() {
            v[k] = value.clone();
        }
        v
    }
    fn record(&self, key: &str, text: &str) -> String {
        let v=self.call("record",json!({"idempotency_key":key,"run_id":RUN,"client":"consumer-test","kind":"execution","text":text,"occurred_at":"2026-01-01T09:00:00Z"}));
        assert_eq!(v["status"], "accepted", "{v}");
        v["result"]["record_id"].as_str().unwrap().into()
    }
    fn review(&self, key: &str, id: &str, text: &str, consumer: &str, decision: &str) -> Value {
        let mut v=self.req(key,json!({"consumer_id":consumer,"input_record_id":id,"decision":decision,"reason":"Bounded evidence, unverified branches remain unknown.","dependencies":[{"type":"record","id":id,"sha256":sha(text)}]}));
        if decision == "proposed" {
            v["output_sha256"] = json!(sha("Bounded pointer.\n"));
        }
        v
    }
    fn status(&self, view: &str) -> Value {
        self.call(
            "workflow-status",
            json!({"workflow_id":WORKFLOW,"view":view}),
        )
    }
}
#[test]
fn independent_consumers_keep_distinct_decisions_and_repeat_reviews_are_noops() {
    let h = Harness::new();
    let text = "Readback did not find the target; fallback coverage remains unknown.";
    let id = h.record("record", text);
    let a_req = h.review("a", &id, text, "mechanism", "proposed");
    let a = h.call("consumer-review", a_req.clone());
    assert_eq!(a["status"], "accepted", "{a}");
    let b = h.call(
        "consumer-review",
        h.review("b", &id, text, "lessons", "deferred"),
    );
    assert_eq!(b["status"], "accepted", "{b}");
    assert_ne!(a["result"]["decision_id"], b["result"]["decision_id"]);
    assert_ne!(a["result"]["block_id"], b["result"]["block_id"]);
    assert_eq!(
        h.call("consumer-review", a_req.clone())["status"],
        "replayed"
    );
    let mut same = a_req;
    same["idempotency_key"] = json!("same-a");
    let n = h.call("consumer-review", same);
    assert_eq!(n["status"], "noop", "{n}");
    assert_eq!(n["revision"], b["revision"]);
    let items = h.status("decisions");
    assert_eq!(
        items["result"]["items"].as_array().unwrap().len(),
        2,
        "{items}"
    );
}
impl Harness {
    fn approval(&self, key: &str, decision: &Value) -> Value {
        self.req(key,json!({"decision_id":decision["result"]["decision_id"],"decision_sha256":decision["result"]["decision_sha256"],"proposal":{"target_id":"mechanism_doc","block_id":decision["result"]["block_id"],"base_exists":false,"base_sha256":sha(""),"after_sha256":sha("rendered output"),"output_sha256":sha("Bounded pointer.\n"),"plan_sha256":sha("exact preview")},"confirmation":"confirmed"}))
    }
}
#[test]
fn exact_approval_preserves_proposal_identity_and_pins_the_full_diff() {
    let h = Harness::new();
    let text = "One observed readback.";
    let id = h.record("record", text);
    let review = h.review("review", &id, text, "mechanism", "proposed");
    let decision = h.call("consumer-review", review.clone());
    assert_eq!(decision["status"], "accepted", "{decision}");
    let req = h.approval("approve", &decision);
    let approved = h.call("consumer-approve", req.clone());
    assert_eq!(approved["status"], "accepted", "{approved}");
    assert_eq!(approved["result"]["decision"], "approved");
    assert_eq!(approved["result"]["publication"], "not_applied");
    let mut repeated = review;
    repeated["idempotency_key"] = json!("review-again");
    let same = h.call("consumer-review", repeated);
    assert_eq!(same["status"], "noop", "{same}");
    assert_eq!(same["result"]["decision"], "approved");
    let mut same = req.clone();
    same["idempotency_key"] = json!("approve-again");
    assert_eq!(h.call("consumer-approve", same)["status"], "noop");
    let mut changed = req;
    changed["idempotency_key"] = json!("changed-diff");
    changed["proposal"]["base_exists"] = json!(true);
    assert_eq!(h.call("consumer-approve", changed)["code"], "conflict");
    let read = h.status("inputs");
    assert_eq!(read["result"]["items"][0]["trust"], "execution_reported");
}
impl Harness {
    fn publish(&self, key: &str, approved: &Value, outcome: &str) -> Value {
        let mut v=self.req(key,json!({"approval_id":approved["result"]["approval_id"],"outcome":outcome,"observed_at":"2026-01-01T09:10:00Z"}));
        match outcome {
            "applied" => {
                v["observed_sha256"] = approved["result"]["proposal"]["after_sha256"].clone()
            }
            "failed" => v["error_code"] = json!("io_error"),
            _ => {
                v["observed_sha256"] = json!(sha("changed"));
                v["error_code"] = json!("baseline_changed");
            }
        }
        v
    }
}
#[test]
fn publication_attempts_preserve_failures_and_replay_actual_current_state() {
    let h = Harness::new();
    let text = "Observed once.";
    let id = h.record("record", text);
    let d = h.call(
        "consumer-review",
        h.review("review", &id, text, "mechanism", "proposed"),
    );
    assert_eq!(d["status"], "accepted", "{d}");
    let approved = h.call("consumer-approve", h.approval("approve", &d));
    assert_eq!(approved["status"], "accepted", "{approved}");
    let failed_req = h.publish("failed", &approved, "failed");
    let failed = h.call("publication-result", failed_req.clone());
    assert_eq!(failed["status"], "accepted", "{failed}");
    assert_eq!(failed["result"]["publication"], "failed");
    assert_eq!(
        h.call("publication-result", failed_req.clone())["status"],
        "replayed"
    );
    let applied_req = h.publish("applied", &approved, "applied");
    let applied = h.call("publication-result", applied_req.clone());
    assert_eq!(applied["status"], "accepted", "{applied}");
    assert_eq!(applied["result"]["publication"], "applied");
    assert_ne!(
        applied["result"]["attempt_id"],
        failed["result"]["attempt_id"]
    );
    let replay = h.call("publication-result", failed_req);
    assert_eq!(replay["status"], "replayed");
    assert_eq!(
        replay["result"]["attempt_id"],
        failed["result"]["attempt_id"]
    );
    assert_eq!(replay["result"]["publication"], "applied");
    let mut again = applied_req;
    again["idempotency_key"] = json!("same-effect");
    assert_eq!(h.call("publication-result", again)["status"], "noop");
    let publications = h.status("publications");
    assert_eq!(
        publications["result"]["items"][0]["attempts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(publications["result"]["items"][0]["publication"], "applied");
    let exported = invoke(
        &["task", "--ledger", h.ledger.to_str().unwrap(), "export"],
        None,
        &json!({}),
    );
    assert_eq!(
        exported["result"]["publication_attempts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
const INCIDENT: &str = "50000000-0000-4000-8000-000000000001";
const MECHANISM: &str = "50000000-0000-4000-8000-000000000002";
const CORRECTION: &str = "50000000-0000-4000-8000-000000000003";
impl Harness {
    fn sync(&self, key: &str, source: &str, scan: u64, text: &str) -> Value {
        self.call("source-sync",self.req(key,json!({"collection_id":source,"expected_scan":scan,"observed_at":"2026-01-01T09:00:00Z","availability":"available","units":[{"sha256":sha(text)}]})))
    }
    fn capture(&self, key: &str, source: &str, rev: u64, text: &str) -> Value {
        self.call("source-capture",self.req(key,json!({"source_id":source,"source_revision":rev,"extractor_version":"bounded-v1","text":text,"occurred_at":"2026-01-01T08:00:00Z"})))
    }
    fn correct(&self, key: &str, id: &str, text: &str) -> Value {
        self.call("correct",json!({"idempotency_key":key,"run_id":RUN,"client":"consumer-test","target_id":id,"text":text,"occurred_at":"2026-01-01T11:00:00Z","confirmation":"confirmed"}))
    }
}
#[test]
fn auxiliary_source_correction_invalidates_only_its_consumer_and_expands_origins() {
    let h = Harness::new();
    for (key, id, text) in [
        ("s1", INCIDENT, "incident"),
        ("s2", MECHANISM, "mechanism"),
        ("s3", CORRECTION, "unverified"),
    ] {
        assert_eq!(h.sync(key, id, 0, text)["status"], "accepted");
    }
    let text = "Reported checker incident, coverage unknown.";
    let captured = h.capture("capture", INCIDENT, 1, text);
    assert_eq!(captured["status"], "accepted", "{captured}");
    let id = captured["result"]["record_id"].as_str().unwrap();
    let mut a_req = h.review("a", id, text, "mechanism", "proposed");
    a_req["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"source","id":MECHANISM,"revision":1,"sha256":sha("mechanism")}));
    let a = h.call("consumer-review", a_req.clone());
    assert_eq!(a["status"], "accepted", "{a}");
    let mut b_req = h.review("b", id, text, "lessons", "deferred");
    b_req["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"source","id":CORRECTION,"revision":1,"sha256":sha("unverified")}));
    let b = h.call("consumer-review", b_req);
    assert_eq!(b["status"], "accepted", "{b}");
    let before = h.status("decisions");
    for d in before["result"]["items"].as_array().unwrap() {
        assert_eq!(d["dependencies"].as_array().unwrap().len(), 3);
        assert!(d["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["id"] == INCIDENT));
    }
    let approved = h.call("consumer-approve", h.approval("approve", &a));
    assert_eq!(approved["status"], "accepted", "{approved}");
    let changed = h.sync("s3-v2", CORRECTION, 1, "two reported trials");
    assert_eq!(
        changed["result"]["invalidated"]["decisions"], 1,
        "{changed}"
    );
    assert_eq!(changed["result"]["invalidated"]["approvals"], 0);
    let status = h.status("decisions");
    let ds = status["result"]["items"].as_array().unwrap();
    assert_eq!(ds[0]["validity"], "current");
    assert_eq!(ds[1]["validity"], "stale");
    assert_eq!(
        h.call("consumer-review", a_req)["result"]["decision"],
        "approved"
    );
    let mut next = h.review("b2", id, text, "lessons", "proposed");
    next["dependencies"].as_array_mut().unwrap().push(
        json!({"type":"source","id":CORRECTION,"revision":2,"sha256":sha("two reported trials")}),
    );
    assert_eq!(h.call("consumer-review", next.clone())["code"], "conflict");
    next["previous_decision_id"] = b["result"]["decision_id"].clone();
    let next = h.call("consumer-review", next);
    assert_eq!(next["status"], "accepted", "{next}");
    assert_eq!(next["result"]["generation"], 2);
    assert_eq!(next["result"]["block_id"], b["result"]["block_id"]);
}
#[test]
fn reconsider_requires_current_predecessor_and_stales_its_exact_approval() {
    let h = Harness::new();
    let text = "Single observation.";
    let id = h.record("record", text);
    let a = h.call(
        "consumer-review",
        h.review("review", &id, text, "mechanism", "proposed"),
    );
    let approved = h.call("consumer-approve", h.approval("approve", &a));
    assert_eq!(approved["status"], "accepted");
    let mut next = h.review("next", &id, text, "mechanism", "deferred");
    assert_eq!(h.call("consumer-review", next.clone())["code"], "conflict");
    next["previous_decision_id"] = a["result"]["decision_id"].clone();
    assert_eq!(h.call("consumer-review", next.clone())["code"], "conflict");
    next["reconsider"] = json!(true);
    let next = h.call("consumer-review", next);
    assert_eq!(next["status"], "accepted", "{next}");
    assert_eq!(next["result"]["generation"], 2);
    let publications = h.status("publications");
    assert_eq!(publications["result"]["items"][0]["validity"], "stale");
}
#[test]
fn applied_effect_after_profile_change_is_retained_without_reauthorizing_publication() {
    let h = Harness::new();
    let text = "One observation.";
    let id = h.record("record", text);
    let d = h.call(
        "consumer-review",
        h.review("review", &id, text, "mechanism", "proposed"),
    );
    let req = h.approval("approve", &d);
    let a = h.call("consumer-approve", req.clone());
    assert_eq!(a["status"], "accepted", "{a}");
    let mut profile = Harness::profile();
    profile["revision"] = json!(2);
    profile["purpose"] = json!("Revised purpose.");
    let mut bind=h.req("bind2",json!({"profile_sha256":sha(profile.to_string()),"binding_sha256":sha("synthetic binding"),"profile":profile,"expected_profile_revision":1,"confirmation":"confirmed"}));
    bind["profile_revision"] = json!(2);
    assert_eq!(h.call("workflow-bind", bind)["status"], "accepted");
    let applied = h.call(
        "publication-result",
        h.publish("effect-after-change", &a, "applied"),
    );
    assert_eq!(applied["status"], "accepted", "{applied}");
    assert_eq!(applied["result"]["validity"], "stale");
    assert_eq!(applied["result"]["publication"], "applied");
    let mut incorrect = h.publish("incorrect-profile", &a, "applied");
    incorrect["profile_revision"] = json!(2);
    assert_eq!(
        h.call("publication-result", incorrect)["code"],
        "stale_snapshot"
    );
    let replay = h.call("consumer-approve", req);
    assert_eq!(replay["status"], "replayed");
    assert_eq!(replay["result"]["validity"], "stale");
    assert_eq!(replay["result"]["publication"], "applied");
}
#[test]
fn neutralization_requires_fixed_text_policy_and_an_actual_published_history() {
    let h = Harness::new();
    let text = "Single reported trial.";
    let id = h.record("record", text);
    let d = h.call(
        "consumer-review",
        h.review("review", &id, text, "mechanism", "proposed"),
    );
    let a = h.call("consumer-approve", h.approval("approve", &d));
    assert_eq!(a["status"], "accepted");
    let corrected = h.correct(
        "correction",
        &id,
        "The trial was not independently verified.",
    );
    assert_eq!(corrected["status"], "accepted", "{corrected}");
    let mut n = h.approval("neutralize", &d);
    n.as_object_mut().unwrap().remove("confirmation");
    n["maintenance"] = json!("neutralize");
    n["proposal"]["base_exists"] = json!(true);
    n["proposal"]["base_sha256"] = a["result"]["proposal"]["after_sha256"].clone();
    n["proposal"]["output_sha256"] = json!(sha(
        "MemoryNexus: source evidence changed; review required.\n"
    ));
    n["proposal"]["after_sha256"] = json!(sha("rendered neutral block"));
    assert_eq!(
        h.call("consumer-approve", n.clone())["code"],
        "approval_required"
    );
    let effect = h.call(
        "publication-result",
        h.publish("outstanding-effect", &a, "applied"),
    );
    assert_eq!(effect["status"], "accepted", "{effect}");
    assert_eq!(effect["result"]["validity"], "stale");
    let mut arbitrary = n.clone();
    arbitrary["proposal"]["output_sha256"] = json!(sha("New unapproved prose.\n"));
    assert_eq!(
        h.call("consumer-approve", arbitrary)["code"],
        "approval_required"
    );
    let neutral = h.call("consumer-approve", n);
    assert_eq!(neutral["status"], "accepted", "{neutral}");
    assert_eq!(neutral["result"]["validity"], "stale");
    let done = h.call(
        "publication-result",
        h.publish("neutral-effect", &neutral, "applied"),
    );
    assert_eq!(done["status"], "accepted", "{done}");
    assert_eq!(done["result"]["validity"], "stale");
}
#[test]
fn strict_dependencies_are_canonical_bounded_and_do_not_write_on_rejection() {
    let h = Harness::new();
    let one = h.record("one", "One fact.");
    let two = h.record("two", "Second fact.");
    let mut req = h.review("a", &one, "One fact.", "mechanism", "deferred");
    req["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"record","id":two,"sha256":sha("Second fact.")}));
    let original = h.call("consumer-review", req.clone());
    assert_eq!(original["status"], "accepted", "{original}");
    req["dependencies"].as_array_mut().unwrap().reverse();
    assert_eq!(h.call("consumer-review", req.clone())["status"], "replayed");
    let before = h.status("decisions");
    let mut duplicate = req.clone();
    duplicate["idempotency_key"] = json!("duplicate");
    let dep = duplicate["dependencies"][0].clone();
    duplicate["dependencies"].as_array_mut().unwrap().push(dep);
    assert_eq!(
        h.call("consumer-review", duplicate)["code"],
        "invalid_input"
    );
    let mut wrong_hash = req.clone();
    wrong_hash["idempotency_key"] = json!("wrong-hash");
    wrong_hash["dependencies"][0]["sha256"] = json!(sha("not that content"));
    assert_eq!(
        h.call("consumer-review", wrong_hash)["code"],
        "stale_snapshot"
    );
    let mut unknown = req.clone();
    unknown["idempotency_key"] = json!("unknown-field");
    unknown["dependencies"][0]["target"] = json!("invented");
    assert_eq!(h.call("consumer-review", unknown)["code"], "invalid_input");
    req["client"] = json!("changed caller");
    assert_eq!(h.call("consumer-review", req)["code"], "conflict");
    assert_eq!(h.status("decisions"), before);
}
#[test]
fn expanded_dependency_limit_rejects_without_truncating_the_origin() {
    let h = Harness::new();
    assert_eq!(h.sync("sync", INCIDENT, 0, "raw")["status"], "accepted");
    let text = "Bounded source evidence.";
    let c = h.capture("capture", INCIDENT, 1, text);
    assert_eq!(c["status"], "accepted", "{c}");
    let id = c["result"]["record_id"].as_str().unwrap();
    let mut req = h.review("review", id, text, "mechanism", "deferred");
    for i in 0..19 {
        let body = format!("Independent fact {i}.");
        let r = h.record(&format!("r{i}"), &body);
        req["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type":"record","id":r,"sha256":sha(body)}));
    }
    assert_eq!(req["dependencies"].as_array().unwrap().len(), 20);
    let before = h.status("decisions");
    assert_eq!(h.call("consumer-review", req)["code"], "invalid_input");
    assert_eq!(h.status("decisions"), before);
}
#[test]
fn withdrawal_scrubs_decision_reason_and_replay_without_reenabling_the_source() {
    let h = Harness::new();
    assert_eq!(
        h.sync("sync", INCIDENT, 0, "raw source")["status"],
        "accepted"
    );
    let text = "Withdrawn bounded observation.";
    let c = h.capture("capture", INCIDENT, 1, text);
    let id = c["result"]["record_id"].as_str().unwrap();
    let req = h.review("review", id, text, "mechanism", "proposed");
    let d = h.call("consumer-review", req.clone());
    assert_eq!(d["status"], "accepted", "{d}");
    let a = h.call("consumer-approve", h.approval("approve", &d));
    assert_eq!(a["status"], "accepted");
    let retracted=h.call("retract",json!({"idempotency_key":"retract","run_id":RUN,"client":"consumer-test","target_id":id,"reason":"Owner withdraws retained content.","confirmation":"confirmed"}));
    assert_eq!(retracted["status"], "accepted", "{retracted}");
    let decisions = h.status("decisions");
    let d = &decisions["result"]["items"][0];
    assert_eq!(d["validity"], "retracted");
    assert!(d.get("reason").is_none());
    assert_eq!(
        h.call("consumer-review", req.clone())["result"]["validity"],
        "retracted"
    );
    let mut fresh = req;
    fresh["idempotency_key"] = json!("fresh-withdrawn");
    assert_eq!(h.call("consumer-review", fresh)["code"], "conflict");
    assert_eq!(
        h.capture("recapture", INCIDENT, 1, text)["code"],
        "conflict"
    );
    let live = h.record("new-record", "Different live observation.");
    let mut direct = h.review(
        "direct-source",
        &live,
        "Different live observation.",
        "lessons",
        "deferred",
    );
    direct["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"source","id":INCIDENT,"revision":1,"sha256":sha("raw source")}));
    assert_eq!(h.call("consumer-review", direct)["code"], "conflict");
    let effect = h.call(
        "publication-result",
        h.publish("late-effect", &a, "applied"),
    );
    assert_eq!(effect["status"], "accepted", "{effect}");
    assert_eq!(effect["result"]["validity"], "retracted");
    assert!(!effect.to_string().contains(text));
}
#[test]
fn concurrent_cli_review_requests_converge_and_competing_generations_have_one_winner() {
    let h = Harness::new();
    let text = "Shared evidence.";
    let id = h.record("record", text);
    let first = h.review("same-key", &id, text, "mechanism", "deferred");
    let results = std::thread::scope(|s| {
        let a = s.spawn(|| h.call("consumer-review", first.clone()));
        let b = s.spawn(|| h.call("consumer-review", first.clone()));
        [a.join().unwrap(), b.join().unwrap()]
    });
    let mut states: Vec<_> = results
        .iter()
        .map(|v| v["status"].as_str().unwrap())
        .collect();
    states.sort();
    assert_eq!(states, vec!["accepted", "replayed"]);
    assert_eq!(
        results[0]["result"]["decision_id"],
        results[1]["result"]["decision_id"]
    );
    let one = h.review("new-key-1", &id, text, "lessons", "deferred");
    let mut two = one.clone();
    two["idempotency_key"] = json!("new-key-2");
    let results = std::thread::scope(|s| {
        let a = s.spawn(|| h.call("consumer-review", one));
        let b = s.spawn(|| h.call("consumer-review", two));
        [a.join().unwrap(), b.join().unwrap()]
    });
    let mut states: Vec<_> = results
        .iter()
        .map(|v| v["status"].as_str().unwrap())
        .collect();
    states.sort();
    assert_eq!(states, vec!["accepted", "noop"]);
    let previous = results[0]["result"]["decision_id"].clone();
    let mut one = h.review("competing-1", &id, text, "lessons", "proposed");
    one["previous_decision_id"] = previous;
    one["reconsider"] = json!(true);
    let mut two = one.clone();
    two["idempotency_key"] = json!("competing-2");
    two["reason"] = json!("Different deliberate reconsideration.");
    let results = std::thread::scope(|s| {
        let a = s.spawn(|| h.call("consumer-review", one));
        let b = s.spawn(|| h.call("consumer-review", two));
        [a.join().unwrap(), b.join().unwrap()]
    });
    let mut states: Vec<_> = results
        .iter()
        .map(|v| v["status"].as_str().unwrap())
        .collect();
    states.sort();
    assert_eq!(states, vec!["accepted", "error"]);
    assert!(results.iter().any(|r| r["code"] == "conflict"));
}
#[test]
fn retained_intents_are_scoped_and_result_hashes_cannot_be_changed() {
    let h = Harness::new();
    let text = "One scoped report.";
    let id = h.record("record", text);
    let d = h.call(
        "consumer-review",
        h.review("review", &id, text, "mechanism", "proposed"),
    );
    let approved = h.call("consumer-approve", h.approval("approve", &d));
    assert_eq!(approved["status"], "accepted");
    let second = "40000000-0000-4000-8000-000000000002";
    let mut profile = Harness::profile();
    profile["workflow_id"] = json!(second);
    profile["collections"] = json!([]);
    let mut bind=h.req("second",json!({"profile_sha256":sha(profile.to_string()),"binding_sha256":sha("second binding"),"profile":profile,"expected_profile_revision":0,"confirmation":"confirmed"}));
    bind["workflow_id"] = json!(second);
    assert_eq!(h.call("workflow-bind", bind)["status"], "accepted");
    let before = h.status("publications");
    let mut crossed = h.approval("wrong-workflow", &d);
    crossed["workflow_id"] = json!(second);
    assert_eq!(
        h.call("consumer-approve", crossed)["code"],
        "scope_mismatch"
    );
    let mut crossed = h.publish("wrong-result-workflow", &approved, "applied");
    crossed["workflow_id"] = json!(second);
    assert_eq!(
        h.call("publication-result", crossed)["code"],
        "scope_mismatch"
    );
    let mut wrong = h.publish("wrong-hash", &approved, "applied");
    wrong["observed_sha256"] = json!(sha("different file"));
    assert_eq!(h.call("publication-result", wrong)["code"], "conflict");
    let mut unknown = h.publish("unknown", &approved, "applied");
    unknown["approval_id"] = json!("a0000000-0000-4000-8000-000000000099");
    assert_eq!(h.call("publication-result", unknown)["code"], "not_found");
    assert_eq!(h.status("publications"), before);
}
#[test]
fn automatic_review_retention_policy_requires_confirmation_and_candidates_are_not_inputs() {
    let h = Harness::new();
    let text = "One scoped report.";
    let id = h.record("record", text);
    let mut profile = Harness::profile();
    profile["revision"] = json!(2);
    profile["capture"]["automatic_reviews"] = json!(false);
    let mut bind=h.req("bind2",json!({"profile_sha256":sha(profile.to_string()),"binding_sha256":sha("synthetic binding"),"profile":profile,"expected_profile_revision":1,"confirmation":"confirmed"}));
    bind["profile_revision"] = json!(2);
    assert_eq!(h.call("workflow-bind", bind)["status"], "accepted");
    let mut req = h.review("review", &id, text, "mechanism", "deferred");
    req["profile_revision"] = json!(2);
    assert_eq!(
        h.call("consumer-review", req.clone())["code"],
        "approval_required"
    );
    req["confirmation"] = json!("confirmed");
    assert_eq!(h.call("consumer-review", req)["status"], "accepted");
    let candidate=h.call("review",json!({"idempotency_key":"lesson","run_id":RUN,"client":"consumer-test","text":"A tentative lesson.","source_ids":[id],"occurred_at":"2026-01-01T10:00:00Z"}));
    assert_eq!(candidate["status"], "accepted", "{candidate}");
    let mut req = h.review(
        "lesson-input",
        candidate["result"]["record_id"].as_str().unwrap(),
        "A tentative lesson.",
        "lessons",
        "deferred",
    );
    req["profile_revision"] = json!(2);
    req["confirmation"] = json!("confirmed");
    assert_eq!(h.call("consumer-review", req)["code"], "invalid_input");
}
#[test]
fn auxiliary_record_dependencies_expand_existing_base_links() {
    let h = Harness::new();
    let one = h.record("one", "Observation one.");
    let two = h.record("two", "Observation two.");
    let candidate=h.call("review",json!({"idempotency_key":"lesson","run_id":RUN,"client":"consumer-test","text":"Tentative link between two observations.","source_ids":[one,two],"occurred_at":"2026-01-01T10:00:00Z"}));
    assert_eq!(candidate["status"], "accepted", "{candidate}");
    let mut req = h.review(
        "consumer",
        &one,
        "Observation one.",
        "mechanism",
        "deferred",
    );
    req["dependencies"].as_array_mut().unwrap().push(json!({"type":"record","id":candidate["result"]["record_id"],"sha256":sha("Tentative link between two observations.")}));
    let r = h.call("consumer-review", req);
    assert_eq!(r["status"], "accepted", "{r}");
    let s = h.status("decisions");
    let deps = s["result"]["items"][0]["dependencies"].as_array().unwrap();
    assert_eq!(deps.len(), 3);
    assert!(deps.iter().any(|d| d["id"] == two));
    let correction = h.correct("correct", &two, "Observation two was unverified.");
    assert_eq!(correction["status"], "accepted", "{correction}");
    assert_eq!(
        h.status("decisions")["result"]["items"][0]["validity"],
        "stale"
    );
}

#[test]
fn maintenance_approval_rejects_a_newer_block_owner_but_preserves_exact_replay() {
    let h = Harness::new();
    let text = "Unchanged source report.";
    let id = h.record("record", text);
    let d = h.call(
        "consumer-review",
        h.review("first", &id, text, "mechanism", "proposed"),
    );
    let a = h.call("consumer-approve", h.approval("approve-first", &d));
    assert_eq!(
        h.call(
            "publication-result",
            h.publish("publish-first", &a, "applied")
        )["status"],
        "accepted"
    );
    let mut next = h.review("second", &id, text, "mechanism", "proposed");
    next["previous_decision_id"] = d["result"]["decision_id"].clone();
    next["reconsider"] = json!(true);
    next["reason"] = json!("Reassess the same source without changing the published body.");
    let successor = h.call("consumer-review", next);
    assert_eq!(successor["status"], "accepted", "{successor}");
    let a2 = h.call("consumer-approve", h.approval("approve-second", &successor));
    assert_eq!(a2["status"], "accepted", "{a2}");
    assert_eq!(
        h.call(
            "publication-result",
            h.publish("publish-second", &a2, "applied")
        )["status"],
        "accepted"
    );
    let mut n = h.approval("old-neutral", &d);
    n.as_object_mut().unwrap().remove("confirmation");
    n["maintenance"] = json!("neutralize");
    n["proposal"]["base_exists"] = json!(true);
    n["proposal"]["output_sha256"] = json!(sha(
        "MemoryNexus: source evidence changed; review required.\n"
    ));
    n["proposal"]["after_sha256"] = json!(sha("neutralized file"));
    let before = h.status("publications");
    let rejected = h.call("consumer-approve", n);
    assert_eq!(rejected["code"], "approval_required", "{rejected}");
    assert_eq!(h.status("publications"), before);
    // Historical approval replay must not be confused with new write authority.
    assert_eq!(
        h.call("consumer-approve", h.approval("approve-first", &d))["status"],
        "replayed"
    );
}
