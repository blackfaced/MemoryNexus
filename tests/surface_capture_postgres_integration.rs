use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use memorynexus::{
    api,
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
async fn capture_surface_stores_memory_writes_trace_and_publishes_observation_event() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_capture_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "capture",
            "action": "capture_observation",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "title": "Today's spelling words",
                "task_kind": "english_spelling",
                "source": "typed",
                "prompt_items": [
                    {"item_kind": "english_word", "expected_text": "because", "metadata": {}},
                    {"item_kind": "english_word", "expected_text": "friend", "metadata": {}},
                    {"item_kind": "english_word", "expected_text": "enough", "metadata": {}}
                ],
                "tags": ["dictation", "spelling"]
            },
            "context": {
                "mode": "fast",
                "locale": "en-US",
                "device": "desktop",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.expect("response should be json");
    let trace_id = uuid_field(&body, "/data/generated_trace_id");
    let memory_id = uuid_field(&body, "/data/result/memory_id");
    assert_eq!(body["data"]["surface"], "capture");
    assert_eq!(body["data"]["action"], "capture_observation");
    assert_eq!(
        body["data"]["result"]["event"]["observation_captured"]["source_trace_id"],
        trace_id.to_string()
    );
    assert_eq!(
        body["data"]["result"]["event"]["observation_captured"]["payload_refs"],
        json!([{
            "kind": "observation",
            "id": memory_id
        }])
    );

    let memory: (Uuid, Uuid, Option<Uuid>, String, String, Value) = sqlx::query_as(
        r#"
        SELECT user_id, space_id, namespace_id, content, source_type, source_metadata
        FROM memories
        WHERE id = $1
        "#,
    )
    .bind(memory_id)
    .fetch_one(&pool)
    .await
    .expect("captured memory should exist");

    assert_eq!(memory.0, fixture.owner_user_id);
    assert_eq!(memory.1, fixture.space_id);
    assert_eq!(memory.2, Some(fixture.namespace_id));
    assert_eq!(memory.3, "because\nfriend\nenough");
    assert_eq!(memory.4, "surface_capture");
    assert_eq!(
        memory.5["surface"],
        json!("capture"),
        "memory should carry Surface provenance"
    );
    assert_eq!(memory.5["input_source"], "typed");
    assert_eq!(
        memory.5["capture"]["dictation"]["task_kind"],
        "english_spelling"
    );
    assert_eq!(memory.5["capture"]["dictation"]["item_count"], 3);
    assert_eq!(memory.5["capture"]["dictation"].get("evidence_refs"), None);
    assert_eq!(
        memory.5["capture"]["dictation"].get("input_confirmation"),
        None
    );

    let trace: (
        Uuid,
        Option<Uuid>,
        String,
        String,
        String,
        String,
        Vec<Uuid>,
        Value,
    ) = sqlx::query_as(
        r#"
        SELECT space_id, namespace_id, source_type, task_type, mode, runtime, generated_memory_ids, metadata
        FROM traces
        WHERE id = $1
        "#,
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("capture trace should exist");

    assert_eq!(trace.0, fixture.space_id);
    assert_eq!(trace.1, Some(fixture.namespace_id));
    assert_eq!(trace.2, "mcp");
    assert_eq!(trace.3, "capture");
    assert_eq!(trace.4, "fast");
    assert_eq!(trace.5, "deterministic");
    assert_eq!(trace.6, vec![memory_id]);
    assert_eq!(trace.7["namespace"], "child.english.spelling");
    assert_eq!(trace.7["input_source"], "typed");
    assert_eq!(trace.7["capture"]["dictation"]["item_count"], 3);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn capture_surface_rejects_missing_auth_invalid_namespace_and_viewer_writes() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_capture_fixture(&pool).await;
    let base_url = spawn_api(pool).await;
    let client = Client::new();
    let payload = json!({
        "namespace": "child.english.spelling",
        "surface": "capture",
        "action": "capture_observation",
        "actor": fixture.owner_user_id,
        "adapter": "web",
        "payload": {
            "content": "one short observation"
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
    let invalid_namespace = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&owner_token)
        .json(&json!({
            "namespace": "child.english.unknown",
            "surface": "capture",
            "action": "capture_observation",
            "actor": fixture.owner_user_id,
            "adapter": "web",
            "payload": {
                "content": "one short observation"
            },
            "context": {
                "mode": "fast",
                "runtime_preference": "deterministic"
            }
        }))
        .send()
        .await
        .expect("request should send");
    assert_eq!(invalid_namespace.status(), StatusCode::BAD_REQUEST);

    let viewer_token = token_for(fixture.viewer_user_id, &fixture.viewer_email);
    let viewer_write = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&viewer_token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "capture",
            "action": "capture_observation",
            "actor": fixture.viewer_user_id,
            "adapter": "web",
            "payload": {
                "content": "viewer cannot write this"
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
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn capture_surface_validates_evidence_refs_without_persisting_descriptors() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_capture_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "capture",
            "action": "capture_observation",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "title": "OCR-confirmed dictation",
                "content": "because, friend",
                "input_source": "agent_ocr",
                "input_confirmation": {
                    "status": "confirmed",
                    "method": "explicit_acceptance"
                },
                "evidence_refs": [{
                    "provider": "agent_ocr",
                    "locator": "s3://study/archive/token-guidelines.pdf",
                    "media_type": "image/png",
                    "metadata": {"page": 2, "label": "weekly review"}
                }],
                "metadata": {"surface_note": "descriptor-free persistence"}
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value = response.json().await.expect("response should be json");
    let trace_id = uuid_field(&body, "/data/generated_trace_id");
    let memory_id = uuid_field(&body, "/data/result/memory_id");

    let memory: (Value,) = sqlx::query_as("SELECT source_metadata FROM memories WHERE id = $1")
        .bind(memory_id)
        .fetch_one(&pool)
        .await
        .expect("captured memory should exist");
    assert_eq!(memory.0.get("evidence_refs"), None);
    assert!(
        !memory.0.to_string().contains("token-guidelines"),
        "memory metadata must not persist raw evidence locators"
    );

    let trace: (Option<String>, Option<String>, Value) =
        sqlx::query_as("SELECT input_summary, output_summary, metadata FROM traces WHERE id = $1")
            .bind(trace_id)
            .fetch_one(&pool)
            .await
            .expect("capture trace should exist");
    let trace_text = format!(
        "{}{}{}",
        trace.0.unwrap_or_default(),
        trace.1.unwrap_or_default(),
        trace.2
    );
    assert!(!trace_text.contains("evidence_refs"));
    assert!(!trace_text.contains("token-guidelines"));
    assert!(!trace_text.contains("weekly review"));
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn capture_surface_rejects_invalid_evidence_ref_before_memory_or_trace_write() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_capture_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let before_memories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
        .fetch_one(&pool)
        .await
        .expect("memory count should query");
    let before_traces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traces")
        .fetch_one(&pool)
        .await
        .expect("trace count should query");

    let response = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace": "child.english.spelling",
            "surface": "capture",
            "action": "capture_observation",
            "actor": fixture.owner_user_id,
            "adapter": "mcp",
            "payload": {
                "content": "confirmed text survives only if evidence is safe",
                "input_source": "agent_ocr",
                "input_confirmation": {
                    "status": "confirmed",
                    "method": "explicit_acceptance"
                },
                "evidence_refs": [{
                    "provider": "agent_ocr",
                    "locator": "https://example.test/media/1?X-Amz-Signature=fixture-secret",
                    "media_type": "image/png",
                    "metadata": {}
                }]
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = response.json().await.expect("response should be json");
    let diagnostic = body.to_string();
    assert!(diagnostic.contains("invalid_evidence_reference"));
    assert!(diagnostic.contains("locator_query_denied"));
    assert!(!diagnostic.contains("fixture-secret"));
    assert!(!diagnostic.contains("X-Amz-Signature=fixture-secret"));
    assert_eq!(body.pointer("/data/result/event"), None);

    let after_memories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
        .fetch_one(&pool)
        .await
        .expect("memory count should query");
    let after_traces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traces")
        .fetch_one(&pool)
        .await
        .expect("trace count should query");
    assert_eq!(after_memories, before_memories);
    assert_eq!(after_traces, before_traces);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_capture_persists_only_confirmed_values_supports_readback_and_correction() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_capture_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let typed = post_sleep_capture(
        &client,
        &base_url,
        &token,
        fixture.owner_user_id,
        json!({
            "local_date": "2026-07-13",
            "sleep_duration_minutes": 450,
            "sleep_start_local_time": "22:30",
            "sleep_end_local_time": "06:00",
            "daytime_energy": 3,
            "caffeine_within_six_hours_of_sleep": false,
            "screen_minutes_in_final_hour": 20,
            "input_source": "typed",
            "input_confirmation": {
                "status": "confirmed",
                "method": "explicit_acceptance"
            }
        }),
    )
    .await;
    assert_eq!(typed.status(), StatusCode::CREATED);
    let typed_body: Value = typed.json().await.expect("response should be JSON");
    let typed_memory_id = uuid_field(&typed_body, "/data/result/memory_id");
    let typed_trace_id = uuid_field(&typed_body, "/data/generated_trace_id");
    assert_eq!(
        typed_body["data"]["result"]["namespace_id"],
        fixture.sleep_namespace_id.to_string()
    );

    let readback = client
        .get(format!("{base_url}/api/v1/memories/{typed_memory_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("memory readback should send");
    assert_eq!(readback.status(), StatusCode::OK);
    let readback: Value = readback.json().await.expect("readback should be JSON");
    assert_eq!(readback["data"]["id"], typed_memory_id.to_string());
    assert_eq!(
        readback["data"]["namespace_id"],
        fixture.sleep_namespace_id.to_string()
    );
    assert!(readback["data"]["content"]
        .as_str()
        .expect("content should be text")
        .contains("Confirmed sleep and energy check-in"));

    let accepted_ocr = post_sleep_capture(
        &client,
        &base_url,
        &token,
        fixture.owner_user_id,
        json!({
            "local_date": "2026-07-14",
            "sleep_duration_minutes": 420,
            "daytime_energy": 4,
            "input_source": "agent_ocr",
            "input_confirmation": {
                "status": "confirmed",
                "method": "explicit_acceptance"
            },
            "evidence_refs": [{
                "provider": "agent_ocr",
                "locator": "s3://private/sleep-checkin-raw.png",
                "media_type": "image/png",
                "metadata": {"page": 1}
            }]
        }),
    )
    .await;
    assert_eq!(accepted_ocr.status(), StatusCode::CREATED);

    let corrected = post_sleep_capture(
        &client,
        &base_url,
        &token,
        fixture.owner_user_id,
        json!({
            "local_date": "2026-07-13",
            "sleep_duration_minutes": 480,
            "daytime_energy": 4,
            "input_source": "agent_ocr",
            "input_confirmation": {
                "status": "confirmed",
                "method": "explicit_correction"
            },
            "corrects_record_id": typed_memory_id,
            "evidence_refs": [{
                "provider": "agent_ocr",
                "locator": "s3://private/sleep-checkin-correction.png",
                "media_type": "image/png",
                "metadata": {"page": 1}
            }]
        }),
    )
    .await;
    assert_eq!(corrected.status(), StatusCode::CREATED);
    let corrected: Value = corrected.json().await.expect("response should be JSON");
    let corrected_memory_id = uuid_field(&corrected, "/data/result/memory_id");
    let corrected_trace_id = uuid_field(&corrected, "/data/generated_trace_id");

    let memory: (Value,) = sqlx::query_as("SELECT source_metadata FROM memories WHERE id = $1")
        .bind(corrected_memory_id)
        .fetch_one(&pool)
        .await
        .expect("corrected memory should exist");
    assert_eq!(
        memory.0["capture"]["personal_feedback"]["corrects_record_id"],
        typed_memory_id.to_string()
    );
    let persisted = memory.0.to_string();
    for forbidden in [
        "sleep-checkin-raw.png",
        "sleep-checkin-correction.png",
        "evidence_refs",
        "raw_ocr_text",
        "provider_reasoning",
    ] {
        assert!(
            !persisted.contains(forbidden),
            "Memory metadata must not contain {forbidden}"
        );
    }

    let old_memory: (Value,) = sqlx::query_as("SELECT source_metadata FROM memories WHERE id = $1")
        .bind(typed_memory_id)
        .fetch_one(&pool)
        .await
        .expect("superseded memory should remain as correction provenance");
    assert_eq!(
        old_memory.0["capture"]["personal_feedback"]["superseded_by_memory_id"],
        corrected_memory_id.to_string()
    );

    let trace: (Value,) = sqlx::query_as("SELECT metadata FROM traces WHERE id = $1")
        .bind(corrected_trace_id)
        .fetch_one(&pool)
        .await
        .expect("correction trace should exist");
    let trace_text = trace.0.to_string();
    assert_eq!(
        trace.0["capture"]["personal_feedback"]["corrects_record_id"],
        typed_memory_id.to_string()
    );
    for forbidden in [
        "480",
        "sleep-checkin-correction.png",
        "evidence_refs",
        "raw_ocr_text",
    ] {
        assert!(
            !trace_text.contains(forbidden),
            "Trace metadata must not contain {forbidden}"
        );
    }
    assert_ne!(typed_trace_id, corrected_trace_id);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_capture_rejects_invalid_or_cross_space_input_without_side_effects() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_capture_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let before_memories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
        .fetch_one(&pool)
        .await
        .expect("memory count should query");
    let before_traces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traces")
        .fetch_one(&pool)
        .await
        .expect("trace count should query");

    for payload in [
        json!({
            "local_date": "2026-07-13",
            "sleep_duration_minutes": 59,
            "daytime_energy": 3,
            "input_source": "typed",
            "input_confirmation": {"status": "confirmed", "method": "explicit_acceptance"}
        }),
        json!({
            "local_date": "2026-07-13",
            "sleep_duration_minutes": 450,
            "daytime_energy": 3,
            "input_source": "agent_ocr"
        }),
        json!({
            "local_date": "2026-07-13",
            "sleep_duration_minutes": 450,
            "daytime_energy": 3,
            "input_source": "typed",
            "input_confirmation": {"status": "confirmed", "method": "explicit_acceptance"},
            "medical_notes": "must be rejected"
        }),
    ] {
        let response =
            post_sleep_capture(&client, &base_url, &token, fixture.owner_user_id, payload).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let other_space_id = seed_space(&pool, fixture.owner_user_id, "Other Space").await;
    let other_namespace_id = seed_namespace(
        &pool,
        other_space_id,
        fixture.owner_user_id,
        "personal.health.sleep",
        "reflective",
    )
    .await;
    let cross_space_memory_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO memories (user_id, space_id, namespace_id, content, memory_type, is_shared, source_type, source_metadata)
        VALUES ($1, $2, $3, 'other Space check-in', 'text', false, 'surface_capture',
                '{"capture":{"personal_feedback":{"record_type":"sleep_energy_check_in","local_date":"2026-07-13"}}}')
        RETURNING id
        "#,
    )
    .bind(fixture.owner_user_id)
    .bind(other_space_id)
    .bind(other_namespace_id)
    .fetch_one(&pool)
    .await
    .expect("cross-Space check-in should seed");
    let cross_space = post_sleep_capture(
        &client,
        &base_url,
        &token,
        fixture.owner_user_id,
        json!({
            "local_date": "2026-07-13",
            "sleep_duration_minutes": 450,
            "daytime_energy": 3,
            "input_source": "agent_ocr",
            "input_confirmation": {"status": "confirmed", "method": "explicit_correction"},
            "corrects_record_id": cross_space_memory_id
        }),
    )
    .await;
    assert_eq!(cross_space.status(), StatusCode::BAD_REQUEST);

    let viewer_token = token_for(fixture.viewer_user_id, &fixture.viewer_email);
    let viewer_write = post_sleep_capture(
        &client,
        &base_url,
        &viewer_token,
        fixture.viewer_user_id,
        json!({
            "local_date": "2026-07-15",
            "sleep_duration_minutes": 450,
            "daytime_energy": 3,
            "input_source": "typed",
            "input_confirmation": {"status": "confirmed", "method": "explicit_acceptance"}
        }),
    )
    .await;
    assert_eq!(viewer_write.status(), StatusCode::UNAUTHORIZED);

    sqlx::query("UPDATE namespaces SET status = 'archived' WHERE id = $1")
        .bind(fixture.sleep_namespace_id)
        .execute(&pool)
        .await
        .expect("sleep namespace should archive");
    let inactive_namespace = post_sleep_capture(
        &client,
        &base_url,
        &token,
        fixture.owner_user_id,
        json!({
            "local_date": "2026-07-15",
            "sleep_duration_minutes": 450,
            "daytime_energy": 3,
            "input_source": "typed",
            "input_confirmation": {"status": "confirmed", "method": "explicit_acceptance"}
        }),
    )
    .await;
    assert_eq!(inactive_namespace.status(), StatusCode::BAD_REQUEST);

    let after_memories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
        .fetch_one(&pool)
        .await
        .expect("memory count should query");
    let after_traces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traces")
        .fetch_one(&pool)
        .await
        .expect("trace count should query");
    assert_eq!(
        after_memories,
        before_memories + 1,
        "only the cross-Space fixture may exist"
    );
    assert_eq!(after_traces, before_traces);
}

fn uuid_field(value: &Value, pointer: &str) -> Uuid {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("expected uuid at {pointer}: {value}"))
}

fn token_for(user_id: Uuid, email: &str) -> String {
    JwtAuth::default()
        .generate(user_id, email)
        .expect("test jwt should generate")
}

async fn spawn_api(pool: PgPool) -> String {
    let state = app_state(pool);
    let app: Router = api::routes().with_state(state);
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

struct NoopVectorRepository;

#[async_trait::async_trait]
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

struct CaptureFixture {
    owner_user_id: Uuid,
    owner_email: String,
    viewer_user_id: Uuid,
    viewer_email: String,
    space_id: Uuid,
    namespace_id: Uuid,
    sleep_namespace_id: Uuid,
}

async fn postgres_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for ignored PostgreSQL integration tests");
    db::init_pool(&database_url)
        .await
        .expect("should connect to PostgreSQL")
}

async fn seed_capture_fixture(pool: &PgPool) -> CaptureFixture {
    let suffix = Uuid::new_v4();
    let owner_email = format!("surface-owner-{suffix}@example.com");
    let viewer_email = format!("surface-viewer-{suffix}@example.com");
    let owner_user_id = seed_user(pool, &owner_email, &format!("surface-owner-{suffix}")).await;
    let viewer_user_id = seed_user(pool, &viewer_email, &format!("surface-viewer-{suffix}")).await;
    let space_id = seed_space(pool, owner_user_id, &format!("Surface Capture {suffix}")).await;

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
        "skill",
    )
    .await;
    let sleep_namespace_id = seed_namespace(
        pool,
        space_id,
        owner_user_id,
        "personal.health.sleep",
        "reflective",
    )
    .await;

    CaptureFixture {
        owner_user_id,
        owner_email,
        viewer_user_id,
        viewer_email,
        space_id,
        namespace_id,
        sleep_namespace_id,
    }
}

async fn post_sleep_capture(
    client: &Client,
    base_url: &str,
    token: &str,
    actor: Uuid,
    payload: Value,
) -> reqwest::Response {
    client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(token)
        .json(&json!({
            "namespace": "personal.health.sleep",
            "surface": "capture",
            "action": "capture_observation",
            "actor": actor,
            "adapter": "mcp",
            "payload": payload,
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("sleep Capture request should send")
}

async fn seed_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'surface-capture-test')
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
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("namespace seed should insert")
}
