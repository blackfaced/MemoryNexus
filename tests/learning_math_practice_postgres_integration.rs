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
        user::PostgresUserRepository,
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
async fn learning_math_practice_routes_cover_session_lifecycle_and_access_control() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_users_and_spaces(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();

    let owner_token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let outsider_token = token_for(fixture.outsider_user_id, &fixture.outsider_email);

    let wrong_kind_response = client
        .post(format!("{base_url}/api/v1/learning/math/practice-sessions"))
        .bearer_auth(&owner_token)
        .json(&json!({
            "space_id": fixture.wrong_kind_space_id,
            "practice_goal": "Improve fraction word problems",
            "exercise": "Solve five fraction problems"
        }))
        .send()
        .await
        .expect("wrong-kind create request should send");
    assert_eq!(wrong_kind_response.status(), StatusCode::BAD_REQUEST);

    let create_response = client
        .post(format!("{base_url}/api/v1/learning/math/practice-sessions"))
        .bearer_auth(&owner_token)
        .json(&json!({
            "space_id": fixture.practice_space_id,
            "practice_goal": "Improve fraction word problems",
            "exercise": "Solve five fraction word problems",
            "capture_memory": true
        }))
        .send()
        .await
        .expect("create request should send");
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: Value = create_response
        .json()
        .await
        .expect("create response should be json");
    let session_id = uuid_field(&created, "/data/id");
    let namespace_id = uuid_field(&created, "/data/namespace_id");
    assert_eq!(
        created
            .pointer("/data/practice_goal")
            .and_then(Value::as_str),
        Some("Improve fraction word problems")
    );
    assert_eq!(
        created.pointer("/data/exercise").and_then(Value::as_str),
        Some("Solve five fraction word problems")
    );

    let namespace_kind: String = sqlx::query_scalar("SELECT kind FROM namespaces WHERE id = $1")
        .bind(namespace_id)
        .fetch_one(&pool)
        .await
        .expect("created namespace should be readable");
    assert_eq!(namespace_kind, "skill");

    let create_snapshot_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM memories
        WHERE space_id = $1
          AND source_type = 'feedback_loop_event'
          AND source_metadata->>'feedback_loop_id' = $2
          AND source_metadata->>'event_kind' = 'create'
        "#,
    )
    .bind(fixture.practice_space_id)
    .bind(session_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("create snapshot count should query");
    assert_eq!(create_snapshot_count, 1);

    let list_response = client
        .get(format!(
            "{base_url}/api/v1/learning/math/practice-sessions?space_id={}",
            fixture.practice_space_id
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .expect("list request should send");
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: Value = list_response
        .json()
        .await
        .expect("list response should be json");
    assert_eq!(
        listed.pointer("/data/total").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        listed.pointer("/data/items/0/id").and_then(Value::as_str),
        Some(session_id.to_string().as_str())
    );

    let get_response = client
        .get(format!(
            "{base_url}/api/v1/learning/math/practice-sessions/{session_id}"
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .expect("get request should send");
    assert_eq!(get_response.status(), StatusCode::OK);

    let unauthorized_get = client
        .get(format!(
            "{base_url}/api/v1/learning/math/practice-sessions/{session_id}"
        ))
        .bearer_auth(&outsider_token)
        .send()
        .await
        .expect("unauthorized get request should send");
    assert_eq!(unauthorized_get.status(), StatusCode::UNAUTHORIZED);

    let attempt_response = client
        .patch(format!(
            "{base_url}/api/v1/learning/math/practice-sessions/{session_id}/attempt"
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "answer": "I solved 3 out of 5 and mixed up units",
            "capture_memory": true
        }))
        .send()
        .await
        .expect("attempt patch should send");
    assert_eq!(attempt_response.status(), StatusCode::OK);
    let attempted: Value = attempt_response
        .json()
        .await
        .expect("attempt response should be json");
    assert_eq!(
        attempted.pointer("/data/answer").and_then(Value::as_str),
        Some("I solved 3 out of 5 and mixed up units")
    );

    let feedback_response = client
        .patch(format!(
            "{base_url}/api/v1/learning/math/practice-sessions/{session_id}/feedback"
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "mistake_pattern": "Units changed between steps",
            "feedback": "Write units next to every number",
            "practice_adjustment": "Add a unit-labeling step",
            "next_exercise": "Try three unit-conversion fraction problems",
            "status": "completed",
            "capture_memory": true
        }))
        .send()
        .await
        .expect("feedback patch should send");
    assert_eq!(feedback_response.status(), StatusCode::OK);
    let feedback: Value = feedback_response
        .json()
        .await
        .expect("feedback response should be json");
    assert_eq!(
        feedback
            .pointer("/data/mistake_pattern")
            .and_then(Value::as_str),
        Some("Units changed between steps")
    );
    assert_eq!(
        feedback
            .pointer("/data/next_exercise")
            .and_then(Value::as_str),
        Some("Try three unit-conversion fraction problems")
    );
    assert_eq!(
        feedback.pointer("/data/status").and_then(Value::as_str),
        Some("completed")
    );

    let patch_snapshot_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM memories
        WHERE space_id = $1
          AND source_type = 'feedback_loop_event'
          AND source_metadata->>'feedback_loop_id' = $2
          AND source_metadata->>'event_kind' = 'patch'
        "#,
    )
    .bind(fixture.practice_space_id)
    .bind(session_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("patch snapshot count should query");
    assert_eq!(patch_snapshot_count, 2);

    let stem_create_response = client
        .post(format!(
            "{base_url}/api/v1/namespaces/{}/practice-sessions",
            fixture.stem_namespace_id
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "practice_goal": "Improve STEM fraction reasoning",
            "exercise": "Solve three fraction word problems and explain units",
            "capture_memory": true
        }))
        .send()
        .await
        .expect("canonical namespace create request should send");
    assert_eq!(stem_create_response.status(), StatusCode::CREATED);
    let stem_created: Value = stem_create_response
        .json()
        .await
        .expect("canonical create response should be json");
    let stem_session_id = uuid_field(&stem_created, "/data/id");
    assert_eq!(
        stem_created
            .pointer("/data/namespace_id")
            .and_then(Value::as_str),
        Some(fixture.stem_namespace_id.to_string().as_str())
    );
    assert_eq!(
        stem_created
            .pointer("/data/space_id")
            .and_then(Value::as_str),
        Some(fixture.practice_space_id.to_string().as_str())
    );

    let stem_list_response = client
        .get(format!(
            "{base_url}/api/v1/namespaces/{}/practice-sessions",
            fixture.stem_namespace_id
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .expect("canonical namespace list request should send");
    assert_eq!(stem_list_response.status(), StatusCode::OK);
    let stem_listed: Value = stem_list_response
        .json()
        .await
        .expect("canonical list response should be json");
    assert_eq!(
        stem_listed.pointer("/data/total").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        stem_listed
            .pointer("/data/items/0/id")
            .and_then(Value::as_str),
        Some(stem_session_id.to_string().as_str())
    );

    let stem_attempt_response = client
        .patch(format!(
            "{base_url}/api/v1/namespaces/{}/practice-sessions/{stem_session_id}/attempt",
            fixture.stem_namespace_id
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "answer": "I explained each unit before calculating"
        }))
        .send()
        .await
        .expect("canonical namespace attempt patch should send");
    assert_eq!(stem_attempt_response.status(), StatusCode::OK);

    let wrong_namespace_get = client
        .get(format!(
            "{base_url}/api/v1/namespaces/{}/practice-sessions/{stem_session_id}",
            fixture.other_skill_namespace_id
        ))
        .bearer_auth(&owner_token)
        .send()
        .await
        .expect("wrong namespace get request should send");
    assert_eq!(wrong_namespace_get.status(), StatusCode::UNAUTHORIZED);

    let mismatched_space_create = client
        .post(format!(
            "{base_url}/api/v1/namespaces/{}/practice-sessions",
            fixture.stem_namespace_id
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "space_id": fixture.wrong_kind_space_id,
            "practice_goal": "Mismatched Space",
            "exercise": "Should be rejected"
        }))
        .send()
        .await
        .expect("mismatched space create request should send");
    assert_eq!(mismatched_space_create.status(), StatusCode::BAD_REQUEST);

    let unauthorized_create = client
        .post(format!("{base_url}/api/v1/learning/math/practice-sessions"))
        .bearer_auth(&outsider_token)
        .json(&json!({
            "space_id": fixture.practice_space_id,
            "practice_goal": "Improve fraction word problems",
            "exercise": "Solve five fraction problems"
        }))
        .send()
        .await
        .expect("unauthorized create request should send");
    assert_eq!(unauthorized_create.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires PostgreSQL and DATABASE_URL"]
async fn weekly_learning_review_route_enforces_space_namespace_and_lens_boundaries() {
    let pool = postgres_pool().await;
    db::run_migrations(&pool)
        .await
        .expect("migrations should run");
    let fixture = seed_users_and_spaces(&pool).await;
    let base_url = spawn_api(pool.clone()).await;
    let client = Client::new();

    let owner_token = token_for(fixture.owner_user_id, &fixture.owner_email);
    let outsider_token = token_for(fixture.outsider_user_id, &fixture.outsider_email);
    let learning_lens_id = seed_lens(
        &pool,
        fixture.practice_space_id,
        fixture.owner_user_id,
        "Weekly Learning Review",
    )
    .await;
    let cross_space_lens_id = seed_lens(
        &pool,
        fixture.wrong_kind_space_id,
        fixture.owner_user_id,
        "Cross Space Review",
    )
    .await;
    let window_start = Utc::now() - Duration::days(7);
    let window_end = Utc::now() + Duration::minutes(1);

    let happy_response = client
        .post(format!(
            "{base_url}/api/v1/namespaces/{}/learning-reviews",
            fixture.stem_namespace_id
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "lens_id": learning_lens_id,
            "window_start": window_start,
            "window_end": window_end,
            "limit": 10
        }))
        .send()
        .await
        .expect("owner learning review request should send");
    assert_eq!(happy_response.status(), StatusCode::CREATED);
    let happy: Value = happy_response
        .json()
        .await
        .expect("happy response should be json");
    assert_eq!(
        happy.pointer("/data/report_type").and_then(Value::as_str),
        Some("weekly_learning_review")
    );
    assert_eq!(
        happy
            .pointer("/data/report/namespace/id")
            .and_then(Value::as_str),
        Some(fixture.stem_namespace_id.to_string().as_str())
    );

    let unauthorized_response = client
        .post(format!(
            "{base_url}/api/v1/namespaces/{}/learning-reviews",
            fixture.stem_namespace_id
        ))
        .bearer_auth(&outsider_token)
        .json(&json!({
            "lens_id": learning_lens_id,
            "window_start": window_start,
            "window_end": window_end
        }))
        .send()
        .await
        .expect("outsider learning review request should send");
    assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);

    let missing_namespace_response = client
        .post(format!(
            "{base_url}/api/v1/namespaces/{}/learning-reviews",
            Uuid::new_v4()
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "lens_id": learning_lens_id,
            "window_start": window_start,
            "window_end": window_end
        }))
        .send()
        .await
        .expect("missing namespace learning review request should send");
    assert_eq!(
        missing_namespace_response.status(),
        StatusCode::UNAUTHORIZED
    );

    let cross_space_lens_response = client
        .post(format!(
            "{base_url}/api/v1/namespaces/{}/learning-reviews",
            fixture.stem_namespace_id
        ))
        .bearer_auth(&owner_token)
        .json(&json!({
            "lens_id": cross_space_lens_id,
            "window_start": window_start,
            "window_end": window_end
        }))
        .send()
        .await
        .expect("cross-space lens learning review request should send");
    assert_eq!(cross_space_lens_response.status(), StatusCode::BAD_REQUEST);
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
        users: Arc::new(PostgresUserRepository::new(pool.clone())),
        vectors: Arc::new(NoopVectorRepository),
    };
    AppState::new(pool, repositories, None)
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

struct PracticeFixture {
    owner_user_id: Uuid,
    owner_email: String,
    outsider_user_id: Uuid,
    outsider_email: String,
    practice_space_id: Uuid,
    wrong_kind_space_id: Uuid,
    stem_namespace_id: Uuid,
    other_skill_namespace_id: Uuid,
}

async fn postgres_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for ignored PostgreSQL integration tests");
    db::init_pool(&database_url)
        .await
        .expect("should connect to PostgreSQL")
}

async fn seed_users_and_spaces(pool: &PgPool) -> PracticeFixture {
    let suffix = Uuid::new_v4();
    let owner_email = format!("learning-math-owner-{suffix}@example.com");
    let outsider_email = format!("learning-math-outsider-{suffix}@example.com");
    let owner_user_id =
        seed_user(pool, &owner_email, &format!("learning-math-owner-{suffix}")).await;
    let outsider_user_id = seed_user(
        pool,
        &outsider_email,
        &format!("learning-math-outsider-{suffix}"),
    )
    .await;
    let practice_space_id = seed_space(
        pool,
        owner_user_id,
        &format!("Learning Math Practice {suffix}"),
    )
    .await;
    let wrong_kind_space_id = seed_space(
        pool,
        owner_user_id,
        &format!("Learning Math Wrong Kind {suffix}"),
    )
    .await;

    sqlx::query(
        r#"
        INSERT INTO namespaces (space_id, name, kind, created_by)
        VALUES ($1, 'learning.math', 'reflective', $2)
        "#,
    )
    .bind(wrong_kind_space_id)
    .bind(owner_user_id)
    .execute(pool)
    .await
    .expect("wrong-kind namespace should insert");

    let stem_namespace_id = seed_namespace(
        pool,
        practice_space_id,
        owner_user_id,
        "learning.stem",
        "skill",
    )
    .await;
    let other_skill_namespace_id = seed_namespace(
        pool,
        practice_space_id,
        owner_user_id,
        "learning.science",
        "skill",
    )
    .await;

    PracticeFixture {
        owner_user_id,
        owner_email,
        outsider_user_id,
        outsider_email,
        practice_space_id,
        wrong_kind_space_id,
        stem_namespace_id,
        other_skill_namespace_id,
    }
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

async fn seed_user(pool: &PgPool, email: &str, username: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (email, username, password_hash)
        VALUES ($1, $2, 'learning-math-integration-test')
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
