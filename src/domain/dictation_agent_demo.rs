use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationAgentDemoError(String);

impl DictationAgentDemoError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for DictationAgentDemoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DictationAgentDemoError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationAgentToolCall {
    pub tool_name: String,
    pub product_action: String,
    pub surface: String,
    pub action: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationAgentDemo {
    pub tool_calls: Vec<DictationAgentToolCall>,
    pub generated_trace_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaPromptDecision {
    Accepted,
    Corrected(String),
}

pub fn build_minimal_dictation_agent_demo(
    fixture: &Value,
) -> Result<DictationAgentDemo, DictationAgentDemoError> {
    let namespace = required_str(fixture, "namespace")?;
    let actor = required_str(fixture, "actor")?;
    let space_id = required_str(fixture, "space_id")?;
    let locale = required_str(fixture, "locale")?;
    let word_list = required_object(fixture, "word_list")?;
    let attempt = required_object(fixture, "attempt")?;
    let review = required_object(fixture, "review")?;
    let planning = required_object(fixture, "planning")?;
    let observation = required_object(fixture, "observation")?;

    let capture_payload = json!({
        "space_id": space_id,
        "source": required_str(word_list, "source")?,
        "task_kind": required_str(word_list, "task_kind")?,
        "title": required_str(word_list, "title")?,
        "prompt_items": required_array(word_list, "prompt_items")?,
    });
    validate_text_first_payload(&capture_payload)?;

    let attempt_payload = json!({
        "space_id": space_id,
        "source": required_str(attempt, "source")?,
        "task_id": required_str(attempt, "task_id")?,
        "submitted_items": required_array(attempt, "submitted_items")?,
    });
    validate_text_first_payload(&attempt_payload)?;

    let review_payload = json!({
        "space_id": space_id,
        "attempt_id": required_str(review, "attempt_id")?,
        "evaluation_id": required_str(review, "evaluation_id")?,
        "timeframe": required_str(review, "timeframe")?,
        "question": "Explain the spelling mistake pattern from today's dictation result",
    });

    let planning_payload = json!({
        "space_id": space_id,
        "target_date": required_str(planning, "target_date")?,
        "duration_minutes": required_u64(planning, "duration_minutes")?,
        "objective": "Prepare tomorrow's focused 10-minute spelling practice",
    });

    let observation_payload = json!({
        "space_id": space_id,
        "timeframe": required_str(observation, "timeframe")?,
        "summary_goal": "Show recent dictation trend and current focus",
    });

    let tool_calls = vec![
        SurfaceCallTemplate {
            tool_name: "surface_capture_observation",
            product_action: "record today's list",
            surface: "capture",
            action: "capture_observation",
            namespace,
            actor,
            payload: capture_payload,
            context: context("fast", locale),
        }
        .into_tool_call(),
        SurfaceCallTemplate {
            tool_name: "surface_submit_attempt",
            product_action: "submit a dictation result",
            surface: "performance",
            action: "submit_attempt",
            namespace,
            actor,
            payload: attempt_payload,
            context: context("fast", locale),
        }
        .into_tool_call(),
        SurfaceCallTemplate {
            tool_name: "surface_review_evidence",
            product_action: "explain mistake patterns",
            surface: "reflection",
            action: "review_evidence",
            namespace,
            actor,
            payload: review_payload,
            context: context("focused", locale),
        }
        .into_tool_call(),
        SurfaceCallTemplate {
            tool_name: "surface_generate_next_task",
            product_action: "prepare tomorrow's focused practice",
            surface: "planning",
            action: "generate_next_task",
            namespace,
            actor,
            payload: planning_payload,
            context: context("focused", locale),
        }
        .into_tool_call(),
        SurfaceCallTemplate {
            tool_name: "surface_get_state_summary",
            product_action: "show a recent trend",
            surface: "observation",
            action: "get_state_summary",
            namespace,
            actor,
            payload: observation_payload,
            context: context("focused", locale),
        }
        .into_tool_call(),
    ];

    Ok(DictationAgentDemo {
        tool_calls,
        generated_trace_ids: generated_trace_ids(fixture)?,
    })
}

pub fn map_media_prompt_confirmation(
    source: &str,
    normalized_text: &str,
    decision: MediaPromptDecision,
) -> Result<Value, DictationAgentDemoError> {
    if !matches!(source, "agent_ocr" | "agent_transcribed" | "mixed") {
        return Err(DictationAgentDemoError::new(
            "media prompt confirmation requires a media-derived source",
        ));
    }

    let (confirmed_text, method) = match decision {
        MediaPromptDecision::Accepted => (normalized_text.to_string(), "explicit_acceptance"),
        MediaPromptDecision::Corrected(corrected_text) => (corrected_text, "explicit_correction"),
    };

    Ok(json!({
        "source": source,
        "confirmed_text": confirmed_text,
        "input_confirmation": {
            "status": "confirmed",
            "method": method
        }
    }))
}

struct SurfaceCallTemplate<'a> {
    tool_name: &'a str,
    product_action: &'a str,
    surface: &'a str,
    action: &'a str,
    namespace: &'a str,
    actor: &'a str,
    payload: Value,
    context: Value,
}

impl SurfaceCallTemplate<'_> {
    fn into_tool_call(self) -> DictationAgentToolCall {
        DictationAgentToolCall {
            tool_name: self.tool_name.to_string(),
            product_action: self.product_action.to_string(),
            surface: self.surface.to_string(),
            action: self.action.to_string(),
            arguments: json!({
                "namespace": self.namespace,
                "actor": self.actor,
                "payload": self.payload,
                "context": self.context,
            }),
        }
    }
}

fn context(mode: &str, locale: &str) -> Value {
    json!({
        "mode": mode,
        "locale": locale,
        "device": "agent",
        "runtime_preference": "deterministic"
    })
}

fn generated_trace_ids(fixture: &Value) -> Result<Vec<String>, DictationAgentDemoError> {
    let responses = required_array(fixture, "surface_responses")?;
    responses
        .iter()
        .enumerate()
        .map(|(index, response)| {
            response
                .get("generated_trace_id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| {
                    DictationAgentDemoError::new(format!(
                        "surface_responses[{index}].generated_trace_id is required"
                    ))
                })
        })
        .collect()
}

fn validate_text_first_payload(payload: &Value) -> Result<(), DictationAgentDemoError> {
    let source = payload
        .get("source")
        .and_then(Value::as_str)
        .ok_or_else(|| DictationAgentDemoError::new("text-first payload source is required"))?;
    if !matches!(source, "typed" | "pasted") {
        return Err(DictationAgentDemoError::new(
            "initial dictation agent demo only accepts typed or pasted sources",
        ));
    }
    reject_media_only_fields(payload)
}

fn reject_media_only_fields(value: &Value) -> Result<(), DictationAgentDemoError> {
    const MEDIA_ONLY_FIELDS: &[&str] = &[
        "evidence_refs",
        "input_confirmation",
        "provider",
        "locator",
        "media_type",
        "content_hash",
        "original_name",
        "captured_at",
        "transcript",
        "transcript_source",
        "media_descriptor",
        "media_provenance",
    ];

    match value {
        Value::Object(object) => {
            for field in MEDIA_ONLY_FIELDS {
                if object.contains_key(*field) {
                    return Err(DictationAgentDemoError::new(format!(
                        "typed/pasted dictation agent payload cannot include {field}"
                    )));
                }
            }
            for nested in object.values() {
                reject_media_only_fields(nested)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                reject_media_only_fields(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn required_object<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a Value, DictationAgentDemoError> {
    value
        .get(field)
        .filter(|nested| nested.is_object())
        .ok_or_else(|| DictationAgentDemoError::new(format!("{field} object is required")))
}

fn required_array<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a Vec<Value>, DictationAgentDemoError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| DictationAgentDemoError::new(format!("{field} array is required")))
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str, DictationAgentDemoError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| DictationAgentDemoError::new(format!("{field} string is required")))
}

fn required_u64(value: &Value, field: &str) -> Result<u64, DictationAgentDemoError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| DictationAgentDemoError::new(format!("{field} number is required")))
}
