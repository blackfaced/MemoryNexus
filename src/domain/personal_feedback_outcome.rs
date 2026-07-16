//! Typed outcome and disposition semantics for the private dogfood lifecycle.
//! They intentionally describe only owner-reported adherence, never benefit.

use serde::{Deserialize, Serialize};

pub const PERSONAL_FEEDBACK_OUTCOME_POLICY_VERSION: &str = "personal_feedback_sleep_outcome_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalFeedbackOutcomeValue {
    Performed,
    Skipped,
    NotEvaluable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalFeedbackDisposition {
    Continue,
    Stop,
    Retest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalFeedbackOutcomeState {
    Performed,
    Skipped,
    NotEvaluable,
    AwaitingOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonalFeedbackDispositionCandidate {
    pub policy_version: &'static str,
    pub outcome_state: PersonalFeedbackOutcomeState,
    pub disposition: Option<PersonalFeedbackDisposition>,
    pub rationale: &'static str,
}

pub fn disposition_candidate(
    outcome: Option<PersonalFeedbackOutcomeValue>,
) -> PersonalFeedbackDispositionCandidate {
    match outcome {
        Some(PersonalFeedbackOutcomeValue::Performed) => PersonalFeedbackDispositionCandidate {
            policy_version: PERSONAL_FEEDBACK_OUTCOME_POLICY_VERSION,
            outcome_state: PersonalFeedbackOutcomeState::Performed,
            disposition: Some(PersonalFeedbackDisposition::Continue),
            rationale: "The offered action was tried; this does not establish benefit.",
        },
        Some(PersonalFeedbackOutcomeValue::Skipped) => PersonalFeedbackDispositionCandidate {
            policy_version: PERSONAL_FEEDBACK_OUTCOME_POLICY_VERSION,
            outcome_state: PersonalFeedbackOutcomeState::Skipped,
            disposition: Some(PersonalFeedbackDisposition::Retest),
            rationale: "The action was skipped; this is not ineffectiveness.",
        },
        Some(PersonalFeedbackOutcomeValue::NotEvaluable) => PersonalFeedbackDispositionCandidate {
            policy_version: PERSONAL_FEEDBACK_OUTCOME_POLICY_VERSION,
            outcome_state: PersonalFeedbackOutcomeState::NotEvaluable,
            disposition: Some(PersonalFeedbackDisposition::Retest),
            rationale:
                "The outcome was not evaluable; this is not evidence of benefit or lack of benefit.",
        },
        None => PersonalFeedbackDispositionCandidate {
            policy_version: PERSONAL_FEEDBACK_OUTCOME_POLICY_VERSION,
            outcome_state: PersonalFeedbackOutcomeState::AwaitingOutcome,
            disposition: None,
            rationale: "Awaiting a current owner-reported outcome; no failure or stop is inferred.",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_performed_can_suggest_continue_and_never_claims_benefit() {
        let candidate = disposition_candidate(Some(PersonalFeedbackOutcomeValue::Performed));
        assert_eq!(
            candidate.disposition,
            Some(PersonalFeedbackDisposition::Continue)
        );
        assert!(candidate.rationale.contains("does not establish benefit"));
    }

    #[test]
    fn skipped_not_evaluable_and_missing_are_not_ineffectiveness() {
        for value in [
            Some(PersonalFeedbackOutcomeValue::Skipped),
            Some(PersonalFeedbackOutcomeValue::NotEvaluable),
            None,
        ] {
            let candidate = disposition_candidate(value);
            assert!(!candidate.rationale.contains("is ineffective"));
        }
        assert_eq!(disposition_candidate(None).disposition, None);
    }
}
