use chrono::{TimeZone, Utc};
use memorynexus::domain::growth_model::{aggregate_growth_model, EvidenceId, GrowthEvidenceRecord};
use memorynexus::domain::practice_plan::{PracticePlanGeneration, PracticePlanSource};
use uuid::Uuid;

fn repeated_growth_model(
    signal_label: &str,
) -> (
    memorynexus::domain::growth_model::GrowthModel,
    Vec<EvidenceId>,
) {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let evidence_ids = vec![
        EvidenceId::Trace(Uuid::new_v4()),
        EvidenceId::FeedbackLoop(Uuid::new_v4()),
    ];

    let aggregation = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: evidence_ids[0],
                signal_labels: vec![signal_label.to_string()],
                explanation: None,
            },
            GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: evidence_ids[1],
                signal_labels: vec![signal_label.to_string()],
                explanation: None,
            },
        ],
        Utc.with_ymd_and_hms(2026, 6, 25, 21, 0, 0).unwrap(),
    );

    (aggregation.model, evidence_ids)
}

#[test]
fn chinese_wrong_character_becomes_short_tomorrow_dictation_practice() {
    let (growth_model, evidence_ids) = repeated_growth_model("wrong_character");
    let generation_trace_id = Uuid::new_v4();

    let plan = PracticePlanGeneration::from_growth_model(&growth_model, generation_trace_id)
        .into_plan()
        .expect("repeated wrong_character evidence should produce a plan");

    assert_eq!(plan.purpose, "Tomorrow dictation practice");
    assert_eq!(
        plan.source,
        PracticePlanSource::GrowthModel(growth_model.id)
    );
    assert_eq!(plan.evidence_ids, evidence_ids);
    assert_eq!(plan.generation_trace_id, Some(generation_trace_id));
    assert_eq!(plan.target_growth_model_id, Some(growth_model.id));
    assert!(plan.content.contains("10-minute daily dictation"));
    assert!(plan.content.contains("word list"));
    assert!(plan.content.contains("wrong character"));
    assert!(plan.content.contains("listen"));
    assert!(plan.content.contains("write"));
    assert!(plan.content.contains("check"));
    assert!(plan.content.contains("rewrite"));
    assert!(!plan.content.contains("GrowthModel"));
    assert!(!plan.content.contains("EvidenceBacked"));
}

#[test]
fn chinese_missing_character_asks_for_recent_misses_without_broad_curriculum() {
    let (growth_model, _) = repeated_growth_model("missing_character");
    let generation_trace_id = Uuid::new_v4();

    let plan = PracticePlanGeneration::from_growth_model(&growth_model, generation_trace_id)
        .into_plan()
        .expect("repeated missing_character evidence should produce a plan");

    assert!(plan.content.contains("10-minute daily dictation"));
    assert!(plan.content.contains("missing character"));
    assert!(plan.content.contains("small word list"));
    assert!(plan.content.contains("recent misses"));
    assert!(!plan.content.contains("curriculum"));
}

#[test]
fn english_missing_letter_becomes_ten_minute_spelling_practice() {
    let (growth_model, evidence_ids) = repeated_growth_model("missing_letter");
    let generation_trace_id = Uuid::new_v4();

    let plan = PracticePlanGeneration::from_growth_model(&growth_model, generation_trace_id)
        .into_plan()
        .expect("repeated missing_letter evidence should produce a plan");

    assert_eq!(plan.evidence_ids, evidence_ids);
    assert!(plan.content.contains("10-minute spelling practice"));
    assert!(plan.content.contains("spelling attempt"));
    assert!(plan.content.contains("missing letter"));
    assert!(plan.content.contains("spell it slowly"));
    assert!(plan.content.contains("chunk"));
}

#[test]
fn english_word_order_error_becomes_sentence_dictation_not_grammar_lesson() {
    let (growth_model, _) = repeated_growth_model("word_order_error");
    let generation_trace_id = Uuid::new_v4();

    let plan = PracticePlanGeneration::from_growth_model(&growth_model, generation_trace_id)
        .into_plan()
        .expect("repeated word_order_error evidence should produce a plan");

    assert!(plan.content.contains("10-minute sentence dictation"));
    assert!(plan.content.contains("word order"));
    assert!(plan.content.contains("listen"));
    assert!(plan.content.contains("write"));
    assert!(!plan.content.contains("grammar lesson"));
}

#[test]
fn sparse_growth_model_keeps_safe_evidence_gap() {
    let space_id = Uuid::new_v4();
    let namespace_id = Uuid::new_v4();
    let generation_trace_id = Uuid::new_v4();
    let aggregation = aggregate_growth_model(
        space_id,
        namespace_id,
        vec![GrowthEvidenceRecord {
            space_id,
            namespace_id,
            evidence_id: EvidenceId::Trace(Uuid::new_v4()),
            signal_labels: vec!["wrong_character".to_string()],
            explanation: None,
        }],
        Utc.with_ymd_and_hms(2026, 6, 25, 21, 0, 0).unwrap(),
    );

    match PracticePlanGeneration::from_growth_model(&aggregation.model, generation_trace_id) {
        PracticePlanGeneration::EvidenceGap(gap) => {
            assert_eq!(gap.space_id, space_id);
            assert_eq!(gap.namespace_id, namespace_id);
            assert_eq!(gap.generation_trace_id, generation_trace_id);
            assert_eq!(gap.evidence_ids, Vec::<EvidenceId>::new());
        }
        PracticePlanGeneration::Plan(plan) => {
            panic!("sparse evidence must not invent tomorrow practice: {plan:?}");
        }
    }
}
