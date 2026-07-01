use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct DictationBenchFixture {
    id: String,
    namespace: String,
    locale: String,
    task_kind: String,
    task: FixtureTask,
    attempts: Vec<FixtureAttempt>,
    expected_mistake_patterns: Vec<ExpectedMistakePattern>,
    expected_next_practice: ExpectedNextPractice,
    deterministic_evaluation_notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureTask {
    id: String,
    source: String,
    prompt_items: Vec<PromptItem>,
}

#[derive(Debug, Deserialize)]
struct PromptItem {
    id: String,
    item_kind: String,
    expected_text: String,
    order_index: usize,
}

#[derive(Debug, Deserialize)]
struct FixtureAttempt {
    id: String,
    source: String,
    submitted_items: Vec<SubmittedItem>,
}

#[derive(Debug, Deserialize)]
struct SubmittedItem {
    prompt_item_id: Option<String>,
    actual_text: String,
    order_index: usize,
}

#[derive(Debug, Deserialize)]
struct ExpectedMistakePattern {
    mistake_type: String,
    attempt_ids: Vec<String>,
    prompt_item_ids: Vec<String>,
    recurrence: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedNextPractice {
    outcome: String,
    duration_minutes: Option<u16>,
    target_mistake_types: Vec<String>,
    expectations: Vec<String>,
}

#[test]
fn dictation_bench_fixture_corpus_is_parseable_and_covers_required_scenarios() {
    let fixtures = load_fixtures();

    assert!(
        fixtures.len() >= 5,
        "DictationBench needs at least five first-corpus fixtures"
    );

    let mut task_kinds = BTreeSet::new();
    let mut has_multi_day_improvement = false;
    let mut has_insufficient_evidence = false;
    let mut has_plan = false;
    let mut has_evidence_gap = false;

    for fixture in &fixtures {
        assert_fixture_shape(fixture);
        task_kinds.insert(fixture.task_kind.as_str());

        if fixture
            .expected_mistake_patterns
            .iter()
            .any(|pattern| pattern.recurrence == "improving")
        {
            has_multi_day_improvement = true;
        }

        match fixture.expected_next_practice.outcome.as_str() {
            "plan" => has_plan = true,
            "evidence_gap" => has_evidence_gap = true,
            other => panic!(
                "{} has unsupported next-practice outcome {other}",
                fixture.id
            ),
        }

        if fixture.expected_next_practice.outcome == "evidence_gap" {
            has_insufficient_evidence = true;
        }
    }

    assert!(
        task_kinds.contains("chinese_dictation"),
        "corpus must include Chinese dictation"
    );
    assert!(
        task_kinds.contains("english_spelling"),
        "corpus must include English spelling"
    );
    assert!(
        task_kinds.contains("english_sentence_dictation"),
        "corpus must include English sentence dictation"
    );
    assert!(
        has_multi_day_improvement,
        "corpus must include a multi-day improvement fixture"
    );
    assert!(
        has_insufficient_evidence && has_evidence_gap,
        "corpus must include an insufficient-evidence fixture"
    );
    assert!(has_plan, "corpus must include a plan-producing fixture");
}

fn load_fixtures() -> Vec<DictationBenchFixture> {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dictation_bench");

    let mut entries = fs::read_dir(&fixture_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", fixture_dir.display()))
        .map(|entry| {
            entry
                .expect("fixture directory entry should be readable")
                .path()
        })
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    entries.sort();

    assert!(
        !entries.is_empty(),
        "expected JSON fixtures in {}",
        fixture_dir.display()
    );

    entries
        .into_iter()
        .map(|path| {
            let contents = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            serde_json::from_str(&contents)
                .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()))
        })
        .collect()
}

fn assert_fixture_shape(fixture: &DictationBenchFixture) {
    assert!(!fixture.id.is_empty(), "fixture id is required");
    assert!(
        matches!(
            fixture.namespace.as_str(),
            "child.chinese.dictation"
                | "child.english.spelling"
                | "child.english.sentence-dictation"
        ),
        "{} has unexpected namespace {}",
        fixture.id,
        fixture.namespace
    );
    assert!(
        !fixture.locale.is_empty(),
        "{} locale is required",
        fixture.id
    );
    assert_eq!(
        fixture.task.source, "test_fixture",
        "{} task source must stay local",
        fixture.id
    );
    assert!(
        !fixture.task.id.is_empty(),
        "{} task id is required",
        fixture.id
    );
    assert!(
        !fixture.task.prompt_items.is_empty(),
        "{} needs prompt items",
        fixture.id
    );
    assert!(
        !fixture.attempts.is_empty(),
        "{} needs submitted attempts",
        fixture.id
    );
    assert!(
        !fixture.expected_mistake_patterns.is_empty(),
        "{} needs expected mistake patterns",
        fixture.id
    );
    assert!(
        !fixture.deterministic_evaluation_notes.is_empty(),
        "{} needs local deterministic evaluation notes",
        fixture.id
    );

    let prompt_items = fixture
        .task
        .prompt_items
        .iter()
        .map(|item| {
            assert!(
                !item.id.is_empty(),
                "{} prompt item id is required",
                fixture.id
            );
            assert!(
                !item.item_kind.is_empty(),
                "{} prompt item kind is required",
                fixture.id
            );
            assert!(
                !item.expected_text.is_empty(),
                "{} expected text is required",
                fixture.id
            );
            (item.id.as_str(), item.order_index)
        })
        .collect::<BTreeMap<_, _>>();

    let attempts = fixture
        .attempts
        .iter()
        .map(|attempt| {
            assert!(
                !attempt.id.is_empty(),
                "{} attempt id is required",
                fixture.id
            );
            assert_eq!(
                attempt.source, "test_fixture",
                "{} attempt source must stay local",
                fixture.id
            );
            assert!(
                !attempt.submitted_items.is_empty(),
                "{} attempt {} needs submitted items",
                fixture.id,
                attempt.id
            );
            for item in &attempt.submitted_items {
                assert!(
                    match item.prompt_item_id.as_deref() {
                        Some(prompt_id) => prompt_items.contains_key(prompt_id),
                        None => true,
                    },
                    "{} attempt {} references an unknown prompt item",
                    fixture.id,
                    attempt.id
                );
                let _ = (&item.actual_text, item.order_index);
            }
            attempt.id.as_str()
        })
        .collect::<BTreeSet<_>>();

    for pattern in &fixture.expected_mistake_patterns {
        assert!(
            allowed_mistake_types(&fixture.task_kind).contains(pattern.mistake_type.as_str()),
            "{} uses unsupported mistake type {} for {}",
            fixture.id,
            pattern.mistake_type,
            fixture.task_kind
        );
        assert!(
            matches!(
                pattern.recurrence.as_str(),
                "single" | "recurring" | "improving" | "insufficient_evidence"
            ),
            "{} has unsupported recurrence {}",
            fixture.id,
            pattern.recurrence
        );
        assert!(
            pattern
                .attempt_ids
                .iter()
                .all(|attempt_id| attempts.contains(attempt_id.as_str())),
            "{} pattern references an unknown attempt",
            fixture.id
        );
        assert!(
            pattern
                .prompt_item_ids
                .iter()
                .all(|prompt_id| prompt_items.contains_key(prompt_id.as_str())),
            "{} pattern references an unknown prompt item",
            fixture.id
        );
    }

    let next_practice = &fixture.expected_next_practice;
    match next_practice.outcome.as_str() {
        "plan" => {
            assert_eq!(
                next_practice.duration_minutes,
                Some(10),
                "{} plan fixtures should target the MVP ten-minute practice",
                fixture.id
            );
            assert!(
                !next_practice.target_mistake_types.is_empty(),
                "{} plan fixtures need target mistake types",
                fixture.id
            );
            assert!(
                !next_practice.expectations.is_empty(),
                "{} plan fixtures need concrete expectations",
                fixture.id
            );
        }
        "evidence_gap" => {
            assert!(
                next_practice.target_mistake_types.is_empty(),
                "{} evidence gaps must not invent target mistake types",
                fixture.id
            );
            assert!(
                next_practice
                    .expectations
                    .iter()
                    .any(|expectation| expectation.contains("Collect more confirmed attempts")),
                "{} evidence gaps must ask for more confirmed attempts",
                fixture.id
            );
        }
        other => panic!("{} has unsupported outcome {other}", fixture.id),
    }
}

fn allowed_mistake_types(task_kind: &str) -> BTreeSet<&'static str> {
    let types: &[&str] = match task_kind {
        "chinese_dictation" => &[
            "wrong_character",
            "visually_similar_character",
            "homophone",
            "missing_stroke",
            "extra_stroke",
            "stroke_order_issue",
            "component_placement_issue",
            "missing_item",
            "extra_item",
            "punctuation_error",
            "spacing_error",
            "unclassified",
        ],
        "english_spelling" => &[
            "missing_letter",
            "extra_letter",
            "letter_order_error",
            "double_letter_error",
            "sound_spelling_mapping_error",
            "capitalization_error",
            "missing_item",
            "extra_item",
            "punctuation_error",
            "spacing_error",
            "unclassified",
        ],
        "english_sentence_dictation" => &[
            "missing_word",
            "extra_word",
            "word_order_error",
            "missing_letter",
            "extra_letter",
            "letter_order_error",
            "double_letter_error",
            "sound_spelling_mapping_error",
            "capitalization_error",
            "punctuation_error",
            "spacing_error",
            "unclassified",
        ],
        other => panic!("unsupported task kind {other}"),
    };

    types.iter().copied().collect()
}
