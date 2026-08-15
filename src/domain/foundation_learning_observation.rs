use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FOUNDATION_INITIAL_GOAL: &str = "Record an initial foundation attempt";
pub const FOUNDATION_CORRECTION_GOAL: &str = "Practice a foundation correction";
pub const FOUNDATION_REINFORCEMENT_GOAL: &str = "Reinforce a foundation skill";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationMistakeType {
    ArithmeticComputation,
    PlaceValueCarry,
    PlaceValueBorrow,
    MultiplicationFact,
    OperationSign,
    TaskInterpretation,
    TimeReading,
    WordProblemModeling,
    UnclassifiedConfirmedError,
}

impl FoundationMistakeType {
    pub fn from_normalized(value: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(value.to_string())).ok()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ArithmeticComputation => "arithmetic_computation",
            Self::PlaceValueCarry => "place_value_carry",
            Self::PlaceValueBorrow => "place_value_borrow",
            Self::MultiplicationFact => "multiplication_fact",
            Self::OperationSign => "operation_sign",
            Self::TaskInterpretation => "task_interpretation",
            Self::TimeReading => "time_reading",
            Self::WordProblemModeling => "word_problem_modeling",
            Self::UnclassifiedConfirmedError => "unclassified_confirmed_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObservationSourceIdentity {
    pub source_product: String,
    pub source_installation_id: Uuid,
    pub record_type: String,
    pub record_id: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationAttemptRole {
    Initial,
    Correction,
    Reinforcement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FoundationEvidenceKind {
    LearningAttempt {
        role: FoundationAttemptRole,
        is_correct: bool,
        mistake_type: Option<FoundationMistakeType>,
    },
    LearningSession {
        role: FoundationAttemptRole,
        activity_count: u32,
        successful_activity_count: u32,
    },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationLearningEvidenceRecord {
    pub space_id: Uuid,
    pub namespace_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub evidence_trust: String,
    pub source_identity: ObservationSourceIdentity,
    pub kind: FoundationEvidenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationObservationStatus {
    Ready,
    EvidenceGap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FoundationObservationFacts {
    pub eligible_evidence_count: usize,
    pub initial_attempt_count: usize,
    pub correction_attempt_count: usize,
    pub reinforcement_attempt_count: usize,
    pub successful_correction_count: usize,
    pub successful_reinforcement_count: usize,
    pub correction_session_count: usize,
    pub reinforcement_session_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurringErrorPattern {
    pub mistake_type: String,
    pub occurrence_count: usize,
    pub supporting_sources: Vec<ObservationSourceIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelayedRecurrence {
    pub mistake_type: String,
    pub first_observed_at: DateTime<Utc>,
    pub recurred_at: DateTime<Utc>,
    pub supporting_sources: Vec<ObservationSourceIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FoundationDeterministicAggregates {
    pub recurring_errors: Vec<RecurringErrorPattern>,
    pub delayed_recurrence: Vec<DelayedRecurrence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationEvidenceGap {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationHypothesis {
    pub statement: String,
    pub supporting_sources: Vec<ObservationSourceIdentity>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationLearningObservation {
    pub status: FoundationObservationStatus,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub facts: FoundationObservationFacts,
    pub deterministic_aggregates: FoundationDeterministicAggregates,
    pub hypotheses: Vec<FoundationHypothesis>,
    pub optional_next_step_prompts: Vec<String>,
    pub evidence_gaps: Vec<FoundationEvidenceGap>,
    pub selected_sources: Vec<ObservationSourceIdentity>,
    pub excluded_evidence_count: usize,
    pub responsibility_note: String,
}

pub fn build_foundation_learning_observation(
    space_id: Uuid,
    namespace_id: Uuid,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    evidence: Vec<FoundationLearningEvidenceRecord>,
) -> FoundationLearningObservation {
    let total_count = evidence.len();
    let mut seen = HashSet::new();
    let mut selected = evidence
        .into_iter()
        .filter(|record| {
            record.space_id == space_id
                && record.namespace_id == namespace_id
                && record.evidence_trust == "contract_trusted"
                && record.occurred_at >= window_start
                && record.occurred_at <= window_end
                && eligible_foundation_kind(&record.kind)
                && seen.insert(record.source_identity.clone())
        })
        .collect::<Vec<_>>();
    selected.sort_by_key(|record| (record.occurred_at, record.source_identity.clone()));

    let mut facts = FoundationObservationFacts {
        eligible_evidence_count: selected.len(),
        ..FoundationObservationFacts::default()
    };
    let mut errors: BTreeMap<
        FoundationMistakeType,
        Vec<(DateTime<Utc>, ObservationSourceIdentity)>,
    > = BTreeMap::new();

    for record in &selected {
        match &record.kind {
            FoundationEvidenceKind::LearningAttempt {
                role,
                is_correct,
                mistake_type,
            } => {
                match role {
                    FoundationAttemptRole::Initial => facts.initial_attempt_count += 1,
                    FoundationAttemptRole::Correction => {
                        facts.correction_attempt_count += 1;
                        facts.successful_correction_count += usize::from(*is_correct);
                    }
                    FoundationAttemptRole::Reinforcement => {
                        facts.reinforcement_attempt_count += 1;
                        facts.successful_reinforcement_count += usize::from(*is_correct);
                    }
                }
                if !is_correct {
                    if let Some(mistake_type) = mistake_type {
                        errors
                            .entry(*mistake_type)
                            .or_default()
                            .push((record.occurred_at, record.source_identity.clone()));
                    }
                }
            }
            FoundationEvidenceKind::LearningSession { role, .. } => match role {
                FoundationAttemptRole::Initial => {}
                FoundationAttemptRole::Correction => facts.correction_session_count += 1,
                FoundationAttemptRole::Reinforcement => facts.reinforcement_session_count += 1,
            },
            FoundationEvidenceKind::Unsupported => unreachable!("filtered above"),
        }
    }

    let mut deterministic_aggregates = FoundationDeterministicAggregates::default();
    for (mistake_type, occurrences) in errors {
        if occurrences.len() < 2 {
            continue;
        }
        deterministic_aggregates
            .recurring_errors
            .push(RecurringErrorPattern {
                mistake_type: mistake_type.as_str().to_string(),
                occurrence_count: occurrences.len(),
                supporting_sources: occurrences
                    .iter()
                    .map(|(_, identity)| identity.clone())
                    .collect(),
            });
        let first = &occurrences[0];
        if let Some(recurrence) = occurrences
            .iter()
            .skip(1)
            .find(|(occurred_at, _)| *occurred_at - first.0 >= Duration::hours(24))
        {
            deterministic_aggregates
                .delayed_recurrence
                .push(DelayedRecurrence {
                    mistake_type: mistake_type.as_str().to_string(),
                    first_observed_at: first.0,
                    recurred_at: recurrence.0,
                    supporting_sources: vec![first.1.clone(), recurrence.1.clone()],
                });
        }
    }

    let mut evidence_gaps = Vec::new();
    if selected.is_empty() {
        evidence_gaps.push(FoundationEvidenceGap {
            code: "no_current_trusted_evidence".to_string(),
            message: "No current contract-trusted foundation evidence is available in this window."
                .to_string(),
        });
    } else if selected.len() < 3 {
        evidence_gaps.push(FoundationEvidenceGap {
            code: "sparse_evidence".to_string(),
            message: "Fewer than three eligible records; no mastery or regression conclusion is supported."
                .to_string(),
        });
    }
    if facts.reinforcement_attempt_count + facts.reinforcement_session_count == 0
        && !selected.is_empty()
    {
        evidence_gaps.push(FoundationEvidenceGap {
            code: "no_reinforcement_evidence".to_string(),
            message: "No eligible reinforcement evidence is present in this window.".to_string(),
        });
    }

    let optional_next_step_prompts = if deterministic_aggregates.recurring_errors.is_empty() {
        Vec::new()
    } else {
        vec![
            "Review the cited recurring error evidence before deciding a next practice step."
                .to_string(),
        ]
    };
    let source_products = selected
        .iter()
        .map(|record| record.source_identity.source_product.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    let responsibility_note = if source_products.is_empty() {
        "Exact correction obligations and schedules remain with the source application; MemoryNexus reports only longitudinal evidence."
            .to_string()
    } else {
        format!(
            "Exact correction obligations and schedules remain with the cited source application(s) [{source_products}]; MemoryNexus reports only longitudinal evidence."
        )
    };
    FoundationLearningObservation {
        status: if selected.is_empty() {
            FoundationObservationStatus::EvidenceGap
        } else {
            FoundationObservationStatus::Ready
        },
        window_start,
        window_end,
        facts,
        deterministic_aggregates,
        hypotheses: Vec::new(),
        optional_next_step_prompts,
        evidence_gaps,
        selected_sources: selected
            .iter()
            .map(|record| record.source_identity.clone())
            .collect(),
        excluded_evidence_count: total_count.saturating_sub(selected.len()),
        responsibility_note,
    }
}

fn eligible_foundation_kind(kind: &FoundationEvidenceKind) -> bool {
    match kind {
        FoundationEvidenceKind::LearningAttempt {
            is_correct,
            mistake_type,
            ..
        } => (*is_correct && mistake_type.is_none()) || (!is_correct && mistake_type.is_some()),
        FoundationEvidenceKind::LearningSession {
            activity_count,
            successful_activity_count,
            ..
        } => successful_activity_count <= activity_count,
        FoundationEvidenceKind::Unsupported => false,
    }
}
