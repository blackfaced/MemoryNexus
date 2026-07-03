use memorynexus::domain::reflection::{
    build_reflection_insight, EvidenceRef, EvidenceRefKind, ReflectionEvidence, ReflectionRequest,
};
use memorynexus::domain::LensStrategyRef;
use serde_json::json;
use uuid::Uuid;

#[test]
fn reflection_insight_is_stable_and_evidence_backed() {
    let space_id = Uuid::nil();
    let namespace_id = Uuid::from_u128(1);
    let request = ReflectionRequest {
        space_id,
        namespace_id,
        namespace: "child.english.spelling".to_string(),
        lens_strategy: None,
        question: Some("Explain what this means".to_string()),
        evidence: vec![
            ReflectionEvidence {
                source: EvidenceRef {
                    kind: EvidenceRefKind::Trace,
                    id: Uuid::from_u128(11),
                },
                summary: "Target: because\nSubmitted: becuase".to_string(),
            },
            ReflectionEvidence {
                source: EvidenceRef {
                    kind: EvidenceRefKind::FeedbackLoop,
                    id: Uuid::from_u128(12),
                },
                summary:
                    "The attempt needs review because the submitted spelling changes letter order."
                        .to_string(),
            },
        ],
    };

    let first = build_reflection_insight(&request);
    let second = build_reflection_insight(&request);

    assert_eq!(first, second);
    assert_eq!(first.status, "insight_ready");
    assert_eq!(first.space_id, space_id);
    assert_eq!(first.namespace_id, namespace_id);
    assert_eq!(first.namespace, "child.english.spelling");
    assert_eq!(first.evidence_count, 2);
    assert_eq!(first.confidence, "medium");
    assert_eq!(
        first.summary,
        "Explain what this means: reviewing 2 evidence items in child.english.spelling: Target: because Submitted: becuase"
    );
    assert_eq!(
        first.explanation,
        "This is a deterministic reflection over provided evidence only; no additional memory, lens projection, or model inference was used."
    );

    let serialized = serde_json::to_value(&first).unwrap();
    assert_eq!(
        serialized,
        json!({
            "status": "insight_ready",
            "space_id": space_id,
            "namespace_id": namespace_id,
            "namespace": "child.english.spelling",
            "lens_strategy": null,
            "evidence_count": 2,
            "confidence": "medium",
            "summary": "Explain what this means: reviewing 2 evidence items in child.english.spelling: Target: because Submitted: becuase",
            "evidence_summaries": [
                {
                    "source": {
                        "kind": "trace",
                        "id": Uuid::from_u128(11)
                    },
                    "summary": "Target: because Submitted: becuase"
                },
                {
                    "source": {
                        "kind": "feedback_loop",
                        "id": Uuid::from_u128(12)
                    },
                    "summary": "The attempt needs review because the submitted spelling changes letter order."
                }
            ],
            "explanation": "This is a deterministic reflection over provided evidence only; no additional memory, lens projection, or model inference was used."
        })
    );
}

#[test]
fn reflection_question_changes_deterministic_summary() {
    let base = ReflectionRequest {
        space_id: Uuid::nil(),
        namespace_id: Uuid::from_u128(1),
        namespace: "child.english.spelling".to_string(),
        lens_strategy: None,
        question: Some("Review the mistake pattern".to_string()),
        evidence: vec![ReflectionEvidence {
            source: EvidenceRef {
                kind: EvidenceRefKind::Trace,
                id: Uuid::from_u128(11),
            },
            summary: "Target: because Submitted: becuase".to_string(),
        }],
    };
    let mut alternate = base.clone();
    alternate.question = Some("Explain the next coaching focus".to_string());

    assert_ne!(
        build_reflection_insight(&base).summary,
        build_reflection_insight(&alternate).summary
    );
}

#[test]
fn reflection_insight_handles_missing_evidence_without_fabricated_certainty() {
    let insight = build_reflection_insight(&ReflectionRequest {
        space_id: Uuid::nil(),
        namespace_id: Uuid::from_u128(1),
        namespace: "personal.thoughts".to_string(),
        lens_strategy: None,
        question: None,
        evidence: Vec::new(),
    });

    assert_eq!(insight.status, "insufficient_evidence");
    assert_eq!(insight.evidence_count, 0);
    assert_eq!(insight.confidence, "none");
    assert_eq!(
        insight.summary,
        "No evidence was provided for reflection in personal.thoughts."
    );
    assert!(insight.evidence_summaries.is_empty());
}

#[test]
fn lens_strategy_ref_serializes_with_stable_name() {
    let lens_strategy = LensStrategyRef::new("project_context");

    assert_eq!(lens_strategy.name(), "project_context");
    assert_eq!(
        serde_json::to_value(&lens_strategy).unwrap(),
        json!({ "name": "project_context" })
    );
}

#[test]
fn reflection_request_and_response_can_carry_lens_strategy_ref() {
    let lens_strategy = LensStrategyRef::new("learning_review");
    let request = ReflectionRequest {
        space_id: Uuid::nil(),
        namespace_id: Uuid::from_u128(1),
        namespace: "child.english.spelling".to_string(),
        lens_strategy: Some(lens_strategy.clone()),
        question: Some("Review the mistake pattern".to_string()),
        evidence: vec![ReflectionEvidence {
            source: EvidenceRef {
                kind: EvidenceRefKind::Trace,
                id: Uuid::from_u128(11),
            },
            summary: "Target: because Submitted: becuase".to_string(),
        }],
    };

    let insight = build_reflection_insight(&request);

    assert_eq!(insight.lens_strategy, Some(lens_strategy));
    assert_eq!(
        insight.summary,
        "Review the mistake pattern: reviewing 1 evidence items in child.english.spelling: Target: because Submitted: becuase"
    );
    assert_eq!(
        serde_json::to_value(&insight).unwrap()["lens_strategy"],
        json!({ "name": "learning_review" })
    );
}
