use memorynexus::eval::{
    evaluate_dictation_bench_improvement, load_dictation_bench_fixtures,
    DictationBenchImprovementLabel,
};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dictation_bench")
}

#[test]
fn multi_day_fixture_reports_improved_with_ordered_timeline() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_improvement(&fixtures);

    assert_eq!(report.total_fixture_count, 5);
    assert_eq!(report.improved_count, 1);
    assert_eq!(report.repeated_count, 3);
    assert_eq!(report.skipped_count, 0);
    assert_eq!(report.insufficient_evidence_count, 1);

    let fixture = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.multi-day-missing-letter-improves.v1")
        .expect("multi-day fixture should be reported");
    let pattern = &fixture.pattern_results[0];

    assert_eq!(pattern.label, DictationBenchImprovementLabel::Improved);
    assert!(pattern.passed);
    assert_eq!(pattern.expected_mistake_type, "missing_letter");
    assert_eq!(pattern.attempt_timeline.len(), 3);
    assert_eq!(
        pattern
            .attempt_timeline
            .iter()
            .map(|entry| entry.attempt_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "dictbench-attempt-en-improve-day1",
            "dictbench-attempt-en-improve-day2",
            "dictbench-attempt-en-improve-day3",
        ]
    );
    assert_eq!(
        pattern.attempt_timeline[0].submitted_at.as_deref(),
        Some("2026-06-24T20:00:00Z")
    );
    assert_eq!(
        pattern.attempt_timeline[0].detected_mistake_types,
        vec!["missing_letter"]
    );
    assert!(!pattern.attempt_timeline[0].correct);
    assert_eq!(
        pattern.attempt_timeline[2].detected_mistake_types,
        vec!["correct"]
    );
    assert!(pattern.attempt_timeline[2].correct);
}

#[test]
fn recurring_fixture_without_later_correction_reports_repeated() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_improvement(&fixtures);

    let fixture = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.spelling-letter-order.v1")
        .expect("recurring fixture should be reported");
    let pattern = &fixture.pattern_results[0];

    assert_eq!(pattern.label, DictationBenchImprovementLabel::Repeated);
    assert!(pattern.passed);
    assert_eq!(pattern.attempt_timeline.len(), 2);
    assert!(pattern.attempt_timeline.iter().all(|entry| !entry.correct));
}

#[test]
fn sparse_fixture_reports_insufficient_evidence() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_improvement(&fixtures);

    let fixture = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.zh.insufficient-evidence-unclassified.v1")
        .expect("insufficient fixture should be reported");
    let pattern = &fixture.pattern_results[0];

    assert_eq!(
        pattern.label,
        DictationBenchImprovementLabel::InsufficientEvidence
    );
    assert!(pattern.passed);
    assert!(pattern
        .notes
        .iter()
        .any(|note| note.contains("not enough deterministic evidence")));
}

#[test]
fn missing_relevant_attempt_reports_skipped() {
    let mut fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let fixture = fixtures
        .iter_mut()
        .find(|fixture| fixture.id == "dictbench.en.spelling-letter-order.v1")
        .expect("fixture should exist");
    fixture
        .attempts
        .retain(|attempt| attempt.id != "dictbench-attempt-en-spelling-002");

    let report = evaluate_dictation_bench_improvement(&fixtures);
    let fixture = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.spelling-letter-order.v1")
        .expect("mutated fixture should be reported");
    let pattern = &fixture.pattern_results[0];

    assert_eq!(pattern.label, DictationBenchImprovementLabel::Skipped);
    assert!(!pattern.passed);
    assert!(pattern
        .notes
        .iter()
        .any(|note| note.contains("missing expected attempt")));
}

#[test]
fn improvement_metrics_remain_local_provider_free_and_zero_cost() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_improvement(&fixtures);

    assert!(report.metrics.total_evidence_count > 0);
    assert_eq!(
        report.metrics.total_evidence_count,
        report.metrics.total_trace_count
    );
    assert_eq!(
        report.metrics.local_evidence_count,
        report.metrics.total_evidence_count
    );
    assert_eq!(report.metrics.local_ratio, 1.0);
    assert!(report.metrics.total_latency_ms > 0);
    assert!(report.metrics.average_latency_ms > 0.0);
    assert_eq!(report.metrics.estimated_cost, 0.0);
    assert_eq!(report.metrics.cost_currency, "USD");
    assert!(report
        .metrics
        .notes
        .iter()
        .any(|note| note.contains("simulated deterministic latency")));
}
