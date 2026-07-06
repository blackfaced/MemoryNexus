use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use axum::Router;
use memorynexus::{
    auth::JwtAuth,
    db::{
        self, feedback_loop::PostgresFeedbackLoopRepository, lens::PostgresLensRepository,
        lens_run::PostgresLensRunRepository, memory::PostgresMemoryRepository,
        namespace::PostgresNamespaceRepository, profile::PostgresCognitiveProfileRepository,
        reminder::PostgresReminderRepository,
        review_report::PostgresCognitiveReviewReportRepository,
        space::PostgresCognitiveSpaceRepository, tag::PostgresTagRepository,
        trace::PostgresTraceRepository, user::PostgresUserRepository,
    },
    state::{AppState, Repositories},
    vector::repository::{MemoryVector, RepositoryError, VectorRepository, VectorSearchResult},
};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn planning_surface_generates_next_task_and_writes_planning_trace() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "planning",
            "action": "generate_next_task",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "objective": "Review the because spelling pattern"
            },
            "context": {
                "mode": "focused",
                "locale": "en-US",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("response should be json");
    let trace_id = uuid_field(&body, "/data/generated_trace_id");

    assert_eq!(
        body.pointer("/data/surface").and_then(Value::as_str),
        Some("planning")
    );
    assert_eq!(
        body.pointer("/data/action").and_then(Value::as_str),
        Some("generate_next_task")
    );
    assert_eq!(
        body.pointer("/data/result/status").and_then(Value::as_str),
        Some("next_task_ready")
    );
    assert_eq!(
        body.pointer("/data/result/space_id")
            .and_then(Value::as_str),
        Some(fixture.space_id.to_string().as_str())
    );
    assert_eq!(
        body.pointer("/data/result/namespace_id")
            .and_then(Value::as_str),
        Some(fixture.namespace_id.to_string().as_str())
    );
    assert_eq!(
        body.pointer("/data/result/namespace")
            .and_then(Value::as_str),
        Some("child.english.spelling")
    );
    assert_eq!(
        body.pointer("/data/result/plan_kind")
            .and_then(Value::as_str),
        Some("response_only_draft")
    );
    assert_eq!(
        body.pointer("/data/result/persistence")
            .and_then(Value::as_str),
        Some("not_persisted")
    );
    assert_eq!(
        body.pointer("/data/result/next_task/prompt")
            .and_then(Value::as_str),
        Some("Review the because spelling pattern")
    );
    assert_eq!(body.pointer("/data/result/practice_plan_id"), None);

    let trace: PlanningTraceRow = sqlx::query_as(
        r#"
        SELECT
            space_id,
            namespace_id,
            source_type,
            task_type,
            mode,
            runtime,
            model_provider,
            output_summary,
            metadata
        FROM traces
        WHERE id = $1
        "#,
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("planning trace should exist");

    assert_eq!(trace.space_id, fixture.space_id);
    assert_eq!(trace.namespace_id, Some(fixture.namespace_id));
    assert_eq!(trace.source_type, "mcp");
    assert_eq!(trace.task_type, "planning");
    assert_eq!(trace.mode, "focused");
    assert_eq!(trace.runtime, "deterministic");
    assert_eq!(trace.model_provider.as_deref(), Some("deterministic"));
    assert_eq!(
        trace.output_summary.as_deref(),
        Some("Generated response-only next task for child.english.spelling: Review the because spelling pattern")
    );
    assert_eq!(trace.metadata["surface"], json!("planning"));
    assert_eq!(trace.metadata["action"], json!("generate_next_task"));
    assert_eq!(trace.metadata["adapter"], json!("mcp"));
    assert_eq!(trace.metadata["deterministic"], json!(true));
    assert_eq!(trace.metadata["plan_kind"], json!("response_only_draft"));
    assert_eq!(trace.metadata["persistence"], json!("not_persisted"));
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn planning_surface_adjusts_plan_and_writes_planning_trace() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "planning",
            "action": "adjust_plan",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "objective": "Fit tomorrow practice into one short review",
                "proposed_plan": {
                    "title": "Tomorrow practice",
                    "steps": ["review because", "review friend", "write five sentences"]
                },
                "evidence": [
                    {
                        "kind": "attempt_summary",
                        "summary": "because was misspelled twice"
                    }
                ],
                "constraints": ["keep it under 10 minutes"]
            },
            "context": {
                "mode": "focused",
                "locale": "en-US",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("response should be json");
    let trace_id = uuid_field(&body, "/data/generated_trace_id");

    assert_eq!(
        body.pointer("/data/surface").and_then(Value::as_str),
        Some("planning")
    );
    assert_eq!(
        body.pointer("/data/action").and_then(Value::as_str),
        Some("adjust_plan")
    );
    assert_eq!(
        body.pointer("/data/result/status").and_then(Value::as_str),
        Some("adjusted_plan_ready")
    );
    assert_eq!(
        body.pointer("/data/result/plan_kind")
            .and_then(Value::as_str),
        Some("response_only_adjustment")
    );
    assert_eq!(
        body.pointer("/data/result/persistence")
            .and_then(Value::as_str),
        Some("not_persisted")
    );
    assert_eq!(
        body.pointer("/data/result/adjusted_plan/prompt")
            .and_then(Value::as_str),
        Some("Fit tomorrow practice into one short review")
    );
    assert_eq!(
        body.pointer("/data/result/evidence_summary/record_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(body.pointer("/data/result/practice_plan_id"), None);

    let trace: PlanningTraceRow = sqlx::query_as(
        r#"
        SELECT
            space_id,
            namespace_id,
            source_type,
            task_type,
            mode,
            runtime,
            model_provider,
            output_summary,
            metadata
        FROM traces
        WHERE id = $1
        "#,
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("planning trace should exist");

    assert_eq!(trace.space_id, fixture.space_id);
    assert_eq!(trace.namespace_id, Some(fixture.namespace_id));
    assert_eq!(trace.source_type, "mcp");
    assert_eq!(trace.task_type, "planning");
    assert_eq!(trace.mode, "focused");
    assert_eq!(trace.runtime, "deterministic");
    assert_eq!(trace.model_provider.as_deref(), Some("deterministic"));
    assert_eq!(
        trace.output_summary.as_deref(),
        Some("Adjusted response-only plan for child.english.spelling: Fit tomorrow practice into one short review")
    );
    assert_eq!(trace.metadata["surface"], json!("planning"));
    assert_eq!(trace.metadata["action"], json!("adjust_plan"));
    assert_eq!(trace.metadata["adapter"], json!("mcp"));
    assert_eq!(trace.metadata["deterministic"], json!(true));
    assert_eq!(
        trace.metadata["plan_kind"],
        json!("response_only_adjustment")
    );
    assert_eq!(trace.metadata["persistence"], json!("not_persisted"));
    assert_eq!(trace.metadata["evidence_record_count"], json!(1));
    assert_eq!(trace.metadata["constraint_count"], json!(1));
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn planning_surface_rejects_missing_auth_actor_mismatch_and_viewer_writes() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let payload = json!({
        "namespace": "child.english.spelling",
        "surface": "planning",
        "action": "generate_next_task",
        "actor": fixture.owner_user_id,
        "adapter": "web",
        "payload": {
            "space_id": fixture.space_id
        },
        "context": {
            "mode": "fast",
            "runtime_preference": "deterministic"
        }
    });

    let unauthenticated = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .json(&payload)
        .send()
        .await
        .expect("request should send");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let owner_token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let actor_mismatch = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&owner_token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "planning",
            "action": "generate_next_task",
            "actor": fixture.viewer_user_id,
            "adapter": "web",
            "payload": {
                "space_id": fixture.space_id
            },
            "context": {
                "mode": "fast",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("request should send");
    assert_eq!(actor_mismatch.status(), StatusCode::UNAUTHORIZED);

    let viewer_token = token_for(fixture.viewer_user_id, &fixture.viewer_email);
    let viewer_write = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&viewer_token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "planning",
            "action": "generate_next_task",
            "actor": fixture.viewer_user_id,
            "adapter": "web",
            "payload": {
                "space_id": fixture.space_id
            },
            "context": {
                "mode": "fast",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("request should send");
    assert_eq!(viewer_write.status(), StatusCode::UNAUTHORIZED);

    assert_eq!(
        planning_trace_count(&pool, fixture.space_id, fixture.namespace_id).await,
        0,
        "rejected planning requests must not write Planning traces"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn planning_surface_rejects_cross_space_and_inactive_namespaces() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let other_space_id = seed_space(
        &pool,
        fixture.owner_user_id,
        &format!("Surface Planning Other {}", Uuid::new_v4()),
    )
    .await;
    seed_namespace(
        &pool,
        other_space_id,
        fixture.owner_user_id,
        "child.chinese.dictation",
        "active",
    )
    .await;
    seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "child.english.archived",
        "archived",
    )
    .await;

    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let owner_token = token_for(fixture.owner_user_id, &fixture.owner_email);

    for (label, namespace) in [
        ("cross-space namespace", "child.chinese.dictation"),
        ("inactive namespace", "child.english.archived"),
    ] {
        let trace_count_before =
            planning_trace_count(&pool, fixture.space_id, fixture.namespace_id).await;
        let response = client
            .post(format!("{base_url}/api/v1/surfaces"))
            .bearer_auth(&owner_token)
            .json(&json!({
                "namespace": namespace,
                "surface": "planning",
                "action": "generate_next_task",
                "actor": fixture.owner_user_id,
                "adapter": "mcp",
                "payload": {
                    "space_id": fixture.space_id
                },
                "context": {
                    "mode": "fast",
                    "runtime_preference": "deterministic"
                }
            }))
            .send()
            .await
            .unwrap_or_else(|error| panic!("{label} request should send: {error}"));

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "{label} should be rejected"
        );
        assert_eq!(
            planning_trace_count(&pool, fixture.space_id, fixture.namespace_id).await,
            trace_count_before,
            "{label} must not write a Planning Trace"
        );
    }
}

struct Fixture {
    owner_user_id: Uuid,
    owner_email: String,
    viewer_user_id: Uuid,
    viewer_email: String,
    space_id: Uuid,
    namespace_id: Uuid,
}

#[derive(Debug, FromRow)]
struct PlanningTraceRow {
    space_id: Uuid,
    namespace_id: Option<Uuid>,
    source_type: String,
    task_type: String,
    mode: String,
    runtime: String,
    model_provider: Option<String>,
    output_summary: Option<String>,
    metadata: Value,
}

async fn seed_fixture(pool: &PgPool) -> Fixture {
    let suffix = Uuid::new_v4();
    let owner_email = format!("surface-planning-owner-{suffix}@example.com");
    let viewer_email = format!("surface-planning-viewer-{suffix}@example.com");
    let owner_user_id = seed_user(
        pool,
        &owner_email,
        &format!("surface-planning-owner-{suffix}"),
    )
    .await;
    let viewer_user_id = seed_user(
        pool,
        &viewer_email,
        &format!("surface-planning-viewer-{suffix}"),
    )
    .await;
    let space_id = seed_space(pool, owner_user_id, &format!("Surface Planning {suffix}")).await;

    sqlx::query(
        r#"
        INSERT INTO cognitive_space_members (space_id, user_id, role)
        VALUES ($1, $2, 'viewer')
        "#,
    )
    .bind(space_id)
    .bind(viewer_user_id)
    .execute(pool)
    .await
    .expect("viewer membership should insert");

    let namespace_id = seed_namespace(
        pool,
        space_id,
        owner_user_id,
        "child.english.spelling",
        "active",
    )
    .await;

    Fixture {
        owner_user_id,
        owner_email,
        viewer_user_id,
        viewer_email,
        space_id,
        namespace_id,
    }
}

async fn seed_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'surface-planning-test')
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(username)
    .fetch_one(pool)
    .await
    .expect("user seed should insert")
}

async fn seed_space(pool: &PgPool, user_id: Uuid, name: &str) -> Uuid {
    let space_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO cognitive_spaces (name, owner_user_id, space_type)
        VALUES ($1, $2, 'personal')
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("space seed should insert");

    sqlx::query(
        r#"
        INSERT INTO cognitive_space_members (space_id, user_id, role)
        VALUES ($1, $2, 'owner')
        "#,
    )
    .bind(space_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("space membership seed should insert");

    space_id
}

async fn seed_namespace(
    pool: &PgPool,
    space_id: Uuid,
    created_by: Uuid,
    name: &str,
    status: &str,
) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (space_id, name, kind, status, created_by)
        VALUES ($1, $2, 'skill', $3, $4)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(name)
    .bind(status)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("namespace seed should insert")
}

async fn planning_trace_count(pool: &PgPool, space_id: Uuid, namespace_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM traces
        WHERE space_id = $1
          AND namespace_id = $2
          AND task_type = 'planning'
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_one(pool)
    .await
    .expect("planning trace count should query")
}

async fn postgres_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration test");
    PgPool::connect(&database_url)
        .await
        .expect("postgres pool should connect")
}

async fn spawn_api(pool: PgPool) -> String {
    let state = app_state(pool);
    let app: Router = memorynexus::api::routes().with_state(state);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let addr: SocketAddr = listener.local_addr().expect("test server should have addr");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server should run");
    });

    format!("http://{addr}")
}

fn app_state(pool: PgPool) -> AppState {
    let repositories = Repositories {
        feedback_loops: Arc::new(PostgresFeedbackLoopRepository::new(pool.clone())),
        lenses: Arc::new(PostgresLensRepository::new(pool.clone())),
        lens_runs: Arc::new(PostgresLensRunRepository::new(pool.clone())),
        memories: Arc::new(PostgresMemoryRepository::new(pool.clone())),
        namespaces: Arc::new(PostgresNamespaceRepository::new(pool.clone())),
        profiles: Arc::new(PostgresCognitiveProfileRepository::new(pool.clone())),
        reminders: Arc::new(PostgresReminderRepository::new(pool.clone())),
        review_reports: Arc::new(PostgresCognitiveReviewReportRepository::new(pool.clone())),
        spaces: Arc::new(PostgresCognitiveSpaceRepository::new(pool.clone())),
        tags: Arc::new(PostgresTagRepository::new(pool.clone())),
        traces: Arc::new(PostgresTraceRepository::new(pool.clone())),
        users: Arc::new(PostgresUserRepository::new(pool.clone())),
        vectors: Arc::new(NoopVectorRepository),
    };
    AppState::new(pool, repositories, None)
}

fn token_for(user_id: Uuid, email: &str) -> String {
    JwtAuth::default()
        .generate(user_id, email)
        .expect("test jwt should generate")
}

fn uuid_field(value: &Value, pointer: &str) -> Uuid {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("expected uuid at {pointer}: {value}"))
}

struct NoopVectorRepository;

#[async_trait]
impl VectorRepository for NoopVectorRepository {
    async fn store(&self, _vector: MemoryVector) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn store_batch(&self, _vectors: Vec<MemoryVector>) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn delete(&self, _memory_id: Uuid) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn delete_batch(&self, _memory_ids: Vec<Uuid>) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn exists(&self, _memory_id: Uuid) -> Result<bool, RepositoryError> {
        Ok(false)
    }

    async fn search(
        &self,
        _vector: &[f32],
        _user_id: Uuid,
        _space_id: Uuid,
        _limit: usize,
        _threshold: Option<f32>,
    ) -> Result<Vec<VectorSearchResult>, RepositoryError> {
        Ok(vec![])
    }

    async fn get(&self, _memory_id: Uuid) -> Result<Option<MemoryVector>, RepositoryError> {
        Ok(None)
    }
}
