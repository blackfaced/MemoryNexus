use memorynexus::eval::{
    evaluate_dictation_bench_next_practice, load_dictation_bench_fixtures,
    DictationBenchNextPracticeGeneratedOutcome, DictationBenchNextPracticeQuality,
};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dictation_bench")
}

#[test]
fn dictation_bench_next_practice_report_scores_plan_and_evidence_gap_quality() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_next_practice(&fixtures);

    assert_eq!(report.total_fixture_count, 5);
    assert_eq!(report.useful_count, 5);
    assert_eq!(report.neutral_count, 0);
    assert_eq!(report.bad_count, 0);
    assert_eq!(report.fixture_results.len(), 5);

    let letter_order = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.spelling-letter-order.v1")
        .expect("letter order fixture should be reported");
    assert_eq!(letter_order.expected_outcome, "plan");
    assert!(matches!(
        letter_order.generated_outcome,
        DictationBenchNextPracticeGeneratedOutcome::Plan
    ));
    assert_eq!(
        letter_order.quality,
        DictationBenchNextPracticeQuality::Useful
    );
    assert_eq!(
        letter_order.expected_target_mistake_types,
        vec!["letter_order_error"]
    );
    assert!(letter_order
        .generated_summary
        .contains("letter_order_error"));
    assert!(letter_order.generated_summary.contains("10-minute"));
    assert_eq!(letter_order.evidence_count, 2);
    assert!(!letter_order.evidence_ids.is_empty());

    let sentence = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.sentence-missing-word.v1")
        .expect("missing word fixture should be reported");
    assert_eq!(sentence.expected_target_mistake_types, vec!["missing_word"]);
    assert!(sentence.generated_summary.contains("missing_word"));
    assert!(sentence.generated_summary.contains("sentence dictation"));
    assert_eq!(sentence.quality, DictationBenchNextPracticeQuality::Useful);

    let insufficient = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.zh.insufficient-evidence-unclassified.v1")
        .expect("insufficient evidence fixture should be reported");
    assert_eq!(insufficient.expected_outcome, "evidence_gap");
    assert!(matches!(
        insufficient.generated_outcome,
        DictationBenchNextPracticeGeneratedOutcome::EvidenceGap
    ));
    assert_eq!(
        insufficient.quality,
        DictationBenchNextPracticeQuality::Useful
    );
    assert_eq!(
        insufficient.expected_target_mistake_types,
        Vec::<String>::new()
    );
    assert_eq!(insufficient.evidence_count, 0);
    assert!(insufficient
        .notes
        .iter()
        .any(|note| note.contains("matched expected evidence_gap")));
}

#[test]
fn dictation_bench_next_practice_report_is_json_serializable_for_eval_cli() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_next_practice(&fixtures);
    let json = serde_json::to_value(report).expect("report should serialize");

    assert_eq!(json["total_fixture_count"], 5);
    assert_eq!(json["useful_count"], 5);
    assert_eq!(json["neutral_count"], 0);
    assert_eq!(json["bad_count"], 0);
    assert!(json["fixture_results"].is_array());
    assert!(json["fixture_results"][0]["expected_outcome"].is_string());
    assert!(json["fixture_results"][0]["generated_outcome"].is_string());
    assert!(json["fixture_results"][0]["quality"].is_string());
    assert!(json["fixture_results"][0]["generated_summary"].is_string());
}

#[test]
fn dictation_bench_next_practice_report_surfaces_bad_generated_plan() {
    let mut fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let fixture = fixtures
        .iter_mut()
        .find(|fixture| fixture.id == "dictbench.en.spelling-letter-order.v1")
        .expect("letter order fixture should exist");
    fixture.expected_next_practice.outcome = "evidence_gap".to_string();
    fixture.expected_next_practice.duration_minutes = None;
    fixture.expected_next_practice.target_mistake_types = Vec::new();

    let report = evaluate_dictation_bench_next_practice(&fixtures);

    assert_eq!(report.total_fixture_count, 5);
    assert_eq!(report.bad_count, 1);
    assert_eq!(report.neutral_count, 0);
    assert_eq!(report.useful_count, 4);

    let mismatch = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.spelling-letter-order.v1")
        .expect("mismatched fixture should be reported");
    assert_eq!(mismatch.expected_outcome, "evidence_gap");
    assert!(matches!(
        mismatch.generated_outcome,
        DictationBenchNextPracticeGeneratedOutcome::Plan
    ));
    assert_eq!(mismatch.quality, DictationBenchNextPracticeQuality::Bad);
    assert!(mismatch
        .notes
        .iter()
        .any(|note| note.contains("expected evidence_gap")));
}
