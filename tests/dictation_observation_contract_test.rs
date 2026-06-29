use chrono::{Duration, TimeZone, Utc};
use memorynexus::domain::dictation_observation::{
    build_dictation_observation_summary, DictationObservationEvidenceRecord,
    DictationObservationStatus, DictationStabilitySignal,
};
use memorynexus::domain::growth_model::{EvidenceId, GrowthEvidenceRecord};
use serde_json::Value;
use uuid::Uuid;

#[test]
fn seven_day_dictation_history_returns_recurring_errors_focus_and_evidence_ids() {
    let now = Utc.with_ymd_and_hms(2026, 6, 29, 20, 0, 0).unwrap();
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let missing_letter_trace = EvidenceId::Trace(Uuid::new_v4());
    let missing_letter_loop = EvidenceId::FeedbackLoop(Uuid::new_v4());
    let spacing_trace = EvidenceId::Trace(Uuid::new_v4());
    let old_trace = EvidenceId::Trace(Uuid::new_v4());

    let summary = build_dictation_observation_summary(
        space_id,
        namespace_id,
        now,
        vec![
            observed_record(
                space_id,
                namespace_id,
                missing_letter_trace,
                "missing_letter",
                now - Duration::days(6),
            ),
            observed_record(
                space_id,
                namespace_id,
                missing_letter_loop,
                "missing_letter",
                now - Duration::days(1),
            ),
            observed_record(
                space_id,
                namespace_id,
                spacing_trace,
                "spacing_error",
                now - Duration::days(2),
            ),
            observed_record(
                space_id,
                namespace_id,
                old_trace,
                "wrong_character",
                now - Duration::days(8),
            ),
        ],
    );

    assert_eq!(summary.status, DictationObservationStatus::Ready);
    assert_eq!(summary.timeframe, "7d");
    assert_eq!(summary.evidence_record_count, 3);
    assert_eq!(summary.recurring_mistake_types, vec!["missing_letter"]);
    assert_eq!(
        summary.stability_signal,
        DictationStabilitySignal::NeedsFocus
    );
    assert_eq!(
        summary.current_focus,
        "Review missing_letter with short targeted practice"
    );
    assert_eq!(
        summary.supporting_evidence_ids,
        vec![missing_letter_trace, missing_letter_loop]
    );
    assert!(!summary.supporting_evidence_ids.contains(&old_trace));
    assert_eq!(summary.growth_model_id, namespace_id);
    assert_eq!(summary.growth_model_status, "derived_from_growth_evidence");
}

#[test]
fn sparse_history_returns_needs_more_evidence_without_fabricating_trend() {
    let now = Utc.with_ymd_and_hms(2026, 6, 29, 20, 0, 0).unwrap();
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let trace_id = EvidenceId::Trace(Uuid::new_v4());

    let summary = build_dictation_observation_summary(
        space_id,
        namespace_id,
        now,
        vec![observed_record(
            space_id,
            namespace_id,
            trace_id,
            "missing_letter",
            now - Duration::days(1),
        )],
    );

    assert_eq!(summary.status, DictationObservationStatus::Ready);
    assert_eq!(summary.timeframe, "7d");
    assert_eq!(summary.evidence_record_count, 1);
    assert!(summary.recurring_mistake_types.is_empty());
    assert_eq!(
        summary.stability_signal,
        DictationStabilitySignal::NeedsMoreEvidence
    );
    assert_eq!(summary.current_focus, "Collect more confirmed attempts");
    assert!(summary.supporting_evidence_ids.is_empty());
    assert!(summary
        .evidence_gaps
        .contains(&"insufficient compatible evidence for recurring pattern".to_string()));
}

#[test]
fn empty_history_returns_empty_state() {
    let now = Utc.with_ymd_and_hms(2026, 6, 29, 20, 0, 0).unwrap();
    let summary =
        build_dictation_observation_summary(Uuid::new_v4(), Uuid::new_v4(), now, Vec::new());

    assert_eq!(summary.status, DictationObservationStatus::Empty);
    assert_eq!(summary.timeframe, "7d");
    assert_eq!(summary.evidence_record_count, 0);
    assert!(summary.recurring_mistake_types.is_empty());
    assert_eq!(
        summary.stability_signal,
        DictationStabilitySignal::NeedsMoreEvidence
    );
    assert_eq!(summary.current_focus, "No recent dictation history yet");
    assert!(summary.supporting_evidence_ids.is_empty());
}

#[test]
fn observation_summary_serialization_excludes_media_descriptor_fields() {
    let now = Utc.with_ymd_and_hms(2026, 6, 29, 20, 0, 0).unwrap();
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let first_trace = EvidenceId::Trace(Uuid::new_v4());
    let second_trace = EvidenceId::Trace(Uuid::new_v4());

    let summary = build_dictation_observation_summary(
        space_id,
        namespace_id,
        now,
        vec![
            observed_record(
                space_id,
                namespace_id,
                first_trace,
                "missing_letter",
                now - Duration::days(3),
            ),
            observed_record(
                space_id,
                namespace_id,
                second_trace,
                "missing_letter",
                now - Duration::days(1),
            ),
        ],
    );

    let value = serde_json::to_value(summary).expect("summary should serialize");
    assert_eq!(
        value.pointer("/supporting_evidence_ids/0/kind"),
        Some(&Value::String("trace".to_string()))
    );
    assert_descriptor_field_absent(&value, "evidence_refs");
    assert_descriptor_field_absent(&value, "input_confirmation");
    assert_descriptor_field_absent(&value, "input_source");
    assert_descriptor_field_absent(&value, "locator");
    assert_descriptor_field_absent(&value, "metadata");
    assert_descriptor_field_absent(&value, "transcript");
    assert_descriptor_field_absent(&value, "provider");
}

fn observed_record(
    space_id: Uuid,
    namespace_id: Uuid,
    evidence_id: EvidenceId,
    signal_label: &str,
    observed_at: chrono::DateTime<Utc>,
) -> DictationObservationEvidenceRecord {
    DictationObservationEvidenceRecord {
        observed_at,
        growth_evidence: GrowthEvidenceRecord {
            space_id,
            namespace_id,
            evidence_id,
            signal_labels: vec![signal_label.to_string()],
            explanation: None,
        },
    }
}

fn assert_descriptor_field_absent(value: &Value, field: &str) {
    match value {
        Value::Object(object) => {
            assert!(
                !object.contains_key(field),
                "summary must not contain descriptor field {field}: {value}"
            );
            for nested in object.values() {
                assert_descriptor_field_absent(nested, field);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_descriptor_field_absent(item, field);
            }
        }
        _ => {}
    }
}
