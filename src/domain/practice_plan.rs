use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::dream_candidate::{
    ConsolidationResultId, DreamCandidateEffectiveness, DreamCandidateEvaluation, DreamCandidateId,
    NamespaceId, TraceId,
};
use super::SpaceId;

pub type PracticePlanId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum PracticePlanSource {
    DreamCandidate(DreamCandidateId),
    ConsolidationResult(ConsolidationResultId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticePlanStatus {
    Selected,
    Executed,
    Evaluated,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticePlan {
    pub id: PracticePlanId,
    pub space_id: SpaceId,
    pub namespace_id: Option<NamespaceId>,
    pub source: PracticePlanSource,
    pub purpose: String,
    pub content: String,
    pub expected_effect: Option<String>,
    pub status: PracticePlanStatus,
    pub selected_at: Option<DateTime<Utc>>,
    pub executed_trace_ids: Vec<TraceId>,
    pub evaluation_trace_ids: Vec<TraceId>,
    pub evaluation_result: Option<DreamCandidateEvaluation>,
    pub created_at: DateTime<Utc>,
}

impl PracticePlan {
    pub fn from_dream_candidate(
        space_id: SpaceId,
        namespace_id: Option<NamespaceId>,
        dream_candidate_id: DreamCandidateId,
        purpose: impl Into<String>,
        content: impl Into<String>,
        expected_effect: Option<&str>,
    ) -> Self {
        Self::new(
            space_id,
            namespace_id,
            PracticePlanSource::DreamCandidate(dream_candidate_id),
            purpose,
            content,
            expected_effect,
        )
    }

    pub fn from_consolidation_result(
        space_id: SpaceId,
        namespace_id: Option<NamespaceId>,
        consolidation_result_id: ConsolidationResultId,
        purpose: impl Into<String>,
        content: impl Into<String>,
        expected_effect: Option<&str>,
    ) -> Self {
        Self::new(
            space_id,
            namespace_id,
            PracticePlanSource::ConsolidationResult(consolidation_result_id),
            purpose,
            content,
            expected_effect,
        )
    }

    pub fn mark_selected(&mut self, selected_at: DateTime<Utc>) {
        self.status = PracticePlanStatus::Selected;
        self.selected_at = Some(selected_at);
    }

    pub fn record_execution(&mut self, trace_id: TraceId) {
        self.executed_trace_ids.push(trace_id);
        self.status = PracticePlanStatus::Executed;
    }

    pub fn record_evaluation(
        &mut self,
        trace_id: TraceId,
        effectiveness: DreamCandidateEffectiveness,
        summary: impl Into<String>,
    ) {
        self.evaluation_trace_ids.push(trace_id);
        self.evaluation_result = Some(DreamCandidateEvaluation {
            trace_id,
            effectiveness,
            summary: summary.into(),
            evaluated_at: Utc::now(),
        });
        self.status = PracticePlanStatus::Evaluated;
    }

    pub fn cancel(&mut self) {
        self.status = PracticePlanStatus::Cancelled;
    }

    fn new(
        space_id: SpaceId,
        namespace_id: Option<NamespaceId>,
        source: PracticePlanSource,
        purpose: impl Into<String>,
        content: impl Into<String>,
        expected_effect: Option<&str>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            space_id,
            namespace_id,
            source,
            purpose: purpose.into(),
            content: content.into(),
            expected_effect: expected_effect.map(str::to_string),
            status: PracticePlanStatus::Selected,
            selected_at: Some(Utc::now()),
            executed_trace_ids: Vec::new(),
            evaluation_trace_ids: Vec::new(),
            evaluation_result: None,
            created_at: Utc::now(),
        }
    }
}
