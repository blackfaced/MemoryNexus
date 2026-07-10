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

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn manual_consolidation_surface_includes_eligible_knowledge_context_candidates() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool, "knowledge-context").await;
    let knowledge_context_id = seed_knowledge_context(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        KnowledgeContextSeed {
            context_state: "valid",
            candidate_state: "approved",
            policy_state: "active",
            context_type: "rubric_context",
            expiry: Utc::now() + Duration::days(30),
            conflict_notes: json!([]),
        },
    )
    .await;
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

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.expect("response should be json");
    let candidate_context = &body["data"]["result"]["candidate_context"];
    assert_eq!(candidate_context["selected_knowledge_context_count"], 1);
    assert_eq!(candidate_context["ignored_knowledge_context_count"], 0);
    assert_eq!(
        candidate_context["knowledge_context_ids"],
        json!([knowledge_context_id])
    );
    assert_eq!(
        candidate_context["dream_candidates"][0]["knowledge_context_id"],
        json!(knowledge_context_id)
    );
    assert_eq!(
        candidate_context["dream_candidates"][0]["evidence_priority"]["local_evidence"],
        "primary"
    );
    assert_eq!(
        candidate_context["dream_candidates"][0]["direct_mutation"]["growth_model"],
        false
    );
    assert_eq!(
        candidate_context["dream_candidates"][0]["direct_mutation"]["practice_plan"],
        false
    );

    let sleep_cycle_id: Uuid = serde_json::from_value(body["data"]["result"]["cycle_id"].clone())
        .expect("cycle id should parse");
    let stored = find_sleep_cycle(&pool, sleep_cycle_id).await;
    assert_eq!(
        stored.metadata["candidate_context"]["knowledge_context_ids"],
        json!([knowledge_context_id])
    );
    assert_eq!(
        stored.metadata["candidate_context"]["dream_candidates"][0]["knowledge_context_id"],
        json!(knowledge_context_id)
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn manual_consolidation_surface_filters_ineligible_ambient_knowledge_context() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool, "knowledge-filtering").await;
    let selected_id = seed_knowledge_context(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        KnowledgeContextSeed {
            context_state: "candidate",
            candidate_state: "approved",
            policy_state: "active",
            context_type: "review_context",
            expiry: Utc::now() + Duration::days(30),
            conflict_notes: json!([{"kind": "local_external_tension"}]),
        },
    )
    .await;
    let _expired_id = seed_knowledge_context(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        KnowledgeContextSeed {
            context_state: "valid",
            candidate_state: "approved",
            policy_state: "active",
            context_type: "practice_context",
            expiry: Utc::now() - Duration::days(1),
            conflict_notes: json!([]),
        },
    )
    .await;
    let _rejected_candidate_id = seed_knowledge_context(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        KnowledgeContextSeed {
            context_state: "valid",
            candidate_state: "rejected",
            policy_state: "active",
            context_type: "practice_context",
            expiry: Utc::now() + Duration::days(30),
            conflict_notes: json!([]),
        },
    )
    .await;
    let other_namespace_name = format!("{}.other", fixture.namespace_name);
    let other_namespace_id = seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        &other_namespace_name,
    )
    .await;
    let _cross_namespace_id = seed_knowledge_context(
        &pool,
        fixture.space_id,
        other_namespace_id,
        KnowledgeContextSeed {
            context_state: "valid",
            candidate_state: "approved",
            policy_state: "active",
            context_type: "practice_context",
            expiry: Utc::now() + Duration::days(30),
            conflict_notes: json!([]),
        },
    )
    .await;
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
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "evidence_window_start": window_start,
                "evidence_window_end": window_end
            },
            "context": {
                "mode": "deep",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.expect("response should be json");
    let candidate_context = &body["data"]["result"]["candidate_context"];
    assert_eq!(candidate_context["selected_knowledge_context_count"], 1);
    assert_eq!(candidate_context["ignored_knowledge_context_count"], 2);
    assert_eq!(
        candidate_context["knowledge_context_ids"],
        json!([selected_id])
    );
    assert_eq!(
        candidate_context["dream_candidates"][0]["purpose"],
        "contradiction_exploration"
    );
    assert_eq!(
        candidate_context["dream_candidates"][0]["knowledge_context_id"],
        json!(selected_id)
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn manual_consolidation_surface_rejects_explicit_ineligible_knowledge_context_reference() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool, "explicit-knowledge-reject").await;
    let rejected_id = seed_knowledge_context(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        KnowledgeContextSeed {
            context_state: "rejected",
            candidate_state: "approved",
            policy_state: "active",
            context_type: "rubric_context",
            expiry: Utc::now() + Duration::days(30),
            conflict_notes: json!([]),
        },
    )
    .await;
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
                "evidence_window_end": window_end,
                "knowledge_context_ids": [rejected_id]
            },
            "context": {
                "mode": "deep",
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

#[derive(Debug, sqlx::FromRow)]
struct StoredSleepCycle {
    space_id: Uuid,
    namespace_id: Uuid,
    status: String,
    input_trace_ids: Vec<Uuid>,
    triggering_trace_id: Option<Uuid>,
    metadata: Value,
}

struct KnowledgeContextSeed {
    context_state: &'static str,
    candidate_state: &'static str,
    policy_state: &'static str,
    context_type: &'static str,
    expiry: chrono::DateTime<Utc>,
    conflict_notes: Value,
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

async fn seed_knowledge_context(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    seed: KnowledgeContextSeed,
) -> Uuid {
    let acquisition_trace_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_acquisition_traces (
            id, space_id, namespace_id, submitted_by, acquisition_kind, discovery_method,
            extraction_method, private_context_used, source_handles, source_observed_at,
            validation_summary, redacted_diagnostics, metadata
        )
        VALUES ($1, $2, $3, 'fixture', 'knowledge_context', 'manual', 'human_summary',
            false, '[]', NOW(), '{}', '{}', '{}')
        "#,
    )
    .bind(acquisition_trace_id)
    .bind(space_id)
    .bind(namespace_id)
    .execute(pool)
    .await
    .expect("knowledge acquisition trace should insert");

    let source_candidate_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_source_candidates (
            id, space_id, namespace_id, state, proposed_source, proposed_use, proposer,
            acquisition_trace_id, private_context_used, provenance, quality_signals,
            freshness, expiry, downstream_link_candidates, metadata
        )
        VALUES ($1, $2, $3, $4, $5, 'fixture source for manual dreaming', 'fixture',
            $6, false, '{}', '{}', '{}', $7, '[]', '{}')
        "#,
    )
    .bind(source_candidate_id)
    .bind(space_id)
    .bind(namespace_id)
    .bind(seed.candidate_state)
    .bind(json!({"kind": "fixture", "locator": "https://example.test/rubric"}))
    .bind(acquisition_trace_id)
    .bind(seed.expiry)
    .execute(pool)
    .await
    .expect("knowledge source candidate should insert");

    let source_policy_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_source_policies (
            id, space_id, namespace_id, state, source_candidate_id, source_descriptor,
            allowed_use, disallowed_use, privacy_policy, refresh_policy, quality_thresholds,
            freshness_requirements, expiry, approved_by, approved_at, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, '["dream_candidate"]', '["direct_growth_model_update", "direct_practice_plan_update"]',
            '{}', '{}', '{}', '{}', $7, 'fixture', NOW(), '{}')
        "#,
    )
    .bind(source_policy_id)
    .bind(space_id)
    .bind(namespace_id)
    .bind(seed.policy_state)
    .bind(source_candidate_id)
    .bind(json!({"kind": "fixture", "locator": "https://example.test/rubric"}))
    .bind(seed.expiry)
    .execute(pool)
    .await
    .expect("knowledge source policy should insert");

    let knowledge_context_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_contexts (
            id, space_id, namespace_id, source_policy_id, source_candidate_id,
            acquisition_trace_id, state, context_type, structured_claims, provenance,
            quality_signals, freshness, expiry, evidence_snippets, private_context_used,
            downstream_links, conflict_notes, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
            false, $15, $16, '{}')
        "#,
    )
    .bind(knowledge_context_id)
    .bind(space_id)
    .bind(namespace_id)
    .bind(source_policy_id)
    .bind(source_candidate_id)
    .bind(acquisition_trace_id)
    .bind(seed.context_state)
    .bind(seed.context_type)
    .bind(json!([{
        "claim_id": "claim-1",
        "claim_type": "rubric_item",
        "text": "Missing internal letters should be reviewed separately from transposition.",
        "confidence": 0.9,
        "source_fragment_ref": "fixture-section",
        "evidence_snippet_ids": ["snippet-1"],
        "limitations": []
    }]))
    .bind(json!({
        "source_descriptor": {"kind": "fixture", "locator": "https://example.test/rubric"},
        "observed_at": Utc::now(),
        "extracted_at": Utc::now(),
        "extractor": "fixture"
    }))
    .bind(json!({
        "reliability": "reviewed",
        "relevance": "high",
        "extraction_confidence": 0.9,
        "contradiction_status": if seed.conflict_notes.as_array().is_some_and(|notes| !notes.is_empty()) {
            "candidate_conflict"
        } else {
            "none"
        }
    }))
    .bind(json!({
        "observed_at": Utc::now(),
        "stale_after": seed.expiry
    }))
    .bind(seed.expiry)
    .bind(json!([{
        "snippet_id": "snippet-1",
        "text": "Classify missing internal letters as a separate review item."
    }]))
    .bind(json!([{"kind": "dream_candidate", "id": Uuid::new_v4()}]))
    .bind(seed.conflict_notes)
    .execute(pool)
    .await
    .expect("knowledge context should insert");

    knowledge_context_id
}

async fn find_sleep_cycle(pool: &PgPool, sleep_cycle_id: Uuid) -> StoredSleepCycle {
    sqlx::query_as(
        r#"
        SELECT space_id, namespace_id, status, input_trace_ids, triggering_trace_id, metadata
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
