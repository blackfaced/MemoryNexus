use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::domain::evidence::{validate_evidence_request, InputConfirmation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationTaskKind {
    ChineseDictation,
    EnglishSpelling,
    EnglishSentenceDictation,
}

impl fmt::Display for DictationTaskKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ChineseDictation => "chinese_dictation",
            Self::EnglishSpelling => "english_spelling",
            Self::EnglishSentenceDictation => "english_sentence_dictation",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationItemKind {
    ChineseCharacter,
    ChineseWord,
    ChinesePhrase,
    ChineseSentence,
    EnglishWord,
    EnglishPhrase,
    EnglishSentence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationSource {
    Typed,
    Pasted,
    AgentOcr,
    AgentTranscribed,
    Mixed,
}

impl DictationSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Typed => "typed",
            Self::Pasted => "pasted",
            Self::AgentOcr => "agent_ocr",
            Self::AgentTranscribed => "agent_transcribed",
            Self::Mixed => "mixed",
        }
    }

    const fn is_manual(self) -> bool {
        matches!(self, Self::Typed | Self::Pasted)
    }
}

impl fmt::Display for DictationSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptItemInput {
    pub item_kind: DictationItemKind,
    pub expected_text: String,
    pub display_text: Option<String>,
    pub hint: Option<String>,
    pub locale: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationPromptItem {
    pub order_index: usize,
    pub item_kind: DictationItemKind,
    pub expected_text: String,
    pub display_text: Option<String>,
    pub hint: Option<String>,
    pub locale: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationCaptureInput {
    pub namespace: String,
    pub task_kind: DictationTaskKind,
    pub source: DictationSource,
    pub title: Option<String>,
    pub prompt_items: Vec<PromptItemInput>,
    pub input_confirmation: Option<InputConfirmation>,
    #[serde(default)]
    pub evidence_refs: Vec<Value>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationCapture {
    pub namespace: String,
    pub task_kind: DictationTaskKind,
    pub source: DictationSource,
    pub title: Option<String>,
    pub prompt_items: Vec<DictationPromptItem>,
    pub canonical_text: String,
    pub evidence_ref_count: usize,
    pub persistence_metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmittedItemInput {
    pub actual_text: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationSubmittedItem {
    pub order_index: usize,
    pub actual_text: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationAttemptInput {
    pub namespace: String,
    pub task_kind: DictationTaskKind,
    pub source: DictationSource,
    pub task: Option<String>,
    pub goal: Option<String>,
    pub prompt_items: Vec<PromptItemInput>,
    pub submitted_items: Vec<SubmittedItemInput>,
    pub input_confirmation: Option<InputConfirmation>,
    #[serde(default)]
    pub evidence_refs: Vec<Value>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationAttempt {
    pub namespace: String,
    pub task_kind: DictationTaskKind,
    pub source: DictationSource,
    pub task: Option<String>,
    pub goal: Option<String>,
    pub prompt_items: Vec<DictationPromptItem>,
    pub submitted_items: Vec<DictationSubmittedItem>,
    pub summary: String,
    pub evaluation: DictationEvaluation,
    pub evidence_ref_count: usize,
    pub persistence_metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationEvaluation {
    pub summary: String,
    pub item_results: Vec<DictationItemResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationItemResult {
    pub order_index: usize,
    pub expected_text: Option<String>,
    pub actual_text: Option<String>,
    pub status: String,
    pub mistake_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationValidationError(String);

impl DictationValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for DictationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DictationValidationError {}

pub fn build_dictation_capture(
    input: DictationCaptureInput,
) -> Result<DictationCapture, DictationValidationError> {
    validate_namespace_task_kind(&input.namespace, input.task_kind)?;
    validate_source_fields(
        input.source,
        input.input_confirmation.as_ref(),
        &input.evidence_refs,
        "capture",
    )?;

    if input.prompt_items.is_empty() {
        return Err(DictationValidationError::new(
            "dictation prompt_items cannot be empty",
        ));
    }

    let prompt_items = normalize_prompt_items(input.task_kind, input.prompt_items)?;

    let canonical_text = prompt_items
        .iter()
        .map(|item| item.expected_text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let evidence_ref_count = input.evidence_refs.len();
    let persistence_metadata = json!({
        "domain": "dictation",
        "task_kind": input.task_kind,
        "source": input.source,
        "item_count": prompt_items.len(),
        "items": prompt_items,
        "metadata": input.metadata,
    });

    Ok(DictationCapture {
        namespace: input.namespace,
        task_kind: input.task_kind,
        source: input.source,
        title: input.title,
        prompt_items,
        canonical_text,
        evidence_ref_count,
        persistence_metadata,
    })
}

pub fn build_dictation_attempt(
    input: DictationAttemptInput,
) -> Result<DictationAttempt, DictationValidationError> {
    validate_namespace_task_kind(&input.namespace, input.task_kind)?;
    validate_source_fields(
        input.source,
        input.input_confirmation.as_ref(),
        &input.evidence_refs,
        "attempt",
    )?;

    if input.prompt_items.is_empty() {
        return Err(DictationValidationError::new(
            "dictation prompt_items cannot be empty",
        ));
    }
    if input.submitted_items.is_empty() {
        return Err(DictationValidationError::new(
            "dictation submitted_items cannot be empty",
        ));
    }

    let prompt_items = normalize_prompt_items(input.task_kind, input.prompt_items)?;
    let submitted_items = input
        .submitted_items
        .into_iter()
        .enumerate()
        .map(|(index, item)| DictationSubmittedItem {
            order_index: index,
            actual_text: item.actual_text.trim().to_string(),
            metadata: item.metadata,
        })
        .collect::<Vec<_>>();
    let summary = summarize_attempt(&prompt_items, &submitted_items);
    let evaluation = evaluate_attempt(input.task_kind, &prompt_items, &submitted_items);
    let evidence_ref_count = input.evidence_refs.len();
    let persistence_metadata = json!({
        "domain": "dictation",
        "record_kind": "attempt",
        "task_kind": input.task_kind,
        "source": input.source,
        "item_count": prompt_items.len(),
        "submitted_item_count": submitted_items.len(),
        "prompt_items": prompt_items,
        "submitted_items": submitted_items,
        "evaluation": evaluation,
        "metadata": input.metadata,
    });

    Ok(DictationAttempt {
        namespace: input.namespace,
        task_kind: input.task_kind,
        source: input.source,
        task: input.task,
        goal: input.goal,
        prompt_items,
        submitted_items,
        summary,
        evaluation,
        evidence_ref_count,
        persistence_metadata,
    })
}

fn validate_source_fields(
    source: DictationSource,
    input_confirmation: Option<&InputConfirmation>,
    evidence_refs: &[Value],
    record_kind: &str,
) -> Result<(), DictationValidationError> {
    if source.is_manual() {
        if input_confirmation.is_some() {
            return Err(DictationValidationError::new(format!(
                "typed or pasted dictation {record_kind} cannot include input_confirmation"
            )));
        }
        if !evidence_refs.is_empty() {
            return Err(DictationValidationError::new(format!(
                "typed or pasted dictation {record_kind} cannot include evidence_refs"
            )));
        }
        return Ok(());
    }

    if input_confirmation.is_none() {
        return Err(DictationValidationError::new(format!(
            "input_confirmation is required for media-derived dictation {record_kind}"
        )));
    }

    let refs = if evidence_refs.is_empty() {
        None
    } else {
        Some(evidence_refs)
    };
    validate_evidence_request(Some(source.as_str()), input_confirmation, refs).map_err(|error| {
        if error.path == "input_confirmation" {
            DictationValidationError::new(
                "input_confirmation must be confirmed by explicit acceptance or correction",
            )
        } else {
            DictationValidationError::new(error.to_string())
        }
    })
}

fn normalize_prompt_items(
    task_kind: DictationTaskKind,
    prompt_items: Vec<PromptItemInput>,
) -> Result<Vec<DictationPromptItem>, DictationValidationError> {
    let mut normalized = Vec::with_capacity(prompt_items.len());
    for (index, item) in prompt_items.into_iter().enumerate() {
        let expected_text = item.expected_text.trim().to_string();
        if expected_text.is_empty() {
            return Err(DictationValidationError::new(format!(
                "dictation prompt_items[{index}].expected_text cannot be empty"
            )));
        }
        validate_item_kind(task_kind, item.item_kind, index)?;
        normalized.push(DictationPromptItem {
            order_index: index,
            item_kind: item.item_kind,
            expected_text,
            display_text: item.display_text,
            hint: item.hint,
            locale: item.locale,
            metadata: item.metadata,
        });
    }
    Ok(normalized)
}

fn summarize_attempt(
    prompt_items: &[DictationPromptItem],
    submitted_items: &[DictationSubmittedItem],
) -> String {
    let max_len = prompt_items.len().max(submitted_items.len());
    (0..max_len)
        .map(|index| {
            let expected = prompt_items
                .get(index)
                .map(|item| item.expected_text.as_str())
                .unwrap_or("<extra>");
            let actual = submitted_items
                .get(index)
                .map(|item| item.actual_text.as_str())
                .unwrap_or("<missing>");
            format!("{expected} -> {actual}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn evaluate_attempt(
    task_kind: DictationTaskKind,
    prompt_items: &[DictationPromptItem],
    submitted_items: &[DictationSubmittedItem],
) -> DictationEvaluation {
    let max_len = prompt_items.len().max(submitted_items.len());
    let item_results = (0..max_len)
        .map(|index| {
            let expected = prompt_items
                .get(index)
                .map(|item| item.expected_text.clone());
            let actual = submitted_items
                .get(index)
                .map(|item| item.actual_text.clone());
            evaluate_item(task_kind, index, expected, actual)
        })
        .collect::<Vec<_>>();
    let summary = if item_results.iter().all(|result| result.status == "correct") {
        "correct"
    } else {
        "needs_review"
    }
    .to_string();

    DictationEvaluation {
        summary,
        item_results,
    }
}

fn evaluate_item(
    task_kind: DictationTaskKind,
    order_index: usize,
    expected: Option<String>,
    actual: Option<String>,
) -> DictationItemResult {
    let (status, mistake_types) = match (expected.as_deref(), actual.as_deref()) {
        (Some(expected), Some(actual)) if expected == actual => {
            ("correct", vec!["correct".to_string()])
        }
        (Some(_), Some("")) => ("missing", vec![missing_type(task_kind).to_string()]),
        (Some(expected), Some(actual)) => {
            ("incorrect", classify_mistake(task_kind, expected, actual))
        }
        (Some(_), None) => ("missing", vec![missing_type(task_kind).to_string()]),
        (None, Some(_)) => ("extra", vec![extra_type(task_kind).to_string()]),
        (None, None) => ("unclassified", vec!["unclassified".to_string()]),
    };

    DictationItemResult {
        order_index,
        expected_text: expected,
        actual_text: actual,
        status: status.to_string(),
        mistake_types,
    }
}

fn classify_mistake(task_kind: DictationTaskKind, expected: &str, actual: &str) -> Vec<String> {
    if strip_whitespace(expected) == strip_whitespace(actual) {
        return vec!["spacing_error".to_string()];
    }
    if expected.to_lowercase() == actual.to_lowercase() {
        return vec!["capitalization_error".to_string()];
    }
    if strip_punctuation(expected) == strip_punctuation(actual) {
        return vec!["punctuation_error".to_string()];
    }

    match task_kind {
        DictationTaskKind::ChineseDictation => {
            if expected.chars().count() == actual.chars().count() {
                vec!["wrong_character".to_string()]
            } else {
                vec!["unclassified".to_string()]
            }
        }
        DictationTaskKind::EnglishSpelling => classify_english_spelling(expected, actual),
        DictationTaskKind::EnglishSentenceDictation => classify_english_sentence(expected, actual),
    }
}

fn classify_english_sentence(expected: &str, actual: &str) -> Vec<String> {
    let expected_words = words(expected);
    let actual_words = words(actual);
    if expected_words.len() > actual_words.len()
        && actual_words
            .iter()
            .all(|word| expected_words.iter().any(|expected| expected == word))
    {
        return vec!["missing_word".to_string()];
    }
    if actual_words.len() > expected_words.len()
        && expected_words
            .iter()
            .all(|word| actual_words.iter().any(|actual| actual == word))
    {
        return vec!["extra_word".to_string()];
    }
    if expected_words.len() == actual_words.len()
        && same_multiset(&expected_words, &actual_words)
        && expected_words != actual_words
    {
        return vec!["word_order_error".to_string()];
    }

    classify_english_spelling(expected, actual)
}

fn classify_english_spelling(expected: &str, actual: &str) -> Vec<String> {
    if is_missing_letters(expected, actual) {
        vec!["missing_letter".to_string()]
    } else if is_missing_letters(actual, expected) {
        vec!["extra_letter".to_string()]
    } else if same_sorted_chars(expected, actual) {
        vec!["letter_order_error".to_string()]
    } else {
        vec!["unclassified".to_string()]
    }
}

fn missing_type(task_kind: DictationTaskKind) -> &'static str {
    match task_kind {
        DictationTaskKind::EnglishSentenceDictation => "missing_word",
        _ => "missing_item",
    }
}

fn extra_type(task_kind: DictationTaskKind) -> &'static str {
    match task_kind {
        DictationTaskKind::EnglishSentenceDictation => "extra_word",
        _ => "extra_item",
    }
}

fn strip_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<String>()
}

fn strip_punctuation(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric() || character.is_whitespace())
        .collect::<String>()
}

fn words(value: &str) -> Vec<String> {
    strip_punctuation(value)
        .to_lowercase()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn same_multiset(left: &[String], right: &[String]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort();
    right.sort();
    left == right
}

fn same_sorted_chars(left: &str, right: &str) -> bool {
    let mut left_chars = left.to_lowercase().chars().collect::<Vec<_>>();
    let mut right_chars = right.to_lowercase().chars().collect::<Vec<_>>();
    left_chars.sort_unstable();
    right_chars.sort_unstable();
    left_chars == right_chars
}

fn is_missing_letters(expected: &str, actual: &str) -> bool {
    let expected = expected.to_lowercase();
    let actual = actual.to_lowercase();
    if actual.chars().count() >= expected.chars().count() {
        return false;
    }

    let mut actual_chars = actual.chars();
    let mut current = actual_chars.next();
    for expected_char in expected.chars() {
        if current == Some(expected_char) {
            current = actual_chars.next();
        }
    }
    current.is_none()
}

fn validate_namespace_task_kind(
    namespace: &str,
    task_kind: DictationTaskKind,
) -> Result<(), DictationValidationError> {
    let expected = match namespace {
        "child.chinese.dictation" => DictationTaskKind::ChineseDictation,
        "child.english.spelling" => DictationTaskKind::EnglishSpelling,
        "child.english.sentence-dictation" => DictationTaskKind::EnglishSentenceDictation,
        _ => {
            return Err(DictationValidationError::new(format!(
                "unsupported dictation namespace: {namespace}"
            )));
        }
    };

    if task_kind != expected {
        return Err(DictationValidationError::new(format!(
            "dictation task_kind {task_kind} does not match namespace {namespace}"
        )));
    }

    Ok(())
}

fn validate_item_kind(
    task_kind: DictationTaskKind,
    item_kind: DictationItemKind,
    index: usize,
) -> Result<(), DictationValidationError> {
    let valid = match task_kind {
        DictationTaskKind::ChineseDictation => matches!(
            item_kind,
            DictationItemKind::ChineseCharacter
                | DictationItemKind::ChineseWord
                | DictationItemKind::ChinesePhrase
                | DictationItemKind::ChineseSentence
        ),
        DictationTaskKind::EnglishSpelling => matches!(
            item_kind,
            DictationItemKind::EnglishWord | DictationItemKind::EnglishPhrase
        ),
        DictationTaskKind::EnglishSentenceDictation => {
            matches!(item_kind, DictationItemKind::EnglishSentence)
        }
    };

    if valid {
        Ok(())
    } else {
        Err(DictationValidationError::new(format!(
            "dictation prompt_items[{index}].item_kind does not match task_kind {task_kind}"
        )))
    }
}
