use memorynexus::domain::{
    dictation::{
        build_dictation_attempt, DictationAttemptInput, DictationItemKind, DictationSource,
        DictationTaskKind, PromptItemInput, SubmittedItemInput,
    },
    evidence::{InputConfirmation, InputConfirmationMethod, InputConfirmationStatus},
};
use serde_json::json;

fn explicit_acceptance() -> InputConfirmation {
    InputConfirmation {
        status: InputConfirmationStatus::Confirmed,
        method: InputConfirmationMethod::ExplicitAcceptance,
    }
}

fn valid_evidence_ref() -> serde_json::Value {
    json!({
        "provider": "agent_ocr",
        "locator": "s3://dictation/archive/attempt-2026-06-24.png",
        "media_type": "image/png",
        "transcript": "becaus",
        "transcript_source": "agent_ocr",
        "metadata": {"worksheet": "monday"}
    })
}

fn english_spelling_attempt(source: DictationSource) -> DictationAttemptInput {
    DictationAttemptInput {
        namespace: "child.english.spelling".to_string(),
        task_kind: DictationTaskKind::EnglishSpelling,
        source,
        task: Some("Today's spelling words".to_string()),
        goal: Some("Practice child.english.spelling".to_string()),
        prompt_items: vec![PromptItemInput {
            item_kind: DictationItemKind::EnglishWord,
            expected_text: "because".to_string(),
            display_text: None,
            hint: None,
            locale: Some("en".to_string()),
            metadata: json!({}),
        }],
        submitted_items: vec![SubmittedItemInput {
            actual_text: "becaus".to_string(),
            metadata: json!({}),
        }],
        input_confirmation: None,
        evidence_refs: Vec::new(),
        metadata: json!({"session": "monday"}),
    }
}

#[test]
fn dictation_attempt_builds_sanitized_summary_and_evaluation() {
    let attempt = build_dictation_attempt(english_spelling_attempt(DictationSource::Typed))
        .expect("typed spelling attempt should build");

    assert_eq!(attempt.namespace, "child.english.spelling");
    assert_eq!(attempt.task_kind, DictationTaskKind::EnglishSpelling);
    assert_eq!(attempt.prompt_items[0].expected_text, "because");
    assert_eq!(attempt.submitted_items[0].actual_text, "becaus");
    assert_eq!(attempt.summary, "because -> becaus");
    assert_eq!(attempt.evaluation.summary, "needs_review");
    assert_eq!(attempt.evaluation.item_results[0].status, "incorrect");
    assert_eq!(
        attempt.evaluation.item_results[0].mistake_types,
        vec!["missing_letter"]
    );
    assert_eq!(attempt.persistence_metadata.get("evidence_refs"), None);
    assert_eq!(attempt.persistence_metadata.get("input_confirmation"), None);
}

#[test]
fn dictation_attempt_represents_chinese_words_and_english_sentences() {
    let chinese = build_dictation_attempt(DictationAttemptInput {
        namespace: "child.chinese.dictation".to_string(),
        task_kind: DictationTaskKind::ChineseDictation,
        source: DictationSource::Typed,
        task: None,
        goal: None,
        prompt_items: vec![PromptItemInput {
            item_kind: DictationItemKind::ChineseWord,
            expected_text: "春天".to_string(),
            display_text: None,
            hint: None,
            locale: Some("zh-Hans".to_string()),
            metadata: json!({}),
        }],
        submitted_items: vec![SubmittedItemInput {
            actual_text: "春大".to_string(),
            metadata: json!({}),
        }],
        input_confirmation: None,
        evidence_refs: Vec::new(),
        metadata: json!({}),
    })
    .unwrap();
    let sentence = build_dictation_attempt(DictationAttemptInput {
        namespace: "child.english.sentence-dictation".to_string(),
        task_kind: DictationTaskKind::EnglishSentenceDictation,
        source: DictationSource::Typed,
        task: None,
        goal: None,
        prompt_items: vec![PromptItemInput {
            item_kind: DictationItemKind::EnglishSentence,
            expected_text: "I have a good friend.".to_string(),
            display_text: None,
            hint: None,
            locale: Some("en".to_string()),
            metadata: json!({}),
        }],
        submitted_items: vec![SubmittedItemInput {
            actual_text: "I have good friend.".to_string(),
            metadata: json!({}),
        }],
        input_confirmation: None,
        evidence_refs: Vec::new(),
        metadata: json!({}),
    })
    .unwrap();

    assert_eq!(
        chinese.evaluation.item_results[0].mistake_types,
        vec!["wrong_character"]
    );
    assert_eq!(
        sentence.evaluation.item_results[0].mistake_types,
        vec!["missing_word"]
    );
}

#[test]
fn typed_and_pasted_dictation_attempt_reject_media_only_fields() {
    for source in [DictationSource::Typed, DictationSource::Pasted] {
        let mut with_confirmation = english_spelling_attempt(source);
        with_confirmation.input_confirmation = Some(explicit_acceptance());
        assert_eq!(
            build_dictation_attempt(with_confirmation)
                .unwrap_err()
                .to_string(),
            "typed or pasted dictation attempt cannot include input_confirmation"
        );

        let mut with_evidence = english_spelling_attempt(source);
        with_evidence.evidence_refs = vec![valid_evidence_ref()];
        assert_eq!(
            build_dictation_attempt(with_evidence)
                .unwrap_err()
                .to_string(),
            "typed or pasted dictation attempt cannot include evidence_refs"
        );
    }
}

#[test]
fn media_derived_dictation_attempt_accepts_confirmed_methods_without_persisting_descriptors() {
    for method in [
        InputConfirmationMethod::ExplicitAcceptance,
        InputConfirmationMethod::ExplicitCorrection,
    ] {
        let mut input = english_spelling_attempt(DictationSource::AgentOcr);
        input.input_confirmation = Some(InputConfirmation {
            status: InputConfirmationStatus::Confirmed,
            method,
        });
        input.evidence_refs = vec![valid_evidence_ref()];

        let attempt = build_dictation_attempt(input).expect("confirmed media attempt should build");

        assert_eq!(attempt.evidence_ref_count, 1);
        assert_eq!(attempt.persistence_metadata.get("evidence_refs"), None);
        assert_eq!(attempt.persistence_metadata.get("input_confirmation"), None);
        assert!(!attempt
            .persistence_metadata
            .to_string()
            .contains("attempt-2026-06-24"));
        assert!(!attempt
            .persistence_metadata
            .to_string()
            .contains("worksheet"));
    }
}

#[test]
fn media_derived_dictation_attempt_rejects_missing_or_invalid_confirmation() {
    for source in [
        DictationSource::AgentOcr,
        DictationSource::AgentTranscribed,
        DictationSource::Mixed,
    ] {
        let mut missing = english_spelling_attempt(source);
        missing.evidence_refs = vec![valid_evidence_ref()];
        assert_eq!(
            build_dictation_attempt(missing).unwrap_err().to_string(),
            "input_confirmation is required for media-derived dictation attempt"
        );

        let mut invalid = english_spelling_attempt(source);
        invalid.input_confirmation = Some(InputConfirmation {
            status: InputConfirmationStatus::Unknown,
            method: InputConfirmationMethod::ExplicitAcceptance,
        });
        assert_eq!(
            build_dictation_attempt(invalid).unwrap_err().to_string(),
            "input_confirmation must be confirmed by explicit acceptance or correction"
        );
    }
}
