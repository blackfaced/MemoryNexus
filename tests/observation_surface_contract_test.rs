use std::fs;

use memorynexus::domain::surface::{Surface, SurfaceAction, SurfaceResponse, SurfaceVisibility};
use serde_json::{json, Value};
use uuid::Uuid;

#[test]
fn observation_trace_task_type_is_added_by_forward_migration() {
    let base_sql =
        fs::read_to_string("migrations/014_traces.sql").expect("trace migration should exist");
    let observation_sql = fs::read_to_string("migrations/018_trace_observation_task_type.sql")
        .expect("observation trace migration should exist");

    assert!(
        !base_sql.contains("'observation'"),
        "014 is already merged and must not be edited for new trace task types"
    );
    assert!(observation_sql.contains("DROP CONSTRAINT IF EXISTS traces_task_type_check"));
    assert!(observation_sql.contains("ADD CONSTRAINT traces_task_type_check"));
    assert!(observation_sql.contains("'observation'"));
}

#[test]
fn observation_state_summary_response_is_adapter_shaped() {
    let trace_id = Uuid::new_v4();
    let response = SurfaceResponse::new(
        Surface::Observation,
        SurfaceAction::GetStateSummary,
        json!({
            "status": "state_summary_ready",
            "space_id": Uuid::new_v4(),
            "namespace_id": Uuid::new_v4(),
            "namespace": "child.english.spelling",
            "summary": "Observed namespace state from deterministic local counts.",
            "counts": {
                "memories": 2,
                "traces": 3,
                "feedback_loops": {
                    "active": 1,
                    "completed": 1,
                    "paused": 0,
                    "total": 2
                }
            },
            "trends": {
                "recent_trace_count": 3,
                "latest_trace_task_type": "practice"
            },
            "growth_model": {
                "status": "not_persisted",
                "growth_model_id": Value::Null
            },
            "dictation_observation": {
                "status": "ready",
                "timeframe": "7d",
                "evidence_record_count": 2,
                "recurring_mistake_types": ["missing_letter"],
                "stability_signal": "needs_focus",
                "current_focus": "Review missing_letter with short targeted practice",
                "supporting_evidence_ids": [
                    {"kind": "trace", "id": Uuid::new_v4()},
                    {"kind": "feedback_loop", "id": Uuid::new_v4()}
                ],
                "evidence_gaps": [],
                "growth_model_id": Uuid::new_v4(),
                "growth_model_status": "derived_from_growth_evidence"
            }
        }),
        trace_id,
        Vec::new(),
        SurfaceVisibility::User,
    );

    let result = response.result;
    assert_eq!(result["status"], "state_summary_ready");
    assert_eq!(result["growth_model"]["status"], "not_persisted");
    assert_eq!(result["growth_model"]["growth_model_id"], Value::Null);
    assert_eq!(result["dictation_observation"]["timeframe"], "7d");
    assert_eq!(
        result["dictation_observation"]["recurring_mistake_types"][0],
        "missing_letter"
    );
    assert_eq!(
        result["dictation_observation"]["supporting_evidence_ids"][0]["kind"],
        "trace"
    );
    assert_eq!(result["dictation_observation"].get("evidence_refs"), None);
    assert_eq!(result.get("raw_rows"), None);
    assert_eq!(result.get("memory_atoms"), None);
    assert_eq!(result.get("cognitive_scenes"), None);
}
