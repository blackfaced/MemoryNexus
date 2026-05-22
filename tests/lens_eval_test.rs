use memorynexus::eval::{evaluate_cases, lens_eval_fixtures};

#[test]
fn lens_eval_fixtures_cover_three_deterministic_scenarios() {
    let cases = lens_eval_fixtures();

    assert!(cases.len() >= 3);
    assert!(cases
        .iter()
        .any(|case| case.lens_strategy == "project_context"));
    assert!(cases.iter().any(|case| case.lens_strategy == "risk_review"));
    assert!(cases
        .iter()
        .any(|case| case.lens_strategy == "learning_review"));
    assert!(cases.iter().all(|case| !case.requires_provider));
}

#[test]
fn lens_eval_scores_retrieval_citations_summary_and_provider_fallback() {
    let report = evaluate_cases(&lens_eval_fixtures());

    assert_eq!(report.total_cases, 3);
    assert_eq!(report.passed_cases, 3);
    assert!(report.overall_score >= 0.95);
    assert!(report.results.iter().all(|result| result.passed));
    assert!(report
        .results
        .iter()
        .all(|result| result.dimension_scores.retrieval_relevance >= 1.0));
    assert!(report
        .results
        .iter()
        .all(|result| result.dimension_scores.citation_correctness >= 1.0));
    assert!(report
        .results
        .iter()
        .all(|result| result.dimension_scores.provider_fallback >= 1.0));
}

#[test]
fn lens_eval_has_a_risk_case_that_requires_unresolved_contradictions() {
    let report = evaluate_cases(&lens_eval_fixtures());
    let risk = report
        .results
        .iter()
        .find(|result| result.case_id == "risk_review_contradiction")
        .expect("risk fixture should exist");

    assert_eq!(risk.dimension_scores.contradiction_signal, 1.0);
}
