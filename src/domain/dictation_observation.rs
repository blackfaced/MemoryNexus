use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::growth_model::{
    aggregate_growth_model, EvidenceId, GrowthEvidenceRecord, GrowthModelId, NamespaceId,
};
use super::SpaceId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictationObservationEvidenceRecord {
    pub observed_at: DateTime<Utc>,
    pub growth_evidence: GrowthEvidenceRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationObservationStatus {
    Ready,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationStabilitySignal {
    Stable,
    Improving,
    NeedsFocus,
    NeedsMoreEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictationObservationSummary {
    pub status: DictationObservationStatus,
    pub timeframe: String,
    pub evidence_record_count: usize,
    pub recurring_mistake_types: Vec<String>,
    pub stability_signal: DictationStabilitySignal,
    pub current_focus: String,
    pub supporting_evidence_ids: Vec<EvidenceId>,
    pub evidence_gaps: Vec<String>,
    pub growth_model_id: GrowthModelId,
    pub growth_model_status: String,
}

pub fn build_dictation_observation_summary(
    space_id: SpaceId,
    namespace_id: NamespaceId,
    now: DateTime<Utc>,
    evidence_records: Vec<DictationObservationEvidenceRecord>,
) -> DictationObservationSummary {
    let window_start = now - Duration::days(7);
    let recent_records = evidence_records
        .into_iter()
        .filter(|record| record.observed_at >= window_start && record.observed_at <= now)
        .collect::<Vec<_>>();

    if recent_records.is_empty() {
        return DictationObservationSummary {
            status: DictationObservationStatus::Empty,
            timeframe: "7d".to_string(),
            evidence_record_count: 0,
            recurring_mistake_types: Vec::new(),
            stability_signal: DictationStabilitySignal::NeedsMoreEvidence,
            current_focus: "No recent dictation history yet".to_string(),
            supporting_evidence_ids: Vec::new(),
            evidence_gaps: vec!["no recent dictation history".to_string()],
            growth_model_id: namespace_id,
            growth_model_status: "derived_from_growth_evidence".to_string(),
        };
    }

    let mut attempt_evidence_ids = Vec::new();
    for record in &recent_records {
        if !attempt_evidence_ids.contains(&record.growth_evidence.evidence_id) {
            attempt_evidence_ids.push(record.growth_evidence.evidence_id);
        }
    }
    let evidence_record_count = attempt_evidence_ids.len();
    let growth_records = recent_records
        .into_iter()
        .map(|record| record.growth_evidence)
        .collect::<Vec<GrowthEvidenceRecord>>();
    let aggregation = aggregate_growth_model(space_id, namespace_id, growth_records, now);
    let recurring_mistake_types = aggregation
        .model
        .recurring_patterns
        .iter()
        .filter_map(|pattern| dictation_mistake_type(&pattern.pattern))
        .collect::<Vec<_>>();
    let stability_signal = if recurring_mistake_types.is_empty() {
        DictationStabilitySignal::NeedsMoreEvidence
    } else {
        DictationStabilitySignal::NeedsFocus
    };

    DictationObservationSummary {
        status: DictationObservationStatus::Ready,
        timeframe: "7d".to_string(),
        evidence_record_count,
        recurring_mistake_types,
        stability_signal,
        current_focus: aggregation.model.recommended_focus.focus,
        supporting_evidence_ids: aggregation.model.recommended_focus.evidence_ids,
        evidence_gaps: aggregation.evidence_gaps,
        growth_model_id: aggregation.model.id,
        growth_model_status: "derived_from_growth_evidence".to_string(),
    }
}

fn dictation_mistake_type(pattern: &str) -> Option<String> {
    pattern
        .strip_prefix("repeated dictation mistake type: ")
        .map(str::to_string)
}
