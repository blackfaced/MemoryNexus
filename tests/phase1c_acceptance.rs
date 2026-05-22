//! Local CLI acceptance test.
//!
//! This test is ignored by default because it requires local PostgreSQL and
//! Qdrant. Run it with:
//!
//! ```bash
//! docker compose up -d postgres qdrant
//! createdb -h localhost -U postgres memorynexus_acceptance
//! MEMORYNEXUS_ACCEPTANCE=1 \
//! MEMORYNEXUS_ACCEPTANCE_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/memorynexus_acceptance \
//! QDRANT_URL=http://localhost:6333 \
//! MEMORYNEXUS_EMBEDDING_PROVIDER=local \
//! cargo test --test phase1c_acceptance -- --ignored --nocapture
//! ```

use std::{
    net::TcpListener,
    panic,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

#[test]
#[ignore = "requires local PostgreSQL and Qdrant"]
fn cli_drives_space_memory_semantic_search_and_lens_run() {
    require_acceptance_env();

    let run_id = unique_run_id();
    let collection = format!("memorynexus_phase1c_{run_id}");
    let api_port = free_port();
    let api_url = format!("http://127.0.0.1:{api_port}");
    let mut server = start_server(&collection, api_port);

    let result = panic::catch_unwind(|| run_acceptance_flow(&run_id, &api_url));
    let _ = server.kill();
    let _ = server.wait();

    if let Err(error) = result {
        panic::resume_unwind(error);
    }
}

fn require_acceptance_env() {
    assert_eq!(
        std::env::var("MEMORYNEXUS_ACCEPTANCE").as_deref(),
        Ok("1"),
        "set MEMORYNEXUS_ACCEPTANCE=1 to run the local acceptance test"
    );
}

fn run_acceptance_flow(run_id: &str, api_url: &str) {
    wait_for_health(api_url);

    let email = format!("phase1c-{run_id}@example.com");
    let auth = cli(
        [
            "auth",
            "register",
            "--email",
            &email,
            "--name",
            "Phase1C",
            "--password",
            "secret123",
        ],
        None,
        api_url,
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
            "Phase 1C Acceptance Space",
            "--description",
            "End-to-end cognitive space acceptance",
        ],
        Some(&token),
        api_url,
    );
    assert_ok(&space);
    let space_id = space["data"]["id"]
        .as_str()
        .expect("space response should include data.id")
        .to_string();

    let content = format!("phase1c semantic qdrant smoke memory {run_id}");
    let memory = cli(
        [
            "memory",
            "add",
            "--space",
            &space_id,
            "--title",
            "Phase 1C acceptance memory",
            "--content",
            &content,
            "--tags",
            "phase1c,acceptance,semantic",
        ],
        Some(&token),
        api_url,
    );
    assert_ok(&memory);

    let keyword = cli(
        [
            "search",
            "phase1c semantic qdrant",
            "--space",
            &space_id,
            "--limit",
            "5",
        ],
        Some(&token),
        api_url,
    );
    assert_search_hit(&keyword, "keyword", &space_id);

    let semantic = cli(
        [
            "search",
            "phase1c semantic qdrant",
            "--space",
            &space_id,
            "--semantic",
            "--limit",
            "5",
        ],
        Some(&token),
        api_url,
    );
    assert_search_hit(&semantic, "semantic", &space_id);

    let lens = cli(
        [
            "lens",
            "create",
            "--space",
            &space_id,
            "--name",
            "Phase 2A Lens",
            "--description",
            "Acceptance Lens",
            "--strategy",
            "project_context",
            "--output",
            "brief",
            "--retrieval",
            "semantic",
        ],
        Some(&token),
        api_url,
    );
    assert_ok(&lens);
    let lens_id = lens["data"]["id"]
        .as_str()
        .expect("lens response should include data.id")
        .to_string();
    assert_eq!(lens["data"]["space_id"], Value::String(space_id.clone()));

    let lens_list = cli(
        ["lens", "list", "--space", &space_id],
        Some(&token),
        api_url,
    );
    assert_ok(&lens_list);
    let lenses = lens_list["data"]["items"]
        .as_array()
        .expect("lens list response should include data.items");
    assert!(
        lenses.iter().any(|lens| lens["id"] == lens_id),
        "lens list should include created lens: {lens_list}"
    );

    let lens_get = cli(["lens", "get", &lens_id], Some(&token), api_url);
    assert_ok(&lens_get);
    assert_eq!(lens_get["data"]["id"], Value::String(lens_id.clone()));

    let lens_run = cli(
        [
            "lens",
            "run",
            &lens_id,
            "--query",
            "phase1c semantic qdrant",
            "--limit",
            "5",
        ],
        Some(&token),
        api_url,
    );
    assert_ok(&lens_run);
    assert_eq!(lens_run["data"]["lens_id"], Value::String(lens_id));
    assert_eq!(lens_run["data"]["space_id"], Value::String(space_id));
    assert_eq!(
        lens_run["data"]["status"],
        Value::String("completed".to_string())
    );
    let run_id = lens_run["data"]["id"]
        .as_str()
        .expect("lens run response should include data.id")
        .to_string();
    let input_memory_ids = lens_run["data"]["input_memory_ids"]
        .as_array()
        .expect("lens run response should include input_memory_ids");
    assert!(
        !input_memory_ids.is_empty(),
        "lens run should record recalled memories: {lens_run}"
    );
    assert_eq!(
        lens_run["data"]["output"]["query"],
        Value::String("phase1c semantic qdrant".to_string())
    );
    assert_eq!(
        lens_run["data"]["output"]["search_mode"],
        Value::String("semantic".to_string())
    );

    let lens_run_get = cli(["lens", "run", "get", &run_id], Some(&token), api_url);
    assert_ok(&lens_run_get);
    assert_eq!(lens_run_get["data"]["id"], Value::String(run_id));
}

fn start_server(collection: &str, api_port: u16) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_memorynexus"));
    command
        .env("DATABASE_URL", acceptance_database_url())
        .env("MEMORYNEXUS_BIND_ADDR", format!("127.0.0.1:{api_port}"))
        .env("QDRANT_COLLECTION", collection)
        .env("MEMORYNEXUS_EMBEDDING_PROVIDER", "local")
        .env(
            "QDRANT_URL",
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string()),
        )
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command.spawn().expect("failed to start memorynexus API")
}

fn wait_for_health(api_url: &str) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(output) = Command::new(env!("CARGO_BIN_EXE_memorynexus-cli"))
            .env("MEMORYNEXUS_API_URL", api_url)
            .arg("health")
            .output()
        {
            if output.status.success() {
                return;
            }
        }
        thread::sleep(Duration::from_millis(250));
    }

    panic!("memorynexus API did not become healthy on {api_url}");
}

fn cli<const N: usize>(args: [&str; N], token: Option<&str>, api_url: &str) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_memorynexus-cli"));
    command.args(args);
    command.env("MEMORYNEXUS_API_URL", api_url);

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

fn assert_search_hit(response: &Value, mode: &str, space_id: &str) {
    assert_ok(response);
    assert_eq!(
        response["data"]["search_mode"],
        Value::String(mode.to_string())
    );

    let items = response["data"]["items"]
        .as_array()
        .expect("search response should include data.items");
    assert!(
        !items.is_empty(),
        "search should return at least one item: {response}"
    );
    assert_eq!(items[0]["space_id"], Value::String(space_id.to_string()));
}

fn unique_run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_millis();
    format!("{millis}")
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("should bind a free local port")
        .local_addr()
        .expect("should read bound local addr")
        .port()
}

fn acceptance_database_url() -> String {
    resolve_acceptance_database_url(
        std::env::var("MEMORYNEXUS_ACCEPTANCE_DATABASE_URL").ok(),
        std::env::var("DATABASE_URL").ok(),
    )
}

fn resolve_acceptance_database_url(
    acceptance_database_url: Option<String>,
    database_url: Option<String>,
) -> String {
    acceptance_database_url.or(database_url).unwrap_or_else(|| {
        "postgresql://postgres:postgres@localhost:5432/memorynexus_acceptance".to_string()
    })
}

#[test]
fn acceptance_database_url_prefers_dedicated_database() {
    assert_eq!(
        resolve_acceptance_database_url(
            Some("postgresql://example/acceptance".to_string()),
            Some("postgresql://example/default".to_string())
        ),
        "postgresql://example/acceptance"
    );
}

#[test]
fn acceptance_database_url_falls_back_to_safe_database_name() {
    assert_eq!(
        resolve_acceptance_database_url(None, None),
        "postgresql://postgres:postgres@localhost:5432/memorynexus_acceptance"
    );
}
