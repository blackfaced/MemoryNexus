use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{MemoryId, SpaceId};

pub type GrowthModelId = Uuid;
pub type NamespaceId = Uuid;
pub type TraceId = Uuid;
pub type FeedbackLoopId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum EvidenceId {
    Trace(TraceId),
    FeedbackLoop(FeedbackLoopId),
    Memory(MemoryId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBackedClaim {
    pub label: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBackedPattern {
    pub pattern: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthStage {
    pub label: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBackedFocus {
    pub focus: String,
    pub rationale: String,
    pub evidence_ids: Vec<EvidenceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthModel {
    pub id: GrowthModelId,
    pub space_id: SpaceId,
    pub namespace_id: NamespaceId,
    pub strengths: Vec<EvidenceBackedClaim>,
    pub weaknesses: Vec<EvidenceBackedClaim>,
    pub recurring_patterns: Vec<EvidenceBackedPattern>,
    pub current_stage: GrowthStage,
    pub recommended_focus: EvidenceBackedFocus,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthEvidenceRecord {
    pub space_id: SpaceId,
    pub namespace_id: NamespaceId,
    pub evidence_id: EvidenceId,
    pub signal_labels: Vec<String>,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthModelAggregation {
    pub model: GrowthModel,
    pub evidence_gaps: Vec<String>,
}

pub fn aggregate_growth_model(
    space_id: SpaceId,
    namespace_id: NamespaceId,
    evidence_records: Vec<GrowthEvidenceRecord>,
    updated_at: DateTime<Utc>,
) -> GrowthModelAggregation {
    let mut evidence_by_signal = BTreeMap::<String, Vec<EvidenceId>>::new();
    let mut evidence_gaps = Vec::<String>::new();

    if evidence_records.is_empty() {
        evidence_gaps.push("no evidence records provided".to_string());
    }

    for record in evidence_records {
        if record.space_id != space_id {
            push_gap_once(
                &mut evidence_gaps,
                "ignored evidence outside target space".to_string(),
            );
            continue;
        }
        if record.namespace_id != namespace_id {
            push_gap_once(
                &mut evidence_gaps,
                "ignored evidence outside target namespace".to_string(),
            );
            continue;
        }

        let labels = record
            .signal_labels
            .into_iter()
            .map(|label| label.trim().to_string())
            .filter(|label| is_growth_signal(label))
            .collect::<BTreeSet<_>>();

        if labels.is_empty() {
            push_gap_once(
                &mut evidence_gaps,
                "ignored evidence without deterministic signal labels".to_string(),
            );
            continue;
        }

        for label in labels {
            let evidence_ids = evidence_by_signal.entry(label).or_default();
            if !evidence_ids.contains(&record.evidence_id) {
                evidence_ids.push(record.evidence_id);
            }
        }
    }

    let recurring_patterns = evidence_by_signal
        .iter()
        .filter_map(|(label, evidence_ids)| {
            if evidence_ids.len() < 2 {
                return None;
            }

            Some(EvidenceBackedPattern {
                pattern: format!("repeated dictation mistake type: {label}"),
                evidence_ids: evidence_ids.clone(),
            })
        })
        .collect::<Vec<_>>();

    if recurring_patterns.is_empty()
        && !evidence_gaps
            .iter()
            .any(|gap| gap == "no evidence records provided")
    {
        push_gap_once(
            &mut evidence_gaps,
            "insufficient compatible evidence for recurring pattern".to_string(),
        );
    }

    let pattern_evidence = recurring_patterns
        .iter()
        .flat_map(|pattern| pattern.evidence_ids.iter().copied())
        .collect::<Vec<_>>();
    let weaknesses = recurring_patterns
        .iter()
        .map(|pattern| EvidenceBackedClaim {
            label: pattern.pattern.clone(),
            evidence_ids: pattern.evidence_ids.clone(),
        })
        .collect::<Vec<_>>();
    let current_stage = if recurring_patterns.is_empty() {
        GrowthStage {
            label: "needs more evidence".to_string(),
            evidence_ids: Vec::new(),
        }
    } else {
        GrowthStage {
            label: "recurring pattern detected".to_string(),
            evidence_ids: pattern_evidence.clone(),
        }
    };
    let recommended_focus = recommended_focus(&recurring_patterns);

    GrowthModelAggregation {
        model: GrowthModel {
            id: namespace_id,
            space_id,
            namespace_id,
            strengths: Vec::new(),
            weaknesses,
            recurring_patterns,
            current_stage,
            recommended_focus,
            updated_at,
        },
        evidence_gaps,
    }
}

fn recommended_focus(recurring_patterns: &[EvidenceBackedPattern]) -> EvidenceBackedFocus {
    if let Some(pattern) = recurring_patterns.first() {
        let signal = pattern
            .pattern
            .strip_prefix("repeated dictation mistake type: ")
            .unwrap_or(pattern.pattern.as_str());
        return EvidenceBackedFocus {
            focus: format!("Review {signal} with short targeted practice"),
            rationale: format!("Recent evidence shows a recurring {signal} pattern"),
            evidence_ids: pattern.evidence_ids.clone(),
        };
    }

    EvidenceBackedFocus {
        focus: "Collect more confirmed attempts".to_string(),
        rationale: "Current evidence is too sparse for a recurring pattern".to_string(),
        evidence_ids: Vec::new(),
    }
}

fn is_growth_signal(label: &str) -> bool {
    !label.is_empty() && label != "correct" && label != "unclassified"
}

fn push_gap_once(evidence_gaps: &mut Vec<String>, gap: String) {
    if !evidence_gaps.contains(&gap) {
        evidence_gaps.push(gap);
    }
}
