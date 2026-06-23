use memorynexus::domain::practice_plan::{build_next_task_plan, PlanningRequest};
use uuid::Uuid;

#[test]
fn deterministic_next_task_plan_is_adapter_shaped_and_response_only() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();

    let plan = build_next_task_plan(&PlanningRequest {
        space_id,
        namespace_id,
        namespace: "child.english.spelling".to_string(),
        objective: Some("Review the because spelling pattern".to_string()),
    });
    let serialized = serde_json::to_value(&plan).unwrap();

    assert_eq!(serialized["status"], "next_task_ready");
    assert_eq!(serialized["space_id"], space_id.to_string());
    assert_eq!(serialized["namespace_id"], namespace_id.to_string());
    assert_eq!(serialized["namespace"], "child.english.spelling");
    assert_eq!(serialized["plan_kind"], "response_only_draft");
    assert_eq!(serialized["persistence"], "not_persisted");
    assert_eq!(
        serialized["next_task"]["title"],
        "Next task for child.english.spelling"
    );
    assert_eq!(
        serialized["next_task"]["prompt"],
        "Review the because spelling pattern"
    );
    assert_eq!(serialized["next_task"]["runtime"], "deterministic");
    assert_eq!(serialized.get("practice_plan_id"), None);
    assert_eq!(serialized.get("growth_model"), None);
    assert_eq!(serialized.get("engine_objects"), None);
}

#[test]
fn deterministic_next_task_plan_uses_generic_fallback_without_product_roles() {
    let plan = build_next_task_plan(&PlanningRequest {
        space_id: Uuid::new_v4(),
        namespace_id: Uuid::new_v4(),
        namespace: "personal.thoughts".to_string(),
        objective: None,
    });

    assert_eq!(
        plan.next_task.prompt,
        "Continue focused work in personal.thoughts."
    );
    assert!(!plan.next_task.prompt.contains("parent"));
    assert!(!plan.next_task.prompt.contains("child"));
}
