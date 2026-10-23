use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
fn invoke(args: &[&str], ledger: Option<&Path>, input: &str) -> Value {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_memorynexus-feedback-cli"));
    cmd.args(args).env_remove("MEMORYNEXUS_TASK_LEDGER");
    if let Some(p) = ledger {
        cmd.env("MEMORYNEXUS_TASK_LEDGER", p);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| panic!("{:?}", out));
    assert_eq!(
        out.status.code(),
        Some(if v["status"] == "error" { 2 } else { 0 }),
        "{v}"
    );
    assert!(out.stderr.is_empty(), "{:?}", out.stderr);
    v
}
fn sha(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../docs/repository-memory/examples/sequence-v1.json"
    ))
    .unwrap()
}
#[test]
fn workflow_help_is_storage_free_and_v1_ledger_requires_explicit_migration() {
    let h = invoke(&["task", "--help"], None, "");
    for action in [
        "workflow-bind",
        "source-sync",
        "source-capture",
        "consumer-review",
        "consumer-approve",
        "publication-result",
        "workflow-status",
        "migrate",
    ] {
        assert!(
            h["result"]["commands"]
                .as_array()
                .unwrap()
                .contains(&json!(action)),
            "{h}"
        );
    }
    let d = tempfile::TempDir::new().unwrap();
    let config = d.path().join("task.json");
    let ledger = d.path().join("task.sqlite");
    let init=invoke(&["task","--config",config.to_str().unwrap(),"init"],Some(&ledger),&json!({"idempotency_key":"init","display_name":"test","confirmation":"confirmed","automatic_capture":["execution","review"]}).to_string());
    assert_eq!(init["status"], "accepted");
    let got = invoke(
        &[
            "task",
            "--config",
            config.to_str().unwrap(),
            "workflow-status",
        ],
        Some(&ledger),
        "{}",
    );
    assert_eq!(got["code"], "unsupported_version");
    assert_eq!(got["workflow_contract"], 1);
}
struct Harness {
    _dir: tempfile::TempDir,
    config: PathBuf,
    ledger: PathBuf,
    task_id: String,
    incarnation: String,
}
impl Harness {
    fn new() -> Self {
        Self::new_with_foreign(false)
    }
    fn new_with_foreign(foreign: bool) -> Self {
        let d = tempfile::TempDir::new().unwrap();
        let config = d.path().join("config.json");
        let v1 = d.path().join("v1.sqlite");
        let ledger = d.path().join("v2.sqlite");
        let backup = d.path().join("backup.sqlite");
        let init=invoke(&["task","--config",config.to_str().unwrap(),"init"],Some(&v1),&json!({"idempotency_key":"init","display_name":"synthetic","confirmation":"confirmed","automatic_capture":["execution","review"]}).to_string());
        assert_eq!(init["status"], "accepted", "{init}");
        if foreign {
            let other = d.path().join("other.json");
            let r=invoke(&["task","--config",other.to_str().unwrap(),"init"],Some(&v1),&json!({"idempotency_key":"other-init","display_name":"other task","confirmation":"confirmed","automatic_capture":["execution","review"]}).to_string());
            assert_eq!(r["status"], "accepted");
        }
        let b = invoke(
            &["task", "--ledger", v1.to_str().unwrap(), "backup"],
            None,
            &json!({"path":backup}).to_string(),
        );
        assert_eq!(b["status"], "ok", "{b}");
        let m=invoke(&["task","--ledger",v1.to_str().unwrap(),"migrate"],None,&json!({"path":ledger,"backup_path":backup,"backup_sha256":sha(std::fs::read(&backup).unwrap()),"expected_version":1,"target_version":2,"confirmation":"confirmed"}).to_string());
        assert_eq!(m["status"], "ok", "{m}");
        Self {
            _dir: d,
            config,
            ledger,
            task_id: init["task_id"].as_str().unwrap().to_owned(),
            incarnation: m["result"]["incarnation"].as_str().unwrap().to_owned(),
        }
    }
    fn call(&self, a: &str, v: Value) -> Value {
        invoke(
            &["task", "--config", self.config.to_str().unwrap(), a],
            Some(&self.ledger),
            &v.to_string(),
        )
    }
    fn req(&self, key: &str, fields: Value) -> Value {
        let mut v = json!({"idempotency_key":key,"run_id":"60000000-0000-4000-8000-000000000001","client":"test","workflow_id":"40000000-0000-4000-8000-000000000001","profile_revision":1,"incarnation":self.incarnation});
        for (k, x) in fields.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    }
    fn bind_request(&self) -> Value {
        let p = fixture()["profile"].clone();
        self.req("bind",json!({"expected_profile_revision":0,"profile_sha256":sha(p.to_string()),"binding_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc","profile":p,"confirmation":"confirmed"}))
    }
    fn bind(&self) {
        let r = self.call("workflow-bind", self.bind_request());
        assert_eq!(r["status"], "accepted", "{r}");
    }
    fn status(&self, view: &str) -> Value {
        self.call(
            "workflow-status",
            json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":view}),
        )
    }
}
#[test]
fn binding_is_exact_confirmed_and_replays_before_cas_without_duplicate_revision() {
    let h = Harness::new();
    let r = h.bind_request();
    let mut bad = r.clone();
    bad["profile"]["capture"]["unsupported"] = json!(true);
    assert_eq!(h.call("workflow-bind", bad)["code"], "invalid_input");
    let a = h.call("workflow-bind", r.clone());
    assert_eq!(a["status"], "accepted", "{a}");
    assert_eq!(a["revision"], 2);
    assert_eq!(h.call("workflow-bind", r.clone())["status"], "replayed");
    let mut r2 = r.clone();
    r2["idempotency_key"] = json!("new-bind");
    let n = h.call("workflow-bind", r2);
    assert_eq!(n["status"], "noop", "{n}");
    assert_eq!(n["revision"], 2);
    let header = h.call("workflow-status", json!({}));
    assert_eq!(header["result"]["items"][0]["ledger_version"], 2);
    assert_eq!(header["task_id"], h.task_id);
    let s = h.status("collections");
    assert_eq!(s["result"]["items"].as_array().unwrap().len(), 3);
    assert_eq!(s["result"]["items"][0]["scan"], 0);
    let mut bad = r;
    bad["client"] = json!("different");
    assert_eq!(h.call("workflow-bind", bad)["code"], "conflict");
}
const SOURCE: &str = "50000000-0000-4000-8000-000000000001";
impl Harness {
    fn sync(&self, key: &str, scan: u64, hash: &str) -> Value {
        self.call("source-sync",self.req(key,json!({"collection_id":SOURCE,"expected_scan":scan,"observed_at":"2026-10-23T00:00:00Z","availability":"available","units":[{"sha256":hash}]})))
    }
    fn capture(&self, key: &str, rev: u64, text: &str) -> Value {
        self.call("source-capture",self.req(key,json!({"source_id":SOURCE,"source_revision":rev,"extractor_version":"bounded-v1","text":text,"occurred_at":"2026-10-22T00:00:00Z"})))
    }
}
#[test]
fn sources_capture_bounded_drafts_and_refresh_one_chain_without_context_promotion() {
    let h = Harness::new();
    h.bind();
    let missing=h.req("missing",json!({"collection_id":SOURCE,"expected_scan":0,"observed_at":"2026-10-23T00:00:00Z","availability":"unavailable","reason":"missing","units":[]}));
    let m = h.call("source-sync", missing);
    assert_eq!(m["status"], "accepted", "{m}");
    assert_eq!(m["result"]["sources"], json!([]));
    assert_eq!(h.status("sources")["result"]["items"], json!([]));
    let s = h.sync("sync", 1, &sha("source"));
    assert_eq!(s["status"], "accepted", "{s}");
    assert_eq!(s["result"]["sources"][0]["source_revision"], 1);
    assert_eq!(h.sync("same", 0, &sha("source"))["status"], "noop");
    let c = h.capture(
        "capture",
        1,
        "Observed target absent; fallback remains unverified.",
    );
    assert_eq!(c["status"], "accepted", "{c}");
    assert_eq!(c["result"]["trust"], "source_reported");
    assert_eq!(h.call("context", json!({}))["result"]["entries"], json!([]));
    assert_eq!(h.status("inputs")["result"]["items"][0]["kind"], "draft");
    assert_eq!(
        h.capture(
            "same-capture",
            1,
            "Observed target absent; fallback remains unverified."
        )["status"],
        "noop"
    );
    assert_eq!(
        h.capture("different", 1, "Different content")["code"],
        "conflict"
    );
    let changed = h.sync("changed", 2, &sha("changed"));
    assert_eq!(changed["status"], "accepted", "{changed}");
    assert_eq!(changed["result"]["invalidated"]["records"], 1);
    let stale = h.capture(
        "capture",
        1,
        "Observed target absent; fallback remains unverified.",
    );
    assert_eq!(stale["status"], "replayed");
    assert_eq!(stale["result"]["state"], "stale");
    let fresh = h.capture(
        "fresh",
        2,
        "The revised document reports a successful check.",
    );
    assert_eq!(fresh["status"], "accepted", "{fresh}");
    assert_eq!(fresh["result"]["root_record_id"], c["result"]["record_id"]);
    assert_ne!(fresh["result"]["record_id"], c["result"]["record_id"]);
}
#[test]
fn append_streams_keep_offset_identity_and_refresh_all_known_rows_at_eof() {
    let h = Harness::new();
    let f = fixture()["jsonl_identity_case"].clone();
    let cid = f["collection_id"].clone();
    let mut bind = h.bind_request();
    bind["profile"]["collections"] = json!([{"collection_id":cid,"format":"append_jsonl","path":"inbox/events.jsonl","selector":{"kind":"fields","keys":["body"]},"extractor_version":"bounded-v1","authority":"source report"}]);
    bind["profile_sha256"] = json!(sha(bind["profile"].to_string()));
    assert_eq!(h.call("workflow-bind", bind.clone())["status"], "accepted");
    let units: Vec<Value> = f["units_with_derived_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| json!({"start":u["start"],"end":u["end"],"sha256":u["sha256"]}))
        .collect();
    let req=h.req("stream1",json!({"collection_id":cid,"expected_scan":0,"observed_at":"2026-10-23T00:00:00Z","availability":"available","units":units,"cursor":f["cursor"]}));
    let first = h.call("source-sync", req.clone());
    assert_eq!(first["status"], "accepted", "{first}");
    for (i, u) in f["units_with_derived_ids"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(first["result"]["sources"][i]["source_id"], u["source_id"]);
    }
    let mut replay = req.clone();
    replay["idempotency_key"] = json!("same-stream");
    assert_eq!(h.call("source-sync", replay)["status"], "noop");
    let mut conflict = req;
    conflict["idempotency_key"] = json!("overlap");
    conflict["units"][0]["sha256"] = json!(sha("edited"));
    assert_eq!(h.call("source-sync", conflict)["code"], "conflict");
    // Append 19 more raw rows: metadata is caller-asserted; timestamps are not a cursor.
    let units: Vec<Value> = (0..19)
        .map(|i| json!({"start":214+i*10,"end":224+i*10,"sha256":sha("same row\n")}))
        .collect();
    let cursor = json!({"offset":404,"prefix_sha256":sha("full verified prefix")});
    let second=h.call("source-sync",h.req("stream2",json!({"collection_id":cid,"expected_scan":1,"observed_at":"2026-10-23T00:00:00Z","availability":"available","units":units,"cursor":cursor})));
    assert_eq!(second["status"], "accepted", "{second}");
    let page = h.status("sources");
    assert_eq!(page["result"]["items"].as_array().unwrap().len(), 20);
    assert_eq!(page["result"]["complete"], false);
    let rev = page["revision"].clone();
    let after = page["result"]["next_after"].clone();
    let tail = h.call(
        "workflow-status",
        json!({"workflow_id":bind["workflow_id"],"view":"sources","at_revision":rev,"after":after}),
    );
    assert_eq!(tail["result"]["items"].as_array().unwrap().len(), 2);
    bind["idempotency_key"] = json!("refresh-profile");
    bind["expected_profile_revision"] = json!(1);
    bind["profile_revision"] = json!(2);
    bind["profile"]["revision"] = json!(2);
    bind["profile"]["collections"][0]["extractor_version"] = json!("bounded-v2");
    bind["profile_sha256"] = json!(sha(bind["profile"].to_string()));
    assert_eq!(h.call("workflow-bind", bind)["status"], "accepted");
    let mut eof=h.req("eof-refresh",json!({"collection_id":cid,"expected_scan":2,"observed_at":"2026-10-23T00:00:00Z","availability":"available","units":[],"cursor":cursor}));
    eof["profile_revision"] = json!(2);
    let out = h.call("source-sync", eof.clone());
    assert_eq!(out["status"], "accepted", "{out}");
    assert_eq!(out["result"]["sources"], json!([]));
    assert_eq!(out["result"]["cursor"], cursor);
    let page = h.status("sources");
    assert!(page["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["source_revision"] == 2));
    let tail=h.call("workflow-status",json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":"sources","at_revision":page["revision"],"after":page["result"]["next_after"]}));
    assert!(tail["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["source_revision"] == 2));
    assert_eq!(h.call("workflow-status",json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":"sources","at_revision":rev,"after":after}))["code"],"conflict");
    eof["idempotency_key"] = json!("same-eof");
    assert_eq!(h.call("source-sync", eof)["status"], "noop");
}
#[test]
fn confirmed_correction_preserves_draft_origin_and_retraction_scrubs_full_lesson_chains() {
    let h = Harness::new();
    h.bind();
    h.sync("s", 0, &sha("source"));
    let d1 = h.capture("d1", 1, "Source confidential observation.");
    let r1 = d1["result"]["record_id"].clone();
    let legacy = |key: &str, text: &str| json!({"idempotency_key":key,"run_id":"60000000-0000-4000-8000-000000000001","client":"test","text":text,"occurred_at":"2026-10-23T00:00:00Z"});
    let mut review = legacy("lesson", "Derived confidential lesson.");
    review["source_ids"] = json!([r1]);
    let l1 = h.call("review", review.clone());
    assert_eq!(l1["status"], "accepted", "{l1}");
    let mut correct = legacy("lesson-correct", "Endorsed confidential lesson.");
    correct["target_id"] = l1["result"]["record_id"].clone();
    correct["confirmation"] = json!("confirmed");
    let l2 = h.call("correct", correct);
    assert_eq!(l2["status"], "accepted", "{l2}");
    let mut correct = legacy("draft-correct", "Corrected observation.");
    correct["target_id"] = r1.clone();
    correct["confirmation"] = json!("confirmed");
    let d2 = h.call("correct", correct);
    assert_eq!(d2["status"], "accepted", "{d2}");
    let repeated = h.capture("new-key", 1, "Source confidential observation.");
    assert_eq!(repeated["status"], "noop", "{repeated}");
    assert_eq!(repeated["result"]["record_id"], r1);
    assert_eq!(repeated["result"]["state"], "superseded");
    assert_eq!(h.call("context", json!({}))["result"]["entries"], json!([]));
    let head = h.status("inputs");
    assert_eq!(head["result"]["items"][0]["id"], d2["result"]["record_id"]);
    assert_eq!(head["result"]["items"][0]["origin"]["source_id"], SOURCE);
    let retracted=h.call("retract",json!({"idempotency_key":"retract","run_id":"60000000-0000-4000-8000-000000000001","client":"test","target_id":r1,"reason":"withdraw retention","confirmation":"confirmed"}));
    assert_eq!(retracted["status"], "accepted", "{retracted}");
    let exported = invoke(
        &["task", "--ledger", h.ledger.to_str().unwrap(), "export"],
        None,
        "{}",
    );
    for r in exported["result"]["records"].as_array().unwrap() {
        assert!(r.get("text").is_none() || r["text"] == "", "{r}");
        assert!(r.get("evidence_refs").is_none() || r["evidence_refs"] == json!([]));
    }
    assert_eq!(h.status("sources")["result"]["items"][0]["blocked"], true);
    assert_eq!(
        h.capture("after-retract", 1, "Source confidential observation.")["code"],
        "conflict"
    );
    let replay = h.call("review", review);
    assert_eq!(replay["status"], "replayed");
    assert_eq!(replay["result"]["state"], "superseded");
    assert!(!replay.to_string().contains("confidential"));
}
#[test]
fn strict_json_scope_and_shared_receipt_namespace_reject_without_mutation() {
    let h = Harness::new();
    h.bind();
    let before = invoke(
        &["task", "--ledger", h.ledger.to_str().unwrap(), "export"],
        None,
        "{}",
    );
    for text in [
        "{\"view\":\"summary\",\"view\":\"sources\"}",
        "{\"view\":null}",
        "{\"limit\":0}",
        "{\"limit\":1.0}",
        "{\"limit\":9007199254740992}",
        "{\"view\":5}",
        "{\"extra\":true}",
    ] {
        let r = invoke(
            &[
                "task",
                "--config",
                h.config.to_str().unwrap(),
                "workflow-status",
            ],
            Some(&h.ledger),
            text,
        );
        assert_eq!(r["code"], "invalid_input", "{text}: {r}");
    }
    let old=h.call("record",json!({"idempotency_key":"bind","run_id":"60000000-0000-4000-8000-000000000001","client":"test","kind":"execution","text":"unaccepted","occurred_at":"2026-10-23T00:00:00Z"}));
    assert_eq!(old["code"], "conflict", "{old}");
    let new_config = h._dir.path().join("new.json");
    let init=invoke(&["task","--config",new_config.to_str().unwrap(),"init"],Some(&h.ledger),&json!({"idempotency_key":"another","display_name":"cannot implicitly init v2","confirmation":"confirmed","automatic_capture":[]}).to_string());
    assert_eq!(init["code"], "unsupported_version");
    assert!(!new_config.exists());
    let after = invoke(
        &["task", "--ledger", h.ledger.to_str().unwrap(), "export"],
        None,
        "{}",
    );
    assert_eq!(before, after);
}
#[test]
fn profile_can_select_existing_unicode_and_camel_case_json_field_names() {
    let h = Harness::new();
    let mut b = h.bind_request();
    b["profile"]["collections"][0]["format"] = json!("append_jsonl");
    b["profile"]["collections"][0]["selector"] =
        json!({"kind":"fields","keys":["事件正文","eventTime"]});
    b["profile_sha256"] = json!(sha(b["profile"].to_string()));
    let r = h.call("workflow-bind", b);
    assert_eq!(r["status"], "accepted", "{r}");
}
#[test]
fn concurrent_processes_converge_on_keys_business_identity_and_source_cas() {
    let h = Harness::new();
    h.bind();
    let a = std::thread::scope(|scope| {
        let x = scope.spawn(|| h.sync("same", 0, &sha("first")));
        let y = scope.spawn(|| h.sync("same", 0, &sha("first")));
        [x.join().unwrap(), y.join().unwrap()]
    });
    let mut statuses: Vec<_> = a.iter().map(|x| x["status"].as_str().unwrap()).collect();
    statuses.sort();
    assert_eq!(statuses, ["accepted", "replayed"]);
    let a = std::thread::scope(|scope| {
        let x = scope.spawn(|| h.capture("c1", 1, "reported result"));
        let y = scope.spawn(|| h.capture("c2", 1, "reported result"));
        [x.join().unwrap(), y.join().unwrap()]
    });
    let mut statuses: Vec<_> = a.iter().map(|x| x["status"].as_str().unwrap()).collect();
    statuses.sort();
    assert_eq!(statuses, ["accepted", "noop"]);
    assert_eq!(a[0]["result"]["record_id"], a[1]["result"]["record_id"]);
    let a = std::thread::scope(|scope| {
        let x = scope.spawn(|| h.sync("new-a", 1, &sha("second-a")));
        let y = scope.spawn(|| h.sync("new-b", 1, &sha("second-b")));
        [x.join().unwrap(), y.join().unwrap()]
    });
    assert_eq!(
        a.iter().filter(|x| x["status"] == "accepted").count(),
        1,
        "{a:?}"
    );
    assert_eq!(
        a.iter().filter(|x| x["code"] == "conflict").count(),
        1,
        "{a:?}"
    );
    assert_eq!(
        h.status("sources")["result"]["items"][0]["source_revision"],
        2
    );
}
#[test]
fn capture_requires_retention_authority_and_source_changes_do_not_endorse_or_scrub() {
    let h = Harness::new();
    let mut b = h.bind_request();
    b["profile"]["capture"]["automatic_drafts"] = json!(false);
    b["profile_sha256"] = json!(sha(b["profile"].to_string()));
    assert_eq!(h.call("workflow-bind", b)["status"], "accepted");
    h.sync("s", 0, &sha("body"));
    assert_eq!(
        h.capture("c", 1, "uncertain source report")["code"],
        "approval_required"
    );
    let mut c=h.req("c",json!({"source_id":SOURCE,"source_revision":1,"extractor_version":"bounded-v1","text":"uncertain source report","occurred_at":"2026-10-23T00:00:00Z","confirmation":"confirmed"}));
    let r = h.call("source-capture", c.clone());
    assert_eq!(r["status"], "accepted", "{r}");
    assert_eq!(r["result"]["trust"], "source_reported");
    c["idempotency_key"] = json!("trust-launder");
    c["verified"] = json!(true);
    assert_eq!(h.call("source-capture", c)["code"], "invalid_input");
    let missing=h.req("gone",json!({"collection_id":SOURCE,"expected_scan":1,"observed_at":"2026-10-23T01:00:00Z","availability":"unavailable","units":[],"reason":"unreadable"}));
    let gone = h.call("source-sync", missing.clone());
    assert_eq!(gone["status"], "accepted", "{gone}");
    let mut same = missing;
    same["idempotency_key"] = json!("gone-again");
    assert_eq!(h.call("source-sync", same)["status"], "noop");
    let e = invoke(
        &["task", "--ledger", h.ledger.to_str().unwrap(), "export"],
        None,
        "{}",
    );
    let record = &e["result"]["records"][0];
    assert_eq!(record["text"], "uncertain source report");
    assert_eq!(record["state"], "stale");
    assert_eq!(record["trust"], "source_reported");
    let back = h.sync("back", 2, &sha("body"));
    assert_eq!(back["result"]["sources"][0]["source_revision"], 3);
    assert_eq!(h.status("sources")["result"]["items"][0]["blocked"], false);
}
#[test]
fn inspection_is_revision_pinned_ordered_and_does_not_cross_tasks() {
    let h = Harness::new_with_foreign(true);
    h.bind();
    h.sync("source", 0, &sha("body"));
    let draft = h.capture("draft", 1, "retained source report");
    let x=h.call("record",json!({"idempotency_key":"ordinary","run_id":"60000000-0000-4000-8000-000000000001","client":"test","kind":"execution","text":"execution","occurred_at":"2026-10-23T00:00:00Z"}));
    let rid = draft["result"]["record_id"].clone();
    let xid = x["result"]["record_id"].clone();
    let input=h.call("workflow-status",json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":"inputs","ids":[xid,rid]}));
    assert_eq!(input["result"]["items"][0]["id"], xid);
    assert_eq!(input["result"]["items"][1]["id"], rid);
    let receipts=h.call("workflow-status",json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":"receipts","keys":["ordinary","draft","bind"]}));
    assert_eq!(receipts["result"]["items"][0]["result_id"], xid);
    assert_eq!(receipts["result"]["items"][1]["result"]["record_id"], rid);
    assert_eq!(receipts["result"]["items"][2]["action"], "workflow-bind");
    let other = h._dir.path().join("other.json");
    let cross = invoke(
        &[
            "task",
            "--config",
            other.to_str().unwrap(),
            "workflow-status",
        ],
        Some(&h.ledger),
        &json!({"workflow_id":"40000000-0000-4000-8000-000000000001"}).to_string(),
    );
    assert_eq!(cross["code"], "scope_mismatch");
    let unknown=h.call("workflow-status",json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":"inputs","ids":["10000000-0000-4000-8000-000000000099"]}));
    assert_eq!(unknown["code"], "not_found");
    let duplicate=h.call("workflow-status",json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":"inputs","ids":[rid,rid]}));
    assert_eq!(duplicate["code"], "invalid_input");
}
#[test]
fn blocked_drafts_are_not_pending_inputs_but_explicit_tombstones_are_inspectable() {
    let h = Harness::new();
    h.bind();
    h.sync("s", 0, &sha("body"));
    let d = h.capture("d", 1, "private bounded note");
    let id = d["result"]["record_id"].clone();
    let r=h.call("retract",json!({"idempotency_key":"withdraw","run_id":"60000000-0000-4000-8000-000000000001","client":"test","target_id":id,"reason":"withdraw","confirmation":"confirmed"}));
    assert_eq!(r["status"], "accepted");
    assert_eq!(h.status("inputs")["result"]["items"], json!([]));
    let e = h.call(
        "workflow-status",
        json!({"workflow_id":"40000000-0000-4000-8000-000000000001","view":"inputs","ids":[id]}),
    );
    assert_eq!(e["result"]["items"][0]["state"], "retracted");
    assert!(e["result"]["items"][0].get("text").is_none());
}
