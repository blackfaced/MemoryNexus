use chrono::{TimeZone, Utc};
use memorynexus::domain::dream_candidate::DreamCandidateEffectiveness;
use memorynexus::domain::growth_model::{aggregate_growth_model, EvidenceId, GrowthEvidenceRecord};
use memorynexus::domain::practice_plan::{
    PracticePlanGeneration, PracticePlanSource, PracticePlanStatus,
};
use uuid::Uuid;

#[test]
fn recurring_dictation_pattern_generates_evidence_linked_ten_minute_plan() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let first_trace = EvidenceId::Trace(Uuid::new_v4());
    let second_feedback_loop = EvidenceId::FeedbackLoop(Uuid::new_v4());
    let generation_trace_id = Uuid::new_v4();

    let growth = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: first_trace,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: Some("because -> becaus".to_string()),
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: second_feedback_loop,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: Some("friend -> frend".to_string()),
            },
        ],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    let generated = PracticePlanGeneration::from_growth_model(&growth.model, generation_trace_id);
    let plan = generated
        .into_plan()
        .expect("recurring evidence should produce a plan");

    assert_eq!(plan.space_id, space_id);
    assert_eq!(plan.namespace_id, Some(namespace_id));
    assert_eq!(
        plan.source,
        PracticePlanSource::GrowthModel(growth.model.id)
    );
    assert_eq!(plan.status, PracticePlanStatus::Selected);
    assert_eq!(
        plan.target_pattern.as_deref(),
        Some("repeated dictation mistake type: missing_letter")
    );
    assert!(plan.content.contains("10 minutes"));
    assert!(plan.content.contains("missed letter"));
    assert!(!plan.content.contains("EvidenceBacked"));
    assert!(!plan.content.contains("GrowthModel"));
    assert_eq!(
        plan.expected_effect.as_deref(),
        Some("Reduce missing-letter spelling mistakes in the next dictation attempt.")
    );
    assert_eq!(
        plan.evidence_ids,
        growth.model.recommended_focus.evidence_ids
    );
    assert_eq!(plan.evidence_ids, vec![first_trace, second_feedback_loop]);
    assert_eq!(plan.generation_trace_id, Some(generation_trace_id));
    assert_eq!(plan.target_growth_model_id, Some(growth.model.id));
}

#[test]
fn practice_plan_generation_json_round_trip_preserves_plan_status() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let first_trace = EvidenceId::Trace(Uuid::new_v4());
    let second_feedback_loop = EvidenceId::FeedbackLoop(Uuid::new_v4());
    let generation_trace_id = Uuid::new_v4();
    let growth = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: first_trace,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: None,
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: second_feedback_loop,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: None,
            },
        ],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );
    let generated = PracticePlanGeneration::from_growth_model(&growth.model, generation_trace_id);

    let value = serde_json::to_value(&generated).expect("PracticePlanGeneration should serialize");
    assert_eq!(value["result"], "plan");
    assert_eq!(value["status"], "selected");

    let round_trip: PracticePlanGeneration =
        serde_json::from_value(value).expect("PracticePlanGeneration should deserialize");

    assert_eq!(round_trip, generated);
}

#[test]
fn sparse_growth_model_returns_evidence_gap_instead_of_targeted_plan() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let generation_trace_id = Uuid::new_v4();
    let growth = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![GrowthEvidenceRecord {
            space_id,
            namespace_id,
            evidence_id: EvidenceId::Trace(Uuid::new_v4()),
            signal_labels: vec!["missing_letter".to_string()],
            explanation: None,
        }],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    let generated = PracticePlanGeneration::from_growth_model(&growth.model, generation_trace_id);

    match generated {
        PracticePlanGeneration::EvidenceGap(gap) => {
            assert_eq!(gap.space_id, space_id);
            assert_eq!(gap.namespace_id, namespace_id);
            assert_eq!(gap.growth_model_id, growth.model.id);
            assert_eq!(gap.generation_trace_id, generation_trace_id);
            assert_eq!(gap.evidence_ids, Vec::<EvidenceId>::new());
            assert_eq!(
                gap.reason,
                "needs more confirmed attempts before targeting a recurring pattern"
            );
        }
        PracticePlanGeneration::Plan(plan) => {
            panic!("sparse evidence must not produce targeted plan: {plan:?}");
        }
    }
}

#[test]
fn generated_plan_keeps_later_execution_and_evaluation_trace_lifecycle() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let evidence_trace = EvidenceId::Trace(Uuid::new_v4());
    let evidence_loop = EvidenceId::FeedbackLoop(Uuid::new_v4());
    let generation_trace_id = Uuid::new_v4();
    let execution_trace_id = Uuid::new_v4();
    let evaluation_trace_id = Uuid::new_v4();

    let growth = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: evidence_trace,
                signal_labels: vec!["double_letter_error".to_string()],
                explanation: None,
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: evidence_loop,
                signal_labels: vec!["double_letter_error".to_string()],
                explanation: None,
            },
        ],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    let mut plan = PracticePlanGeneration::from_growth_model(&growth.model, generation_trace_id)
        .into_plan()
        .expect("recurring evidence should produce a plan");

    plan.record_execution(execution_trace_id);
    plan.record_evaluation(
        evaluation_trace_id,
        DreamCandidateEffectiveness::Useful,
        "The next attempt had fewer double-letter mistakes.",
    );

    assert_eq!(plan.generation_trace_id, Some(generation_trace_id));
    assert_eq!(plan.executed_trace_ids, vec![execution_trace_id]);
    assert_eq!(plan.evaluation_trace_ids, vec![evaluation_trace_id]);
    assert_eq!(plan.status, PracticePlanStatus::Evaluated);
}
