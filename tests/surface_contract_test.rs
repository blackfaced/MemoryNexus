use memorynexus::domain::surface::{
    RuntimePreference, Surface, SurfaceAction, SurfaceAdapter, SurfaceContext, SurfaceRequest,
    SurfaceResponse, SurfaceVisibility,
};
use serde_json::{json, Value};
use std::fs;
use uuid::Uuid;

fn sample_request(surface: Surface, action: SurfaceAction) -> SurfaceRequest {
    SurfaceRequest {
        namespace: "child.english.spelling".to_string(),
        surface,
        action,
        actor: Uuid::new_v4(),
        adapter: SurfaceAdapter::Mcp,
        payload: json!({
            "word": "because",
            "attempt": "becuase"
        }),
        context: SurfaceContext {
            mode: Some("fast".to_string()),
            locale: Some("en-US".to_string()),
            device: Some("desktop".to_string()),
            runtime_preference: Some(RuntimePreference::Deterministic),
        },
    }
}

#[test]
fn surface_request_serializes_and_deserializes_gateway_contract() {
    let request = sample_request(Surface::Performance, SurfaceAction::SubmitAttempt);

    let serialized = serde_json::to_value(&request).unwrap();

    assert_eq!(serialized["namespace"], "child.english.spelling");
    assert_eq!(serialized["surface"], "performance");
    assert_eq!(serialized["action"], "submit_attempt");
    assert_eq!(serialized["adapter"], "mcp");
    assert_eq!(serialized["context"]["runtime_preference"], "deterministic");
    assert_eq!(serialized["payload"]["attempt"], "becuase");

    let round_tripped: SurfaceRequest = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped, request);
    round_tripped.validate().unwrap();
}

#[test]
fn surface_response_serializes_shaped_result_without_raw_engine_object_by_default() {
    let trace_id = Uuid::new_v4();
    let response = SurfaceResponse::new(
        Surface::Planning,
        SurfaceAction::GenerateNextTask,
        json!({
            "title": "Tomorrow spelling practice",
            "items": ["because", "friend"]
        }),
        trace_id,
        vec!["review yesterday's double-letter mistakes".to_string()],
        SurfaceVisibility::User,
    );

    let serialized = serde_json::to_value(&response).unwrap();

    assert_eq!(serialized["surface"], "planning");
    assert_eq!(serialized["action"], "generate_next_task");
    assert_eq!(serialized["generated_trace_id"], trace_id.to_string());
    assert_eq!(
        serialized["follow_up_suggestions"],
        json!(["review yesterday's double-letter mistakes"])
    );
    assert_eq!(serialized["visibility"], "user");
    assert_eq!(serialized["result"]["title"], "Tomorrow spelling practice");
    assert_eq!(serialized.get("engine_objects"), None);

    let round_tripped: SurfaceResponse = serde_json::from_value(serialized).unwrap();
    assert_eq!(round_tripped.surface, Surface::Planning);
    assert_eq!(round_tripped.generated_trace_id, trace_id);
}

#[test]
fn invalid_surface_action_combinations_are_rejected() {
    let invalid = [
        (Surface::Capture, SurfaceAction::SubmitAttempt),
        (Surface::Performance, SurfaceAction::CaptureObservation),
        (Surface::Reflection, SurfaceAction::GenerateNextTask),
        (Surface::Planning, SurfaceAction::GetStateSummary),
        (Surface::Observation, SurfaceAction::AdjustPlan),
        (Surface::Observation, SurfaceAction::ReviewEvidence),
    ];

    for (surface, action) in invalid {
        let request = sample_request(surface, action);
        let err = request.validate().unwrap_err();
        assert_eq!(err.surface, surface);
        assert_eq!(err.action, action);
    }
}

#[test]
fn valid_surface_action_combinations_are_accepted() {
    for (surface, action) in [
        (Surface::Capture, SurfaceAction::CaptureObservation),
        (Surface::Performance, SurfaceAction::SubmitAttempt),
        (Surface::Reflection, SurfaceAction::ReviewEvidence),
        (Surface::Planning, SurfaceAction::GenerateNextTask),
        (Surface::Planning, SurfaceAction::AdjustPlan),
        (Surface::Observation, SurfaceAction::GetStateSummary),
        (Surface::Observation, SurfaceAction::RequestConsolidation),
    ] {
        sample_request(surface, action).validate().unwrap();
    }
}

#[test]
fn manual_consolidation_resolver_uses_only_active_namespaces() {
    let source =
        fs::read_to_string("src/api/surfaces.rs").expect("surface api source should be readable");

    assert!(
        source.contains("namespace.status == \"active\""),
        "manual consolidation must reject archived namespaces"
    );
}

#[test]
fn surface_gateway_route_is_registered_once() {
    let source = fs::read_to_string("src/api/mod.rs").expect("api source should be readable");

    assert_eq!(
        source.matches("\"/api/v1/surfaces\"").count(),
        1,
        "duplicate Surface Gateway route registrations can panic at startup"
    );
}

#[test]
fn surface_result_stays_adapter_shaped_json() {
    let response = SurfaceResponse::new(
        Surface::Reflection,
        SurfaceAction::ReviewEvidence,
        json!({
            "summary": "The same spelling pattern appeared twice.",
            "confidence": "medium"
        }),
        Uuid::new_v4(),
        Vec::new(),
        SurfaceVisibility::Coach,
    );

    let result: Value = response.result;
    assert_eq!(
        result["summary"],
        "The same spelling pattern appeared twice."
    );
    assert_eq!(result.get("memory_atoms"), None);
    assert_eq!(result.get("cognitive_scenes"), None);
    assert_eq!(result.get("growth_model"), None);
}
