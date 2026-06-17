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
use sqlx::PgPool;
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn performance_surface_submit_attempt_updates_feedback_loop_and_writes_trace() {
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
            "surface": "performance",
            "action": "submit_attempt",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "feedback_loop_id": fixture.feedback_loop_id,
                "attempt": {
                    "target": "because",
                    "submitted": "becuase"
                }
            },
            "context": {
                "mode": "fast",
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
        Some("performance")
    );
    assert_eq!(
        body.pointer("/data/action").and_then(Value::as_str),
        Some("submit_attempt")
    );
    assert_eq!(
        body.pointer("/data/result/status").and_then(Value::as_str),
        Some("attempt_recorded")
    );
    assert_eq!(
        body.pointer("/data/result/evaluation")
            .and_then(Value::as_str),
        Some("needs_review")
    );
    assert_eq!(
        body.pointer("/data/result/deep_consolidation")
            .and_then(Value::as_bool),
        Some(false)
    );

    let attempt: String = sqlx::query_scalar("SELECT attempt FROM feedback_loops WHERE id = $1")
        .bind(fixture.feedback_loop_id)
        .fetch_one(&pool)
        .await
        .expect("attempt should query");
    assert!(attempt.contains("because"));
    assert!(attempt.contains("becuase"));

    let trace: (Uuid, String, String, String, String, Vec<Uuid>) = sqlx::query_as(
        r#"
        SELECT namespace_id, source_type, task_type, mode, runtime, generated_feedback_loop_ids
        FROM traces
        WHERE id = $1
        "#,
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("trace should exist");
    assert_eq!(trace.0, fixture.namespace_id);
    assert_eq!(trace.1, "mcp");
    assert_eq!(trace.2, "practice");
    assert_eq!(trace.3, "fast");
    assert_eq!(trace.4, "deterministic");
    assert_eq!(trace.5, vec![fixture.feedback_loop_id]);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn performance_surface_rejects_cross_namespace_and_cross_space_attempts() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let wrong_namespace = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.chinese.dictation",
            "surface": "performance",
            "action": "submit_attempt",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "feedback_loop_id": fixture.feedback_loop_id,
                "attempt": "wrong namespace"
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("wrong namespace request should send");
    assert_eq!(wrong_namespace.status(), StatusCode::BAD_REQUEST);

    let cross_space = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "performance",
            "action": "submit_attempt",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.other_space_id,
                "feedback_loop_id": fixture.feedback_loop_id,
                "attempt": "wrong space"
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("cross-space request should send");
    assert_eq!(cross_space.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn performance_surface_rejects_archived_namespace() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    sqlx::query("UPDATE namespaces SET status = 'archived' WHERE id = $1")
        .bind(fixture.namespace_id)
        .execute(&pool)
        .await
        .expect("namespace archive should update");
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "performance",
            "action": "submit_attempt",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "feedback_loop_id": fixture.feedback_loop_id,
                "attempt": {
                    "target": "because",
                    "submitted": "becuase"
                }
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("archived namespace request should send");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

struct Fixture {
    owner_user_id: Uuid,
    owner_email: String,
    space_id: Uuid,
    other_space_id: Uuid,
    namespace_id: Uuid,
    feedback_loop_id: Uuid,
}

async fn seed_fixture(pool: &PgPool) -> Fixture {
    let suffix = Uuid::new_v4();
    let owner_email = format!("surface-performance-{suffix}@example.com");
    let owner_user_id =
        seed_user(pool, &owner_email, &format!("surface-performance-{suffix}")).await;
    let space_id = seed_space(
        pool,
        owner_user_id,
        &format!("Surface Performance {suffix}"),
    )
    .await;
    let other_space_id = seed_space(
        pool,
        owner_user_id,
        &format!("Surface Performance Other {suffix}"),
    )
    .await;
    let namespace_id = seed_namespace(
        pool,
        space_id,
        owner_user_id,
        "child.english.spelling",
        "skill",
    )
    .await;
    seed_namespace(
        pool,
        space_id,
        owner_user_id,
        "child.chinese.dictation",
        "skill",
    )
    .await;
    let feedback_loop_id = seed_feedback_loop(pool, space_id, namespace_id, owner_user_id).await;

    Fixture {
        owner_user_id,
        owner_email,
        space_id,
        other_space_id,
        namespace_id,
        feedback_loop_id,
    }
}

async fn seed_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'surface-performance-integration-test')
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(username)
    .fetch_one(pool)
    .await
    .expect("user seed should insert")
}

async fn seed_space(pool: &PgPool, owner_user_id: Uuid, name: &str) -> Uuid {
    let space_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO cognitive_spaces (name, owner_user_id, space_type)
        VALUES ($1, $2, 'personal')
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(owner_user_id)
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
    .bind(owner_user_id)
    .execute(pool)
    .await
    .expect("space membership seed should insert");

    space_id
}

async fn seed_namespace(
    pool: &PgPool,
    space_id: Uuid,
    owner_user_id: Uuid,
    name: &str,
    kind: &str,
) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (space_id, name, kind, created_by)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(name)
    .bind(kind)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .expect("namespace seed should insert")
}

async fn seed_feedback_loop(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    owner_user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO feedback_loops (space_id, namespace_id, goal, task, status, created_by)
        VALUES ($1, $2, 'Practice spelling', 'Spell because', 'active', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .expect("feedback loop seed should insert")
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
