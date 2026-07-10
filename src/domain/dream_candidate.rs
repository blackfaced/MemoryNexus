use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{cognitive_scene::CognitiveSceneId, SpaceId};

pub type SleepCycleId = Uuid;
pub type ConsolidationResultId = Uuid;
pub type DreamCandidateId = Uuid;
pub type NamespaceId = Uuid;
pub type TraceId = Uuid;
pub type FeedbackLoopId = Uuid;
pub type GrowthModelId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamCandidateSource {
    pub sleep_cycle_id: SleepCycleId,
    pub consolidation_result_id: ConsolidationResultId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamCandidatePurpose {
    PracticeGeneration,
    ScenarioSimulation,
    ContradictionExploration,
    ReviewQuestion,
    PlanningPrompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamCandidateStatus {
    Proposed,
    Selected,
    Executed,
    Evaluated,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DreamCandidateEffectiveness {
    Useful,
    Neutral,
    Harmful,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamCandidateEvaluation {
    pub trace_id: TraceId,
    pub effectiveness: DreamCandidateEffectiveness,
    pub summary: String,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamCandidate {
    pub id: DreamCandidateId,
    pub space_id: SpaceId,
    pub namespace_id: Option<NamespaceId>,
    pub source_sleep_cycle_id: SleepCycleId,
    pub source_consolidation_result_id: ConsolidationResultId,
    pub source_knowledge_context_ids: Vec<Uuid>,
    pub purpose: DreamCandidatePurpose,
    pub title: Option<String>,
    pub content: String,
    pub rationale: Option<String>,
    pub expected_effect: Option<String>,
    pub target_feedback_loop_id: Option<FeedbackLoopId>,
    pub target_cognitive_scene_id: Option<CognitiveSceneId>,
    pub target_growth_model_id: Option<GrowthModelId>,
    pub status: DreamCandidateStatus,
    pub selected_at: Option<DateTime<Utc>>,
    pub executed_trace_ids: Vec<TraceId>,
    pub evaluation_trace_ids: Vec<TraceId>,
    pub evaluation_result: Option<DreamCandidateEvaluation>,
    pub created_at: DateTime<Utc>,
}

impl DreamCandidate {
    pub fn new(
        space_id: SpaceId,
        namespace_id: Option<NamespaceId>,
        source: DreamCandidateSource,
        purpose: DreamCandidatePurpose,
        content: impl Into<String>,
        expected_effect: Option<&str>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            space_id,
            namespace_id,
            source_sleep_cycle_id: source.sleep_cycle_id,
            source_consolidation_result_id: source.consolidation_result_id,
            source_knowledge_context_ids: Vec::new(),
            purpose,
            title: None,
            content: content.into(),
            rationale: None,
            expected_effect: expected_effect.map(str::to_string),
            target_feedback_loop_id: None,
            target_cognitive_scene_id: None,
            target_growth_model_id: None,
            status: DreamCandidateStatus::Proposed,
            selected_at: None,
            executed_trace_ids: Vec::new(),
            evaluation_trace_ids: Vec::new(),
            evaluation_result: None,
            created_at: Utc::now(),
        }
    }

    pub fn select(&mut self, selected_at: DateTime<Utc>) {
        self.status = DreamCandidateStatus::Selected;
        self.selected_at = Some(selected_at);
    }

    pub fn reject(&mut self) {
        self.status = DreamCandidateStatus::Rejected;
    }

    pub fn expire(&mut self) {
        self.status = DreamCandidateStatus::Expired;
    }

    pub fn cite_knowledge_context(&mut self, knowledge_context_id: Uuid) {
        if !self
            .source_knowledge_context_ids
            .contains(&knowledge_context_id)
        {
            self.source_knowledge_context_ids.push(knowledge_context_id);
        }
    }

    pub fn record_execution(&mut self, trace_id: TraceId) {
        self.executed_trace_ids.push(trace_id);
        self.status = DreamCandidateStatus::Executed;
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
        self.status = DreamCandidateStatus::Evaluated;
    }
}
