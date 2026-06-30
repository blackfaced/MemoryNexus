use memorynexus::domain::dictation_agent_demo::{
    build_minimal_dictation_agent_demo, map_media_prompt_confirmation, MediaPromptDecision,
};
use serde_json::{json, Value};

#[test]
fn minimal_dictation_agent_demo_maps_product_actions_to_all_generic_surface_tools() {
    let fixture: Value = serde_json::from_str(include_str!(
        "fixtures/dictation_agent/minimal_english_spelling_demo.json"
    ))
    .expect("fixture should be valid JSON");

    let demo = build_minimal_dictation_agent_demo(&fixture).expect("demo should build");

    let expected = [
        (
            "surface_capture_observation",
            "record today's list",
            "capture",
            "capture_observation",
            "fast",
        ),
        (
            "surface_submit_attempt",
            "submit a dictation result",
            "performance",
            "submit_attempt",
            "fast",
        ),
        (
            "surface_review_evidence",
            "explain mistake patterns",
            "reflection",
            "review_evidence",
            "focused",
        ),
        (
            "surface_generate_next_task",
            "prepare tomorrow's focused practice",
            "planning",
            "generate_next_task",
            "focused",
        ),
        (
            "surface_get_state_summary",
            "show a recent trend",
            "observation",
            "get_state_summary",
            "focused",
        ),
    ];

    assert_eq!(demo.tool_calls.len(), expected.len());

    for (call, (tool, product_action, surface, action, mode)) in
        demo.tool_calls.iter().zip(expected)
    {
        assert_eq!(call.tool_name, tool);
        assert_eq!(call.product_action, product_action);
        assert_eq!(call.surface, surface);
        assert_eq!(call.action, action);
        assert_eq!(call.arguments["namespace"], "child.english.spelling");
        assert_eq!(
            call.arguments["actor"],
            "00000000-0000-0000-0000-000000000001"
        );
        assert_eq!(
            call.arguments["payload"]["space_id"],
            "22222222-2222-2222-2222-222222222222"
        );
        assert!(matches!(
            call.arguments["payload"]["source"].as_str(),
            Some("typed" | "pasted") | None
        ));
        assert_eq!(call.arguments["context"]["mode"], mode);
        assert_eq!(
            call.arguments["context"]["runtime_preference"],
            "deterministic"
        );
        assert_no_media_only_fields(&call.arguments["payload"]);
    }

    assert_eq!(
        demo.generated_trace_ids,
        vec![
            "trace-capture-0001",
            "trace-performance-0002",
            "trace-reflection-0003",
            "trace-planning-0004",
            "trace-observation-0005",
        ]
    );
}

#[test]
fn dictation_media_prompt_policy_maps_acceptance_and_correction_without_persisting_descriptors() {
    let accepted = map_media_prompt_confirmation(
        "agent_ocr",
        "because\nfriend",
        MediaPromptDecision::Accepted,
    )
    .expect("accepted prompt should map");

    assert_eq!(accepted["source"], "agent_ocr");
    assert_eq!(accepted["confirmed_text"], "because\nfriend");
    assert_eq!(
        accepted["input_confirmation"],
        json!({"status": "confirmed", "method": "explicit_acceptance"})
    );
    assert!(accepted.get("evidence_refs").is_none());

    let corrected = map_media_prompt_confirmation(
        "agent_ocr",
        "becaus\nfriend",
        MediaPromptDecision::Corrected("because\nfriend".to_string()),
    )
    .expect("corrected prompt should map");

    assert_eq!(corrected["source"], "agent_ocr");
    assert_eq!(corrected["confirmed_text"], "because\nfriend");
    assert_eq!(
        corrected["input_confirmation"],
        json!({"status": "confirmed", "method": "explicit_correction"})
    );
    assert!(corrected.get("evidence_refs").is_none());
    assert!(corrected.get("locator").is_none());
    assert!(corrected.get("metadata").is_none());
}

fn assert_no_media_only_fields(value: &Value) {
    match value {
        Value::Object(object) => {
            for field in [
                "evidence_refs",
                "input_confirmation",
                "provider",
                "locator",
                "media_type",
                "content_hash",
                "original_name",
                "captured_at",
                "transcript",
                "transcript_source",
                "media_descriptor",
                "media_provenance",
            ] {
                assert!(
                    !object.contains_key(field),
                    "text-first payload must not include {field}: {value}"
                );
            }
            for nested in object.values() {
                assert_no_media_only_fields(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_media_only_fields(item);
            }
        }
        _ => {}
    }
}
