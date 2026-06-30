use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use axum::Router;
use chrono::{Duration, Utc};
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
async fn manual_consolidation_surface_creates_completed_sleep_cycle_with_empty_window() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool, "empty-window").await;
    let base_url = spawn_api(pool.clone()).await;
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let window_start = Utc::now() - Duration::days(2);
    let window_end = Utc::now() - Duration::days(1);

    let response = Client::new()
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(token)
        .json(&json!({
            "namespace": fixture.namespace_name,
            "surface": "observation",
            "action": "request_consolidation",
            "actor": fixture.owner_user_id,
            "adapter": "cli",
            "payload": {
                "space_id": fixture.space_id,
                "evidence_window_start": window_start,
                "evidence_window_end": window_end
            },
            "context": {
                "mode": "deep",
                "locale": "en-US",
                "device": "terminal",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.expect("response should be json");
    let data = &body["data"];
    assert_eq!(data["surface"], "observation");
    assert_eq!(data["action"], "request_consolidation");
    assert_eq!(data["result"]["status"], "completed");
    assert_eq!(data["result"]["input_trace_count"], 0);
    assert_eq!(data["result"]["input_trace_ids"], json!([]));

    let sleep_cycle_id: Uuid =
        serde_json::from_value(data["result"]["cycle_id"].clone()).expect("cycle id should parse");
    let stored = find_sleep_cycle(&pool, sleep_cycle_id).await;
    assert_eq!(stored.status, "completed");
    assert_eq!(stored.space_id, fixture.space_id);
    assert_eq!(stored.namespace_id, fixture.namespace_id);
    assert!(stored.input_trace_ids.is_empty());
    assert!(stored.triggering_trace_id.is_some());
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn manual_consolidation_surface_rejects_cross_space_request() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool, "cross-space").await;
    let base_url = spawn_api(pool.clone()).await;
    let outsider_token = token_for(fixture.outsider_user_id, &fixture.outsider_email);
    let window_start = Utc::now() - Duration::hours(1);
    let window_end = Utc::now();

    let response = Client::new()
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(outsider_token)
        .json(&json!({
            "namespace": fixture.namespace_name,
            "surface": "observation",
            "action": "request_consolidation",
            "actor": fixture.outsider_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "evidence_window_start": window_start,
                "evidence_window_end": window_end
            },
            "context": {
                "mode": "deep",
                "locale": null,
                "device": null,
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let created_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM sleep_cycles
        WHERE space_id = $1 AND namespace_id = $2
        "#,
    )
    .bind(fixture.space_id)
    .bind(fixture.namespace_id)
    .fetch_one(&pool)
    .await
    .expect("sleep cycle count should query");
    assert_eq!(created_count, 0);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn manual_consolidation_surface_rejects_archived_namespace_as_bad_request() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool, "archived-namespace").await;
    archive_namespace(&pool, fixture.namespace_id).await;
    let base_url = spawn_api(pool.clone()).await;
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let window_start = Utc::now() - Duration::hours(1);
    let window_end = Utc::now();

    let response = Client::new()
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(token)
        .json(&json!({
            "namespace": fixture.namespace_name,
            "surface": "observation",
            "action": "request_consolidation",
            "actor": fixture.owner_user_id,
            "adapter": "cli",
            "payload": {
                "space_id": fixture.space_id,
                "evidence_window_start": window_start,
                "evidence_window_end": window_end
            },
            "context": {
                "mode": "deep",
                "locale": "en-US",
                "device": "terminal",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let created_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM sleep_cycles
        WHERE space_id = $1 AND namespace_id = $2
        "#,
    )
    .bind(fixture.space_id)
    .bind(fixture.namespace_id)
    .fetch_one(&pool)
    .await
    .expect("sleep cycle count should query");
    assert_eq!(created_count, 0);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn manual_consolidation_surface_stores_selected_trace_evidence() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool, "selected-evidence").await;
    let inside_trace_id = seed_trace(&pool, &fixture, Utc::now() - Duration::minutes(20)).await;
    let _outside_trace_id = seed_trace(&pool, &fixture, Utc::now() - Duration::days(2)).await;
    let base_url = spawn_api(pool.clone()).await;
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let window_start = Utc::now() - Duration::hours(1);
    let window_end = Utc::now();

    let response = Client::new()
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(token)
        .json(&json!({
            "namespace": fixture.namespace_name,
            "surface": "observation",
            "action": "request_consolidation",
            "actor": fixture.owner_user_id,
            "adapter": "web",
            "payload": {
                "space_id": fixture.space_id,
                "evidence_window_start": window_start,
                "evidence_window_end": window_end
            },
            "context": {
                "mode": "deep",
                "locale": "en-US",
                "device": "browser",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.expect("response should be json");
    assert_eq!(body["data"]["result"]["input_trace_count"], 1);
    assert_eq!(
        body["data"]["result"]["input_trace_ids"],
        json!([inside_trace_id])
    );
}

#[derive(Debug, sqlx::FromRow)]
struct StoredSleepCycle {
    space_id: Uuid,
    namespace_id: Uuid,
    status: String,
    input_trace_ids: Vec<Uuid>,
    triggering_trace_id: Option<Uuid>,
}

struct Fixture {
    owner_user_id: Uuid,
    owner_email: String,
    outsider_user_id: Uuid,
    outsider_email: String,
    space_id: Uuid,
    namespace_id: Uuid,
    namespace_name: String,
}

async fn postgres_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for ignored PostgreSQL integration tests");
    db::init_pool(&database_url)
        .await
        .expect("should connect to PostgreSQL")
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

async fn seed_fixture(pool: &PgPool, label: &str) -> Fixture {
    let suffix = Uuid::new_v4();
    let owner_email = format!("surface-owner-{label}-{suffix}@example.com");
    let outsider_email = format!("surface-outsider-{label}-{suffix}@example.com");
    let owner_user_id = seed_user(
        pool,
        &owner_email,
        &format!("surface-owner-{label}-{suffix}"),
    )
    .await;
    let outsider_user_id = seed_user(
        pool,
        &outsider_email,
        &format!("surface-outsider-{label}-{suffix}"),
    )
    .await;
    let space_id = seed_space(
        pool,
        owner_user_id,
        &format!("Surface Manual Consolidation {label} {suffix}"),
    )
    .await;
    let namespace_name = format!("child.english.spelling.{label}.{suffix}");
    let namespace_id = seed_namespace(pool, space_id, owner_user_id, &namespace_name).await;

    Fixture {
        owner_user_id,
        owner_email,
        outsider_user_id,
        outsider_email,
        space_id,
        namespace_id,
        namespace_name,
    }
}

async fn seed_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'postgres-integration-test')
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

async fn seed_namespace(pool: &PgPool, space_id: Uuid, user_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (space_id, name, kind, created_by)
        VALUES ($1, $2, 'skill', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(name)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("namespace seed should insert")
}

async fn archive_namespace(pool: &PgPool, namespace_id: Uuid) {
    sqlx::query(
        r#"
        UPDATE namespaces
        SET status = 'archived'
        WHERE id = $1
        "#,
    )
    .bind(namespace_id)
    .execute(pool)
    .await
    .expect("namespace archive should update");
}

async fn seed_trace(pool: &PgPool, fixture: &Fixture, started_at: chrono::DateTime<Utc>) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO traces (
            space_id,
            namespace_id,
            source_type,
            task_type,
            mode,
            runtime,
            input_summary,
            output_summary,
            started_at,
            completed_at,
            latency_ms,
            status,
            metadata
        )
        VALUES (
            $1,
            $2,
            'test_fixture',
            'practice',
            'focused',
            'deterministic',
            'dictation evidence',
            'dictation result',
            $3,
            $3,
            1,
            'completed',
            $4
        )
        RETURNING id
        "#,
    )
    .bind(fixture.space_id)
    .bind(fixture.namespace_id)
    .bind(started_at)
    .bind(json!({"fixture": "surface_manual_sleep_cycle_postgres_integration"}))
    .fetch_one(pool)
    .await
    .expect("trace seed should insert")
}

async fn find_sleep_cycle(pool: &PgPool, sleep_cycle_id: Uuid) -> StoredSleepCycle {
    sqlx::query_as(
        r#"
        SELECT space_id, namespace_id, status, input_trace_ids, triggering_trace_id
        FROM sleep_cycles
        WHERE id = $1
        "#,
    )
    .bind(sleep_cycle_id)
    .fetch_one(pool)
    .await
    .expect("sleep cycle should exist")
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
