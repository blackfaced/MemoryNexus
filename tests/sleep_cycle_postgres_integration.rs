use chrono::{Duration, Utc};
use memorynexus::{
    db::{
        feedback_loop::{
            CreateFeedbackLoop, FeedbackLoopRepository, PostgresFeedbackLoopRepository,
        },
        sleep_cycles::{
            CompleteSleepCycle, CreateSleepCycle, FailSleepCycle, PostgresSleepCycleRepository,
            SleepCycleRepository,
        },
    },
    domain::sleep_cycle::{SleepCycleStatus, SleepCycleType},
};
use serde_json::json;
use sqlx::{Error, PgPool};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn sleep_cycle_repository_persists_completed_and_failed_lifecycle_states() {
    let pool = postgres_pool().await;
    memorynexus::db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_space_namespace(&pool, "lifecycle").await;
    let repository = PostgresSleepCycleRepository::new(pool.clone());
    let input_trace_id = seed_trace(&pool, &fixture).await;

    let window_start = Utc::now() - Duration::days(1);
    let window_end = Utc::now();
    let sleep_cycle = repository
        .create(CreateSleepCycle {
            space_id: fixture.space_id,
            namespace_id: Some(fixture.namespace_id),
            cycle_type: SleepCycleType::Manual,
            status: SleepCycleStatus::Pending,
            evidence_window_start: window_start,
            evidence_window_end: window_end,
            input_trace_ids: vec![input_trace_id, input_trace_id],
            input_memory_ids: Vec::new(),
            input_feedback_loop_ids: Vec::new(),
            input_review_report_ids: Vec::new(),
            triggering_trace_id: Some(input_trace_id),
            metadata: json!({"reason": "manual trigger"}),
        })
        .await
        .expect("sleep cycle should create");

    assert_eq!(sleep_cycle.space_id, fixture.space_id);
    assert_eq!(sleep_cycle.namespace_id, Some(fixture.namespace_id));
    assert_eq!(sleep_cycle.cycle_type, "manual");
    assert_eq!(sleep_cycle.status, "pending");
    assert_eq!(sleep_cycle.evidence_window_start, window_start);
    assert_eq!(sleep_cycle.evidence_window_end, window_end);
    assert_eq!(
        sleep_cycle.input_trace_ids,
        vec![input_trace_id, input_trace_id]
    );
    assert_eq!(sleep_cycle.triggering_trace_id, Some(input_trace_id));
    assert!(sleep_cycle.started_at.is_some());

    let completed = repository
        .mark_completed(
            sleep_cycle.id,
            CompleteSleepCycle {
                generated_memory_ids: Vec::new(),
                metadata: json!({"summary": "deterministic local consolidation"}),
            },
        )
        .await
        .expect("sleep cycle completion should persist")
        .expect("sleep cycle should exist");

    assert_eq!(completed.status, "completed");
    assert_eq!(completed.error, None);
    assert!(completed.completed_at.is_some());

    let failed_cycle = repository
        .create(CreateSleepCycle {
            space_id: fixture.space_id,
            namespace_id: Some(fixture.namespace_id),
            cycle_type: SleepCycleType::Daily,
            status: SleepCycleStatus::Running,
            evidence_window_start: window_start,
            evidence_window_end: window_end,
            input_trace_ids: Vec::new(),
            input_memory_ids: Vec::new(),
            input_feedback_loop_ids: Vec::new(),
            input_review_report_ids: Vec::new(),
            triggering_trace_id: None,
            metadata: json!({}),
        })
        .await
        .expect("second sleep cycle should create");

    let failed = repository
        .mark_failed(
            failed_cycle.id,
            FailSleepCycle {
                error: "same_space_validation_failed".to_string(),
                metadata: json!({"stage": "validation"}),
            },
        )
        .await
        .expect("sleep cycle failure should persist")
        .expect("sleep cycle should exist");

    assert_eq!(failed.status, "failed");
    assert_eq!(
        failed.error.as_deref(),
        Some("same_space_validation_failed")
    );
    assert!(failed.completed_at.is_some());
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn sleep_cycle_repository_rejects_cross_space_namespace_memory_and_feedback_loop_links() {
    let pool = postgres_pool().await;
    memorynexus::db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let first = seed_space_namespace(&pool, "first").await;
    let second = seed_space_namespace(&pool, "second").await;
    let memory_id = seed_memory(&pool, second.user_id, second.space_id).await;
    let feedback_loop_id = seed_feedback_loop(&pool, &second).await;
    let trace_id = seed_trace(&pool, &second).await;
    let review_report_id = seed_review_report(&pool, &second).await;
    let repository = PostgresSleepCycleRepository::new(pool.clone());

    let cross_space_namespace = repository
        .create(base_create_sleep_cycle(
            first.space_id,
            Some(second.namespace_id),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
        ))
        .await;
    assert!(
        matches!(cross_space_namespace, Err(Error::RowNotFound)),
        "cross-space namespace should fail before insert"
    );

    let cross_space_memory = repository
        .create(base_create_sleep_cycle(
            first.space_id,
            Some(first.namespace_id),
            Vec::new(),
            vec![memory_id],
            Vec::new(),
            None,
        ))
        .await;
    assert!(
        matches!(cross_space_memory, Err(Error::RowNotFound)),
        "cross-space memory should fail before insert"
    );

    let cross_space_feedback_loop = repository
        .create(base_create_sleep_cycle(
            first.space_id,
            Some(first.namespace_id),
            Vec::new(),
            Vec::new(),
            vec![feedback_loop_id],
            None,
        ))
        .await;
    assert!(
        matches!(cross_space_feedback_loop, Err(Error::RowNotFound)),
        "cross-space feedback loop should fail before insert"
    );

    let cross_space_review_report = repository
        .create(CreateSleepCycle {
            input_review_report_ids: vec![review_report_id],
            ..base_create_sleep_cycle(
                first.space_id,
                Some(first.namespace_id),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                None,
            )
        })
        .await;
    assert!(
        matches!(cross_space_review_report, Err(Error::RowNotFound)),
        "cross-space review report should fail before insert"
    );

    let cross_space_trace = repository
        .create(base_create_sleep_cycle(
            first.space_id,
            Some(first.namespace_id),
            vec![trace_id],
            Vec::new(),
            Vec::new(),
            None,
        ))
        .await;
    assert!(
        matches!(cross_space_trace, Err(Error::RowNotFound)),
        "cross-space trace should fail before insert"
    );

    let cross_space_triggering_trace = repository
        .create(base_create_sleep_cycle(
            first.space_id,
            Some(first.namespace_id),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Some(trace_id),
        ))
        .await;
    assert!(
        matches!(cross_space_triggering_trace, Err(Error::RowNotFound)),
        "cross-space triggering trace should fail before insert"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn sleep_cycle_repository_rejects_cross_space_generated_memory_links_on_completion() {
    let pool = postgres_pool().await;
    memorynexus::db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let first = seed_space_namespace(&pool, "first").await;
    let second = seed_space_namespace(&pool, "second").await;
    let repository = PostgresSleepCycleRepository::new(pool.clone());
    let sleep_cycle = repository
        .create(base_create_sleep_cycle(
            first.space_id,
            Some(first.namespace_id),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
        ))
        .await
        .expect("sleep cycle should create");
    let cross_space_memory_id = seed_memory(&pool, second.user_id, second.space_id).await;

    let completion = repository
        .mark_completed(
            sleep_cycle.id,
            CompleteSleepCycle {
                generated_memory_ids: vec![cross_space_memory_id],
                metadata: json!({"summary": "invalid cross-space output"}),
            },
        )
        .await;

    assert!(
        matches!(completion, Err(Error::RowNotFound)),
        "cross-space generated memory should fail before update"
    );
}

struct Fixture {
    user_id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
}

fn base_create_sleep_cycle(
    space_id: Uuid,
    namespace_id: Option<Uuid>,
    input_trace_ids: Vec<Uuid>,
    input_memory_ids: Vec<Uuid>,
    input_feedback_loop_ids: Vec<Uuid>,
    triggering_trace_id: Option<Uuid>,
) -> CreateSleepCycle {
    CreateSleepCycle {
        space_id,
        namespace_id,
        cycle_type: SleepCycleType::Manual,
        status: SleepCycleStatus::Pending,
        evidence_window_start: Utc::now() - Duration::days(1),
        evidence_window_end: Utc::now(),
        input_trace_ids,
        input_memory_ids,
        input_feedback_loop_ids,
        input_review_report_ids: Vec::new(),
        triggering_trace_id,
        metadata: json!({}),
    }
}

async fn postgres_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for ignored PostgreSQL integration tests");
    memorynexus::db::init_pool(&database_url)
        .await
        .expect("should connect to PostgreSQL")
}

async fn seed_space_namespace(pool: &PgPool, label: &str) -> Fixture {
    let suffix = Uuid::new_v4();
    let user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'postgres-integration-test')
        RETURNING id
        "#,
    )
    .bind(format!("sleep-cycle-{label}-{suffix}@example.com"))
    .bind(format!("sleep-cycle-{label}-{suffix}"))
    .fetch_one(pool)
    .await
    .expect("user seed should insert");

    let space_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO cognitive_spaces (name, owner_user_id, space_type)
        VALUES ($1, $2, 'personal')
        RETURNING id
        "#,
    )
    .bind(format!("SleepCycle {label} {suffix}"))
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

    let namespace_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (space_id, name, kind, created_by)
        VALUES ($1, $2, 'skill', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(format!("child.english.spelling.{label}.{suffix}"))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("namespace seed should insert");

    Fixture {
        user_id,
        space_id,
        namespace_id,
    }
}

async fn seed_memory(pool: &PgPool, user_id: Uuid, space_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO memories (
            user_id,
            space_id,
            title,
            content,
            memory_type,
            is_shared,
            source_type,
            source_metadata
        )
        VALUES ($1, $2, 'Dictation note', 'missed double letter', 'text', false, 'manual', $3)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(space_id)
    .bind(json!({}))
    .fetch_one(pool)
    .await
    .expect("memory seed should insert")
}

async fn seed_feedback_loop(pool: &PgPool, fixture: &Fixture) -> Uuid {
    let repository = PostgresFeedbackLoopRepository::new(pool.clone());
    repository
        .create(CreateFeedbackLoop {
            space_id: fixture.space_id,
            namespace_id: fixture.namespace_id,
            goal: "Improve spelling accuracy".to_string(),
            task: "Practice double-letter words".to_string(),
            attempt: None,
            evaluation: None,
            feedback: None,
            adjustment: None,
            next_task: None,
            status: "active".to_string(),
            created_by: fixture.user_id,
        })
        .await
        .expect("feedback loop seed should insert")
        .id
}

async fn seed_review_report(pool: &PgPool, fixture: &Fixture) -> Uuid {
    let lens_id = seed_lens(
        pool,
        fixture.space_id,
        fixture.user_id,
        "sleep-cycle-integration-lens",
    )
    .await;

    sqlx::query_scalar(
        r#"
        INSERT INTO cognitive_review_reports (
            space_id,
            lens_id,
            namespace_id,
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
            'periodic_review',
            NOW() - INTERVAL '1 day',
            NOW(),
            $4,
            '{}',
            '{}',
            'deterministic',
            'deterministic',
            $5
        )
        RETURNING id
        "#,
    )
    .bind(fixture.space_id)
    .bind(lens_id)
    .bind(fixture.namespace_id)
    .bind(json!({"summary": "seed review"}))
    .bind(fixture.user_id)
    .fetch_one(pool)
    .await
    .expect("review report seed should insert")
}

async fn seed_lens(pool: &PgPool, space_id: Uuid, owner_user_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO lenses (
            space_id,
            name,
            strategy,
            output_format,
            retrieval_mode,
            created_by
        )
        VALUES ($1, $2, 'learning_review', 'bullets', 'keyword', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(name)
    .bind(owner_user_id)
    .fetch_one(pool)
    .await
    .expect("lens seed should insert")
}

async fn seed_trace(pool: &PgPool, fixture: &Fixture) -> Uuid {
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
            completed_at,
            latency_ms,
            status,
            metadata
        )
        VALUES (
            $1,
            $2,
            'test_fixture',
            'consolidation',
            'deep',
            'deterministic',
            'seed sleep evidence',
            'seed sleep result',
            NOW(),
            1,
            'completed',
            $3
        )
        RETURNING id
        "#,
    )
    .bind(fixture.space_id)
    .bind(fixture.namespace_id)
    .bind(json!({"fixture": "sleep_cycle_postgres_integration"}))
    .fetch_one(pool)
    .await
    .expect("trace seed should insert")
}
