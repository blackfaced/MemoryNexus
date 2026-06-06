use std::fs;

#[test]
fn weekly_learning_review_uses_namespace_driven_api_surface() {
    let routes = fs::read_to_string("src/api/mod.rs").expect("api routes should be readable");

    assert!(routes.contains("/api/v1/namespaces/:namespace_id/learning-reviews"));
    assert!(routes.contains("review_reports::create_learning_review"));
    assert!(!routes.contains("/api/v1/learning/math/learning-reviews"));
}

#[test]
fn weekly_learning_review_reuses_review_report_and_feedback_loop_models() {
    let api =
        fs::read_to_string("src/api/review_reports.rs").expect("review report API should exist");

    assert!(api.contains("CreateLearningReviewRequest"));
    assert!(api.contains("FeedbackLoopWindowFilter"));
    assert!(api.contains("\"weekly_learning_review\""));
    assert!(api.contains("source_feedback_loop_ids"));
    assert!(api.contains("source_memory_ids"));
    assert!(api.contains("learning_source_memories"));
}

#[test]
fn weekly_learning_review_keeps_space_namespace_and_lens_boundaries() {
    let api =
        fs::read_to_string("src/api/review_reports.rs").expect("review report API should exist");

    assert!(api.contains(".namespaces"));
    assert!(api.contains(".find_for_user(namespace_id, auth_user.user_id)"));
    assert!(api.contains("namespace.kind != \"skill\""));
    assert!(api.contains("lens.space_id != namespace.space_id"));
    assert!(api.contains("space_id: namespace.space_id"));
    assert!(api.contains("namespace_id,"));
}
