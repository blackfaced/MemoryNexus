use std::fs;

#[test]
fn learning_math_practice_routes_expose_session_api() {
    let routes = fs::read_to_string("src/api/mod.rs").expect("api routes should be readable");

    assert!(routes.contains("mod learning_math;"));
    assert!(routes.contains("/api/v1/learning/math/practice-sessions"));
    assert!(routes.contains("learning_math::create"));
    assert!(routes.contains("learning_math::list"));
    assert!(routes.contains("/api/v1/learning/math/practice-sessions/:id"));
    assert!(routes.contains("learning_math::get"));
    assert!(routes.contains("/api/v1/learning/math/practice-sessions/:id/attempt"));
    assert!(routes.contains("learning_math::patch_attempt"));
    assert!(routes.contains("/api/v1/learning/math/practice-sessions/:id/feedback"));
    assert!(routes.contains("learning_math::patch_feedback"));
}

#[test]
fn learning_math_practice_api_is_thin_namespace_feedback_loop_facade() {
    let api =
        fs::read_to_string("src/api/learning_math.rs").expect("learning math API should exist");

    assert!(api.contains("LEARNING_MATH_NAMESPACE"));
    assert!(api.contains("\"learning.math\""));
    assert!(api.contains("CreateNamespace"));
    assert!(api.contains("NamespaceKind::Skill"));
    assert!(api.contains("CreateFeedbackLoop"));
    assert!(api.contains("FeedbackLoopListFilter"));
    assert!(api.contains("PatchFeedbackLoop"));
    assert!(api.contains("create_with_memory_snapshot"));
    assert!(api.contains("patch_with_memory_snapshot"));
}

#[test]
fn learning_math_practice_api_keeps_space_and_namespace_boundary_before_writes() {
    let api =
        fs::read_to_string("src/api/learning_math.rs").expect("learning math API should exist");

    let writer_check = api
        .find("require_space_writer(&state, req.space_id, auth_user.user_id)")
        .expect("create should require writable Cognitive Space");
    let namespace_ensure = api
        .find("ensure_learning_math_namespace(&state, req.space_id, req.namespace_id, auth_user.user_id)")
        .expect("create should create or reuse learning.math namespace");
    let create_loop = api
        .find(".create_with_memory_snapshot(create_feedback_loop, snapshot)")
        .expect("create should write FeedbackLoop with optional Memory snapshot");

    assert!(writer_check < create_loop);
    assert!(namespace_ensure < create_loop);

    let patch_writer_check = api
        .find("require_space_writer(&state, existing.space_id, auth_user.user_id)")
        .expect("patch should require writable session Space");
    let patch_namespace_check = api
        .find("require_learning_math_namespace(")
        .expect("patch should verify session belongs to learning.math");
    let patch_snapshot = api
        .find(".patch_with_memory_snapshot(id, patch, snapshot)")
        .expect("patch should use FeedbackLoop patch with Memory snapshot");

    assert!(patch_writer_check < patch_snapshot);
    assert!(patch_namespace_check < patch_snapshot);
}

#[test]
fn learning_math_practice_list_does_not_create_namespace_on_read() {
    let api =
        fs::read_to_string("src/api/learning_math.rs").expect("learning math API should exist");

    let list_start = api
        .find("pub async fn list(")
        .expect("list handler should exist");
    let get_start = api[list_start..]
        .find("pub async fn get(")
        .map(|offset| list_start + offset)
        .expect("get handler should follow list handler");
    let list_body = &api[list_start..get_start];

    assert!(list_body.contains("find_learning_math_namespace"));
    assert!(!list_body.contains("ensure_learning_math_namespace"));
    assert!(api.contains("items: vec![]"));
}

#[test]
fn learning_math_practice_api_uses_parent_child_friendly_language() {
    let api =
        fs::read_to_string("src/api/learning_math.rs").expect("learning math API should exist");

    for term in [
        "practice_goal",
        "exercise",
        "answer",
        "mistake_pattern",
        "feedback",
        "practice_adjustment",
        "next_exercise",
    ] {
        assert!(api.contains(term), "missing product term: {term}");
    }

    assert!(!api.contains("MemoryAtom"));
    assert!(!api.contains("CognitiveScene"));
    assert!(!api.contains("CognitiveProjection"));
}
