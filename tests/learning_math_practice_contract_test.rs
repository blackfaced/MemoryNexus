use std::fs;

#[test]
fn learning_math_practice_routes_expose_session_api() {
    let routes = fs::read_to_string("src/api/mod.rs").expect("api routes should be readable");

    assert!(routes.contains("mod learning_math;"));
    assert!(routes.contains("/api/v1/namespaces/:namespace_id/practice-sessions"));
    assert!(routes.contains("learning_math::create_in_namespace"));
    assert!(routes.contains("learning_math::list_in_namespace"));
    assert!(routes.contains("/api/v1/namespaces/:namespace_id/practice-sessions/:id"));
    assert!(routes.contains("learning_math::get_in_namespace"));
    assert!(routes.contains("/api/v1/namespaces/:namespace_id/practice-sessions/:id/attempt"));
    assert!(routes.contains("learning_math::patch_attempt_in_namespace"));
    assert!(routes.contains("/api/v1/namespaces/:namespace_id/practice-sessions/:id/feedback"));
    assert!(routes.contains("learning_math::patch_feedback_in_namespace"));
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
fn namespace_practice_api_accepts_skill_namespaces_without_learning_math_route_coupling() {
    let api =
        fs::read_to_string("src/api/learning_math.rs").expect("learning math API should exist");

    assert!(api.contains("require_practice_namespace"));
    assert!(api.contains("NamespaceKind::Skill.as_str()"));
    assert!(api.contains("create_in_namespace"));
    assert!(api.contains("list_in_namespace"));
    assert!(api.contains("find_practice_session_in_namespace"));
}

#[test]
fn learning_math_practice_api_keeps_space_and_namespace_boundary_before_writes() {
    let api =
        fs::read_to_string("src/api/learning_math.rs").expect("learning math API should exist");

    let writer_check = api
        .find("require_space_writer(&state, space_id, auth_user.user_id)")
        .expect("create should require writable Cognitive Space");
    let namespace_ensure = api
        .find(
            "ensure_learning_math_namespace(&state, space_id, req.namespace_id, auth_user.user_id)",
        )
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
fn namespace_practice_api_keeps_path_namespace_and_feedback_loop_in_same_space() {
    let api =
        fs::read_to_string("src/api/learning_math.rs").expect("learning math API should exist");

    let namespace_check = api
        .find("require_practice_namespace(&state, namespace_id, auth_user.user_id)")
        .expect("canonical create should require readable skill Namespace");
    let writer_check = api
        .find("require_space_writer(&state, namespace.space_id, auth_user.user_id)")
        .expect("canonical create should require writable namespace Space");
    let create_loop = api[namespace_check..]
        .find("create_practice_session(")
        .map(|offset| namespace_check + offset)
        .expect("canonical create should write session in namespace Space");
    let create_call = &api[create_loop..];

    assert!(namespace_check < create_loop);
    assert!(writer_check < create_loop);
    assert!(create_call.contains("namespace.space_id"));
    assert!(create_call.contains("namespace.id"));

    assert!(api.contains("feedback_loop.namespace_id != namespace.id"));
    assert!(api
        .contains("require_namespace_in_space(state, namespace_id, namespace.space_id, user_id)"));
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
