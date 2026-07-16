use memorynexus::domain::personal_feedback_outcome::{
    disposition_candidate, PersonalFeedbackDisposition, PersonalFeedbackOutcomeState,
    PersonalFeedbackOutcomeValue, PERSONAL_FEEDBACK_OUTCOME_POLICY_VERSION,
};

#[test]
fn current_outcome_values_have_only_non_clinical_deterministic_candidates() {
    let cases = [
        (
            Some(PersonalFeedbackOutcomeValue::Performed),
            PersonalFeedbackOutcomeState::Performed,
            Some(PersonalFeedbackDisposition::Continue),
        ),
        (
            Some(PersonalFeedbackOutcomeValue::Skipped),
            PersonalFeedbackOutcomeState::Skipped,
            Some(PersonalFeedbackDisposition::Retest),
        ),
        (
            Some(PersonalFeedbackOutcomeValue::NotEvaluable),
            PersonalFeedbackOutcomeState::NotEvaluable,
            Some(PersonalFeedbackDisposition::Retest),
        ),
        (None, PersonalFeedbackOutcomeState::AwaitingOutcome, None),
    ];

    for (outcome, expected_state, expected_disposition) in cases {
        let candidate = disposition_candidate(outcome);
        assert_eq!(
            candidate.policy_version,
            PERSONAL_FEEDBACK_OUTCOME_POLICY_VERSION
        );
        assert_eq!(candidate.outcome_state, expected_state);
        assert_eq!(candidate.disposition, expected_disposition);
        assert!(
            !candidate.rationale.contains("benefit or lack of benefit")
                || outcome == Some(PersonalFeedbackOutcomeValue::NotEvaluable)
        );
        assert!(!candidate.rationale.contains("is ineffective"));
    }
}

#[test]
fn migration_keeps_missing_derived_and_requires_same_scope_lineage() {
    let migration = std::fs::read_to_string("migrations/022_planning_lifecycle_outcomes.sql")
        .expect("outcome migration should exist");

    assert!(migration.contains("'performed', 'skipped', 'not_evaluable'"));
    assert!(!migration.contains("'missing'"));
    assert!(migration.contains("planning_lifecycle_outcomes_one_current_per_date"));
    assert!(migration.contains("planning_lifecycle_outcomes_event_scope_unique"));
    assert!(migration.contains("enforce_planning_lifecycle_outcome_lineage"));
    assert!(migration.contains("corrected.superseded_by_outcome_id IS NOT NULL"));
}
