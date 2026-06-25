use chrono::{TimeZone, Utc};
use memorynexus::domain::growth_model::{
    aggregate_growth_model, EvidenceBackedClaim, EvidenceBackedFocus, EvidenceBackedPattern,
    EvidenceId, GrowthEvidenceRecord, GrowthModel, GrowthStage,
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

#[test]
fn repeated_dictation_mistake_evidence_yields_recurring_growth_pattern() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let first_trace = EvidenceId::Trace(Uuid::new_v4());
    let second_feedback_loop = EvidenceId::FeedbackLoop(Uuid::new_v4());

    let result = aggregate_growth_model(
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

    assert_eq!(result.evidence_gaps, Vec::<String>::new());
    assert_eq!(result.model.space_id, space_id);
    assert_eq!(result.model.namespace_id, namespace_id);
    assert_eq!(result.model.recurring_patterns.len(), 1);
    assert_eq!(
        result.model.recurring_patterns[0].pattern,
        "repeated dictation mistake type: missing_letter"
    );
    assert_eq!(
        result.model.recurring_patterns[0].evidence_ids,
        vec![first_trace, second_feedback_loop]
    );
    assert_eq!(
        result.model.current_stage.label,
        "recurring pattern detected"
    );
    assert_eq!(
        result.model.recommended_focus.focus,
        "Review missing_letter with short targeted practice"
    );
}

#[test]
fn sparse_growth_evidence_records_explicit_gap_instead_of_pattern() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();

    let result = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![GrowthEvidenceRecord {
            space_id,
            namespace_id,
            evidence_id: EvidenceId::Trace(Uuid::new_v4()),
            signal_labels: vec!["letter_order_error".to_string()],
            explanation: None,
        }],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    assert!(result.model.recurring_patterns.is_empty());
    assert_eq!(
        result.evidence_gaps,
        vec!["insufficient compatible evidence for recurring pattern".to_string()]
    );
    assert_eq!(result.model.current_stage.label, "needs more evidence");
    assert_eq!(
        result.model.recommended_focus.focus,
        "Collect more confirmed attempts"
    );
}

#[test]
fn repeated_unclassified_evidence_records_gap_instead_of_recurring_pattern() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();

    let result = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: EvidenceId::Trace(Uuid::new_v4()),
                signal_labels: vec!["unclassified".to_string()],
                explanation: Some(
                    "deterministic text evidence is insufficient to classify".to_string(),
                ),
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: EvidenceId::FeedbackLoop(Uuid::new_v4()),
                signal_labels: vec!["unclassified".to_string()],
                explanation: Some(
                    "deterministic text evidence is insufficient to classify".to_string(),
                ),
            },
        ],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    assert!(result.model.recurring_patterns.is_empty());
    assert_eq!(
        result.evidence_gaps,
        vec![
            "ignored evidence without deterministic signal labels".to_string(),
            "insufficient compatible evidence for recurring pattern".to_string()
        ]
    );
    assert_eq!(
        result.model.recommended_focus.focus,
        "Collect more confirmed attempts"
    );
}

#[test]
fn absent_growth_evidence_records_explicit_gap_instead_of_pattern() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();

    let result = aggregate_growth_model(
        space_id,
        namespace_id,
        Vec::new(),
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    assert!(result.model.recurring_patterns.is_empty());
    assert_eq!(
        result.evidence_gaps,
        vec!["no evidence records provided".to_string()]
    );
    assert_eq!(result.model.current_stage.label, "needs more evidence");
}

#[test]
fn recommended_focus_evidence_ids_match_the_focused_pattern() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let missing_letter_trace = EvidenceId::Trace(Uuid::new_v4());
    let missing_letter_loop = EvidenceId::FeedbackLoop(Uuid::new_v4());
    let spacing_trace = EvidenceId::Trace(Uuid::new_v4());
    let spacing_loop = EvidenceId::FeedbackLoop(Uuid::new_v4());

    let result = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: spacing_trace,
                signal_labels: vec!["spacing_error".to_string()],
                explanation: None,
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: missing_letter_trace,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: None,
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: spacing_loop,
                signal_labels: vec!["spacing_error".to_string()],
                explanation: None,
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: missing_letter_loop,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: None,
            },
        ],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    assert_eq!(
        result.model.recommended_focus.focus,
        "Review missing_letter with short targeted practice"
    );
    assert_eq!(
        result.model.recommended_focus.evidence_ids,
        vec![missing_letter_trace, missing_letter_loop]
    );
}

#[test]
fn cross_namespace_growth_evidence_is_not_aggregated_into_target_namespace() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let other_namespace_id = Uuid::new_v4();
    let target_trace = EvidenceId::Trace(Uuid::new_v4());
    let other_trace = EvidenceId::Trace(Uuid::new_v4());

    let result = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: target_trace,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: None,
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id: other_namespace_id,
                evidence_id: other_trace,
                signal_labels: vec!["missing_letter".to_string()],
                explanation: None,
            },
        ],
        Utc.with_ymd_and_hms(2026, 6, 24, 21, 0, 0).unwrap(),
    );

    assert!(result.model.recurring_patterns.is_empty());
    assert_eq!(
        result.evidence_gaps,
        vec![
            "ignored evidence outside target namespace".to_string(),
            "insufficient compatible evidence for recurring pattern".to_string()
        ]
    );
    assert!(!result
        .model
        .recommended_focus
        .evidence_ids
        .contains(&other_trace));
}
