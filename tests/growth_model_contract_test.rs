use chrono::{TimeZone, Utc};
use memorynexus::domain::growth_model::{
    EvidenceBackedClaim, EvidenceBackedFocus, EvidenceBackedPattern, EvidenceId, GrowthModel,
    GrowthStage,
};
use uuid::Uuid;

#[test]
fn growth_model_json_round_trip_preserves_space_namespace_and_evidence() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let trace_id = EvidenceId::Trace(Uuid::new_v4());
    let feedback_loop_id = EvidenceId::FeedbackLoop(Uuid::new_v4());
    let memory_id = EvidenceId::Memory(Uuid::new_v4());

    let model = GrowthModel {
        id: Uuid::new_v4(),
        space_id,
        namespace_id,
        strengths: vec![EvidenceBackedClaim {
            label: "Self-corrects after seeing a worked example".to_string(),
            evidence_ids: vec![trace_id],
        }],
        weaknesses: vec![EvidenceBackedClaim {
            label: "Loses accuracy when sentence length increases".to_string(),
            evidence_ids: vec![feedback_loop_id],
        }],
        recurring_patterns: vec![EvidenceBackedPattern {
            pattern: "Double-letter spelling errors recur across weekly review".to_string(),
            evidence_ids: vec![trace_id, feedback_loop_id],
        }],
        current_stage: GrowthStage {
            label: "Practicing stable recall".to_string(),
            evidence_ids: vec![memory_id],
        },
        recommended_focus: EvidenceBackedFocus {
            focus: "Short daily review of double-letter words".to_string(),
            rationale: "Recent attempts show repeated double-letter mistakes".to_string(),
            evidence_ids: vec![trace_id, feedback_loop_id, memory_id],
        },
        updated_at: Utc.with_ymd_and_hms(2026, 6, 16, 8, 30, 0).unwrap(),
    };

    let json = serde_json::to_string(&model).expect("GrowthModel should serialize");
    let parsed: GrowthModel =
        serde_json::from_str(&json).expect("GrowthModel should deserialize from its own JSON");

    assert_eq!(parsed, model);
    assert_eq!(parsed.space_id, space_id);
    assert_eq!(parsed.namespace_id, namespace_id);
    assert_eq!(parsed.strengths[0].evidence_ids, vec![trace_id]);
    assert_eq!(
        parsed.recommended_focus.evidence_ids,
        vec![trace_id, feedback_loop_id, memory_id]
    );
}

#[test]
fn growth_model_evidence_ids_are_explicitly_typed_in_json() {
    let evidence_id = Uuid::new_v4();
    let claim = EvidenceBackedClaim {
        label: "Keeps practice attempts short and frequent".to_string(),
        evidence_ids: vec![EvidenceId::Trace(evidence_id)],
    };

    let value = serde_json::to_value(claim).expect("claim should serialize");

    assert_eq!(value["evidence_ids"][0]["kind"], "trace");
    assert_eq!(value["evidence_ids"][0]["id"], evidence_id.to_string());
}
