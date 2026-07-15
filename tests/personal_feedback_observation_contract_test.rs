use chrono::NaiveDate;
use memorynexus::domain::growth_model::EvidenceId;
use memorynexus::domain::personal_feedback_observation::{
    build_personal_feedback_observation_summary, PersonalFeedbackObservationStatus,
    SleepObservationConfirmationMethod, SleepObservationEvidenceRecord,
    SleepObservationInputSource, BASELINE_WINDOW_DURATION_DAYS, BASELINE_WINDOW_KIND,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn zero_to_two_records_return_explicit_evidence_gap_without_observation_or_hypothesis() {
    for count in 0..=2 {
        let summary = build_personal_feedback_observation_summary(records(&[1, 2][..count]));
        assert_eq!(
            summary.status,
            PersonalFeedbackObservationStatus::NeedsMoreEvidence
        );
        assert_eq!(summary.valid_record_count, count);
        assert_eq!(summary.threshold, 3);
        assert_eq!(summary.remaining_record_count, 3 - count);
        assert_eq!(summary.window.kind, BASELINE_WINDOW_KIND);
        assert_eq!(summary.window.duration_days, BASELINE_WINDOW_DURATION_DAYS);
        assert!(summary.window.inclusive);
        assert_eq!(summary.observed, None);
        assert!(summary.hypotheses.is_empty());
        assert!(summary
            .supporting_evidence_ids
            .iter()
            .all(|id| matches!(id, EvidenceId::Memory(_))));
    }
}

#[test]
fn three_records_return_only_bounded_observations() {
    let summary = build_personal_feedback_observation_summary(records(&[11, 12, 13]));
    assert_eq!(
        summary.status,
        PersonalFeedbackObservationStatus::BaselineReady
    );
    assert_eq!(
        summary.window.start_local_date.as_deref(),
        Some("2026-01-01")
    );
    assert_eq!(summary.window.end_local_date.as_deref(), Some("2026-01-14"));
    assert!(summary.hypotheses.is_empty());
    let observed = summary
        .observed
        .expect("threshold produces observed baseline");
    assert_eq!(observed.sleep_duration_minutes.coverage_count, 3);
    assert_eq!(observed.sleep_duration_minutes.min, 420);
    assert_eq!(observed.sleep_duration_minutes.max, 480);
    assert_eq!(observed.sleep_duration_minutes.median, 450.0);
    assert_eq!(observed.daytime_energy.distribution["3"], 3);
    assert_eq!(observed.sleep_timing.coverage_count, 2);
    assert_eq!(observed.input_sources["typed"], 3);
    assert_eq!(observed.confirmations["explicit_acceptance"], 3);
}

#[test]
fn even_median_and_out_of_window_data_are_deterministic() {
    let summary = build_personal_feedback_observation_summary(records(&[0, 1, 2, 3, 14]));
    let observed = summary.observed.expect("five selected records");
    // The record on day 0 is before the latest-evidence anchored 14-day window.
    assert_eq!(summary.valid_record_count, 4);
    assert_eq!(
        summary.window.start_local_date.as_deref(),
        Some("2026-01-02")
    );
    assert_eq!(summary.window.end_local_date.as_deref(), Some("2026-01-15"));
    assert_eq!(observed.sleep_duration_minutes.median, 465.0);
    assert_eq!(observed.sleep_duration_minutes.min, 420);
    assert_eq!(observed.sleep_duration_minutes.max, 480);
}

#[test]
fn duplicate_current_local_date_is_excluded_instead_of_arbitrarily_selected() {
    let mut evidence = records(&[1, 2, 3]);
    let duplicate = SleepObservationEvidenceRecord {
        memory_id: Uuid::new_v4(),
        ..evidence[1].clone()
    };
    evidence.push(duplicate);
    let summary = build_personal_feedback_observation_summary(evidence);
    assert_eq!(
        summary.status,
        PersonalFeedbackObservationStatus::NeedsMoreEvidence
    );
    assert_eq!(summary.valid_record_count, 2);
    assert_eq!(summary.remaining_record_count, 1);
}

#[test]
fn persisted_record_rejects_present_but_malformed_optional_fields_and_correction_ids() {
    let memory_id = Uuid::new_v4();
    for (field, value) in [
        ("sleep_start_local_time", json!(42)),
        ("sleep_end_local_time", json!("25:00")),
        ("caffeine_within_six_hours_of_sleep", json!("yes")),
        ("screen_minutes_in_final_hour", json!(61)),
    ] {
        let mut metadata = valid_persistence_metadata();
        metadata["capture"]["personal_feedback"][field] = value;
        assert!(
            SleepObservationEvidenceRecord::from_persistence_metadata(memory_id, &metadata)
                .is_none(),
            "{field} must exclude malformed evidence"
        );
    }
    let mut correction = valid_persistence_metadata();
    correction["capture"]["personal_feedback"]["input_confirmation"]["method"] =
        json!("explicit_correction");
    correction["capture"]["personal_feedback"]["corrects_record_id"] = json!("not-a-uuid");
    assert!(
        SleepObservationEvidenceRecord::from_persistence_metadata(memory_id, &correction).is_none()
    );
}

fn records(days: &[i64]) -> Vec<SleepObservationEvidenceRecord> {
    days.iter()
        .enumerate()
        .map(|(index, day)| SleepObservationEvidenceRecord {
            memory_id: Uuid::new_v4(),
            local_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + chrono::Duration::days(*day),
            sleep_duration_minutes: [420, 450, 480, 420, 480][index],
            daytime_energy: 3,
            sleep_timing_present: index % 2 == 0,
            input_source: SleepObservationInputSource::Typed,
            confirmation_method: SleepObservationConfirmationMethod::ExplicitAcceptance,
        })
        .collect()
}

fn valid_persistence_metadata() -> serde_json::Value {
    json!({
        "capture": {"personal_feedback": {
            "record_type": "sleep_energy_check_in", "local_date": "2026-01-13",
            "sleep_duration_minutes": 450, "daytime_energy": 3,
            "input_source": "typed",
            "input_confirmation": {"status": "confirmed", "method": "explicit_acceptance"}
        }}
    })
}
