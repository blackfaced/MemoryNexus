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
    )?;

    if input.prompt_items.is_empty() {
        return Err(DictationValidationError::new(
            "dictation prompt_items cannot be empty",
        ));
    }

    let mut prompt_items = Vec::with_capacity(input.prompt_items.len());
    for (index, item) in input.prompt_items.into_iter().enumerate() {
        let expected_text = item.expected_text.trim().to_string();
        if expected_text.is_empty() {
            return Err(DictationValidationError::new(format!(
                "dictation prompt_items[{index}].expected_text cannot be empty"
            )));
        }
        validate_item_kind(input.task_kind, item.item_kind, index)?;
        prompt_items.push(DictationPromptItem {
            order_index: index,
            item_kind: item.item_kind,
            expected_text,
            display_text: item.display_text,
            hint: item.hint,
            locale: item.locale,
            metadata: item.metadata,
        });
    }

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

fn validate_source_fields(
    source: DictationSource,
    input_confirmation: Option<&InputConfirmation>,
    evidence_refs: &[Value],
) -> Result<(), DictationValidationError> {
    if source.is_manual() {
        if input_confirmation.is_some() {
            return Err(DictationValidationError::new(
                "typed or pasted dictation capture cannot include input_confirmation",
            ));
        }
        if !evidence_refs.is_empty() {
            return Err(DictationValidationError::new(
                "typed or pasted dictation capture cannot include evidence_refs",
            ));
        }
        return Ok(());
    }

    if input_confirmation.is_none() {
        return Err(DictationValidationError::new(
            "input_confirmation is required for media-derived dictation capture",
        ));
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
