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
    assert!(api.contains("attempt: normalize_optional(req.attempt)"));
    assert!(repository.contains("pub attempt: Option<String>"));
    assert!(repository.contains("attempt = COALESCE($2, attempt)"));
    assert!(repository.contains("evaluation = COALESCE($3, evaluation)"));
    assert!(repository.contains(".bind(&patch.attempt)"));
}
