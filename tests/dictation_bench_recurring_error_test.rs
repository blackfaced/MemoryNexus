use memorynexus::eval::{evaluate_dictation_bench_recurring_errors, load_dictation_bench_fixtures};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dictation_bench")
}

#[test]
fn dictation_bench_recurring_error_report_scores_expected_patterns() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_recurring_errors(&fixtures);

    assert_eq!(report.total_fixture_count, 5);
    assert_eq!(report.total_expected_pattern_count, 5);
    assert_eq!(report.passed_pattern_count, 5);
    assert_eq!(report.failed_pattern_count, 0);
    assert_eq!(report.fixture_results.len(), 5);

    let recurring = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.spelling-letter-order.v1")
        .expect("letter order fixture should be reported");
    assert!(recurring.passed);
    assert_eq!(recurring.pattern_results.len(), 1);
    assert_eq!(
        recurring.pattern_results[0].expected_mistake_type,
        "letter_order_error"
    );
    assert_eq!(recurring.pattern_results[0].recurrence, "recurring");
    assert_eq!(
        recurring.pattern_results[0].detected_mistake_types,
        vec!["letter_order_error"]
    );
    assert!(recurring.pattern_results[0].passed);

    let improving = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.en.multi-day-missing-letter-improves.v1")
        .expect("improving fixture should be reported");
    assert!(improving.pattern_results[0].passed);
    assert_eq!(improving.pattern_results[0].recurrence, "improving");
    assert_eq!(
        improving.pattern_results[0].detected_mistake_types,
        vec!["missing_letter"]
    );

    let insufficient = report
        .fixture_results
        .iter()
        .find(|fixture| fixture.fixture_id == "dictbench.zh.insufficient-evidence-unclassified.v1")
        .expect("insufficient evidence fixture should be reported");
    assert!(insufficient.pattern_results[0].passed);
    assert_eq!(
        insufficient.pattern_results[0].recurrence,
        "insufficient_evidence"
    );
    assert_eq!(
        insufficient.pattern_results[0].expected_mistake_type,
        "unclassified"
    );
    assert_eq!(
        insufficient.pattern_results[0].detected_mistake_types,
        vec!["wrong_character"]
    );
    assert!(insufficient.pattern_results[0]
        .notes
        .iter()
        .any(|note| note.contains("not scored as recurring")));
}

#[test]
fn dictation_bench_report_is_json_serializable_for_eval_cli() {
    let fixtures = load_dictation_bench_fixtures(&fixture_dir()).expect("fixtures should load");
    let report = evaluate_dictation_bench_recurring_errors(&fixtures);
    let json = serde_json::to_value(&report).expect("report should serialize");

    assert_eq!(json["total_fixture_count"], 5);
    assert_eq!(json["failed_pattern_count"], 0);
    assert!(
        !json["fixture_results"][0]["pattern_results"][0]["expected_mistake_type"]
            .as_str()
            .expect("expected mistake type should be present")
            .is_empty()
    );
}
