use chrono::{Duration, Utc};
use memorynexus::db::feedback_loop::{
    CreateFeedbackLoop, FeedbackLoopMemorySnapshot, FeedbackLoopRepository, PatchFeedbackLoop,
    PostgresFeedbackLoopRepository,
};
use memorynexus::db::memory::{
    FeedbackLoopEventSnapshotFilter, MemoryRepository, PostgresMemoryRepository,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn feedback_loop_memory_snapshot_repository_paths_are_atomic_and_traceable() {
    let pool = postgres_pool().await;
    memorynexus::db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_space_namespace(&pool).await;
    let repository = PostgresFeedbackLoopRepository::new(pool.clone());

    let create_result = repository
        .create_with_memory_snapshot(
            CreateFeedbackLoop {
                space_id: fixture.space_id,
                namespace_id: fixture.namespace_id,
                goal: "Improve fraction word problems".to_string(),
                task: "Solve five fraction word problems".to_string(),
                attempt: None,
                evaluation: Some("3/5 correct; units were mixed".to_string()),
                feedback: Some("Label units before calculating".to_string()),
                adjustment: None,
                next_task: Some("Try three unit-conversion fraction problems".to_string()),
                status: "active".to_string(),
                created_by: fixture.user_id,
            },
            FeedbackLoopMemorySnapshot {
                user_id: fixture.user_id,
                event_kind: "create".to_string(),
                content: [
                    "Practice goal: Improve fraction word problems",
                    "Practice task: Solve five fraction word problems",
                    "Mistake pattern / evaluation: 3/5 correct; units were mixed",
                    "Feedback: Label units before calculating",
                    "Next exercise: Try three unit-conversion fraction problems",
                ]
                .join("\n"),
                included_fields: vec![
                    "goal".to_string(),
                    "task".to_string(),
                    "evaluation".to_string(),
                    "feedback".to_string(),
                    "next_task".to_string(),
                ],
            },
        )
        .await
        .expect("create with memory snapshot should commit");

    let create_memory = create_result
        .memory
        .expect("create capture should return generated memory");
    assert_eq!(create_memory.space_id, fixture.space_id);
    assert_eq!(create_memory.user_id, fixture.user_id);
    assert_eq!(create_memory.memory_type, "text");
    assert!(!create_memory.is_shared);
    assert_eq!(create_memory.source_type, "feedback_loop_event");
    assert_eq!(
        create_memory.source_metadata["feedback_loop_id"],
        create_result.feedback_loop.id.to_string()
    );
    assert_eq!(
        create_memory.source_metadata["namespace_id"],
        fixture.namespace_id.to_string()
    );
    assert_eq!(
        create_memory.source_metadata["space_id"],
        fixture.space_id.to_string()
    );
    assert_eq!(create_memory.source_metadata["event_kind"], "create");
    assert_eq!(
        create_memory.source_metadata["included_fields"],
        serde_json::json!(["goal", "task", "evaluation", "feedback", "next_task"])
    );

    let patch_result = repository
        .patch_with_memory_snapshot(
            create_result.feedback_loop.id,
            PatchFeedbackLoop {
                attempt: Some("Child added denominators directly".to_string()),
                evaluation: None,
                feedback: None,
                adjustment: None,
                next_task: None,
                status: None,
            },
            FeedbackLoopMemorySnapshot {
                user_id: fixture.user_id,
                event_kind: "patch".to_string(),
                content: "Answer / reasoning: Child added denominators directly".to_string(),
                included_fields: vec!["attempt".to_string()],
            },
        )
        .await
        .expect("patch with memory snapshot should commit")
        .expect("feedback loop should exist");

    let patch_memory = patch_result
        .memory
        .expect("patch capture should return generated memory");
    assert_eq!(
        patch_memory.content,
        "Answer / reasoning: Child added denominators directly"
    );
    assert!(!patch_memory.content.contains("3/5 correct"));
    assert!(!patch_memory
        .content
        .contains("Label units before calculating"));
    assert_eq!(patch_memory.source_metadata["event_kind"], "patch");
    assert_eq!(
        patch_memory.source_metadata["included_fields"],
        serde_json::json!(["attempt"])
    );

    let rollback_goal = format!("rollback-check-{}", Uuid::new_v4());
    let rollback_result = repository
        .create_with_memory_snapshot(
            CreateFeedbackLoop {
                space_id: fixture.space_id,
                namespace_id: fixture.namespace_id,
                goal: rollback_goal.clone(),
                task: "This loop should roll back".to_string(),
                attempt: None,
                evaluation: None,
                feedback: None,
                adjustment: None,
                next_task: None,
                status: "active".to_string(),
                created_by: fixture.user_id,
            },
            FeedbackLoopMemorySnapshot {
                user_id: Uuid::new_v4(),
                event_kind: "create".to_string(),
                content: "Practice goal: rollback should fail".to_string(),
                included_fields: vec!["goal".to_string()],
            },
        )
        .await;
    assert!(
        rollback_result.is_err(),
        "invalid snapshot user_id should fail memory insert"
    );

    let leftover_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM feedback_loops WHERE goal = $1")
            .bind(&rollback_goal)
            .fetch_one(&pool)
            .await
            .expect("rollback check query should run");
    assert_eq!(leftover_count, 0);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn feedback_loop_snapshot_query_ignores_unrelated_memory_before_limit() {
    let pool = postgres_pool().await;
    memorynexus::db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_space_namespace(&pool).await;
    let feedback_loop_repository = PostgresFeedbackLoopRepository::new(pool.clone());
    let memory_repository = PostgresMemoryRepository::new(pool.clone());

    let result = feedback_loop_repository
        .create_with_memory_snapshot(
            CreateFeedbackLoop {
                space_id: fixture.space_id,
                namespace_id: fixture.namespace_id,
                goal: "Improve fraction word problems".to_string(),
                task: "Solve one fraction word problem".to_string(),
                attempt: None,
                evaluation: Some("Changed units between steps".to_string()),
                feedback: Some("Label units before calculating".to_string()),
                adjustment: None,
                next_task: Some("Try a unit-conversion problem".to_string()),
                status: "completed".to_string(),
                created_by: fixture.user_id,
            },
            FeedbackLoopMemorySnapshot {
                user_id: fixture.user_id,
                event_kind: "create".to_string(),
                content: "Practice goal: Improve fraction word problems".to_string(),
                included_fields: vec!["goal".to_string()],
            },
        )
        .await
        .expect("feedback loop snapshot should create");
    let snapshot = result.memory.expect("snapshot should exist");

    for index in 0..5 {
        sqlx::query(
            r#"
            INSERT INTO memories (
                user_id,
                space_id,
                title,
                content,
                memory_type,
                is_shared,
                source_type,
                source_metadata,
                created_at
            )
            VALUES ($1, $2, 'Unrelated', $3, 'text', false, 'manual', $4, NOW() + ($5::int * INTERVAL '1 second'))
            "#,
        )
        .bind(fixture.user_id)
        .bind(fixture.space_id)
        .bind(format!("newer unrelated memory {index}"))
        .bind(json!({}))
        .bind(index + 1)
        .execute(&pool)
        .await
        .expect("unrelated memory should insert");
    }

    let memories = memory_repository
        .list_feedback_loop_event_snapshots(
            fixture.user_id,
            FeedbackLoopEventSnapshotFilter {
                space_id: fixture.space_id,
                namespace_id: fixture.namespace_id,
                feedback_loop_ids: vec![result.feedback_loop.id],
                window_start: Utc::now() - Duration::minutes(1),
                window_end: Utc::now() + Duration::minutes(1),
                limit: 1,
            },
        )
        .await
        .expect("snapshot query should succeed");

    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].id, snapshot.id);
    assert_eq!(memories[0].source_type, "feedback_loop_event");
}

struct Fixture {
    user_id: Uuid,
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

async fn seed_space_namespace(pool: &PgPool) -> Fixture {
    let suffix = Uuid::new_v4();
    let user_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'postgres-integration-test')
        RETURNING id
        "#,
    )
    .bind(format!("feedback-loop-{suffix}@example.com"))
    .bind(format!("feedback-loop-{suffix}"))
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
    .bind(format!("FeedbackLoop Integration {suffix}"))
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
    .bind(format!("learning.math.{suffix}"))
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
