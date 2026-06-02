use std::fs;

#[test]
fn feedback_loop_migration_scopes_rows_to_space_and_namespace() {
    let sql = fs::read_to_string("migrations/010_feedback_loops.sql")
        .expect("feedback loop migration should exist");

    assert!(sql.contains("CREATE TABLE IF NOT EXISTS feedback_loops"));
    assert!(sql.contains("space_id UUID NOT NULL REFERENCES cognitive_spaces(id)"));
    assert!(sql.contains("namespace_id UUID NOT NULL"));
    assert!(sql.contains("FOREIGN KEY (namespace_id, space_id)"));
    assert!(sql.contains("REFERENCES namespaces(id, space_id)"));
    assert!(!sql.contains("CREATE TABLE IF NOT EXISTS namespaces"));
}

#[test]
fn feedback_loop_migration_defines_minimal_loop_fields_and_statuses() {
    let sql = fs::read_to_string("migrations/010_feedback_loops.sql")
        .expect("feedback loop migration should exist");

    for column in [
        "goal TEXT NOT NULL",
        "task TEXT NOT NULL",
        "attempt TEXT",
        "evaluation TEXT",
        "feedback TEXT",
        "adjustment TEXT",
        "next_task TEXT",
        "status VARCHAR(50) NOT NULL DEFAULT 'active'",
        "created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
    ] {
        assert!(sql.contains(column), "missing column contract: {column}");
    }

    assert!(sql.contains("feedback_loops_status_check"));
    assert!(sql.contains("status IN ('active', 'completed', 'paused')"));
}

#[test]
fn feedback_loop_routes_expose_create_list_get_and_patch_api() {
    let routes = fs::read_to_string("src/api/mod.rs").expect("api routes should be readable");

    assert!(routes.contains("/api/v1/feedback-loops"));
    assert!(routes.contains("feedback_loops::create"));
    assert!(routes.contains("feedback_loops::list"));
    assert!(routes.contains("/api/v1/feedback-loops/:id"));
    assert!(routes.contains("feedback_loops::get"));
    assert!(routes.contains("feedback_loops::patch"));
}

#[test]
fn feedback_loop_patch_contract_updates_attempt_independently() {
    let api = fs::read_to_string("src/api/feedback_loops.rs")
        .expect("feedback loop API should be readable");
    let repository =
        fs::read_to_string("src/db/feedback_loop.rs").expect("repository should be readable");

    assert!(api.contains("pub attempt: Option<String>"));
    assert!(api.contains("let attempt = normalize_optional(req.attempt);"));
    assert!(repository.contains("pub attempt: Option<String>"));
    assert!(repository.contains("attempt = COALESCE($2, attempt)"));
    assert!(repository.contains("evaluation = COALESCE($3, evaluation)"));
    assert!(repository.contains(".bind(&patch.attempt)"));
}

#[test]
fn feedback_loop_memory_capture_contract_is_opt_in_and_traceable() {
    let api = fs::read_to_string("src/api/feedback_loops.rs")
        .expect("feedback loop API should be readable");
    let repository =
        fs::read_to_string("src/db/feedback_loop.rs").expect("repository should be readable");

    assert!(api.contains("pub capture_memory: Option<bool>"));
    assert!(api.contains("alias = \"create_memory_snapshot\""));
    assert!(repository.contains("'feedback_loop_event'"));
    assert!(repository.contains("\"feedback_loop_id\""));
    assert!(repository.contains("\"namespace_id\""));
    assert!(repository.contains("\"space_id\""));
    assert!(repository.contains("\"event_kind\""));
    assert!(repository.contains("\"included_fields\""));
    assert!(repository.contains("'text'"));
    assert!(repository.contains("false"));
}

#[test]
fn feedback_loop_memory_capture_contract_preserves_space_validation_before_capture() {
    let api = fs::read_to_string("src/api/feedback_loops.rs")
        .expect("feedback loop API should be readable");
    let writer_check = api
        .find("require_space_writer(&state, req.space_id, auth_user.user_id)")
        .expect("create should check writer permission");
    let namespace_check = api
        .find(
            "require_namespace_in_space(&state, req.namespace_id, req.space_id, auth_user.user_id)",
        )
        .expect("create should validate namespace space");
    let create_capture = api
        .find(".create_with_memory_snapshot(create_feedback_loop, snapshot)")
        .expect("create should use atomic feedback loop and memory capture");

    assert!(writer_check < create_capture);
    assert!(namespace_check < create_capture);

    let patch_writer_check = api
        .find("require_space_writer(&state, existing.space_id, auth_user.user_id)")
        .expect("patch should check writer permission");
    let patch_capture = api
        .find(".patch_with_memory_snapshot(id, patch_feedback_loop, snapshot)")
        .expect("patch should use atomic feedback loop and memory capture");

    assert!(patch_writer_check < patch_capture);
    assert!(api.contains("(\"attempt\", \"Answer / reasoning\", attempt.as_deref())"));
}
