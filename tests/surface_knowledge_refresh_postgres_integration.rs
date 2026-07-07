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
async fn capture_accepts_approved_knowledge_candidate_policy_and_context_then_observation_lists_state(
) {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let candidate_id = Uuid::new_v4();
    let policy_id = Uuid::new_v4();
    let candidate_trace_id = Uuid::new_v4();

    let candidate_response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "capture",
            "action": "capture_observation",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "knowledge_source_candidate": {
                    "id": candidate_id,
                    "space_id": fixture.space_id,
                    "namespace_id": fixture.namespace_id,
                    "state": "approved",
                    "proposed_source": {
                        "kind": "canonical_url",
                        "locator": "https://example.test/spelling-rubric"
                    },
                    "proposed_use": "Use as spelling mistake taxonomy context.",
                    "proposer": "test-mcp-adapter",
                    "acquisition_trace": acquisition_trace(candidate_trace_id, fixture.space_id, fixture.namespace_id, "source_candidate", false),
                    "private_context_used": false,
                    "provenance": {"origin": "manual", "observed_at": Utc::now()},
                    "quality_signals": {"reliability": "reviewed", "confidence": 0.91},
                    "freshness": {"observed_at": Utc::now(), "stale_after": Utc::now() + Duration::days(30)},
                    "expiry": Utc::now() + Duration::days(30),
                    "downstream_link_candidates": [],
                    "metadata": {"topic": "spelling"},
                    "source_policy": {
                        "id": policy_id,
                        "state": "active",
                        "source_descriptor": {
                            "kind": "canonical_url",
                            "locator": "https://example.test/spelling-rubric"
                        },
                        "allowed_use": ["claim_support", "review_context"],
                        "disallowed_use": ["direct_growth_model_update", "direct_practice_plan_update"],
                        "privacy_policy": {"private_context_allowed": false},
                        "refresh_policy": {"mode": "manual_only"},
                        "quality_thresholds": {"minimum_confidence": 0.8},
                        "freshness_requirements": {"review_after_days": 30},
                        "expiry": Utc::now() + Duration::days(30),
                        "approved_by": "developer-test",
                        "approved_at": Utc::now(),
                        "metadata": {"policy_kind": "fixture"}
                    }
                }
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("candidate request should send");

    assert_eq!(candidate_response.status(), StatusCode::CREATED);
    let candidate_body: Value = candidate_response.json().await.expect("response json");
    let candidate_surface_trace_id = uuid_field(&candidate_body, "/data/generated_trace_id");
    assert_eq!(
        candidate_body
            .pointer("/data/result/status")
            .and_then(Value::as_str),
        Some("knowledge_source_candidate_accepted")
    );
    assert_eq!(
        candidate_body
            .pointer("/data/result/source_candidate_id")
            .and_then(Value::as_str),
        Some(candidate_id.to_string().as_str())
    );
    assert_eq!(
        candidate_body
            .pointer("/data/result/source_policy_id")
            .and_then(Value::as_str),
        Some(policy_id.to_string().as_str())
    );
    assert_eq!(
        candidate_body
            .pointer("/data/result/acquisition_trace_id")
            .and_then(Value::as_str),
        Some(candidate_trace_id.to_string().as_str())
    );

    let context_id = Uuid::new_v4();
    let context_trace_id = Uuid::new_v4();
    let context_response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "capture",
            "action": "capture_observation",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "knowledge_context": valid_context_payload(
                    context_id,
                    fixture.space_id,
                    fixture.namespace_id,
                    candidate_id,
                    policy_id,
                    context_trace_id
                )
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("context request should send");

    assert_eq!(context_response.status(), StatusCode::CREATED);
    let context_body: Value = context_response.json().await.expect("response json");
    assert_eq!(
        context_body
            .pointer("/data/result/status")
            .and_then(Value::as_str),
        Some("knowledge_context_accepted")
    );
    assert_eq!(
        context_body
            .pointer("/data/result/knowledge_context_id")
            .and_then(Value::as_str),
        Some(context_id.to_string().as_str())
    );
    assert_eq!(
        context_body
            .pointer("/data/result/acquisition_trace_id")
            .and_then(Value::as_str),
        Some(context_trace_id.to_string().as_str())
    );

    let observation_response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "observation",
            "action": "get_state_summary",
            "actor": fixture.owner_user_id,
            "adapter": "dashboard",
            "payload": {"space_id": fixture.space_id},
            "context": {"mode": "focused", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("observation request should send");

    assert_eq!(observation_response.status(), StatusCode::OK);
    let observation_body: Value = observation_response.json().await.expect("response json");
    assert_eq!(
        observation_body
            .pointer("/data/result/knowledge_refresh/source_candidates/0/id")
            .and_then(Value::as_str),
        Some(candidate_id.to_string().as_str())
    );
    assert_eq!(
        observation_body
            .pointer("/data/result/knowledge_refresh/source_policies/0/state")
            .and_then(Value::as_str),
        Some("active")
    );
    assert_eq!(
        observation_body
            .pointer("/data/result/knowledge_refresh/knowledge_contexts/0/id")
            .and_then(Value::as_str),
        Some(context_id.to_string().as_str())
    );
    assert_eq!(
        observation_body
            .pointer("/data/result/knowledge_refresh/knowledge_contexts/0/source_policy_id")
            .and_then(Value::as_str),
        Some(policy_id.to_string().as_str())
    );
    assert_eq!(
        observation_body
            .pointer("/data/result/knowledge_refresh/knowledge_contexts/0/structured_claim_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    let body_text = observation_body.to_string();
    assert!(!body_text.contains("full_source_document"));
    assert!(!body_text.contains("raw_provider_payload"));
    assert!(body_text.contains(&candidate_surface_trace_id.to_string()));
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn capture_rejects_unsafe_knowledge_context_without_trace_or_persistence_side_effects() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let candidate_id =
        seed_candidate(&pool, fixture.space_id, fixture.namespace_id, "approved").await;
    let proposed_candidate_id =
        seed_candidate(&pool, fixture.space_id, fixture.namespace_id, "proposed").await;
    let rejected_candidate_id =
        seed_candidate(&pool, fixture.space_id, fixture.namespace_id, "rejected").await;
    let active_policy_id = seed_policy(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        candidate_id,
        "active",
    )
    .await;
    let paused_policy_id = seed_policy(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        candidate_id,
        "paused",
    )
    .await;
    let expired_policy_id = seed_policy(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        candidate_id,
        "expired",
    )
    .await;
    let proposed_candidate_policy_id = seed_policy(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        proposed_candidate_id,
        "active",
    )
    .await;
    let rejected_candidate_policy_id = seed_policy(
        &pool,
        fixture.space_id,
        fixture.namespace_id,
        rejected_candidate_id,
        "active",
    )
    .await;
    let other_namespace_id = seed_named_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "child.english.sentence-dictation",
    )
    .await;
    let cross_namespace_candidate_id =
        seed_candidate(&pool, fixture.space_id, other_namespace_id, "approved").await;
    let cross_namespace_policy_id = seed_policy(
        &pool,
        fixture.space_id,
        other_namespace_id,
        cross_namespace_candidate_id,
        "active",
    )
    .await;
    let other_space_id = seed_space(
        &pool,
        fixture.owner_user_id,
        &format!("Surface Knowledge Other {}", Uuid::new_v4()),
    )
    .await;
    let cross_space_namespace_id = seed_named_namespace(
        &pool,
        other_space_id,
        fixture.owner_user_id,
        "child.english.spelling",
    )
    .await;
    let cross_space_candidate_id =
        seed_candidate(&pool, other_space_id, cross_space_namespace_id, "approved").await;
    let cross_space_policy_id = seed_policy(
        &pool,
        other_space_id,
        cross_space_namespace_id,
        cross_space_candidate_id,
        "active",
    )
    .await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let cases = [
        ("missing acquisition trace", {
            let mut payload = valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                candidate_id,
                active_policy_id,
                Uuid::new_v4(),
            );
            payload.as_object_mut().unwrap().remove("acquisition_trace");
            payload
        }),
        ("private context without opt-in", {
            let mut payload = valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                candidate_id,
                active_policy_id,
                Uuid::new_v4(),
            );
            payload["private_context_used"] = json!(true);
            payload["acquisition_trace"]["private_context_used"] = json!(true);
            payload
        }),
        (
            "paused policy",
            valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                candidate_id,
                paused_policy_id,
                Uuid::new_v4(),
            ),
        ),
        (
            "expired policy",
            valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                candidate_id,
                expired_policy_id,
                Uuid::new_v4(),
            ),
        ),
        (
            "proposed source candidate",
            valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                proposed_candidate_id,
                proposed_candidate_policy_id,
                Uuid::new_v4(),
            ),
        ),
        (
            "rejected source candidate",
            valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                rejected_candidate_id,
                rejected_candidate_policy_id,
                Uuid::new_v4(),
            ),
        ),
        (
            "cross-namespace policy",
            valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                cross_namespace_candidate_id,
                cross_namespace_policy_id,
                Uuid::new_v4(),
            ),
        ),
        (
            "cross-space policy",
            valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                cross_space_candidate_id,
                cross_space_policy_id,
                Uuid::new_v4(),
            ),
        ),
        ("secret-bearing downstream link", {
            let mut payload = valid_context_payload(
                Uuid::new_v4(),
                fixture.space_id,
                fixture.namespace_id,
                candidate_id,
                active_policy_id,
                Uuid::new_v4(),
            );
            payload["downstream_links"] = json!([{
                "kind": "issue",
                "locator": "https://example.test/private?X-Amz-Signature=fixture-secret"
            }]);
            payload
        }),
    ];

    for (label, knowledge_context) in cases {
        let before_traces = table_count(&pool, "traces").await;
        let before_contexts = table_count(&pool, "knowledge_contexts").await;
        let response = client
            .post(format!("{base_url}/api/v1/surfaces"))
            .bearer_auth(&token)
            .json(&json!({
                "namespace": "child.english.spelling",
                "surface": "capture",
                "action": "capture_observation",
                "actor": fixture.owner_user_id,
                "adapter": "mcp",
                "payload": {"knowledge_context": knowledge_context},
                "context": {"mode": "fast", "runtime_preference": "deterministic"}
            }))
            .send()
            .await
            .unwrap_or_else(|error| panic!("{label} request should send: {error}"));

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{label}");
        let body: Value = response.json().await.expect("response json");
        let diagnostic = body.to_string();
        assert!(!diagnostic.contains("fixture-secret"), "{label}");
        assert_eq!(table_count(&pool, "traces").await, before_traces, "{label}");
        assert_eq!(
            table_count(&pool, "knowledge_contexts").await,
            before_contexts,
            "{label}"
        );
    }
}

fn acquisition_trace(
    id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
    acquisition_kind: &str,
    private_context_used: bool,
) -> Value {
    json!({
        "id": id,
        "space_id": space_id,
        "namespace_id": namespace_id,
        "submitted_by": "test-mcp-adapter",
        "acquisition_kind": acquisition_kind,
        "discovery_method": "manual_entry",
        "extraction_method": "human_summary",
        "private_context_used": private_context_used,
        "opt_in_proof": if private_context_used {
            json!({
                "actor": "fixture-user",
                "method": "explicit_acceptance",
                "namespace_id": namespace_id,
                "allowed_private_context_categories": ["trace_summary"],
                "consented_at": Utc::now()
            })
        } else {
            Value::Null
        },
        "source_handles": [{"kind": "canonical_url", "locator": "https://example.test/spelling-rubric"}],
        "source_observed_at": Utc::now(),
        "validation_summary": {"target": "knowledge_refresh"},
        "redacted_diagnostics": {},
        "metadata": {"fixture": true}
    })
}

fn valid_context_payload(
    id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
    source_candidate_id: Uuid,
    source_policy_id: Uuid,
    acquisition_trace_id: Uuid,
) -> Value {
    json!({
        "id": id,
        "space_id": space_id,
        "namespace_id": namespace_id,
        "source_policy_id": source_policy_id,
        "source_candidate_id": source_candidate_id,
        "acquisition_trace": acquisition_trace(acquisition_trace_id, space_id, namespace_id, "knowledge_context", false),
        "state": "valid",
        "context_type": "rubric_context",
        "structured_claims": [{
            "claim_id": "claim-1",
            "claim_type": "rubric_item",
            "text": "Missing internal letters should be classified separately from transposition.",
            "confidence": 0.9,
            "source_fragment_ref": "rubric-section-1",
            "evidence_snippet_ids": ["snippet-1"],
            "limitations": []
        }],
        "provenance": {
            "source_descriptor": {"kind": "canonical_url", "locator": "https://example.test/spelling-rubric"},
            "observed_at": Utc::now(),
            "extracted_at": Utc::now(),
            "extractor": "human_summary"
        },
        "quality_signals": {
            "reliability": "reviewed",
            "relevance": "high",
            "extraction_confidence": 0.9,
            "contradiction_status": "none"
        },
        "freshness": {
            "observed_at": Utc::now(),
            "stale_after": Utc::now() + Duration::days(30)
        },
        "expiry": Utc::now() + Duration::days(30),
        "evidence_snippets": [{
            "snippet_id": "snippet-1",
            "text": "Classify missing internal letters as a distinct spelling error."
        }],
        "private_context_used": false,
        "downstream_links": [{
            "kind": "dream_candidate",
            "id": Uuid::new_v4()
        }],
        "conflict_notes": [],
        "metadata": {"fixture": true}
    })
}

async fn seed_candidate(pool: &PgPool, space_id: Uuid, namespace_id: Uuid, state: &str) -> Uuid {
    let acquisition_trace_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_acquisition_traces (
            id, space_id, namespace_id, submitted_by, acquisition_kind, discovery_method,
            extraction_method, private_context_used, source_handles, source_observed_at,
            validation_summary, redacted_diagnostics, metadata
        )
        VALUES ($1, $2, $3, 'fixture', 'source_candidate', 'manual', 'none', false, '[]', NOW(), '{}', '{}', '{}')
        "#,
    )
    .bind(acquisition_trace_id)
    .bind(space_id)
    .bind(namespace_id)
    .execute(pool)
    .await
    .expect("acquisition trace seed should insert");

    let candidate_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_source_candidates (
            id, space_id, namespace_id, state, proposed_source, proposed_use, proposer,
            acquisition_trace_id, private_context_used, provenance, quality_signals,
            freshness, expiry, downstream_link_candidates, metadata
        )
        VALUES ($1, $2, $3, $4, '{"kind":"fixture"}', 'fixture', 'fixture',
            $5, false, '{}', '{}', '{}', NOW() + INTERVAL '30 days', '[]', '{}')
        "#,
    )
    .bind(candidate_id)
    .bind(space_id)
    .bind(namespace_id)
    .bind(state)
    .bind(acquisition_trace_id)
    .execute(pool)
    .await
    .expect("candidate seed should insert");
    candidate_id
}

async fn seed_policy(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    candidate_id: Uuid,
    state: &str,
) -> Uuid {
    let policy_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_source_policies (
            id, space_id, namespace_id, state, source_candidate_id, source_descriptor,
            allowed_use, disallowed_use, privacy_policy, refresh_policy, quality_thresholds,
            freshness_requirements, expiry, approved_by, approved_at, metadata
        )
        VALUES ($1, $2, $3, $4, $5, '{"kind":"fixture"}', '[]', '[]', '{}', '{}', '{}', '{}',
            NOW() + INTERVAL '30 days', 'fixture', NOW(), '{}')
        "#,
    )
    .bind(policy_id)
    .bind(space_id)
    .bind(namespace_id)
    .bind(state)
    .bind(candidate_id)
    .execute(pool)
    .await
    .expect("policy seed should insert");
    policy_id
}

async fn table_count(pool: &PgPool, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar(&sql)
        .fetch_one(pool)
        .await
        .expect("count query should run")
}

struct Fixture {
    owner_user_id: Uuid,
    owner_email: String,
    space_id: Uuid,
    namespace_id: Uuid,
}

async fn seed_fixture(pool: &PgPool) -> Fixture {
    let suffix = Uuid::new_v4();
    let owner_email = format!("surface-knowledge-owner-{suffix}@example.com");
    let owner_user_id = seed_user(
        pool,
        &owner_email,
        &format!("surface-knowledge-owner-{suffix}"),
    )
    .await;
    let space_id = seed_space(pool, owner_user_id, &format!("Surface Knowledge {suffix}")).await;
    let namespace_id = seed_namespace(pool, space_id, owner_user_id).await;
    Fixture {
        owner_user_id,
        owner_email,
        space_id,
        namespace_id,
    }
}

async fn seed_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'surface-knowledge-test')
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

async fn seed_namespace(pool: &PgPool, space_id: Uuid, created_by: Uuid) -> Uuid {
    seed_named_namespace(pool, space_id, created_by, "child.english.spelling").await
}

async fn seed_named_namespace(pool: &PgPool, space_id: Uuid, created_by: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (space_id, name, kind, created_by)
        VALUES ($1, $2, 'skill', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(name)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("namespace seed should insert")
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
