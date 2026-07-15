//! Deterministic, non-clinical planning policy for the ADR-025 dogfood slice.
//!
//! This policy deliberately uses only field presence/coverage from confirmed
//! canonical evidence. It does not classify sleep, infer causes, or claim an
//! outcome. The policy is versioned so an active lifecycle remains explainable.

use serde::Serialize;

use super::growth_model::EvidenceId;
use super::personal_feedback_observation::{
    build_personal_feedback_observation_summary, PersonalFeedbackObservationStatus,
    SleepObservationEvidenceRecord, BASELINE_THRESHOLD,
};

pub const PERSONAL_FEEDBACK_POLICY_VERSION: &str = "personal_feedback_sleep_v1";
pub const SCREEN_FREE_FINAL_HOUR_ACTION_ID: &str = "screen_free_final_hour";
pub const CONSISTENT_WAKE_WINDOW_ACTION_ID: &str = "consistent_owner_selected_wake_window";
pub const EXPERIMENT_DURATION_DAYS: u16 = 7;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalFeedbackPlanningStatus {
    NeedsMoreEvidence,
    ActionEvidenceGap,
    ExperimentReady,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonalFeedbackPlanningResult {
    pub status: PersonalFeedbackPlanningStatus,
    pub policy_version: &'static str,
    pub valid_record_count: usize,
    pub threshold: usize,
    pub window: super::personal_feedback_observation::PersonalFeedbackObservationWindow,
    pub supporting_evidence_ids: Vec<EvidenceId>,
    pub action: Option<PersonalFeedbackExperimentAction>,
    pub evidence_gap_reason: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonalFeedbackExperimentAction {
    pub action_id: &'static str,
    pub advisory_text: String,
    pub duration_days: u16,
    pub rationale: String,
    pub expected_observable_signal: String,
    pub selected_evidence_ids: Vec<EvidenceId>,
}

/// V1 reviewed policy table, in explicit priority order:
/// 1. `screen_free_final_hour`: all baseline records include the bounded
///    pre-sleep screen field. The experiment is a seven-day screen-free final
///    hour; signal is confirmed field coverage during the experiment.
/// 2. `consistent_owner_selected_wake_window`: all baseline records include
///    sleep timing. The owner selects a consistent wake window for seven days;
///    signal is confirmed timing coverage during the experiment.
///    Neither action judges a value or predicts an effect.
pub fn select_personal_feedback_experiment(
    evidence: Vec<SleepObservationEvidenceRecord>,
    owner_selected_wake_time_window: Option<&str>,
) -> PersonalFeedbackPlanningResult {
    let summary = build_personal_feedback_observation_summary(evidence.clone());
    if summary.status == PersonalFeedbackObservationStatus::NeedsMoreEvidence {
        return PersonalFeedbackPlanningResult {
            status: PersonalFeedbackPlanningStatus::NeedsMoreEvidence,
            policy_version: PERSONAL_FEEDBACK_POLICY_VERSION,
            valid_record_count: summary.valid_record_count,
            threshold: summary.threshold,
            window: summary.window,
            supporting_evidence_ids: summary.supporting_evidence_ids,
            action: None,
            evidence_gap_reason: Some("baseline_requires_three_confirmed_records"),
        };
    }

    // Reconstruct selected records using the canonical summary's selected IDs;
    // this preserves #223 duplicate-day and rolling-window rules.
    let selected_ids = summary.supporting_evidence_ids.clone();
    let selected = evidence
        .into_iter()
        .filter(|record| selected_ids.contains(&EvidenceId::Memory(record.memory_id)))
        .collect::<Vec<_>>();
    let all_screen_present = selected.len() >= BASELINE_THRESHOLD
        && selected
            .iter()
            .all(|record| record.screen_minutes_before_sleep.is_some());
    let screen_activity_observed = selected.iter().any(|record| {
        record
            .screen_minutes_before_sleep
            .is_some_and(|minutes| minutes > 0)
    });
    let all_timing_present = selected.len() >= BASELINE_THRESHOLD
        && selected.iter().all(|record| record.sleep_timing_present);
    let action = if all_screen_present && screen_activity_observed {
        Some(PersonalFeedbackExperimentAction {
            action_id: SCREEN_FREE_FINAL_HOUR_ACTION_ID,
            advisory_text: "For the next 7 days, try a screen-free final hour before sleep.".to_string(),
            duration_days: EXPERIMENT_DURATION_DAYS,
            rationale: "Your confirmed baseline includes the bounded pre-sleep screen field, so this reversible experiment can be tracked.".to_string(),
            expected_observable_signal: "Confirmed daily records include the pre-sleep screen field during the experiment.".to_string(),
            selected_evidence_ids: selected_ids.clone(),
        })
    } else if all_timing_present
        && owner_selected_wake_time_window.is_some_and(|value| !value.trim().is_empty())
    {
        let window = owner_selected_wake_time_window
            .expect("checked above")
            .trim()
            .to_string();
        Some(PersonalFeedbackExperimentAction {
            action_id: CONSISTENT_WAKE_WINDOW_ACTION_ID,
            advisory_text: format!("For the next 7 days, try to keep your owner-selected wake-time window: {window}."),
            duration_days: EXPERIMENT_DURATION_DAYS,
            rationale: "Your confirmed baseline includes sleep timing and you supplied a wake-time window, so this reversible experiment can be tracked.".to_string(),
            expected_observable_signal: format!("Confirmed daily records record whether the owner-selected wake-time window {window} was followed."),
            selected_evidence_ids: selected_ids.clone(),
        })
    } else {
        None
    };
    PersonalFeedbackPlanningResult {
        status: if action.is_some() {
            PersonalFeedbackPlanningStatus::ExperimentReady
        } else {
            PersonalFeedbackPlanningStatus::ActionEvidenceGap
        },
        policy_version: PERSONAL_FEEDBACK_POLICY_VERSION,
        valid_record_count: summary.valid_record_count,
        threshold: summary.threshold,
        window: summary.window,
        supporting_evidence_ids: selected_ids,
        action,
        evidence_gap_reason: if (all_screen_present && screen_activity_observed)
            || (all_timing_present
                && owner_selected_wake_time_window.is_some_and(|value| !value.trim().is_empty()))
        {
            None
        } else {
            Some("requires_screen_field_or_sleep_timing_coverage_for_every_baseline_record")
        },
    }
}
