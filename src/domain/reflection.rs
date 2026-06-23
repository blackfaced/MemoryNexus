use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRefKind {
    Trace,
    Memory,
    FeedbackLoop,
    ReviewReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub kind: EvidenceRefKind,
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionEvidence {
    pub source: EvidenceRef,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectionRequest {
    pub space_id: Uuid,
    pub namespace_id: Uuid,
    pub namespace: String,
    pub question: Option<String>,
    pub evidence: Vec<ReflectionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionEvidenceSummary {
    pub source: EvidenceRef,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionInsight {
    pub status: String,
    pub space_id: Uuid,
    pub namespace_id: Uuid,
    pub namespace: String,
    pub evidence_count: usize,
    pub confidence: String,
    pub summary: String,
    pub evidence_summaries: Vec<ReflectionEvidenceSummary>,
    pub explanation: String,
}

pub fn build_reflection_insight(request: &ReflectionRequest) -> ReflectionInsight {
    let evidence_summaries = request
        .evidence
        .iter()
        .filter_map(|evidence| {
            let summary = compact_whitespace(&evidence.summary);
            if summary.is_empty() {
                None
            } else {
                Some(ReflectionEvidenceSummary {
                    source: evidence.source.clone(),
                    summary,
                })
            }
        })
        .collect::<Vec<_>>();

    let evidence_count = evidence_summaries.len();
    let (status, confidence, summary) = if evidence_count == 0 {
        (
            "insufficient_evidence",
            "none",
            format!(
                "No evidence was provided for reflection in {}.",
                request.namespace
            ),
        )
    } else {
        (
            "insight_ready",
            confidence_for_evidence_count(evidence_count),
            format!(
                "{}: reviewing {evidence_count} evidence items in {}: {}",
                question_focus(request.question.as_deref()),
                request.namespace,
                evidence_summaries[0].summary
            ),
        )
    };

    ReflectionInsight {
        status: status.to_string(),
        space_id: request.space_id,
        namespace_id: request.namespace_id,
        namespace: request.namespace.clone(),
        evidence_count,
        confidence: confidence.to_string(),
        summary,
        evidence_summaries,
        explanation: "This is a deterministic reflection over provided evidence only; no additional memory, lens projection, or model inference was used.".to_string(),
    }
}

fn confidence_for_evidence_count(evidence_count: usize) -> &'static str {
    match evidence_count {
        0 => "none",
        1 => "low",
        _ => "medium",
    }
}

fn compact_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn question_focus(question: Option<&str>) -> String {
    let question = question.map(compact_whitespace).unwrap_or_default();
    if question.is_empty() {
        "Review evidence".to_string()
    } else {
        question.trim_end_matches(['.', '?', '!']).to_string()
    }
}
