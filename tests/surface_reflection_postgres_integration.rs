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
async fn reflection_surface_reviews_evidence_and_writes_review_trace() {
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
            "surface": "reflection",
            "action": "review_evidence",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "space_id": fixture.space_id,
                "question": "Explain what this attempt means",
                "evidence": [
                    {
                        "source": {
                            "kind": "trace",
                            "id": fixture.evidence_trace_id
                        },
                        "summary": "Target: because\nSubmitted: becuase"
                    },
                    {
                        "source": {
                            "kind": "feedback_loop",
                            "id": fixture.feedback_loop_id
                        },
                        "summary": "The attempt needs review because the submitted spelling changes letter order."
                    }
                ]
            },
            "context": {
                "mode": "focused",
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
        Some("reflection")
    );
    assert_eq!(
        body.pointer("/data/action").and_then(Value::as_str),
        Some("review_evidence")
    );
    assert_eq!(
        body.pointer("/data/result/status").and_then(Value::as_str),
        Some("insight_ready")
    );
    assert_eq!(
        body.pointer("/data/result/namespace_id")
            .and_then(Value::as_str),
        Some(fixture.namespace_id.to_string().as_str())
    );
    assert_eq!(
        body.pointer("/data/result/space_id")
            .and_then(Value::as_str),
        Some(fixture.space_id.to_string().as_str())
    );
    assert_eq!(
        body.pointer("/data/result/confidence")
            .and_then(Value::as_str),
        Some("medium")
    );
    assert_eq!(
        body.pointer("/data/result/evidence_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        body.pointer("/data/result/evidence_summaries/0/source/kind")
            .and_then(Value::as_str),
        Some("trace")
    );
    assert_eq!(
        body.pointer("/data/result/evidence_summaries/0/source/id")
            .and_then(Value::as_str),
        Some(fixture.evidence_trace_id.to_string().as_str())
    );
    assert_eq!(
        body.pointer("/data/result/evidence_summaries/1/source/kind")
            .and_then(Value::as_str),
        Some("feedback_loop")
    );
    assert_eq!(
        body.pointer("/data/result/evidence_summaries/1/source/id")
            .and_then(Value::as_str),
        Some(fixture.feedback_loop_id.to_string().as_str())
    );

    let trace: (Uuid, Option<Uuid>, String, String, String, String, Value) = sqlx::query_as(
        r#"
        SELECT space_id, namespace_id, source_type, task_type, mode, runtime, metadata
        FROM traces
        WHERE id = $1
        "#,
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("reflection trace should exist");

    assert_eq!(trace.0, fixture.space_id);
    assert_eq!(trace.1, Some(fixture.namespace_id));
    assert_eq!(trace.2, "mcp");
    assert_eq!(trace.3, "review");
    assert_eq!(trace.4, "focused");
    assert_eq!(trace.5, "deterministic");
    assert_eq!(trace.6["surface"], json!("reflection"));
    assert_eq!(trace.6["action"], json!("review_evidence"));
    assert_eq!(trace.6["evidence_count"], json!(2));
    assert_eq!(
        trace.6["evidence_refs"],
        json!([
            {
                "kind": "trace",
                "id": fixture.evidence_trace_id
            },
            {
                "kind": "feedback_loop",
                "id": fixture.feedback_loop_id
            }
        ])
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn reflection_surface_returns_low_certainty_without_evidence() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "reflection",
            "action": "review_evidence",
            "actor": fixture.owner_user_id,
            "adapter": "web",
            "payload": {
                "space_id": fixture.space_id,
                "evidence": []
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
    assert_eq!(
        body.pointer("/data/result/status").and_then(Value::as_str),
        Some("insufficient_evidence")
    );
    assert_eq!(
        body.pointer("/data/result/confidence")
            .and_then(Value::as_str),
        Some("none")
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn reflection_surface_rejects_invalid_evidence_refs_without_writing_review_trace() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let other_namespace_id = seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "child.chinese.dictation",
    )
    .await;
    let other_space_id = seed_space(
        &pool,
        fixture.owner_user_id,
        &format!("Surface Reflection Other {}", Uuid::new_v4()),
    )
    .await;
    let other_space_namespace_id = seed_namespace(
        &pool,
        other_space_id,
        fixture.owner_user_id,
        "child.english.spelling",
    )
    .await;

    let same_space_other_namespace = seed_evidence_set(
        &pool,
        fixture.space_id,
        other_namespace_id,
        fixture.owner_user_id,
    )
    .await;
    let other_space = seed_evidence_set(
        &pool,
        other_space_id,
        other_space_namespace_id,
        fixture.owner_user_id,
    )
    .await;
    let missing = EvidenceSet {
        trace_id: Uuid::new_v4(),
        memory_id: Uuid::new_v4(),
        feedback_loop_id: Uuid::new_v4(),
        review_report_id: Uuid::new_v4(),
    };

    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let cases = [
        ("missing trace", "trace", missing.trace_id),
        (
            "cross-namespace trace",
            "trace",
            same_space_other_namespace.trace_id,
        ),
        ("cross-space trace", "trace", other_space.trace_id),
        ("missing memory", "memory", missing.memory_id),
        (
            "cross-namespace memory",
            "memory",
            same_space_other_namespace.memory_id,
        ),
        ("cross-space memory", "memory", other_space.memory_id),
        (
            "missing feedback loop",
            "feedback_loop",
            missing.feedback_loop_id,
        ),
        (
            "cross-namespace feedback loop",
            "feedback_loop",
            same_space_other_namespace.feedback_loop_id,
        ),
        (
            "cross-space feedback loop",
            "feedback_loop",
            other_space.feedback_loop_id,
        ),
        (
            "missing review report",
            "review_report",
            missing.review_report_id,
        ),
        (
            "cross-namespace review report",
            "review_report",
            same_space_other_namespace.review_report_id,
        ),
        (
            "cross-space review report",
            "review_report",
            other_space.review_report_id,
        ),
    ];

    for (label, kind, id) in cases {
        let review_trace_count_before =
            review_trace_count(&pool, fixture.space_id, fixture.namespace_id).await;

        let response = client
            .post(format!("{base_url}/api/v1/surfaces"))
            .bearer_auth(&token)
            .json(&json!({
                "namespace": "child.english.spelling",
                "surface": "reflection",
                "action": "review_evidence",
                "actor": fixture.owner_user_id,
                "adapter": "mcp",
                "payload": {
                    "space_id": fixture.space_id,
                    "question": format!("Review invalid evidence case: {label}"),
                    "evidence": [
                        {
                            "source": {
                                "kind": kind,
                                "id": id
                            },
                            "summary": "This evidence must be rejected before trace creation."
                        }
                    ]
                },
                "context": {
                    "mode": "focused",
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
            review_trace_count(&pool, fixture.space_id, fixture.namespace_id).await,
            review_trace_count_before,
            "{label} must not write a review Trace"
        );
    }
}

struct Fixture {
    owner_user_id: Uuid,
    owner_email: String,
    space_id: Uuid,
    namespace_id: Uuid,
    evidence_trace_id: Uuid,
    feedback_loop_id: Uuid,
}

struct EvidenceSet {
    trace_id: Uuid,
    memory_id: Uuid,
    feedback_loop_id: Uuid,
    review_report_id: Uuid,
}

async fn seed_fixture(pool: &PgPool) -> Fixture {
    let suffix = Uuid::new_v4();
    let owner_email = format!("surface-reflection-{suffix}@example.com");
    let owner_user_id =
        seed_user(pool, &owner_email, &format!("surface-reflection-{suffix}")).await;
    let space_id = seed_space(pool, owner_user_id, &format!("Surface Reflection {suffix}")).await;
    let namespace_id =
        seed_namespace(pool, space_id, owner_user_id, "child.english.spelling").await;
    let feedback_loop_id = seed_feedback_loop(pool, space_id, namespace_id, owner_user_id).await;
    let evidence_trace_id = seed_trace(pool, space_id, namespace_id, feedback_loop_id).await;

    Fixture {
        owner_user_id,
        owner_email,
        space_id,
        namespace_id,
        evidence_trace_id,
        feedback_loop_id,
    }
}

async fn seed_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'surface-reflection-integration-test')
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

async fn seed_namespace(pool: &PgPool, space_id: Uuid, owner_user_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (space_id, name, kind, created_by)
        VALUES ($1, $2, 'skill', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(name)
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
        VALUES ($1, $2, 'Review spelling pattern', 'Spell because', 'active', $3)
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

async fn seed_evidence_set(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    owner_user_id: Uuid,
) -> EvidenceSet {
    let feedback_loop_id = seed_feedback_loop(pool, space_id, namespace_id, owner_user_id).await;
    let trace_id = seed_trace(pool, space_id, namespace_id, feedback_loop_id).await;
    let memory_id = seed_memory(
        pool,
        space_id,
        namespace_id,
        feedback_loop_id,
        owner_user_id,
    )
    .await;
    let lens_id = seed_lens(pool, space_id, namespace_id, owner_user_id).await;
    let review_report_id = seed_review_report(
        pool,
        space_id,
        namespace_id,
        feedback_loop_id,
        lens_id,
        owner_user_id,
    )
    .await;

    EvidenceSet {
        trace_id,
        memory_id,
        feedback_loop_id,
        review_report_id,
    }
}

async fn seed_memory(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    feedback_loop_id: Uuid,
    owner_user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO memories (
            user_id,
            space_id,
            namespace_id,
            feedback_loop_id,
            content,
            memory_type,
            is_shared,
            source_type,
            source_metadata
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            'confirmed evidence memory',
            'text',
            false,
            'test_fixture',
            '{}'
        )
        RETURNING id
        "#,
    )
    .bind(owner_user_id)
    .bind(space_id)
    .bind(namespace_id)
    .bind(feedback_loop_id)
    .fetch_one(pool)
    .await
    .expect("memory seed should insert")
}

async fn seed_lens(pool: &PgPool, space_id: Uuid, namespace_id: Uuid, owner_user_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO lenses (space_id, namespace_id, name, strategy, created_by)
        VALUES ($1, $2, 'Reflection evidence lens', 'detective', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .expect("lens seed should insert")
}

async fn seed_review_report(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    feedback_loop_id: Uuid,
    lens_id: Uuid,
    owner_user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO cognitive_review_reports (
            space_id,
            lens_id,
            namespace_id,
            feedback_loop_id,
            report_type,
            window_start,
            window_end,
            report,
            source_memory_ids,
            source_lens_run_ids,
            summary_provider,
            summary_source,
            created_by
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            'weekly_review',
            NOW() - INTERVAL '1 day',
            NOW(),
            '{"summary":"review evidence"}',
            '{}',
            '{}',
            'deterministic',
            'test_fixture',
            $5
        )
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(lens_id)
    .bind(namespace_id)
    .bind(feedback_loop_id)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .expect("review report seed should insert")
}

async fn review_trace_count(pool: &PgPool, space_id: Uuid, namespace_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM traces
        WHERE space_id = $1
          AND namespace_id = $2
          AND task_type = 'review'
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_one(pool)
    .await
    .expect("review trace count should query")
}

async fn seed_trace(
    pool: &PgPool,
    space_id: Uuid,
    namespace_id: Uuid,
    feedback_loop_id: Uuid,
) -> Uuid {
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
            status,
            related_memory_ids,
            generated_memory_ids,
            generated_lens_run_ids,
            generated_review_report_ids,
            generated_feedback_loop_ids,
            metadata
        )
        VALUES (
            $1,
            $2,
            'test_fixture',
            'practice',
            'fast',
            'deterministic',
            'submitted spelling attempt',
            'needs review',
            NOW(),
            NOW(),
            'completed',
            '{}',
            '{}',
            '{}',
            '{}',
            ARRAY[$3]::uuid[],
            '{}'
        )
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(namespace_id)
    .bind(feedback_loop_id)
    .fetch_one(pool)
    .await
    .expect("trace seed should insert")
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
