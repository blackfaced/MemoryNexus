use memorynexus::domain::dictation::{
    build_dictation_attempt, DictationAttemptInput, DictationItemKind, DictationSource,
    DictationTaskKind, PromptItemInput, SubmittedItemInput,
};
use serde_json::json;

fn attempt(
    namespace: &str,
    task_kind: DictationTaskKind,
    item_kind: DictationItemKind,
    expected: &str,
    actual: &str,
) -> Vec<String> {
    first_result(namespace, task_kind, item_kind, expected, actual).mistake_types
}

fn first_result(
    namespace: &str,
    task_kind: DictationTaskKind,
    item_kind: DictationItemKind,
    expected: &str,
    actual: &str,
) -> memorynexus::domain::dictation::DictationItemResult {
    build_dictation_attempt(DictationAttemptInput {
        namespace: namespace.to_string(),
        task_kind,
        source: DictationSource::Typed,
        task: None,
        goal: None,
        prompt_items: vec![PromptItemInput {
            item_kind,
            expected_text: expected.to_string(),
            display_text: None,
            hint: None,
            locale: None,
            metadata: json!({}),
        }],
        submitted_items: vec![SubmittedItemInput {
            actual_text: actual.to_string(),
            metadata: json!({}),
        }],
        input_confirmation: None,
        evidence_refs: Vec::new(),
        metadata: json!({}),
    })
    .expect("dictation attempt should build")
    .evaluation
    .item_results
    .remove(0)
}

#[test]
fn chinese_dictation_classifies_deterministic_text_differences() {
    let cases = [
        ("春天", "春", "missing_character"),
        ("春天", "春天天", "extra_character"),
        ("春天", "春大", "wrong_character"),
        ("春天。", "春天", "punctuation_error"),
        ("春 天", "春天", "spacing_error"),
        ("春天", "冬", "unclassified"),
    ];

    for (expected, actual, mistake_type) in cases {
        assert_eq!(
            attempt(
                "child.chinese.dictation",
                DictationTaskKind::ChineseDictation,
                DictationItemKind::ChineseWord,
                expected,
                actual,
            ),
            vec![mistake_type],
            "{expected} vs {actual}"
        );
    }
}

#[test]
fn english_spelling_classifies_supported_word_level_mistakes() {
    let cases = [
        ("because", "becaus", "missing_letter"),
        ("because", "beccause", "extra_letter"),
        ("because", "becuase", "letter_order_error"),
        ("Because", "because", "capitalization_error"),
        ("hello!", "hello", "punctuation_error"),
        ("ice cream", "icecream", "spacing_error"),
        ("because", "bcz", "unclassified"),
    ];

    for (expected, actual, mistake_type) in cases {
        assert_eq!(
            attempt(
                "child.english.spelling",
                DictationTaskKind::EnglishSpelling,
                DictationItemKind::EnglishWord,
                expected,
                actual,
            ),
            vec![mistake_type],
            "{expected} vs {actual}"
        );
    }
}

#[test]
fn english_sentence_dictation_classifies_word_and_spelling_mistakes() {
    let cases = [
        (
            "I have a good friend.",
            "I have good friend.",
            "missing_word",
        ),
        (
            "I have a good friend.",
            "I have a very good friend.",
            "extra_word",
        ),
        (
            "I have a good friend.",
            "I have good a friend.",
            "word_order_error",
        ),
        ("I have a dog.", "I hav a dog.", "missing_letter"),
        ("I have a dog.", "I have a doog.", "extra_letter"),
        ("I have a dog.", "I hvae a dog.", "letter_order_error"),
        ("I have a dog.", "i have a dog.", "capitalization_error"),
        ("I have a dog.", "I have a dog", "punctuation_error"),
        ("I have a dog.", "I have  a dog.", "spacing_error"),
        ("I have a dog.", "They saw the cat.", "unclassified"),
    ];

    for (expected, actual, mistake_type) in cases {
        assert_eq!(
            attempt(
                "child.english.sentence-dictation",
                DictationTaskKind::EnglishSentenceDictation,
                DictationItemKind::EnglishSentence,
                expected,
                actual,
            ),
            vec![mistake_type],
            "{expected} vs {actual}"
        );
    }
}

#[test]
fn classification_returns_stable_short_explanations() {
    let result = first_result(
        "child.english.spelling",
        DictationTaskKind::EnglishSpelling,
        DictationItemKind::EnglishWord,
        "because",
        "becaus",
    );

    assert_eq!(result.mistake_types, vec!["missing_letter"]);
    assert_eq!(
        result.explanation,
        "actual text is missing one or more letters from expected text"
    );
}
