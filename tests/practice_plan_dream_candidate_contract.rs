use memorynexus::domain::dream_candidate::{
    DreamCandidate, DreamCandidateEffectiveness, DreamCandidatePurpose, DreamCandidateSource,
    DreamCandidateStatus,
};
use memorynexus::domain::practice_plan::{PracticePlan, PracticePlanSource, PracticePlanStatus};
use serde_json::{json, Value};
use uuid::Uuid;

#[test]
fn dream_candidate_serializes_contract_fields_with_snake_case_values() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let sleep_cycle_id = Uuid::new_v4();
    let consolidation_result_id = Uuid::new_v4();

    let candidate = DreamCandidate::new(
        space_id,
        Some(namespace_id),
        DreamCandidateSource {
            sleep_cycle_id,
            consolidation_result_id,
        },
        DreamCandidatePurpose::PracticeGeneration,
        "Practice similar-sounding characters for 10 minutes.",
        Some("Reduce homophone substitutions in tomorrow's dictation."),
    );

    let value = serde_json::to_value(&candidate).unwrap();

    assert_eq!(value["space_id"], json!(space_id));
    assert_eq!(value["namespace_id"], json!(namespace_id));
    assert_eq!(value["source_sleep_cycle_id"], json!(sleep_cycle_id));
    assert_eq!(
        value["source_consolidation_result_id"],
        json!(consolidation_result_id)
    );
    assert_eq!(value["purpose"], "practice_generation");
    assert_eq!(
        value["content"],
        "Practice similar-sounding characters for 10 minutes."
    );
    assert_eq!(
        value["expected_effect"],
        "Reduce homophone substitutions in tomorrow's dictation."
    );
    assert_eq!(value["status"], "proposed");

    let round_trip: DreamCandidate = serde_json::from_value(value).unwrap();
    assert_eq!(round_trip.status, DreamCandidateStatus::Proposed);
    assert_eq!(
        round_trip.purpose,
        DreamCandidatePurpose::PracticeGeneration
    );
}

#[test]
fn dream_candidate_tracks_selected_executed_and_evaluated_lifecycle() {
    let source = DreamCandidateSource {
        sleep_cycle_id: Uuid::new_v4(),
        consolidation_result_id: Uuid::new_v4(),
    };
    let mut candidate = DreamCandidate::new(
        Uuid::new_v4(),
        None,
        source,
        DreamCandidatePurpose::ReviewQuestion,
        "Which spelling mistake repeated most this week?",
        Some("Surface the user's own error pattern before planning."),
    );
    let selected_at = chrono::Utc::now();
    let execution_trace_id = Uuid::new_v4();
    let evaluation_trace_id = Uuid::new_v4();

    candidate.select(selected_at);
    assert_eq!(candidate.status, DreamCandidateStatus::Selected);
    assert_eq!(candidate.selected_at, Some(selected_at));

    candidate.record_execution(execution_trace_id);
    assert_eq!(candidate.status, DreamCandidateStatus::Executed);
    assert_eq!(candidate.executed_trace_ids, vec![execution_trace_id]);

    candidate.record_evaluation(
        evaluation_trace_id,
        DreamCandidateEffectiveness::Useful,
        "Fewer homophone mistakes in the next attempt.",
    );
    assert_eq!(candidate.status, DreamCandidateStatus::Evaluated);
    assert_eq!(candidate.evaluation_trace_ids, vec![evaluation_trace_id]);
    assert_eq!(
        candidate.evaluation_result.as_ref().unwrap().effectiveness,
        DreamCandidateEffectiveness::Useful
    );
}

#[test]
fn practice_plan_can_reference_dream_candidate_or_consolidation_result() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let dream_candidate_id = Uuid::new_v4();
    let consolidation_result_id = Uuid::new_v4();

    let from_candidate = PracticePlan::from_dream_candidate(
        space_id,
        Some(namespace_id),
        dream_candidate_id,
        "Tomorrow dictation warmup",
        "Review six words with homophone risk.",
        Some("Improve stability on repeated mistake patterns."),
    );

    assert_eq!(
        from_candidate.source,
        PracticePlanSource::DreamCandidate(dream_candidate_id)
    );
    assert_eq!(from_candidate.status, PracticePlanStatus::Selected);

    let from_consolidation = PracticePlan::from_consolidation_result(
        space_id,
        Some(namespace_id),
        consolidation_result_id,
        "Manual follow-up",
        "Ask one review question before the next session.",
        Some("Clarify whether the pattern still needs practice."),
    );

    assert_eq!(
        from_consolidation.source,
        PracticePlanSource::ConsolidationResult(consolidation_result_id)
    );

    let value = serde_json::to_value(&from_candidate).unwrap();
    assert_eq!(value["source"]["type"], "dream_candidate");
    assert_eq!(value["status"], "selected");
}

#[test]
fn practice_plan_tracks_selected_executed_and_evaluated_states() {
    let mut plan = PracticePlan::from_consolidation_result(
        Uuid::new_v4(),
        None,
        Uuid::new_v4(),
        "Ten-minute spelling practice",
        "Practice double-letter words from the last three attempts.",
        Some("Reduce double-letter errors."),
    );
    let selected_at = chrono::Utc::now();
    let executed_trace_id = Uuid::new_v4();
    let evaluated_trace_id = Uuid::new_v4();

    plan.mark_selected(selected_at);
    assert_eq!(plan.status, PracticePlanStatus::Selected);
    assert_eq!(plan.selected_at, Some(selected_at));

    plan.record_execution(executed_trace_id);
    assert_eq!(plan.status, PracticePlanStatus::Executed);
    assert_eq!(plan.executed_trace_ids, vec![executed_trace_id]);

    plan.record_evaluation(
        evaluated_trace_id,
        DreamCandidateEffectiveness::Neutral,
        "The plan was completed, but the next attempt had mixed results.",
    );
    assert_eq!(plan.status, PracticePlanStatus::Evaluated);
    assert_eq!(plan.evaluation_trace_ids, vec![evaluated_trace_id]);
    assert_eq!(
        plan.evaluation_result.as_ref().unwrap().effectiveness,
        DreamCandidateEffectiveness::Neutral
    );
}

#[test]
fn knowledge_context_dream_candidate_cites_external_context_without_plan_or_model_mutation() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let sleep_cycle_id = Uuid::new_v4();
    let consolidation_result_id = Uuid::new_v4();
    let knowledge_context_id = Uuid::new_v4();

    let mut candidate = DreamCandidate::new(
        space_id,
        Some(namespace_id),
        DreamCandidateSource {
            sleep_cycle_id,
            consolidation_result_id,
        },
        DreamCandidatePurpose::ContradictionExploration,
        "Review whether the external rubric conflicts with the user's recent attempts.",
        Some("Turn external context into a hypothesis instead of overwriting local evidence."),
    );
    candidate.cite_knowledge_context(knowledge_context_id);

    let value = serde_json::to_value(&candidate).unwrap();

    assert_eq!(
        value["source_knowledge_context_ids"],
        json!([knowledge_context_id])
    );
    assert_eq!(value["target_growth_model_id"], Value::Null);
    assert_eq!(candidate.target_growth_model_id, None);

    let maybe_plan = PracticePlan::try_from_knowledge_context_candidate(&candidate);
    assert!(maybe_plan.is_none());
}
