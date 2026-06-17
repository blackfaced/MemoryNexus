use chrono::Utc;
use memorynexus::db::trace::{
    CreateCompletedTrace, PostgresTraceRepository, TraceListFilter, TraceMode, TraceRepository,
    TraceRuntime, TraceSourceType, TraceStatus, TraceTaskType,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn trace_repository_create_get_and_list_are_space_scoped() {
    let pool = postgres_pool().await;
    memorynexus::db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_two_spaces(&pool).await;
    let repository = PostgresTraceRepository::new(pool.clone());

    let memory_id = Uuid::new_v4();
    let lens_run_id = Uuid::new_v4();
    let created = repository
        .create_completed(CreateCompletedTrace {
            space_id: fixture.space_id,
            namespace_id: Some(fixture.namespace_id),
            source_type: TraceSourceType::Mcp,
            task_type: TraceTaskType::LensRun,
            mode: TraceMode::Focused,
            runtime: TraceRuntime::Deterministic,
            input_summary: Some("redacted lens query".to_string()),
            output_summary: Some("redacted lens output".to_string()),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            latency_ms: Some(37),
            model_provider: Some("deterministic".to_string()),
            model_name: None,
            token_usage: Some(json!({"input": 0, "output": 0, "total": 0})),
            estimated_cost_usd: Some(0.0),
            local_processing_ratio: Some(1.0),
            related_memory_ids: vec![memory_id],
            generated_memory_ids: vec![],
            generated_lens_run_ids: vec![lens_run_id],
            generated_review_report_ids: vec![],
            generated_feedback_loop_ids: vec![],
            user_feedback: None,
            error: None,
            metadata: json!({"fixture": "trace_repository_create_get_and_list"}),
        })
        .await
        .expect("trace create should succeed");

    assert_eq!(created.space_id, fixture.space_id);
    assert_eq!(created.namespace_id, Some(fixture.namespace_id));
    assert_eq!(created.status, "completed");
    assert_eq!(created.related_memory_ids, vec![memory_id]);
    assert_eq!(created.generated_lens_run_ids, vec![lens_run_id]);

    let found = repository
        .find_for_user(created.id, fixture.user_id)
        .await
        .expect("trace get should query")
        .expect("trace should be visible to space member");
    assert_eq!(found.id, created.id);

    let cross_space_get = repository
        .find_for_user(created.id, fixture.other_user_id)
        .await
        .expect("cross-space get should query");
    assert!(cross_space_get.is_none());

    let visible = repository
        .list_for_user(
            TraceListFilter {
                space_id: fixture.space_id,
                namespace_id: Some(fixture.namespace_id),
                task_type: Some(TraceTaskType::LensRun),
                mode: Some(TraceMode::Focused),
                runtime: Some(TraceRuntime::Deterministic),
                status: Some(TraceStatus::Completed),
                limit: 10,
                offset: 0,
            },
            fixture.user_id,
        )
        .await
        .expect("trace list should query");
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, created.id);

    let cross_space_list = repository
        .list_for_user(
            TraceListFilter {
                space_id: fixture.space_id,
                namespace_id: None,
                task_type: None,
                mode: None,
                runtime: None,
                status: None,
                limit: 10,
                offset: 0,
            },
            fixture.other_user_id,
        )
        .await
        .expect("cross-space list should query");
    assert!(cross_space_list.is_empty());
}

struct Fixture {
    user_id: Uuid,
    other_user_id: Uuid,
    space_id: Uuid,
    namespace_id: Uuid,
}

async fn postgres_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for ignored PostgreSQL integration tests");
    memorynexus::db::init_pool(&database_url)
        .await
        .expect("should connect to PostgreSQL")
}

async fn seed_two_spaces(pool: &PgPool) -> Fixture {
    let suffix = Uuid::new_v4();
    let user_id = seed_user(pool, &format!("trace-{suffix}")).await;
    let other_user_id = seed_user(pool, &format!("trace-other-{suffix}")).await;

    let space_id = seed_space(pool, user_id, &format!("Trace Integration {suffix}")).await;
    let _other_space_id = seed_space(
        pool,
        other_user_id,
        &format!("Other Trace Integration {suffix}"),
    )
    .await;

    let namespace_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (space_id, name, kind, created_by)
        VALUES ($1, $2, 'skill', $3)
        RETURNING id
        "#,
    )
    .bind(space_id)
    .bind(format!("learning.stem.{suffix}"))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("namespace seed should insert");

    Fixture {
        user_id,
        other_user_id,
        space_id,
        namespace_id,
    }
}

async fn seed_user(pool: &PgPool, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'postgres-integration-test')
        RETURNING id
        "#,
    )
    .bind(format!("{username}@example.com"))
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
