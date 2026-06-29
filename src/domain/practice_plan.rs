use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::dream_candidate::{
    ConsolidationResultId, DreamCandidateEffectiveness, DreamCandidateEvaluation, DreamCandidateId,
    NamespaceId, TraceId,
};
use super::growth_model::{EvidenceId, GrowthModel, GrowthModelId};
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
    GrowthModel(GrowthModelId),
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
    pub target_pattern: Option<String>,
    pub content: String,
    pub expected_effect: Option<String>,
    pub evidence_ids: Vec<EvidenceId>,
    pub generation_trace_id: Option<TraceId>,
    pub target_growth_model_id: Option<GrowthModelId>,
    pub status: PracticePlanStatus,
    pub selected_at: Option<DateTime<Utc>>,
    pub executed_trace_ids: Vec<TraceId>,
    pub evaluation_trace_ids: Vec<TraceId>,
    pub evaluation_result: Option<DreamCandidateEvaluation>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticePlanEvidenceGap {
    pub space_id: SpaceId,
    pub namespace_id: NamespaceId,
    pub growth_model_id: GrowthModelId,
    pub reason: String,
    pub evidence_ids: Vec<EvidenceId>,
    pub generation_trace_id: TraceId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum PracticePlanGeneration {
    Plan(Box<PracticePlan>),
    EvidenceGap(PracticePlanEvidenceGap),
}

impl PracticePlanGeneration {
    pub fn from_growth_model(growth_model: &GrowthModel, generation_trace_id: TraceId) -> Self {
        PracticePlan::from_growth_model(growth_model, generation_trace_id)
    }

    pub fn into_plan(self) -> Option<PracticePlan> {
        match self {
            Self::Plan(plan) => Some(*plan),
            Self::EvidenceGap(_) => None,
        }
    }
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

    pub fn from_growth_model(
        growth_model: &GrowthModel,
        generation_trace_id: TraceId,
    ) -> PracticePlanGeneration {
        let focused_evidence = growth_model.recommended_focus.evidence_ids.clone();
        let focused_pattern = growth_model
            .recurring_patterns
            .iter()
            .find(|pattern| pattern.evidence_ids == focused_evidence);

        let Some(pattern) = focused_pattern else {
            return growth_model_evidence_gap(growth_model, generation_trace_id, focused_evidence);
        };

        if focused_evidence.is_empty()
            || growth_model
                .current_stage
                .label
                .eq_ignore_ascii_case("needs more evidence")
        {
            return growth_model_evidence_gap(growth_model, generation_trace_id, focused_evidence);
        }

        let mistake_type = dictation_mistake_type(&pattern.pattern);
        let content = practice_content_for_mistake(mistake_type);
        let expected_effect = expected_effect_for_mistake(mistake_type);

        let mut plan = Self::new(
            growth_model.space_id,
            Some(growth_model.namespace_id),
            PracticePlanSource::GrowthModel(growth_model.id),
            "Tomorrow dictation practice",
            content,
            Some(&expected_effect),
        );
        plan.target_pattern = Some(pattern.pattern.clone());
        plan.evidence_ids = focused_evidence;
        plan.generation_trace_id = Some(generation_trace_id);
        plan.target_growth_model_id = Some(growth_model.id);

        PracticePlanGeneration::Plan(Box::new(plan))
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
            target_pattern: None,
            content: content.into(),
            expected_effect: expected_effect.map(str::to_string),
            evidence_ids: Vec::new(),
            generation_trace_id: None,
            target_growth_model_id: None,
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

fn growth_model_evidence_gap(
    growth_model: &GrowthModel,
    generation_trace_id: TraceId,
    evidence_ids: Vec<EvidenceId>,
) -> PracticePlanGeneration {
    PracticePlanGeneration::EvidenceGap(PracticePlanEvidenceGap {
        space_id: growth_model.space_id,
        namespace_id: growth_model.namespace_id,
        growth_model_id: growth_model.id,
        reason: "needs more confirmed attempts before targeting a recurring pattern".to_string(),
        evidence_ids,
        generation_trace_id,
    })
}

fn dictation_mistake_type(pattern: &str) -> &str {
    pattern
        .strip_prefix("repeated dictation mistake type: ")
        .unwrap_or(pattern)
}

fn practice_content_for_mistake(mistake_type: &str) -> String {
    match mistake_type {
        "wrong_character" => "Tomorrow practice: 10-minute daily dictation for wrong character mistakes. Use a small word list from recent misses: listen, write, check the target, then rewrite each missed item once.".to_string(),
        "missing_character" => "Tomorrow practice: 10-minute daily dictation for missing character mistakes. Use a small word list from recent misses: listen, write every character, check, then rewrite the missed item.".to_string(),
        "extra_character" => "Tomorrow practice: 10-minute daily dictation for extra character mistakes. Use a small word list from recent misses: listen, write only the target characters, check, then rewrite once.".to_string(),
        "missing_letter" => "Tomorrow practice: 10-minute spelling practice for missing letter mistakes. Spend 10 minutes on recent spelling attempt misses: say the word, chunk it, spell it slowly, check, then rewrite the missed letter correctly.".to_string(),
        "extra_letter" => "Tomorrow practice: 10-minute spelling practice for extra letter mistakes. For each recent spelling attempt, chunk the word, spell it slowly, check the target, then rewrite without the extra letter.".to_string(),
        "letter_order_error" => "Tomorrow practice: 10-minute spelling practice for letter order mistakes. Chunk each recent word, spell each chunk slowly, check the order, then rewrite the full word once.".to_string(),
        "double_letter_error" => "Tomorrow practice: 10-minute spelling practice for double-letter mistakes. Mark the repeated letters in each word, spell it slowly, check, then rewrite it twice.".to_string(),
        "capitalization_error" => "Tomorrow practice: 10-minute daily dictation for capitalization mistakes. Copy each recent item once, mark the capital letters, then write it again from memory.".to_string(),
        "spacing_error" => "Tomorrow practice: 10-minute daily dictation for spacing mistakes. Read each recent item aloud, mark word breaks, write it, check, then rewrite with stable spacing.".to_string(),
        "punctuation_error" => "Tomorrow practice: 10-minute sentence dictation for punctuation mistakes. Listen or read once, write the sentence, place punctuation deliberately, then check and rewrite.".to_string(),
        "missing_word" => "Tomorrow practice: 10-minute sentence dictation for missing word mistakes. Listen, write the sentence, underline each word you heard, check, then rewrite the full sentence.".to_string(),
        "extra_word" => "Tomorrow practice: 10-minute sentence dictation for extra word mistakes. Listen, write the sentence, count the target words, check, then rewrite only the target sentence.".to_string(),
        "word_order_error" => "Tomorrow practice: 10-minute sentence dictation for word order mistakes. First listen, arrange the words in order, write the sentence, check, then rewrite from memory.".to_string(),
        _ => format!(
            "Tomorrow practice: 10-minute daily dictation for {mistake_type}. Use recent missed items, write each one slowly, check the target, then correct and rewrite it."
        ),
    }
}

fn expected_effect_for_mistake(mistake_type: &str) -> String {
    match mistake_type {
        "missing_letter" => {
            "Reduce missing-letter spelling mistakes in the next dictation attempt.".to_string()
        }
        "extra_letter" => {
            "Reduce extra-letter spelling mistakes in the next dictation attempt.".to_string()
        }
        "letter_order_error" => {
            "Improve letter order stability in the next dictation attempt.".to_string()
        }
        "double_letter_error" => {
            "Improve double-letter spelling stability in the next dictation attempt.".to_string()
        }
        "capitalization_error" => {
            "Improve capitalization accuracy in the next dictation attempt.".to_string()
        }
        "spacing_error" => "Improve spacing accuracy in the next dictation attempt.".to_string(),
        "punctuation_error" => {
            "Improve punctuation accuracy in the next dictation attempt.".to_string()
        }
        "wrong_character" => {
            "Reduce wrong-character mistakes in the next dictation attempt.".to_string()
        }
        "missing_character" => {
            "Reduce missing-character mistakes in the next dictation attempt.".to_string()
        }
        "extra_character" => {
            "Reduce extra-character mistakes in the next dictation attempt.".to_string()
        }
        "missing_word" => {
            "Reduce missing-word sentence dictation mistakes in the next attempt.".to_string()
        }
        "extra_word" => {
            "Reduce extra-word sentence dictation mistakes in the next attempt.".to_string()
        }
        "word_order_error" => {
            "Improve sentence word order stability in the next attempt.".to_string()
        }
        _ => format!("Improve {mistake_type} accuracy in the next dictation attempt."),
    }
}
