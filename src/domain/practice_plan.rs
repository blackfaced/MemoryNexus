use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::dream_candidate::{
    ConsolidationResultId, DreamCandidateEffectiveness, DreamCandidateEvaluation, DreamCandidateId,
    NamespaceId, TraceId,
};
use super::SpaceId;

pub type PracticePlanId = Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningRequest {
    pub space_id: SpaceId,
    pub namespace_id: NamespaceId,
    pub namespace: String,
    pub objective: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextTaskPlan {
    pub status: String,
    pub space_id: SpaceId,
    pub namespace_id: NamespaceId,
    pub namespace: String,
    pub plan_kind: String,
    pub persistence: String,
    pub next_task: NextTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextTask {
    pub title: String,
    pub prompt: String,
    pub task_type: String,
    pub duration_minutes: u16,
    pub priority: String,
    pub rationale: String,
    pub runtime: String,
}

pub fn build_next_task_plan(request: &PlanningRequest) -> NextTaskPlan {
    let namespace = compact_whitespace(&request.namespace);
    let namespace = if namespace.is_empty() {
        "active namespace".to_string()
    } else {
        namespace
    };
    let prompt = request
        .objective
        .as_deref()
        .map(compact_whitespace)
        .filter(|objective| !objective.is_empty())
        .unwrap_or_else(|| format!("Continue focused work in {namespace}."));

    NextTaskPlan {
        status: "next_task_ready".to_string(),
        space_id: request.space_id,
        namespace_id: request.namespace_id,
        namespace: namespace.clone(),
        plan_kind: "response_only_draft".to_string(),
        persistence: "not_persisted".to_string(),
        next_task: NextTask {
            title: format!("Next task for {namespace}"),
            prompt,
            task_type: "focused_next_action".to_string(),
            duration_minutes: 10,
            priority: "normal".to_string(),
            rationale:
                "Deterministic Planning uses the requested namespace and adapter-provided objective only."
                    .to_string(),
            runtime: "deterministic".to_string(),
        },
    }
}

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

fn compact_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
