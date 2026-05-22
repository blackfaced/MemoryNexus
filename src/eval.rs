//! Deterministic Lens quality evaluation fixtures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensEvalCase {
    pub id: String,
    pub name: String,
    pub lens_strategy: String,
    pub query: String,
    pub requires_provider: bool,
    pub observed: LensEvalObserved,
    pub expected: LensEvalExpected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensEvalObserved {
    pub retrieved_memory_ids: Vec<String>,
    pub cited_memory_ids: Vec<String>,
    pub summary: String,
    pub unresolved_contradiction_count: usize,
    pub profile_source_memory_ids: Vec<String>,
    pub summary_source: String,
    pub summary_fallback_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensEvalExpected {
    pub relevant_memory_ids: Vec<String>,
    pub required_summary_terms: Vec<String>,
    pub active_memory_ids: Vec<String>,
    pub deprioritized_memory_ids: Vec<String>,
    pub requires_unresolved_contradiction: bool,
    pub summary_source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensEvalReport {
    pub total_cases: usize,
    pub passed_cases: usize,
    pub overall_score: f64,
    pub provider_backed_cases: usize,
    pub deterministic_cases: usize,
    pub results: Vec<LensEvalResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensEvalResult {
    pub case_id: String,
    pub name: String,
    pub lens_strategy: String,
    pub passed: bool,
    pub score: f64,
    pub dimension_scores: LensEvalDimensionScores,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LensEvalDimensionScores {
    pub retrieval_relevance: f64,
    pub citation_correctness: f64,
    pub summary_faithfulness: f64,
    pub contradiction_signal: f64,
    pub profile_projection_stability: f64,
    pub provider_fallback: f64,
}

impl LensEvalDimensionScores {
    fn average(self) -> f64 {
        (self.retrieval_relevance
            + self.citation_correctness
            + self.summary_faithfulness
            + self.contradiction_signal
            + self.profile_projection_stability
            + self.provider_fallback)
            / 6.0
    }
}

pub fn lens_eval_fixtures() -> Vec<LensEvalCase> {
    vec![
        LensEvalCase {
            id: "project_context_rust_first".to_string(),
            name: "Project Context keeps Rust-first cognitive direction".to_string(),
            lens_strategy: "project_context".to_string(),
            query: "Summarize MemoryNexus current project direction".to_string(),
            requires_provider: false,
            observed: LensEvalObserved {
                retrieved_memory_ids: vec!["mem_project_direction".to_string()],
                cited_memory_ids: vec!["mem_project_direction".to_string()],
                summary: "MemoryNexus is a Rust-first cognitive substrate where Cognitive Space owns memory and Lens Run interprets it.".to_string(),
                unresolved_contradiction_count: 0,
                profile_source_memory_ids: vec!["mem_project_direction".to_string()],
                summary_source: "deterministic".to_string(),
                summary_fallback_reason: Some("summary provider not configured".to_string()),
            },
            expected: LensEvalExpected {
                relevant_memory_ids: vec!["mem_project_direction".to_string()],
                required_summary_terms: vec![
                    "Rust-first".to_string(),
                    "Cognitive Space".to_string(),
                    "Lens".to_string(),
                ],
                active_memory_ids: vec!["mem_project_direction".to_string()],
                deprioritized_memory_ids: vec![],
                requires_unresolved_contradiction: false,
                summary_source: "deterministic".to_string(),
            },
        },
        LensEvalCase {
            id: "risk_review_contradiction".to_string(),
            name: "Risk Review surfaces unresolved contradiction".to_string(),
            lens_strategy: "risk_review".to_string(),
            query: "Review release risk contradictions".to_string(),
            requires_provider: false,
            observed: LensEvalObserved {
                retrieved_memory_ids: vec![
                    "mem_ship_fast".to_string(),
                    "mem_release_risk".to_string(),
                ],
                cited_memory_ids: vec![
                    "mem_ship_fast".to_string(),
                    "mem_release_risk".to_string(),
                ],
                summary: "The release plan contains an unresolved contradiction between shipping fast and avoiding risky releases.".to_string(),
                unresolved_contradiction_count: 1,
                profile_source_memory_ids: vec![
                    "mem_ship_fast".to_string(),
                    "mem_release_risk".to_string(),
                ],
                summary_source: "deterministic".to_string(),
                summary_fallback_reason: Some("summary provider not configured".to_string()),
            },
            expected: LensEvalExpected {
                relevant_memory_ids: vec![
                    "mem_ship_fast".to_string(),
                    "mem_release_risk".to_string(),
                ],
                required_summary_terms: vec![
                    "unresolved contradiction".to_string(),
                    "shipping fast".to_string(),
                    "risky releases".to_string(),
                ],
                active_memory_ids: vec![
                    "mem_ship_fast".to_string(),
                    "mem_release_risk".to_string(),
                ],
                deprioritized_memory_ids: vec![],
                requires_unresolved_contradiction: true,
                summary_source: "deterministic".to_string(),
            },
        },
        LensEvalCase {
            id: "learning_review_profile_stability".to_string(),
            name: "Learning Review ignores deprioritized scratch memory".to_string(),
            lens_strategy: "learning_review".to_string(),
            query: "Find the next learning step".to_string(),
            requires_provider: false,
            observed: LensEvalObserved {
                retrieved_memory_ids: vec!["mem_rust_practice".to_string()],
                cited_memory_ids: vec!["mem_rust_practice".to_string()],
                summary: "The next learning step is a small Rust practice loop focused on ownership and tests.".to_string(),
                unresolved_contradiction_count: 0,
                profile_source_memory_ids: vec!["mem_rust_practice".to_string()],
                summary_source: "deterministic".to_string(),
                summary_fallback_reason: Some("summary provider not configured".to_string()),
            },
            expected: LensEvalExpected {
                relevant_memory_ids: vec!["mem_rust_practice".to_string()],
                required_summary_terms: vec![
                    "Rust".to_string(),
                    "practice".to_string(),
                    "tests".to_string(),
                ],
                active_memory_ids: vec!["mem_rust_practice".to_string()],
                deprioritized_memory_ids: vec!["mem_learning_scratch".to_string()],
                requires_unresolved_contradiction: false,
                summary_source: "deterministic".to_string(),
            },
        },
    ]
}

pub fn evaluate_cases(cases: &[LensEvalCase]) -> LensEvalReport {
    let results = cases.iter().map(evaluate_case).collect::<Vec<_>>();
    let total_cases = results.len();
    let passed_cases = results.iter().filter(|result| result.passed).count();
    let overall_score = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|result| result.score).sum::<f64>() / results.len() as f64
    };

    LensEvalReport {
        total_cases,
        passed_cases,
        overall_score,
        provider_backed_cases: cases.iter().filter(|case| case.requires_provider).count(),
        deterministic_cases: cases.iter().filter(|case| !case.requires_provider).count(),
        results,
    }
}

fn evaluate_case(case: &LensEvalCase) -> LensEvalResult {
    let dimension_scores = LensEvalDimensionScores {
        retrieval_relevance: expected_coverage(
            &case.expected.relevant_memory_ids,
            &case.observed.retrieved_memory_ids,
        ),
        citation_correctness: citation_correctness(case),
        summary_faithfulness: summary_faithfulness(case),
        contradiction_signal: contradiction_signal(case),
        profile_projection_stability: profile_projection_stability(case),
        provider_fallback: provider_fallback(case),
    };
    let score = dimension_scores.average();
    let passed = score >= 0.95;
    let mut notes = Vec::new();

    if case.requires_provider {
        notes.push("provider-backed evaluation placeholder; not run by default".to_string());
    }
    if case.expected.requires_unresolved_contradiction {
        notes.push("expects at least one unresolved contradiction".to_string());
    }
    if !case.expected.deprioritized_memory_ids.is_empty() {
        notes.push("expects deprioritized memories to stay out of profile sources".to_string());
    }

    LensEvalResult {
        case_id: case.id.clone(),
        name: case.name.clone(),
        lens_strategy: case.lens_strategy.clone(),
        passed,
        score,
        dimension_scores,
        notes,
    }
}

fn expected_coverage(expected: &[String], observed: &[String]) -> f64 {
    if expected.is_empty() {
        return 1.0;
    }

    let matched = expected
        .iter()
        .filter(|id| observed.iter().any(|observed_id| observed_id == *id))
        .count();
    matched as f64 / expected.len() as f64
}

fn citation_correctness(case: &LensEvalCase) -> f64 {
    let citations_from_retrieved = case
        .observed
        .cited_memory_ids
        .iter()
        .all(|id| case.observed.retrieved_memory_ids.contains(id));
    if !citations_from_retrieved {
        return 0.0;
    }

    expected_coverage(
        &case.expected.relevant_memory_ids,
        &case.observed.cited_memory_ids,
    )
}

fn summary_faithfulness(case: &LensEvalCase) -> f64 {
    let summary = case.observed.summary.to_lowercase();
    let required_terms = &case.expected.required_summary_terms;
    if required_terms.is_empty() {
        return 1.0;
    }

    let matched = required_terms
        .iter()
        .filter(|term| summary.contains(&term.to_lowercase()))
        .count();
    matched as f64 / required_terms.len() as f64
}

fn contradiction_signal(case: &LensEvalCase) -> f64 {
    if !case.expected.requires_unresolved_contradiction {
        return 1.0;
    }

    if case.observed.unresolved_contradiction_count > 0 {
        1.0
    } else {
        0.0
    }
}

fn profile_projection_stability(case: &LensEvalCase) -> f64 {
    let active_score = expected_coverage(
        &case.expected.active_memory_ids,
        &case.observed.profile_source_memory_ids,
    );
    let deprioritized_absent = case
        .expected
        .deprioritized_memory_ids
        .iter()
        .all(|id| !case.observed.profile_source_memory_ids.contains(id));

    if deprioritized_absent {
        active_score
    } else {
        0.0
    }
}

fn provider_fallback(case: &LensEvalCase) -> f64 {
    if case.observed.summary_source != case.expected.summary_source {
        return 0.0;
    }

    if case.requires_provider {
        return 1.0;
    }

    if case.observed.summary_fallback_reason.is_some() {
        1.0
    } else {
        0.0
    }
}
