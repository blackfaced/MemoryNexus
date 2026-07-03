//! Deterministic quality evaluation fixtures.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::domain::dictation::{
    build_dictation_attempt, DictationAttemptInput, DictationItemKind, DictationSource,
    DictationTaskKind, PromptItemInput, SubmittedItemInput,
};
use crate::domain::growth_model::{aggregate_growth_model, EvidenceId, GrowthEvidenceRecord};
use crate::domain::practice_plan::PracticePlanGeneration;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchFixture {
    pub id: String,
    pub namespace: String,
    pub locale: String,
    pub task_kind: String,
    pub task: DictationBenchTask,
    pub attempts: Vec<DictationBenchAttempt>,
    pub expected_mistake_patterns: Vec<DictationBenchExpectedPattern>,
    pub expected_next_practice: DictationBenchExpectedNextPractice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchTask {
    pub id: String,
    pub source: String,
    pub prompt_items: Vec<DictationBenchPromptItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchPromptItem {
    pub id: String,
    pub item_kind: String,
    pub expected_text: String,
    pub order_index: usize,
    #[serde(default)]
    pub display_text: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchAttempt {
    pub id: String,
    #[serde(default)]
    pub submitted_at: Option<String>,
    pub source: String,
    pub submitted_items: Vec<DictationBenchSubmittedItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchSubmittedItem {
    pub prompt_item_id: Option<String>,
    pub actual_text: String,
    pub order_index: usize,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchExpectedPattern {
    pub mistake_type: String,
    pub attempt_ids: Vec<String>,
    pub prompt_item_ids: Vec<String>,
    pub recurrence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchExpectedNextPractice {
    pub outcome: String,
    pub duration_minutes: Option<u16>,
    pub target_mistake_types: Vec<String>,
    pub expectations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchRecurringErrorReport {
    pub total_fixture_count: usize,
    pub total_expected_pattern_count: usize,
    pub passed_pattern_count: usize,
    pub failed_pattern_count: usize,
    pub fixture_results: Vec<DictationBenchFixtureResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchFixtureResult {
    pub fixture_id: String,
    pub namespace: String,
    pub task_kind: String,
    pub passed: bool,
    pub pattern_results: Vec<DictationBenchPatternResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchPatternResult {
    pub expected_mistake_type: String,
    pub recurrence: String,
    pub attempt_ids: Vec<String>,
    pub prompt_item_ids: Vec<String>,
    pub detected_mistake_types: Vec<String>,
    pub passed: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchNextPracticeReport {
    pub total_fixture_count: usize,
    pub useful_count: usize,
    pub neutral_count: usize,
    pub bad_count: usize,
    pub fixture_results: Vec<DictationBenchNextPracticeFixtureResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchNextPracticeFixtureResult {
    pub fixture_id: String,
    pub namespace: String,
    pub task_kind: String,
    pub expected_outcome: String,
    pub generated_outcome: DictationBenchNextPracticeGeneratedOutcome,
    pub quality: DictationBenchNextPracticeQuality,
    pub expected_target_mistake_types: Vec<String>,
    pub generated_summary: String,
    pub evidence_ids: Vec<EvidenceId>,
    pub evidence_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationBenchNextPracticeGeneratedOutcome {
    Plan,
    EvidenceGap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationBenchNextPracticeQuality {
    Useful,
    Neutral,
    Bad,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchImprovementReport {
    pub total_fixture_count: usize,
    pub improved_count: usize,
    pub repeated_count: usize,
    pub skipped_count: usize,
    pub insufficient_evidence_count: usize,
    pub metrics: DictationBenchImprovementMetricSummary,
    pub fixture_results: Vec<DictationBenchImprovementFixtureResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchImprovementMetricSummary {
    pub total_trace_count: usize,
    pub total_evidence_count: usize,
    pub local_evidence_count: usize,
    pub local_ratio: f64,
    pub total_latency_ms: u64,
    pub average_latency_ms: f64,
    pub estimated_cost: f64,
    pub cost_currency: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchImprovementFixtureResult {
    pub fixture_id: String,
    pub namespace: String,
    pub task_kind: String,
    pub pattern_results: Vec<DictationBenchImprovementPatternResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchImprovementPatternResult {
    pub expected_mistake_type: String,
    pub expected_recurrence: String,
    pub label: DictationBenchImprovementLabel,
    pub passed: bool,
    pub attempt_ids: Vec<String>,
    pub prompt_item_ids: Vec<String>,
    pub attempt_timeline: Vec<DictationBenchAttemptTimelineEntry>,
    pub notes: Vec<String>,
}

/// Deterministic DictationBench signal labels.
///
/// These labels describe fixture-local signal quality only. They do not claim
/// clinical or educational causality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationBenchImprovementLabel {
    /// Repeated earlier relevant mistakes are followed by a later correct
    /// relevant attempt for the same prompt item.
    Improved,
    /// Repeated relevant mistakes remain present with no later correct
    /// relevant attempt in the fixture window.
    Repeated,
    /// Expected relevant attempts or prompt items are absent or cannot be
    /// evaluated.
    Skipped,
    /// Sparse or unclassified evidence is not treated as improvement or
    /// repeated failure.
    InsufficientEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationBenchAttemptTimelineEntry {
    pub attempt_id: String,
    pub submitted_at: Option<String>,
    pub prompt_item_id: String,
    pub detected_mistake_types: Vec<String>,
    pub correct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationBenchLoadError(String);

impl std::fmt::Display for DictationBenchLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DictationBenchLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DetectedMistake {
    attempt_id: String,
    submitted_at: Option<String>,
    prompt_item_id: String,
    mistake_types: Vec<String>,
    correct: bool,
    sequence_index: usize,
}

pub fn load_dictation_bench_fixtures(
    fixture_dir: &Path,
) -> Result<Vec<DictationBenchFixture>, DictationBenchLoadError> {
    let mut entries = fs::read_dir(fixture_dir)
        .map_err(|err| {
            DictationBenchLoadError(format!("failed to read {}: {err}", fixture_dir.display()))
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|err| DictationBenchLoadError(format!("failed to read entry: {err}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    entries.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    entries.sort();

    entries
        .into_iter()
        .map(|path| {
            let contents = fs::read_to_string(&path).map_err(|err| {
                DictationBenchLoadError(format!("failed to read {}: {err}", path.display()))
            })?;
            serde_json::from_str(&contents).map_err(|err| {
                DictationBenchLoadError(format!("failed to parse {}: {err}", path.display()))
            })
        })
        .collect()
}

pub fn evaluate_dictation_bench_recurring_errors(
    fixtures: &[DictationBenchFixture],
) -> DictationBenchRecurringErrorReport {
    let fixture_results = fixtures
        .iter()
        .map(evaluate_dictation_bench_fixture)
        .collect::<Vec<_>>();
    let total_expected_pattern_count = fixture_results
        .iter()
        .map(|fixture| fixture.pattern_results.len())
        .sum();
    let passed_pattern_count = fixture_results
        .iter()
        .flat_map(|fixture| &fixture.pattern_results)
        .filter(|pattern| pattern.passed)
        .count();
    let failed_pattern_count = total_expected_pattern_count - passed_pattern_count;

    DictationBenchRecurringErrorReport {
        total_fixture_count: fixture_results.len(),
        total_expected_pattern_count,
        passed_pattern_count,
        failed_pattern_count,
        fixture_results,
    }
}

pub fn evaluate_dictation_bench_next_practice(
    fixtures: &[DictationBenchFixture],
) -> DictationBenchNextPracticeReport {
    let fixture_results = fixtures
        .iter()
        .enumerate()
        .map(|(fixture_index, fixture)| {
            evaluate_dictation_bench_next_practice_fixture(fixture_index, fixture)
        })
        .collect::<Vec<_>>();
    let useful_count = fixture_results
        .iter()
        .filter(|fixture| fixture.quality == DictationBenchNextPracticeQuality::Useful)
        .count();
    let neutral_count = fixture_results
        .iter()
        .filter(|fixture| fixture.quality == DictationBenchNextPracticeQuality::Neutral)
        .count();
    let bad_count = fixture_results
        .iter()
        .filter(|fixture| fixture.quality == DictationBenchNextPracticeQuality::Bad)
        .count();

    DictationBenchNextPracticeReport {
        total_fixture_count: fixture_results.len(),
        useful_count,
        neutral_count,
        bad_count,
        fixture_results,
    }
}

pub fn evaluate_dictation_bench_improvement(
    fixtures: &[DictationBenchFixture],
) -> DictationBenchImprovementReport {
    let fixture_results = fixtures
        .iter()
        .map(evaluate_dictation_bench_improvement_fixture)
        .collect::<Vec<_>>();

    let mut improved_count = 0;
    let mut repeated_count = 0;
    let mut skipped_count = 0;
    let mut insufficient_evidence_count = 0;
    let mut total_evidence_count = 0;

    for pattern in fixture_results
        .iter()
        .flat_map(|fixture| fixture.pattern_results.iter())
    {
        total_evidence_count += pattern.attempt_timeline.len();
        match pattern.label {
            DictationBenchImprovementLabel::Improved => improved_count += 1,
            DictationBenchImprovementLabel::Repeated => repeated_count += 1,
            DictationBenchImprovementLabel::Skipped => skipped_count += 1,
            DictationBenchImprovementLabel::InsufficientEvidence => {
                insufficient_evidence_count += 1;
            }
        }
    }

    DictationBenchImprovementReport {
        total_fixture_count: fixture_results.len(),
        improved_count,
        repeated_count,
        skipped_count,
        insufficient_evidence_count,
        metrics: improvement_metric_summary(total_evidence_count),
        fixture_results,
    }
}

fn evaluate_dictation_bench_fixture(
    fixture: &DictationBenchFixture,
) -> DictationBenchFixtureResult {
    let detected = detect_dictation_mistakes(fixture);
    let has_recurring_plan_worthy_pattern = has_recurring_plan_worthy_pattern(&detected);
    let pattern_results = fixture
        .expected_mistake_patterns
        .iter()
        .map(|pattern| {
            evaluate_dictation_bench_pattern(pattern, &detected, has_recurring_plan_worthy_pattern)
        })
        .collect::<Vec<_>>();
    let passed = pattern_results.iter().all(|pattern| pattern.passed);

    DictationBenchFixtureResult {
        fixture_id: fixture.id.clone(),
        namespace: fixture.namespace.clone(),
        task_kind: fixture.task_kind.clone(),
        passed,
        pattern_results,
    }
}

fn evaluate_dictation_bench_next_practice_fixture(
    fixture_index: usize,
    fixture: &DictationBenchFixture,
) -> DictationBenchNextPracticeFixtureResult {
    let detected = detect_dictation_mistakes(fixture);
    let evidence_records = growth_evidence_records_for_fixture(fixture_index, fixture, &detected);
    let space_id = deterministic_uuid(fixture_index, 1, 0);
    let namespace_id = deterministic_uuid(fixture_index, 2, 0);
    let generation_trace_id = deterministic_uuid(fixture_index, 3, 0);
    let aggregation = aggregate_growth_model(
        space_id,
        namespace_id,
        evidence_records,
        Utc.with_ymd_and_hms(2026, 7, 1, 21, 0, 0).unwrap(),
    );
    let generated =
        PracticePlanGeneration::from_growth_model(&aggregation.model, generation_trace_id);

    score_next_practice_generation(fixture, generated, aggregation.evidence_gaps)
}

fn evaluate_dictation_bench_improvement_fixture(
    fixture: &DictationBenchFixture,
) -> DictationBenchImprovementFixtureResult {
    let detected = detect_dictation_mistakes(fixture);
    let pattern_results = fixture
        .expected_mistake_patterns
        .iter()
        .map(|pattern| evaluate_dictation_bench_improvement_pattern(fixture, pattern, &detected))
        .collect::<Vec<_>>();

    DictationBenchImprovementFixtureResult {
        fixture_id: fixture.id.clone(),
        namespace: fixture.namespace.clone(),
        task_kind: fixture.task_kind.clone(),
        pattern_results,
    }
}

fn growth_evidence_records_for_fixture(
    fixture_index: usize,
    fixture: &DictationBenchFixture,
    detected: &[DetectedMistake],
) -> Vec<GrowthEvidenceRecord> {
    let space_id = deterministic_uuid(fixture_index, 1, 0);
    let namespace_id = deterministic_uuid(fixture_index, 2, 0);
    let mut records = Vec::new();

    for (pattern_index, pattern) in fixture.expected_mistake_patterns.iter().enumerate() {
        for (mistake_index, mistake) in detected.iter().enumerate() {
            if !pattern
                .attempt_ids
                .iter()
                .any(|attempt_id| attempt_id == &mistake.attempt_id)
                || !pattern
                    .prompt_item_ids
                    .iter()
                    .any(|prompt_item_id| prompt_item_id == &mistake.prompt_item_id)
                || !mistake
                    .mistake_types
                    .iter()
                    .any(|mistake_type| mistake_type == &pattern.mistake_type)
            {
                continue;
            }

            records.push(GrowthEvidenceRecord {
                space_id,
                namespace_id,
                evidence_id: EvidenceId::Trace(deterministic_uuid(
                    fixture_index,
                    10 + pattern_index,
                    mistake_index,
                )),
                signal_labels: vec![pattern.mistake_type.clone()],
                explanation: Some(format!(
                    "{}:{} showed {}",
                    mistake.attempt_id, mistake.prompt_item_id, pattern.mistake_type
                )),
            });
        }
    }

    records
}

fn evaluate_dictation_bench_improvement_pattern(
    fixture: &DictationBenchFixture,
    pattern: &DictationBenchExpectedPattern,
    detected: &[DetectedMistake],
) -> DictationBenchImprovementPatternResult {
    let mut notes =
        vec!["labels are deterministic fixture signals, not causal learning claims".to_string()];
    let attempt_ids = fixture
        .attempts
        .iter()
        .map(|attempt| attempt.id.as_str())
        .collect::<BTreeSet<_>>();
    let prompt_item_ids = fixture
        .task
        .prompt_items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();

    for attempt_id in &pattern.attempt_ids {
        if !attempt_ids.contains(attempt_id.as_str()) {
            notes.push(format!("missing expected attempt {attempt_id}"));
        }
    }
    for prompt_item_id in &pattern.prompt_item_ids {
        if !prompt_item_ids.contains(prompt_item_id.as_str()) {
            notes.push(format!("missing expected prompt item {prompt_item_id}"));
        }
    }

    let mut relevant = detected
        .iter()
        .filter(|item| {
            pattern
                .prompt_item_ids
                .iter()
                .any(|prompt_item_id| prompt_item_id == &item.prompt_item_id)
        })
        .collect::<Vec<_>>();
    relevant.sort_by(|left, right| {
        left.submitted_at
            .cmp(&right.submitted_at)
            .then(left.sequence_index.cmp(&right.sequence_index))
    });

    for attempt_id in &pattern.attempt_ids {
        for prompt_item_id in &pattern.prompt_item_ids {
            let has_relevant_entry = relevant.iter().any(|item| {
                &item.attempt_id == attempt_id && &item.prompt_item_id == prompt_item_id
            });
            if !has_relevant_entry {
                notes.push(format!(
                    "expected attempt {attempt_id} has no evaluatable item for {prompt_item_id}"
                ));
            }
        }
    }

    let label = if notes.iter().any(|note| {
        note.starts_with("missing expected") || note.contains("has no evaluatable item for")
    }) {
        DictationBenchImprovementLabel::Skipped
    } else if has_later_correct_after_repeated_mistakes(&relevant, pattern) {
        notes.push(
            "repeated earlier mistakes were followed by a later correct relevant attempt"
                .to_string(),
        );
        DictationBenchImprovementLabel::Improved
    } else {
        let matching_attempt_count = matching_mistake_attempt_count(&relevant, pattern);
        if matching_attempt_count >= 2 {
            notes.push(
                "repeated relevant mistakes have no later correct relevant attempt".to_string(),
            );
            DictationBenchImprovementLabel::Repeated
        } else {
            notes.push(
                "not enough deterministic evidence for improvement or repeated failure".to_string(),
            );
            DictationBenchImprovementLabel::InsufficientEvidence
        }
    };

    let passed = label == expected_improvement_label(&pattern.recurrence);
    if !passed {
        notes.push(format!(
            "expected recurrence {} mapped to {:?}",
            pattern.recurrence,
            expected_improvement_label(&pattern.recurrence)
        ));
    }

    DictationBenchImprovementPatternResult {
        expected_mistake_type: pattern.mistake_type.clone(),
        expected_recurrence: pattern.recurrence.clone(),
        label,
        passed,
        attempt_ids: pattern.attempt_ids.clone(),
        prompt_item_ids: pattern.prompt_item_ids.clone(),
        attempt_timeline: relevant
            .into_iter()
            .map(|item| DictationBenchAttemptTimelineEntry {
                attempt_id: item.attempt_id.clone(),
                submitted_at: item.submitted_at.clone(),
                prompt_item_id: item.prompt_item_id.clone(),
                detected_mistake_types: item.mistake_types.clone(),
                correct: item.correct,
            })
            .collect(),
        notes,
    }
}

fn has_later_correct_after_repeated_mistakes(
    relevant: &[&DetectedMistake],
    pattern: &DictationBenchExpectedPattern,
) -> bool {
    pattern.prompt_item_ids.iter().any(|prompt_item_id| {
        let mut mistake_count = 0;
        for item in relevant
            .iter()
            .filter(|item| &item.prompt_item_id == prompt_item_id)
        {
            if item
                .mistake_types
                .iter()
                .any(|mistake_type| mistake_type == &pattern.mistake_type)
            {
                mistake_count += 1;
            } else if item.correct && mistake_count >= 2 {
                return true;
            }
        }
        false
    })
}

fn matching_mistake_attempt_count(
    relevant: &[&DetectedMistake],
    pattern: &DictationBenchExpectedPattern,
) -> usize {
    relevant
        .iter()
        .filter(|item| {
            item.mistake_types
                .iter()
                .any(|mistake_type| mistake_type == &pattern.mistake_type)
        })
        .map(|item| item.attempt_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn expected_improvement_label(recurrence: &str) -> DictationBenchImprovementLabel {
    match recurrence {
        "improving" => DictationBenchImprovementLabel::Improved,
        "recurring" => DictationBenchImprovementLabel::Repeated,
        "insufficient_evidence" => DictationBenchImprovementLabel::InsufficientEvidence,
        "single" => DictationBenchImprovementLabel::InsufficientEvidence,
        _ => DictationBenchImprovementLabel::Skipped,
    }
}

fn improvement_metric_summary(
    total_evidence_count: usize,
) -> DictationBenchImprovementMetricSummary {
    const SIMULATED_LATENCY_MS_PER_EVIDENCE: u64 = 7;

    let total_latency_ms = total_evidence_count as u64 * SIMULATED_LATENCY_MS_PER_EVIDENCE;
    let average_latency_ms = if total_evidence_count == 0 {
        0.0
    } else {
        total_latency_ms as f64 / total_evidence_count as f64
    };

    DictationBenchImprovementMetricSummary {
        total_trace_count: total_evidence_count,
        total_evidence_count,
        local_evidence_count: total_evidence_count,
        local_ratio: 1.0,
        total_latency_ms,
        average_latency_ms,
        estimated_cost: 0.0,
        cost_currency: "USD".to_string(),
        notes: vec![
            "fixture-derived traces are simulated local evidence; no provider, database, OCR, ASR, or vector index is used".to_string(),
            format!(
                "simulated deterministic latency uses {SIMULATED_LATENCY_MS_PER_EVIDENCE}ms per evaluated timeline item, not wall-clock time"
            ),
            "estimated cost remains zero for the local deterministic path".to_string(),
        ],
    }
}

fn score_next_practice_generation(
    fixture: &DictationBenchFixture,
    generated: PracticePlanGeneration,
    evidence_gaps: Vec<String>,
) -> DictationBenchNextPracticeFixtureResult {
    let expected = &fixture.expected_next_practice;
    let mut notes = Vec::new();

    let (
        generated_outcome,
        generated_summary,
        evidence_ids,
        outcome_matches,
        duration_matches,
        target_matches,
    ) = match generated {
        PracticePlanGeneration::Plan(plan) => {
            let generated_text = [
                plan.target_pattern.as_deref().unwrap_or_default(),
                plan.content.as_str(),
                plan.expected_effect.as_deref().unwrap_or_default(),
            ]
            .join(" ");
            let duration_matches = match expected.duration_minutes {
                Some(minutes) => {
                    generated_text.contains(&format!("{minutes}-minute"))
                        || generated_text.contains(&format!("{minutes} minutes"))
                }
                None => true,
            };
            let target_matches = expected.target_mistake_types.iter().all(|mistake_type| {
                generated_text.contains(mistake_type)
                    || generated_text.contains(&mistake_type.replace('_', " "))
                    || generated_text.contains(&mistake_type.replace('_', "-"))
            });

            if expected.outcome == "plan" {
                notes.push("matched expected plan outcome".to_string());
            } else {
                notes.push("generated a plan where fixture expected evidence_gap".to_string());
            }
            if !duration_matches {
                notes.push("generated plan did not match expected duration shape".to_string());
            }
            if !target_matches {
                notes.push(
                    "generated plan did not visibly target expected mistake types".to_string(),
                );
            }

            (
                DictationBenchNextPracticeGeneratedOutcome::Plan,
                generated_text,
                plan.evidence_ids.clone(),
                expected.outcome == "plan",
                duration_matches,
                target_matches,
            )
        }
        PracticePlanGeneration::EvidenceGap(gap) => {
            if expected.outcome == "evidence_gap" {
                notes.push("matched expected evidence_gap outcome".to_string());
            } else {
                notes.push("generated evidence_gap where fixture expected plan".to_string());
            }
            for evidence_gap in evidence_gaps {
                notes.push(format!("growth evidence gap: {evidence_gap}"));
            }

            (
                DictationBenchNextPracticeGeneratedOutcome::EvidenceGap,
                gap.reason,
                gap.evidence_ids,
                expected.outcome == "evidence_gap",
                expected.duration_minutes.is_none(),
                expected.target_mistake_types.is_empty(),
            )
        }
    };
    let evidence_count = evidence_ids.len();

    let quality = if outcome_matches && duration_matches && target_matches {
        DictationBenchNextPracticeQuality::Useful
    } else if outcome_matches {
        DictationBenchNextPracticeQuality::Neutral
    } else {
        DictationBenchNextPracticeQuality::Bad
    };

    DictationBenchNextPracticeFixtureResult {
        fixture_id: fixture.id.clone(),
        namespace: fixture.namespace.clone(),
        task_kind: fixture.task_kind.clone(),
        expected_outcome: expected.outcome.clone(),
        generated_outcome,
        quality,
        expected_target_mistake_types: expected.target_mistake_types.clone(),
        generated_summary,
        evidence_ids,
        evidence_count,
        notes,
    }
}

fn detect_dictation_mistakes(fixture: &DictationBenchFixture) -> Vec<DetectedMistake> {
    let task_kind = parse_task_kind(&fixture.task_kind);
    let prompt_items = fixture
        .task
        .prompt_items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut detected = Vec::new();

    let mut sequence_index = 0;
    for attempt in &fixture.attempts {
        for submitted in &attempt.submitted_items {
            let current_sequence_index = sequence_index;
            sequence_index += 1;
            let Some(prompt_item_id) = submitted.prompt_item_id.as_deref() else {
                continue;
            };
            let Some(prompt_item) = prompt_items.get(prompt_item_id) else {
                continue;
            };
            let Ok(attempt_result) = build_dictation_attempt(DictationAttemptInput {
                namespace: fixture.namespace.clone(),
                task_kind,
                source: DictationSource::Typed,
                task: Some(fixture.task.id.clone()),
                goal: None,
                prompt_items: vec![PromptItemInput {
                    item_kind: parse_item_kind(&prompt_item.item_kind),
                    expected_text: prompt_item.expected_text.clone(),
                    display_text: prompt_item.display_text.clone(),
                    hint: prompt_item.hint.clone(),
                    locale: prompt_item.locale.clone(),
                    metadata: prompt_item.metadata.clone(),
                }],
                submitted_items: vec![SubmittedItemInput {
                    actual_text: submitted.actual_text.clone(),
                    metadata: submitted.metadata.clone(),
                }],
                input_confirmation: None,
                evidence_refs: Vec::new(),
                metadata: json!({
                    "fixture_id": fixture.id,
                    "attempt_id": attempt.id,
                    "prompt_item_id": prompt_item_id,
                }),
            }) else {
                continue;
            };
            let Some(item_result) = attempt_result.evaluation.item_results.first() else {
                continue;
            };
            detected.push(DetectedMistake {
                attempt_id: attempt.id.clone(),
                submitted_at: attempt.submitted_at.clone(),
                prompt_item_id: prompt_item_id.to_string(),
                mistake_types: item_result.mistake_types.clone(),
                correct: item_result.status == "correct",
                sequence_index: current_sequence_index,
            });
        }
    }

    detected
}

fn evaluate_dictation_bench_pattern(
    pattern: &DictationBenchExpectedPattern,
    detected: &[DetectedMistake],
    has_recurring_plan_worthy_pattern: bool,
) -> DictationBenchPatternResult {
    let relevant = detected
        .iter()
        .filter(|mistake| {
            pattern
                .attempt_ids
                .iter()
                .any(|attempt_id| attempt_id == &mistake.attempt_id)
                && pattern
                    .prompt_item_ids
                    .iter()
                    .any(|prompt_item_id| prompt_item_id == &mistake.prompt_item_id)
        })
        .collect::<Vec<_>>();
    let detected_mistake_types = detected_mistake_types(&relevant);
    let matched_attempt_count = relevant
        .iter()
        .filter(|mistake| {
            mistake
                .mistake_types
                .iter()
                .any(|mistake_type| mistake_type == &pattern.mistake_type)
        })
        .map(|mistake| mistake.attempt_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let matched_occurrence_count = relevant
        .iter()
        .filter(|mistake| {
            mistake
                .mistake_types
                .iter()
                .any(|mistake_type| mistake_type == &pattern.mistake_type)
        })
        .count();
    let mut notes = Vec::new();

    let passed = match pattern.recurrence.as_str() {
        "recurring" => {
            notes.push("expected repeated relevant attempts to show this pattern".to_string());
            matched_attempt_count >= 2
        }
        "single" => {
            notes.push("expected one relevant occurrence, not a recurring pattern".to_string());
            matched_occurrence_count == 1
        }
        "improving" => {
            notes.push(
                "improving label scores repeated pattern evidence only; quality belongs to #168"
                    .to_string(),
            );
            matched_attempt_count >= 2
        }
        "insufficient_evidence" => {
            notes.push(
                "insufficient evidence is not scored as recurring or plan-worthy".to_string(),
            );
            matched_occurrence_count <= 1 && !has_recurring_plan_worthy_pattern
        }
        _ => {
            notes.push(format!(
                "unsupported recurrence label {}",
                pattern.recurrence
            ));
            false
        }
    };

    DictationBenchPatternResult {
        expected_mistake_type: pattern.mistake_type.clone(),
        recurrence: pattern.recurrence.clone(),
        attempt_ids: pattern.attempt_ids.clone(),
        prompt_item_ids: pattern.prompt_item_ids.clone(),
        detected_mistake_types,
        passed,
        notes,
    }
}

fn detected_mistake_types(relevant: &[&DetectedMistake]) -> Vec<String> {
    relevant
        .iter()
        .flat_map(|mistake| mistake.mistake_types.iter().cloned())
        .filter(|mistake_type| mistake_type != "correct")
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn has_recurring_plan_worthy_pattern(detected: &[DetectedMistake]) -> bool {
    let mut attempts_by_type = BTreeMap::<&str, BTreeSet<&str>>::new();
    for mistake in detected {
        for mistake_type in &mistake.mistake_types {
            if matches!(mistake_type.as_str(), "correct" | "unclassified") {
                continue;
            }
            attempts_by_type
                .entry(mistake_type.as_str())
                .or_default()
                .insert(mistake.attempt_id.as_str());
        }
    }

    attempts_by_type
        .values()
        .any(|attempt_ids| attempt_ids.len() >= 2)
}

fn deterministic_uuid(fixture_index: usize, group: usize, item_index: usize) -> Uuid {
    Uuid::from_u128(
        0x1670_0000_0000_0000_0000_0000_0000_0000
            + ((fixture_index as u128) << 32)
            + ((group as u128) << 16)
            + item_index as u128,
    )
}

fn parse_task_kind(task_kind: &str) -> DictationTaskKind {
    match task_kind {
        "chinese_dictation" => DictationTaskKind::ChineseDictation,
        "english_spelling" => DictationTaskKind::EnglishSpelling,
        "english_sentence_dictation" => DictationTaskKind::EnglishSentenceDictation,
        other => panic!("unsupported DictationBench task kind {other}"),
    }
}

fn parse_item_kind(item_kind: &str) -> DictationItemKind {
    match item_kind {
        "chinese_character" => DictationItemKind::ChineseCharacter,
        "chinese_word" => DictationItemKind::ChineseWord,
        "chinese_phrase" => DictationItemKind::ChinesePhrase,
        "chinese_sentence" => DictationItemKind::ChineseSentence,
        "english_word" => DictationItemKind::EnglishWord,
        "english_phrase" => DictationItemKind::EnglishPhrase,
        "english_sentence" => DictationItemKind::EnglishSentence,
        other => panic!("unsupported DictationBench item kind {other}"),
    }
}
