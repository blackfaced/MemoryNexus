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
    let feedback_loop_id = uuid_field(&body, "/data/result/feedback_loop_id");
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
    assert_eq!(feedback_loop_id, fixture.feedback_loop_id);
    assert_eq!(
        body.pointer("/data/result/event/attempt_submitted/source_trace_id")
            .and_then(Value::as_str),
        Some(trace_id.to_string().as_str())
    );
    assert_eq!(
        body["data"]["result"]["event"]["attempt_submitted"]["payload_refs"],
        json!([{
            "kind": "attempt",
            "id": fixture.feedback_loop_id
        }])
    );

    let attempt: String = sqlx::query_scalar("SELECT attempt FROM feedback_loops WHERE id = $1")
        .bind(fixture.feedback_loop_id)
        .fetch_one(&pool)
        .await
        .expect("attempt should query");
    assert!(attempt.contains("because"));
    assert!(attempt.contains("becuase"));

    let trace: (Uuid, String, String, String, String, Vec<Uuid>, Value) = sqlx::query_as(
        r#"
        SELECT namespace_id, source_type, task_type, mode, runtime, generated_feedback_loop_ids, metadata
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
    assert_eq!(trace.6["event"], "attempt_submitted");
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn performance_surface_submit_attempt_records_typed_dictation_payload() {
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
                "task_kind": "english_spelling",
                "source": "typed",
                "prompt_items": [
                    {"item_kind": "english_word", "expected_text": "because", "metadata": {}}
                ],
                "submitted_items": [
                    {"actual_text": "becaus", "metadata": {}}
                ],
                "task": "Today's spelling words",
                "goal": "Practice child.english.spelling",
                "metadata": {"session": "monday"}
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("response should be json");
    let trace_id = uuid_field(&body, "/data/generated_trace_id");
    let feedback_loop_id = uuid_field(&body, "/data/result/feedback_loop_id");
    assert_eq!(
        body.pointer("/data/result/evaluation/summary")
            .and_then(Value::as_str),
        Some("needs_review")
    );
    assert_eq!(
        body.pointer("/data/result/evaluation/item_results/0/mistake_types/0")
            .and_then(Value::as_str),
        Some("missing_letter")
    );

    let feedback_loop: (Uuid, Uuid, String, String, String) = sqlx::query_as(
        "SELECT space_id, namespace_id, goal, task, attempt FROM feedback_loops WHERE id = $1",
    )
    .bind(feedback_loop_id)
    .fetch_one(&pool)
    .await
    .expect("feedback loop should exist");
    assert_eq!(feedback_loop.0, fixture.space_id);
    assert_eq!(feedback_loop.1, fixture.namespace_id);
    assert_eq!(feedback_loop.2, "Practice child.english.spelling");
    assert_eq!(feedback_loop.3, "Today's spelling words");
    assert_eq!(feedback_loop.4, "because -> becaus");

    let trace: (Uuid, Option<Uuid>, Option<Value>, Value) = sqlx::query_as(
        "SELECT space_id, namespace_id, user_feedback, metadata FROM traces WHERE id = $1",
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("trace should exist");
    assert_eq!(trace.0, fixture.space_id);
    assert_eq!(trace.1, Some(fixture.namespace_id));
    assert_eq!(trace.2, Some(json!({"attempt": "because -> becaus"})));
    assert_eq!(trace.3["namespace"], "child.english.spelling");
    assert_eq!(trace.3["dictation"]["task_kind"], "english_spelling");
    assert_eq!(trace.3["dictation"]["source"], "typed");
    assert_eq!(trace.3["dictation"]["item_count"], 1);
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

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn performance_surface_validates_evidence_refs_without_persisting_descriptors() {
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
                },
                "input_source": "mixed",
                "input_confirmation": {
                    "status": "confirmed",
                    "method": "explicit_correction"
                },
                "evidence_refs": [{
                    "provider": "agent_transcribed",
                    "locator": "https://example.test/api_key/access_token_notes.txt?version=3#page=2",
                    "media_type": "audio/mpeg",
                    "metadata": {"page": 2, "label": "weekly review"}
                }]
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("response should be json");
    let trace_id = uuid_field(&body, "/data/generated_trace_id");

    let feedback_loop: (String,) =
        sqlx::query_as("SELECT attempt FROM feedback_loops WHERE id = $1")
            .bind(fixture.feedback_loop_id)
            .fetch_one(&pool)
            .await
            .expect("feedback loop should exist");
    assert!(!feedback_loop.0.contains("evidence_refs"));
    assert!(!feedback_loop.0.contains("access_token_notes"));

    let trace: (Option<String>, Option<String>, Option<Value>, Value) = sqlx::query_as(
        "SELECT input_summary, output_summary, user_feedback, metadata FROM traces WHERE id = $1",
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("performance trace should exist");
    let trace_text = format!(
        "{}{}{}{}",
        trace.0.unwrap_or_default(),
        trace.1.unwrap_or_default(),
        trace.2.unwrap_or(Value::Null),
        trace.3
    );
    assert!(!trace_text.contains("evidence_refs"));
    assert!(!trace_text.contains("access_token_notes"));
    assert!(!trace_text.contains("weekly review"));
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn performance_surface_media_dictation_attempt_excludes_evidence_descriptors_from_persistence(
) {
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
                "task_kind": "english_spelling",
                "source": "agent_ocr",
                "input_confirmation": {
                    "status": "confirmed",
                    "method": "explicit_correction"
                },
                "prompt_items": [
                    {"item_kind": "english_word", "expected_text": "because", "metadata": {}}
                ],
                "submitted_items": [
                    {"actual_text": "becaus", "metadata": {}}
                ],
                "evidence_refs": [{
                    "provider": "agent_ocr",
                    "locator": "s3://dictation/archive/attempt-worksheet.png",
                    "media_type": "image/png",
                    "transcript": "raw transcript should not persist",
                    "transcript_source": "agent_ocr",
                    "metadata": {"label": "weekly review", "page": 2}
                }],
                "metadata": {"session": "media-confirmed"}
            },
            "context": {"mode": "fast", "runtime_preference": "deterministic"}
        }))
        .send()
        .await
        .expect("surface request should send");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("response should be json");
    let trace_id = uuid_field(&body, "/data/generated_trace_id");

    let feedback_loop: (String,) =
        sqlx::query_as("SELECT attempt FROM feedback_loops WHERE id = $1")
            .bind(fixture.feedback_loop_id)
            .fetch_one(&pool)
            .await
            .expect("feedback loop should exist");
    assert_eq!(feedback_loop.0, "because -> becaus");

    let trace: (Option<String>, Option<String>, Option<Value>, Value) = sqlx::query_as(
        "SELECT input_summary, output_summary, user_feedback, metadata FROM traces WHERE id = $1",
    )
    .bind(trace_id)
    .fetch_one(&pool)
    .await
    .expect("performance trace should exist");
    let trace_text = format!(
        "{}{}{}{}",
        trace.0.unwrap_or_default(),
        trace.1.unwrap_or_default(),
        trace.2.unwrap_or(Value::Null),
        trace.3
    );
    assert!(!trace_text.contains("evidence_refs"));
    assert!(!trace_text.contains("attempt-worksheet"));
    assert!(!trace_text.contains("raw transcript"));
    assert!(!trace_text.contains("weekly review"));
    assert_eq!(trace.3["dictation"]["evidence_ref_count"], 1);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn performance_surface_rejects_invalid_evidence_ref_before_feedback_or_trace_write() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let before_attempt: Option<String> =
        sqlx::query_scalar("SELECT attempt FROM feedback_loops WHERE id = $1")
            .bind(fixture.feedback_loop_id)
            .fetch_one(&pool)
            .await
            .expect("attempt should query");
    let before_traces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traces")
        .fetch_one(&pool)
        .await
        .expect("trace count should query");

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
                },
                "input_source": "agent_transcribed",
                "input_confirmation": {
                    "status": "confirmed",
                    "method": "explicit_acceptance"
                },
                "evidence_refs": [{
                    "provider": "agent_transcribed",
                    "locator": "https://example.test/media/1",
                    "media_type": "audio/mpeg",
                    "metadata": {"note": "Bearer fixture-secret"}
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
    assert!(diagnostic.contains("secret_value_pattern_denied"));
    assert!(!diagnostic.contains("Bearer fixture-secret"));
    assert_eq!(body.pointer("/data/result/event"), None);

    let after_attempt: Option<String> =
        sqlx::query_scalar("SELECT attempt FROM feedback_loops WHERE id = $1")
            .bind(fixture.feedback_loop_id)
            .fetch_one(&pool)
            .await
            .expect("attempt should query");
    let after_traces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traces")
        .fetch_one(&pool)
        .await
        .expect("trace count should query");
    assert_eq!(after_attempt, before_attempt);
    assert_eq!(after_traces, before_traces);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_outcome_is_idempotent_and_correction_keeps_only_one_current_row() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let namespace_id = seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "personal.health.sleep",
        "reflective",
    )
    .await;
    let feedback_loop_id =
        seed_feedback_loop(&pool, fixture.space_id, namespace_id, fixture.owner_user_id).await;
    let planning_trace_id: Uuid = sqlx::query_scalar("INSERT INTO traces (space_id, namespace_id, source_type, task_type, mode, runtime, status) VALUES ($1,$2,'test_fixture','planning','focused','deterministic','completed') RETURNING id")
        .bind(fixture.space_id).bind(namespace_id).fetch_one(&pool).await.unwrap();
    let lifecycle_id: Uuid = sqlx::query_scalar("INSERT INTO planning_lifecycles (space_id, namespace_id, feedback_loop_id, planning_trace_id, policy_version, action_id, action, selected_evidence_ids, expected_signal) VALUES ($1,$2,$3,$4,'personal_feedback_sleep_v1','screen_free_final_hour','{}','[]','coverage') RETURNING id")
        .bind(fixture.space_id).bind(namespace_id).bind(feedback_loop_id).bind(planning_trace_id).fetch_one(&pool).await.unwrap();
    let wrong_date_memory_id = Uuid::new_v4();
    let superseded_memory_id = Uuid::new_v4();
    for (memory_id, local_date, superseded) in [
        (wrong_date_memory_id, "2026-07-19", false),
        (superseded_memory_id, "2026-07-20", true),
    ] {
        let mut personal_feedback = json!({
            "record_type":"sleep_energy_check_in",
            "local_date":local_date,
            "input_confirmation":{"status":"confirmed","method":"explicit_acceptance"}
        });
        if superseded {
            personal_feedback["superseded_by_memory_id"] = json!(Uuid::new_v4());
        }
        sqlx::query("INSERT INTO memories (id, user_id, space_id, namespace_id, content, memory_type, source_type, source_metadata) VALUES ($1,$2,$3,$4,'confirmed sleep check-in','text','surface_capture',$5)")
            .bind(memory_id)
            .bind(fixture.owner_user_id)
            .bind(fixture.space_id)
            .bind(namespace_id)
            .bind(json!({"capture":{"personal_feedback":personal_feedback}}))
            .execute(&pool)
            .await
            .unwrap();
    }
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let request = |local_date: &str, outcome: &str, event: &str, corrects: Option<Uuid>| {
        json!({
            "namespace":"personal.health.sleep", "surface":"performance", "action":"submit_attempt",
            "actor":fixture.owner_user_id, "adapter":"mcp",
            "payload":{"space_id":fixture.space_id,"personal_feedback_outcome":{
                "lifecycle_id":lifecycle_id,"action_id":"screen_free_final_hour","local_date":local_date,
                "outcome":outcome,"source_event_id":event,"corrects_outcome_id":corrects
            }}, "context":{"mode":"fast","runtime_preference":"deterministic"}
        })
    };
    for evidence_memory_id in [wrong_date_memory_id, superseded_memory_id] {
        let rejected = post_surface(
            &client,
            &base_url,
            &token,
            json!({
                "namespace":"personal.health.sleep", "surface":"performance", "action":"submit_attempt",
                "actor":fixture.owner_user_id, "adapter":"mcp", "payload":{"space_id":fixture.space_id,
                "personal_feedback_outcome":{"lifecycle_id":lifecycle_id,"action_id":"screen_free_final_hour",
                "local_date":"2026-07-20","outcome":"performed","source_event_id":format!("sleep.invalid.{evidence_memory_id}"),"evidence_memory_id":evidence_memory_id}},
                "context":{"mode":"fast","runtime_preference":"deterministic"}
            }),
        )
        .await;
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM planning_lifecycle_outcomes WHERE lifecycle_id = $1"
        )
        .bind(lifecycle_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let first: Value = post_surface(
        &client,
        &base_url,
        &token,
        request("2026-07-20", "performed", "sleep.1", None),
    )
    .await
    .json()
    .await
    .unwrap();
    let outcome_id = uuid_field(&first, "/data/result/outcome_id");
    let trace_id = uuid_field(&first, "/data/generated_trace_id");
    let replay: Value = post_surface(
        &client,
        &base_url,
        &token,
        request("2026-07-20", "performed", "sleep.1", None),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(uuid_field(&replay, "/data/result/outcome_id"), outcome_id);
    assert_eq!(uuid_field(&replay, "/data/generated_trace_id"), trace_id);
    let conflict = post_surface(
        &client,
        &base_url,
        &token,
        request("2026-07-20", "skipped", "sleep.2", None),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let corrected: Value = post_surface(
        &client,
        &base_url,
        &token,
        request("2026-07-20", "skipped", "sleep.3", Some(outcome_id)),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(
        corrected
            .pointer("/data/result/outcome")
            .and_then(Value::as_str),
        Some("skipped")
    );
    let current: (i64, String) = sqlx::query_as("SELECT COUNT(*), max(outcome) FROM planning_lifecycle_outcomes WHERE lifecycle_id = $1 AND is_current")
        .bind(lifecycle_id).fetch_one(&pool).await.unwrap();
    assert_eq!(current, (1, "skipped".to_string()));
    let event_conflict = post_surface(
        &client,
        &base_url,
        &token,
        request("2026-07-21", "not_evaluable", "sleep.1", None),
    )
    .await;
    assert_eq!(event_conflict.status(), StatusCode::CONFLICT);
    let not_evaluable: Value = post_surface(
        &client,
        &base_url,
        &token,
        request("2026-07-21", "not_evaluable", "sleep.4", None),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(
        not_evaluable
            .pointer("/data/result/outcome")
            .and_then(Value::as_str),
        Some("not_evaluable")
    );
    let trace_metadata: Value = sqlx::query_scalar("SELECT metadata FROM traces WHERE id = $1")
        .bind(trace_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!trace_metadata
        .to_string()
        .contains("sleep_duration_minutes"));
    assert!(!trace_metadata.to_string().contains("daytime_energy"));
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn idempotent_learning_outcome_replays_conflicts_and_preserves_zero_side_effect_rejections() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let event_id = format!("adapter.session:{}-1", Uuid::new_v4());
    let request =
        idempotent_outcome_request(&fixture, &event_id, "Completed five spelling words", "fast");

    let first = post_surface(&client, &base_url, &token, request.clone()).await;
    assert_eq!(first.status(), StatusCode::OK);
    let first: Value = first.json().await.expect("first response should be json");
    let feedback_loop_id = uuid_field(&first, "/data/result/feedback_loop_id");
    let trace_id = uuid_field(&first, "/data/generated_trace_id");

    let replay = post_surface(&client, &base_url, &token, request.clone()).await;
    assert_eq!(replay.status(), StatusCode::OK);
    let replay: Value = replay.json().await.expect("replay response should be json");
    assert_eq!(
        replay
            .pointer("/data/result/status")
            .and_then(Value::as_str),
        Some("attempt_replayed")
    );
    assert_eq!(
        uuid_field(&replay, "/data/result/feedback_loop_id"),
        feedback_loop_id
    );
    assert_eq!(uuid_field(&replay, "/data/generated_trace_id"), trace_id);

    let conflict = post_surface(
        &client,
        &base_url,
        &token,
        idempotent_outcome_request(&fixture, &event_id, "Different completed session", "fast"),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let rejected_event_id = format!("adapter.session:{}-secret", Uuid::new_v4());
    let secret = post_surface(
        &client,
        &base_url,
        &token,
        idempotent_outcome_request(
            &fixture,
            &rejected_event_id,
            "Bearer should-not-persist",
            "fast",
        ),
    )
    .await;
    assert_eq!(secret.status(), StatusCode::BAD_REQUEST);

    assert_eq!(count_for_event(&pool, &fixture, &event_id).await, 1);
    assert_eq!(
        count_for_event(&pool, &fixture, &rejected_event_id).await,
        0
    );
    assert_eq!(
        count_feedback_loops(&pool, fixture.space_id, fixture.namespace_id).await,
        2
    );
    assert_eq!(
        count_traces(&pool, fixture.space_id, fixture.namespace_id).await,
        1
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn idempotent_learning_outcome_concurrent_retry_and_trace_failure_are_atomic() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let event_id = format!("adapter.session:{}-concurrent", Uuid::new_v4());
    let request =
        idempotent_outcome_request(&fixture, &event_id, "Completed five spelling words", "fast");
    let left_client = Client::new();
    let right_client = Client::new();
    let (left, right) = tokio::join!(
        post_surface(&left_client, &base_url, &token, request.clone()),
        post_surface(&right_client, &base_url, &token, request),
    );
    let left = left
        .json::<Value>()
        .await
        .expect("left response should be json");
    let right = right
        .json::<Value>()
        .await
        .expect("right response should be json");
    assert_eq!(
        uuid_field(&left, "/data/result/feedback_loop_id"),
        uuid_field(&right, "/data/result/feedback_loop_id")
    );
    assert_eq!(
        uuid_field(&left, "/data/generated_trace_id"),
        uuid_field(&right, "/data/generated_trace_id")
    );
    assert_eq!(count_for_event(&pool, &fixture, &event_id).await, 1);
    assert_eq!(
        count_feedback_loops(&pool, fixture.space_id, fixture.namespace_id).await,
        2
    );
    assert_eq!(
        count_traces(&pool, fixture.space_id, fixture.namespace_id).await,
        1
    );

    let rollback_event_id = format!("adapter.session:{}-rollback", Uuid::new_v4());
    let failed = post_surface(
        &Client::new(),
        &base_url,
        &token,
        idempotent_outcome_request(
            &fixture,
            &rollback_event_id,
            "Completed but invalid trace mode",
            "not-a-mode",
        ),
    )
    .await;
    assert_eq!(failed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        count_for_event(&pool, &fixture, &rollback_event_id).await,
        0
    );
    assert_eq!(
        count_feedback_loops(&pool, fixture.space_id, fixture.namespace_id).await,
        2
    );
    assert_eq!(
        count_traces(&pool, fixture.space_id, fixture.namespace_id).await,
        1
    );
}

fn idempotent_outcome_request(
    fixture: &Fixture,
    event_id: &str,
    summary: &str,
    mode: &str,
) -> Value {
    json!({
        "namespace": "child.english.spelling", "surface": "performance", "action": "submit_attempt",
        "actor": fixture.owner_user_id, "adapter": "mcp",
        "payload": {"space_id": fixture.space_id, "source_event_id": event_id,
            "task": "Daily spelling", "input_source": "typed",
            "normalized_outcome": {"summary": summary, "mistake": {
                "expected_text": "because", "actual_text": "becuase", "mistake_type": "letter_order"
            }}},
        "context": {"mode": mode, "runtime_preference": "deterministic"}
    })
}

async fn post_surface(
    client: &Client,
    base_url: &str,
    token: &str,
    request: Value,
) -> reqwest::Response {
    client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .expect("surface request should send")
}

async fn count_for_event(pool: &PgPool, fixture: &Fixture, event_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM performance_idempotency_records WHERE space_id = $1 AND namespace_id = $2 AND source_event_id = $3")
        .bind(fixture.space_id).bind(fixture.namespace_id).bind(event_id).fetch_one(pool).await.expect("idempotency count should query")
}

async fn count_feedback_loops(pool: &PgPool, space_id: Uuid, namespace_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM feedback_loops WHERE space_id = $1 AND namespace_id = $2",
    )
    .bind(space_id)
    .bind(namespace_id)
    .fetch_one(pool)
    .await
    .expect("feedback loop count should query")
}

async fn count_traces(pool: &PgPool, space_id: Uuid, namespace_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM traces WHERE space_id = $1 AND namespace_id = $2")
        .bind(space_id)
        .bind(namespace_id)
        .fetch_one(pool)
        .await
        .expect("trace count should query")
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
