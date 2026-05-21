//! OpenRouter Lens Run acceptance test.
//!
//! Ignored by default because it requires local PostgreSQL, Qdrant, network
//! access, and an OpenRouter API key. Run it with:
//!
//! ```bash
//! docker compose up -d postgres qdrant
//! MEMORYNEXUS_OPENROUTER_ACCEPTANCE=1 \
//! OPENROUTER_API_KEY=sk-or-v1-... \
//! QDRANT_URL=http://localhost:6333 \
//! MEMORYNEXUS_EMBEDDING_PROVIDER=local \
//! cargo test --test openrouter_acceptance -- --ignored --nocapture
//! ```

use std::{
    panic,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

#[test]
#[ignore = "requires local PostgreSQL, Qdrant, network, and OPENROUTER_API_KEY"]
fn lens_run_uses_openrouter_summary_provider() {
    require_openrouter_acceptance_env();

    let run_id = unique_run_id();
    let collection = format!("memorynexus_openrouter_{run_id}");
    let mut server = start_server(&collection);

    let result = panic::catch_unwind(|| run_openrouter_flow(&run_id));
    let _ = server.kill();
    let _ = server.wait();

    if let Err(error) = result {
        panic::resume_unwind(error);
    }
}

fn require_openrouter_acceptance_env() {
    assert_eq!(
        std::env::var("MEMORYNEXUS_OPENROUTER_ACCEPTANCE").as_deref(),
        Ok("1"),
        "set MEMORYNEXUS_OPENROUTER_ACCEPTANCE=1 to run the OpenRouter acceptance test"
    );
    let key_length = std::env::var("OPENROUTER_API_KEY")
        .map(|key| key.trim().len())
        .unwrap_or_default();
    assert!(
        key_length > 20,
        "OPENROUTER_API_KEY must be present and look like a real key"
    );
}

fn run_openrouter_flow(run_id: &str) {
    wait_for_health();

    let email = format!("openrouter-{run_id}@example.com");
    let auth = cli(
        [
            "auth",
            "register",
            "--email",
            &email,
            "--name",
            "OpenRouterAcceptance",
            "--password",
            "secret123",
        ],
        None,
    );
    assert_ok(&auth);
    let token = auth["data"]["token"]
        .as_str()
        .expect("auth response should include data.token")
        .to_string();

    let space = cli(
        [
            "space",
            "create",
            "--name",
            "OpenRouter Acceptance Space",
            "--description",
            "Provider-backed Lens Run acceptance",
        ],
        Some(&token),
    );
    assert_ok(&space);
    let space_id = space["data"]["id"]
        .as_str()
        .expect("space response should include data.id")
        .to_string();

    let content = "MemoryNexus is a personal cognitive substrate. Phase 2 turns Lens into a runnable interpretation strategy with provenance.";
    let memory = cli(
        [
            "memory",
            "add",
            "--space",
            &space_id,
            "--title",
            "OpenRouter acceptance memory",
            "--content",
            content,
            "--tags",
            "openrouter,acceptance,lens",
        ],
        Some(&token),
    );
    assert_ok(&memory);

    let lens = cli(
        [
            "lens",
            "create",
            "--space",
            &space_id,
            "--name",
            "Project Context",
            "--strategy",
            "project_context",
            "--output",
            "brief",
            "--retrieval",
            "semantic",
        ],
        Some(&token),
    );
    assert_ok(&lens);
    let lens_id = lens["data"]["id"]
        .as_str()
        .expect("lens response should include data.id")
        .to_string();

    let lens_run = cli(
        [
            "lens",
            "run",
            &lens_id,
            "--query",
            "总结 MemoryNexus 当前的项目方向",
            "--limit",
            "5",
        ],
        Some(&token),
    );
    assert_ok(&lens_run);
    assert_eq!(
        lens_run["data"]["status"],
        Value::String("completed".to_string())
    );
    assert_eq!(
        lens_run["data"]["output"]["summary_provider"],
        Value::String("openrouter".to_string())
    );
    assert_eq!(
        lens_run["data"]["output"]["summary_source"],
        Value::String("ai".to_string())
    );
    assert!(
        lens_run["data"]["output"]["summary_fallback_reason"].is_null(),
        "OpenRouter summary should not fallback: {lens_run}"
    );
    assert!(
        lens_run["data"]["output"]["summary"]
            .as_str()
            .map(|summary| !summary.trim().is_empty())
            .unwrap_or(false),
        "OpenRouter summary should be non-empty: {lens_run}"
    );
}

fn start_server(collection: &str) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_memorynexus"));
    command
        .env("QDRANT_COLLECTION", collection)
        .env("MEMORYNEXUS_EMBEDDING_PROVIDER", "local")
        .env("MEMORYNEXUS_SUMMARY_MODEL", "openrouter/free")
        .env("LENS_RUN_SUMMARY_MAX_WORDS", "120")
        .env(
            "QDRANT_URL",
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string()),
        )
        .env(
            "OPENROUTER_API_KEY",
            std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY required"),
        )
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        command.env("DATABASE_URL", database_url);
    }

    command.spawn().expect("failed to start memorynexus API")
}

fn wait_for_health() {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(output) = Command::new(env!("CARGO_BIN_EXE_memorynexus-cli"))
            .arg("health")
            .output()
        {
            if output.status.success() {
                return;
            }
        }
        thread::sleep(Duration::from_millis(250));
    }

    panic!("memorynexus API did not become healthy on http://localhost:8080");
}

fn cli<const N: usize>(args: [&str; N], token: Option<&str>) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_memorynexus-cli"));
    command.args(args);

    if let Some(token) = token {
        command.env("MEMORYNEXUS_TOKEN", token);
    }

    let output = command.output().expect("failed to run memorynexus-cli");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "CLI command failed\nstdout: {stdout}\nstderr: {stderr}"
    );

    serde_json::from_slice(&output.stdout).expect("CLI stdout should be valid JSON")
}

fn assert_ok(response: &Value) {
    assert_eq!(response["ok"], Value::Bool(true), "response: {response}");
}

fn unique_run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis();
    format!("{millis}")
}
