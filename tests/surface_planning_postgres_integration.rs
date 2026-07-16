use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use axum::Router;
use chrono::NaiveDate;
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

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_planning_creates_replays_and_keeps_provenance_scoped() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let sleep_namespace_id = seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "personal.health.sleep",
        "active",
    )
    .await;
    for day in 1..=3 {
        seed_sleep_evidence(&pool, &fixture, sleep_namespace_id, day, Some(15)).await;
    }
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let request = |owner_window: Option<Value>| {
        let client = client.clone();
        let url = format!("{base_url}/api/v1/surfaces");
        let token = token.clone();
        async move {
            client
                .post(url)
                .bearer_auth(token)
                .json(&json!({
                    "namespace": "personal.health.sleep", "surface": "planning",
                    "action": "generate_next_task", "actor": fixture.owner_user_id,
                    "adapter": "mcp", "payload": {
                        "space_id": fixture.space_id,
                        "owner_selected_wake_time_window": owner_window
                    }, "context": {"mode": "focused", "runtime_preference": "deterministic"}
                }))
                .send()
                .await
                .expect("sleep planning request should send")
        }
    };
    let response = request(None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let first: Value = response.json().await.expect("response should be json");
    assert_eq!(
        first.pointer("/data/result/status").and_then(Value::as_str),
        Some("experiment_ready")
    );
    assert_eq!(
        first
            .pointer("/data/result/experiment/action_id")
            .and_then(Value::as_str),
        Some("screen_free_final_hour")
    );
    assert_eq!(
        first
            .pointer("/data/result/experiment/expected_observable_signal")
            .and_then(Value::as_str),
        Some(
            "Confirmed daily records have screen_minutes_in_final_hour == 0 during the experiment."
        )
    );
    let lifecycle_id = uuid_field(&first, "/data/result/experiment/lifecycle_id");
    let trace_id = uuid_field(&first, "/data/generated_trace_id");
    let second: Value = request(Some(
        json!({"start_local_time":"07:00","end_local_time":"07:30"}),
    ))
    .await
    .json()
    .await
    .expect("replay response should be json");
    assert_eq!(
        uuid_field(&second, "/data/result/experiment/lifecycle_id"),
        lifecycle_id
    );
    assert_eq!(
        second
            .pointer("/data/result/experiment/action_id")
            .and_then(Value::as_str),
        Some("screen_free_final_hour")
    );
    seed_sleep_evidence(&pool, &fixture, sleep_namespace_id, 4, None).await;
    let changed_evidence: Value = request(Some(
        json!({"start_local_time":"07:00","end_local_time":"07:30"}),
    ))
    .await
    .json()
    .await
    .expect("changed evidence replay should be json");
    assert_eq!(
        uuid_field(&changed_evidence, "/data/result/experiment/lifecycle_id"),
        lifecycle_id
    );
    assert_eq!(
        changed_evidence
            .pointer("/data/result/experiment/action_id")
            .and_then(Value::as_str),
        Some("screen_free_final_hour")
    );
    assert_eq!(
        changed_evidence
            .pointer("/data/follow_up_suggestions/0")
            .and_then(Value::as_str),
        Some("Keep recording confirmed daily check-ins while trying the selected experiment.")
    );
    let row: (Uuid, Uuid, Uuid, Uuid, Value) = sqlx::query_as("SELECT id, space_id, namespace_id, planning_trace_id, action FROM planning_lifecycles WHERE id = $1")
        .bind(lifecycle_id).fetch_one(&pool).await.expect("lifecycle should exist");
    assert_eq!(row.1, fixture.space_id);
    assert_eq!(row.2, sleep_namespace_id);
    assert_eq!(row.3, trace_id);
    assert_eq!(row.4["action_id"], json!("screen_free_final_hour"));
    let other_space_id = seed_space(
        &pool,
        fixture.owner_user_id,
        &format!("Other sleep scope {}", Uuid::new_v4()),
    )
    .await;
    let other_namespace_id = seed_namespace(
        &pool,
        other_space_id,
        fixture.owner_user_id,
        "personal.health.sleep.other",
        "active",
    )
    .await;
    let other_trace_id: Uuid = sqlx::query_scalar("INSERT INTO traces (space_id, namespace_id, source_type, task_type, mode, runtime, status) VALUES ($1,$2,'test_fixture','planning','focused','deterministic','completed') RETURNING id")
        .bind(other_space_id).bind(other_namespace_id).fetch_one(&pool).await.unwrap();
    let feedback_loop_id: Uuid =
        sqlx::query_scalar("SELECT feedback_loop_id FROM planning_lifecycles WHERE id = $1")
            .bind(lifecycle_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let cross_scope = sqlx::query("INSERT INTO planning_lifecycles (space_id, namespace_id, feedback_loop_id, planning_trace_id, policy_version, action_id, action, selected_evidence_ids, expected_signal) VALUES ($1,$2,$3,$4,'test','test','{}','[]','test')")
        .bind(fixture.space_id).bind(sleep_namespace_id).bind(feedback_loop_id).bind(other_trace_id).execute(&pool).await;
    assert!(
        cross_scope.is_err(),
        "trace provenance must be same Space/Namespace"
    );
    let trace: PlanningTraceRow = sqlx::query_as("SELECT space_id, namespace_id, source_type, task_type, mode, runtime, model_provider, output_summary, metadata FROM traces WHERE id = $1")
        .bind(trace_id).fetch_one(&pool).await.expect("sleep planning trace should exist");
    assert_eq!(trace.space_id, fixture.space_id);
    assert_eq!(trace.namespace_id, Some(sleep_namespace_id));
    assert!(!trace
        .metadata
        .to_string()
        .contains("sleep_duration_minutes"));
    assert!(!trace
        .metadata
        .to_string()
        .contains("screen_minutes_in_final_hour"));
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM planning_lifecycles WHERE space_id = $1 AND namespace_id = $2 AND status = 'active'").bind(fixture.space_id).bind(sleep_namespace_id).fetch_one(&pool).await.unwrap(), 1);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_planning_gaps_and_invalid_window_have_no_lifecycle_side_effects() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let sleep_namespace_id = seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "personal.health.sleep",
        "active",
    )
    .await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let payload = |window: Value| {
        json!({
            "namespace":"personal.health.sleep", "surface":"planning", "action":"generate_next_task",
            "actor":fixture.owner_user_id, "adapter":"mcp", "payload":{"space_id":fixture.space_id,"owner_selected_wake_time_window":window},
            "context":{"mode":"focused","runtime_preference":"deterministic"}
        })
    };
    let before = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM feedback_loops WHERE space_id = $1 AND namespace_id = $2",
    )
    .bind(fixture.space_id)
    .bind(sleep_namespace_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let gap = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&payload(Value::Null))
        .send()
        .await
        .unwrap();
    assert_eq!(gap.status(), StatusCode::OK);
    let gap_body: Value = gap.json().await.unwrap();
    assert_eq!(
        gap_body
            .pointer("/data/result/status")
            .and_then(Value::as_str),
        Some("needs_more_evidence")
    );
    let after_gap = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM feedback_loops WHERE space_id = $1 AND namespace_id = $2",
    )
    .bind(fixture.space_id)
    .bind(sleep_namespace_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after_gap);
    let invalid = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&payload(
            json!({"start_local_time":"07:00","end_local_time":"this contains secret"}),
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    let after_invalid = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM feedback_loops WHERE space_id = $1 AND namespace_id = $2",
    )
    .bind(fixture.space_id)
    .bind(sleep_namespace_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after_invalid);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_planning_concurrent_requests_converge_on_one_active_lifecycle() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let sleep_namespace_id = seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "personal.health.sleep",
        "active",
    )
    .await;
    for day in 1..=3 {
        seed_sleep_evidence(&pool, &fixture, sleep_namespace_id, day, Some(20)).await;
    }
    let base_url = spawn_api(pool.clone()).await;
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let request = |client: Client| {
        let token = token.clone();
        let url = format!("{base_url}/api/v1/surfaces");
        async move {
            client.post(url).bearer_auth(token).json(&json!({
                "namespace":"personal.health.sleep", "surface":"planning", "action":"generate_next_task",
                "actor":fixture.owner_user_id, "adapter":"mcp", "payload":{"space_id":fixture.space_id},
                "context":{"mode":"focused","runtime_preference":"deterministic"}
            })).send().await.expect("concurrent request should send")
        }
    };
    let (left, right) = tokio::join!(request(Client::new()), request(Client::new()));
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM planning_lifecycles WHERE space_id = $1 AND namespace_id = $2 AND status = 'active'")
        .bind(fixture.space_id).bind(sleep_namespace_id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    let loop_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedback_loops WHERE space_id = $1 AND namespace_id = $2 AND status = 'active'")
        .bind(fixture.space_id).bind(sleep_namespace_id).fetch_one(&pool).await.unwrap();
    assert_eq!(loop_count, 1);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_adjustment_uses_current_outcome_and_stop_closes_only_that_lifecycle() {
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
        "active",
    )
    .await;
    for day in 1..=3 {
        seed_sleep_evidence(&pool, &fixture, namespace_id, day, Some(20)).await;
    }
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let generated: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace":"personal.health.sleep", "surface":"planning", "action":"generate_next_task",
            "actor":fixture.owner_user_id, "adapter":"mcp", "payload":{"space_id":fixture.space_id},
            "context":{"mode":"focused","runtime_preference":"deterministic"}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let lifecycle_id = uuid_field(&generated, "/data/result/experiment/lifecycle_id");
    let feedback_loop_id = uuid_field(&generated, "/data/result/experiment/feedback_loop_id");
    let outcome_trace_id: Uuid = sqlx::query_scalar("INSERT INTO traces (space_id, namespace_id, source_type, task_type, mode, runtime, status) VALUES ($1,$2,'test_fixture','practice','fast','deterministic','completed') RETURNING id")
        .bind(fixture.space_id).bind(namespace_id).fetch_one(&pool).await.unwrap();
    let outcome_id: Uuid = sqlx::query_scalar("INSERT INTO planning_lifecycle_outcomes (space_id, namespace_id, lifecycle_id, feedback_loop_id, trace_id, local_date, action_id, outcome, source_event_id, payload_fingerprint) VALUES ($1,$2,$3,$4,$5,'2026-07-20','screen_free_final_hour','skipped','planning.fixture.1',$6) RETURNING id")
        .bind(fixture.space_id).bind(namespace_id).bind(lifecycle_id).bind(feedback_loop_id).bind(outcome_trace_id).bind("f".repeat(64)).fetch_one(&pool).await.unwrap();
    let adjust = |decision: Option<&str>| {
        json!({
            "namespace":"personal.health.sleep", "surface":"planning", "action":"adjust_plan",
            "actor":fixture.owner_user_id, "adapter":"mcp", "payload":{"space_id":fixture.space_id,"personal_feedback_adjustment":{"lifecycle_id":lifecycle_id,"local_date":"2026-07-20","owner_decision":decision}},
            "context":{"mode":"focused","runtime_preference":"deterministic"}
        })
    };
    let candidate: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&adjust(None))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        candidate
            .pointer("/data/result/candidate/disposition")
            .and_then(Value::as_str),
        Some("retest")
    );
    assert_eq!(
        candidate
            .pointer("/data/result/outcome/id")
            .and_then(Value::as_str),
        Some(outcome_id.to_string().as_str())
    );
    let stopped: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&adjust(Some("stop")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        stopped
            .pointer("/data/result/lifecycle_status")
            .and_then(Value::as_str),
        Some("cancelled")
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM planning_lifecycles WHERE id = $1")
            .bind(lifecycle_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "cancelled"
    );
    let decision: (Uuid, Uuid, Uuid) = sqlx::query_as("SELECT outcome_id, outcome_trace_id, decision_trace_id FROM planning_lifecycle_decisions WHERE lifecycle_id = $1 ORDER BY created_at DESC LIMIT 1")
        .bind(lifecycle_id).fetch_one(&pool).await.unwrap();
    assert_eq!(decision.0, outcome_id);
    assert_eq!(decision.1, outcome_trace_id);

    let other_space_id = seed_space(
        &pool,
        fixture.owner_user_id,
        &format!("Other decision scope {}", Uuid::new_v4()),
    )
    .await;
    let other_namespace_id = seed_namespace(
        &pool,
        other_space_id,
        fixture.owner_user_id,
        "personal.health.sleep.other",
        "active",
    )
    .await;
    let other_feedback_loop_id: Uuid = sqlx::query_scalar("INSERT INTO feedback_loops (space_id, namespace_id, goal, task, status, created_by) VALUES ($1,$2,'other','other','active',$3) RETURNING id")
        .bind(other_space_id).bind(other_namespace_id).bind(fixture.owner_user_id).fetch_one(&pool).await.unwrap();
    let other_planning_trace_id: Uuid = sqlx::query_scalar("INSERT INTO traces (space_id, namespace_id, source_type, task_type, mode, runtime, status) VALUES ($1,$2,'test_fixture','planning','focused','deterministic','completed') RETURNING id")
        .bind(other_space_id).bind(other_namespace_id).fetch_one(&pool).await.unwrap();
    let other_lifecycle_id: Uuid = sqlx::query_scalar("INSERT INTO planning_lifecycles (space_id, namespace_id, feedback_loop_id, planning_trace_id, policy_version, action_id, action, selected_evidence_ids, expected_signal) VALUES ($1,$2,$3,$4,'test','screen_free_final_hour','{}','[]','test') RETURNING id")
        .bind(other_space_id).bind(other_namespace_id).bind(other_feedback_loop_id).bind(other_planning_trace_id).fetch_one(&pool).await.unwrap();
    let other_outcome_trace_id: Uuid = sqlx::query_scalar("INSERT INTO traces (space_id, namespace_id, source_type, task_type, mode, runtime, status) VALUES ($1,$2,'test_fixture','practice','fast','deterministic','completed') RETURNING id")
        .bind(other_space_id).bind(other_namespace_id).fetch_one(&pool).await.unwrap();
    let other_outcome_id: Uuid = sqlx::query_scalar("INSERT INTO planning_lifecycle_outcomes (space_id, namespace_id, lifecycle_id, feedback_loop_id, trace_id, local_date, action_id, outcome, source_event_id, payload_fingerprint) VALUES ($1,$2,$3,$4,$5,'2026-07-20','screen_free_final_hour','performed','other-outcome','fixture') RETURNING id")
        .bind(other_space_id).bind(other_namespace_id).bind(other_lifecycle_id).bind(other_feedback_loop_id).bind(other_outcome_trace_id).fetch_one(&pool).await.unwrap();
    let cross_scope_outcome = sqlx::query("UPDATE planning_lifecycle_decisions SET outcome_id = $2, outcome_trace_id = $3 WHERE lifecycle_id = $1")
        .bind(lifecycle_id).bind(other_outcome_id).bind(other_outcome_trace_id).execute(&pool).await;
    assert!(
        cross_scope_outcome.is_err(),
        "decision lineage must reject an outcome or outcome Trace from another scope"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn personal_sleep_adjustment_uses_current_outcome_and_records_owner_stop() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_fixture(&pool).await;
    let sleep_namespace_id = seed_namespace(
        &pool,
        fixture.space_id,
        fixture.owner_user_id,
        "personal.health.sleep",
        "active",
    )
    .await;
    for day in 1..=3 {
        seed_sleep_evidence(&pool, &fixture, sleep_namespace_id, day, Some(20)).await;
    }
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();
    let token = token_for(fixture.owner_user_id, &fixture.owner_email);

    let lifecycle: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace":"personal.health.sleep", "surface":"planning", "action":"generate_next_task",
            "actor":fixture.owner_user_id, "adapter":"mcp", "payload":{"space_id":fixture.space_id},
            "context":{"mode":"focused","runtime_preference":"deterministic"}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let lifecycle_id = uuid_field(&lifecycle, "/data/result/experiment/lifecycle_id");
    let selected_evidence_ids =
        lifecycle["data"]["result"]["experiment"]["selected_evidence_ids"].clone();

    let adjustment = |local_date: &str, owner_decision: Option<&str>| {
        json!({
            "namespace":"personal.health.sleep", "surface":"planning", "action":"adjust_plan",
            "actor":fixture.owner_user_id, "adapter":"mcp",
            "payload":{"space_id":fixture.space_id,"personal_feedback_adjustment":{
                "lifecycle_id":lifecycle_id,"local_date":local_date,"owner_decision":owner_decision
            }}, "context":{"mode":"focused","runtime_preference":"deterministic"}
        })
    };
    let missing: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&adjustment("2026-07-20", None))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        missing["data"]["result"]["outcome"]["state"],
        "awaiting_outcome"
    );
    assert!(missing["data"]["result"]["candidate"]["disposition"].is_null());
    assert_eq!(
        missing["data"]["result"]["selected_evidence_ids"],
        selected_evidence_ids
    );
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
    let decision_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM planning_lifecycle_decisions WHERE lifecycle_id = $1",
    )
    .bind(lifecycle_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let decision_trace_count_before =
        planning_trace_count(&pool, fixture.space_id, sleep_namespace_id).await;
    let missing_decision = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&adjustment("2026-07-20", Some("stop")))
        .send()
        .await
        .unwrap();
    assert_eq!(missing_decision.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM planning_lifecycle_decisions WHERE lifecycle_id = $1"
        )
        .bind(lifecycle_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        decision_count_before
    );
    assert_eq!(
        planning_trace_count(&pool, fixture.space_id, sleep_namespace_id).await,
        decision_trace_count_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM planning_lifecycles WHERE id = $1")
            .bind(lifecycle_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "active"
    );

    let outcome: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&json!({
            "namespace":"personal.health.sleep", "surface":"performance", "action":"submit_attempt",
            "actor":fixture.owner_user_id, "adapter":"mcp", "payload":{"space_id":fixture.space_id,
            "personal_feedback_outcome":{"lifecycle_id":lifecycle_id,"action_id":"screen_free_final_hour",
            "local_date":"2026-07-20","outcome":"skipped","source_event_id":"sleep-adjustment-1"}},
            "context":{"mode":"fast","runtime_preference":"deterministic"}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let outcome_id = uuid_field(&outcome, "/data/result/outcome_id");
    let outcome_trace_id = uuid_field(&outcome, "/data/generated_trace_id");
    let retest: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&adjustment("2026-07-20", None))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retest["data"]["result"]["outcome"]["state"], "skipped");
    assert_eq!(
        retest["data"]["result"]["candidate"]["disposition"],
        "retest"
    );
    assert!(retest.to_string().contains("not ineffectiveness"));

    let stopped: Value = client
        .post(format!("{base_url}/api/v1/surfaces"))
        .bearer_auth(&token)
        .json(&adjustment("2026-07-20", Some("stop")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let decision_trace_id = uuid_field(&stopped, "/data/generated_trace_id");
    assert_eq!(stopped["data"]["result"]["lifecycle_status"], "cancelled");
    let decision: (Uuid, Uuid, Uuid, String) = sqlx::query_as(
        "SELECT outcome_id, outcome_trace_id, decision_trace_id, disposition FROM planning_lifecycle_decisions WHERE lifecycle_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(lifecycle_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        decision,
        (
            outcome_id,
            outcome_trace_id,
            decision_trace_id,
            "stop".to_string()
        )
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM planning_lifecycles WHERE id = $1")
            .bind(lifecycle_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "cancelled"
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM planning_lifecycles WHERE space_id = $1 AND namespace_id = $2 AND status = 'active'")
        .bind(fixture.space_id).bind(sleep_namespace_id).fetch_one(&pool).await.unwrap(), 0);
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

async fn seed_sleep_evidence(
    pool: &PgPool,
    fixture: &Fixture,
    namespace_id: Uuid,
    day: i64,
    screen_minutes: Option<i32>,
) -> Uuid {
    let memory_id = Uuid::new_v4();
    let local_date = NaiveDate::from_ymd_opt(2026, 7, 1)
        .unwrap()
        .checked_add_days(chrono::Days::new(day as u64))
        .unwrap();
    let mut personal_feedback = json!({
        "record_type": "sleep_energy_check_in",
        "local_date": local_date.to_string(),
        "sleep_duration_minutes": 420,
        "daytime_energy": 3,
        "sleep_start_local_time": "23:00",
        "sleep_end_local_time": "06:00",
        "input_source": "typed",
        "input_confirmation": {"status": "confirmed", "method": "explicit_acceptance"}
    });
    if let Some(minutes) = screen_minutes {
        personal_feedback["screen_minutes_in_final_hour"] = json!(minutes);
    }
    sqlx::query(
        "INSERT INTO memories (id, user_id, space_id, namespace_id, title, content, memory_type, source_type, source_metadata) VALUES ($1,$2,$3,$4,'sleep check-in','confirmed sleep check-in','text','surface_capture',$5)",
    )
    .bind(memory_id)
    .bind(fixture.owner_user_id)
    .bind(fixture.space_id)
    .bind(namespace_id)
    .bind(json!({"capture":{"personal_feedback":personal_feedback}}))
    .execute(pool)
    .await
    .expect("sleep evidence should insert");
    memory_id
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
