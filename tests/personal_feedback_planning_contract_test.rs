use chrono::NaiveDate;
use memorynexus::domain::personal_feedback_observation::{
    SleepObservationConfirmationMethod, SleepObservationEvidenceRecord, SleepObservationInputSource,
};
use memorynexus::domain::personal_feedback_planning::{
    select_personal_feedback_experiment, PersonalFeedbackPlanningStatus, WakeTimeWindow,
    WakeTimeWindowInput, CONSISTENT_WAKE_WINDOW_ACTION_ID, PERSONAL_FEEDBACK_POLICY_VERSION,
    SCREEN_FREE_FINAL_HOUR_ACTION_ID,
};
use uuid::Uuid;

fn records(screen: Option<i32>, timing: bool, count: usize) -> Vec<SleepObservationEvidenceRecord> {
    (0..count)
        .map(|index| SleepObservationEvidenceRecord {
            memory_id: Uuid::from_u128(index as u128 + 1),
            local_date: NaiveDate::from_ymd_opt(2026, 7, 1 + index as u32).unwrap(),
            sleep_duration_minutes: 420,
            daytime_energy: 3,
            sleep_timing_present: timing,
            screen_minutes_before_sleep: screen,
            input_source: SleepObservationInputSource::Typed,
            confirmation_method: SleepObservationConfirmationMethod::ExplicitAcceptance,
        })
        .collect()
}

#[test]
fn sparse_baseline_is_an_explicit_gap_without_action() {
    for count in 0..3 {
        let result = select_personal_feedback_experiment(records(Some(20), true, count), None);
        assert_eq!(
            result.status,
            PersonalFeedbackPlanningStatus::NeedsMoreEvidence
        );
        assert!(result.action.is_none());
        assert_eq!(result.valid_record_count, count);
    }
}

#[test]
fn policy_prefers_screen_experiment_then_timing_experiment() {
    let screen = select_personal_feedback_experiment(records(Some(20), true, 3), None);
    assert_eq!(
        screen.status,
        PersonalFeedbackPlanningStatus::ExperimentReady
    );
    assert_eq!(screen.policy_version, PERSONAL_FEEDBACK_POLICY_VERSION);
    assert_eq!(
        screen.action.unwrap().action_id,
        SCREEN_FREE_FINAL_HOUR_ACTION_ID
    );

    let wake_window = WakeTimeWindow::parse(WakeTimeWindowInput {
        start_local_time: "07:00".to_string(),
        end_local_time: "07:30".to_string(),
    })
    .unwrap();
    let timing = select_personal_feedback_experiment(records(None, true, 3), Some(&wake_window));
    assert_eq!(
        timing.status,
        PersonalFeedbackPlanningStatus::ExperimentReady
    );
    assert_eq!(
        timing.action.unwrap().action_id,
        CONSISTENT_WAKE_WINDOW_ACTION_ID
    );
}

#[test]
fn wake_window_is_strictly_bounded_and_typed() {
    for (start, end) in [
        ("7:00", "07:30"),
        ("07:30", "07:00"),
        ("07:00", "10:01"),
        ("07:00", "07:xx"),
    ] {
        assert!(WakeTimeWindow::parse(WakeTimeWindowInput {
            start_local_time: start.to_string(),
            end_local_time: end.to_string()
        })
        .is_err());
    }
    assert!(WakeTimeWindow::parse(WakeTimeWindowInput {
        start_local_time: "07:00".to_string(),
        end_local_time: "07:30".to_string()
    })
    .is_ok());
}

#[test]
fn missing_action_coverage_is_a_deterministic_gap() {
    let result = select_personal_feedback_experiment(records(None, false, 3), None);
    assert_eq!(
        result.status,
        PersonalFeedbackPlanningStatus::ActionEvidenceGap
    );
    assert!(result.action.is_none());
    assert_eq!(
        result.evidence_gap_reason,
        Some("requires_screen_field_or_sleep_timing_coverage_for_every_baseline_record")
    );
}

#[test]
fn already_screen_free_baseline_does_not_repeat_screen_experiment() {
    let result = select_personal_feedback_experiment(records(Some(0), true, 3), None);
    assert_eq!(
        result.status,
        PersonalFeedbackPlanningStatus::ActionEvidenceGap
    );
    assert!(result.action.is_none());
}
