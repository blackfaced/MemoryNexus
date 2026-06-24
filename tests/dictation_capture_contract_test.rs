use memorynexus::domain::{
    dictation::{
        build_dictation_capture, DictationCaptureInput, DictationItemKind, DictationSource,
        DictationTaskKind, PromptItemInput,
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
        "locator": "s3://dictation/archive/worksheet-2026-06-24.png",
        "media_type": "image/png",
        "transcript": "because\nfriend",
        "transcript_source": "agent_ocr",
        "metadata": {}
    })
}

fn english_word_list(source: DictationSource) -> DictationCaptureInput {
    DictationCaptureInput {
        namespace: "child.english.spelling".to_string(),
        task_kind: DictationTaskKind::EnglishSpelling,
        source,
        title: Some("Today's spelling words".to_string()),
        prompt_items: vec![
            PromptItemInput {
                item_kind: DictationItemKind::EnglishWord,
                expected_text: "because".to_string(),
                display_text: None,
                hint: Some("reason".to_string()),
                locale: Some("en".to_string()),
                metadata: json!({}),
            },
            PromptItemInput {
                item_kind: DictationItemKind::EnglishWord,
                expected_text: "friend".to_string(),
                display_text: None,
                hint: None,
                locale: Some("en".to_string()),
                metadata: json!({}),
            },
        ],
        input_confirmation: None,
        evidence_refs: Vec::new(),
        metadata: json!({"list_id": "monday"}),
    }
}

#[test]
fn dictation_capture_rejects_empty_word_list_with_stable_error() {
    let mut input = english_word_list(DictationSource::Typed);
    input.prompt_items.clear();

    let error = build_dictation_capture(input).unwrap_err();

    assert_eq!(error.to_string(), "dictation prompt_items cannot be empty");
}

#[test]
fn dictation_capture_represents_chinese_english_and_sentence_item_kinds() {
    let chinese = build_dictation_capture(DictationCaptureInput {
        namespace: "child.chinese.dictation".to_string(),
        task_kind: DictationTaskKind::ChineseDictation,
        source: DictationSource::Typed,
        title: None,
        prompt_items: vec![
            PromptItemInput {
                item_kind: DictationItemKind::ChineseWord,
                expected_text: "春天".to_string(),
                display_text: None,
                hint: None,
                locale: Some("zh-Hans".to_string()),
                metadata: json!({}),
            },
            PromptItemInput {
                item_kind: DictationItemKind::ChineseSentence,
                expected_text: "春天来了。".to_string(),
                display_text: None,
                hint: None,
                locale: Some("zh-Hans".to_string()),
                metadata: json!({}),
            },
        ],
        input_confirmation: None,
        evidence_refs: Vec::new(),
        metadata: json!({}),
    })
    .unwrap();
    let spelling = build_dictation_capture(english_word_list(DictationSource::Pasted)).unwrap();
    let sentence = build_dictation_capture(DictationCaptureInput {
        namespace: "child.english.sentence-dictation".to_string(),
        task_kind: DictationTaskKind::EnglishSentenceDictation,
        source: DictationSource::Typed,
        title: None,
        prompt_items: vec![PromptItemInput {
            item_kind: DictationItemKind::EnglishSentence,
            expected_text: "I have a good friend.".to_string(),
            display_text: None,
            hint: None,
            locale: Some("en".to_string()),
            metadata: json!({}),
        }],
        input_confirmation: None,
        evidence_refs: Vec::new(),
        metadata: json!({}),
    })
    .unwrap();

    assert_eq!(chinese.task_kind, DictationTaskKind::ChineseDictation);
    assert_eq!(
        chinese.prompt_items[0].item_kind,
        DictationItemKind::ChineseWord
    );
    assert_eq!(
        spelling.prompt_items[0].item_kind,
        DictationItemKind::EnglishWord
    );
    assert_eq!(
        sentence.prompt_items[0].item_kind,
        DictationItemKind::EnglishSentence
    );
}

#[test]
fn typed_and_pasted_dictation_capture_reject_media_only_fields() {
    for source in [DictationSource::Typed, DictationSource::Pasted] {
        let mut with_confirmation = english_word_list(source);
        with_confirmation.input_confirmation = Some(explicit_acceptance());
        assert_eq!(
            build_dictation_capture(with_confirmation)
                .unwrap_err()
                .to_string(),
            "typed or pasted dictation capture cannot include input_confirmation"
        );

        let mut with_evidence = english_word_list(source);
        with_evidence.evidence_refs = vec![valid_evidence_ref()];
        assert_eq!(
            build_dictation_capture(with_evidence)
                .unwrap_err()
                .to_string(),
            "typed or pasted dictation capture cannot include evidence_refs"
        );
    }
}

#[test]
fn media_derived_dictation_capture_requires_confirmed_input_confirmation() {
    for source in [
        DictationSource::AgentOcr,
        DictationSource::AgentTranscribed,
        DictationSource::Mixed,
    ] {
        let mut missing = english_word_list(source);
        missing.evidence_refs = vec![valid_evidence_ref()];
        assert_eq!(
            build_dictation_capture(missing).unwrap_err().to_string(),
            "input_confirmation is required for media-derived dictation capture"
        );

        let mut invalid = english_word_list(source);
        invalid.input_confirmation = Some(InputConfirmation {
            status: InputConfirmationStatus::Unknown,
            method: InputConfirmationMethod::ExplicitAcceptance,
        });
        assert_eq!(
            build_dictation_capture(invalid).unwrap_err().to_string(),
            "input_confirmation must be confirmed by explicit acceptance or correction"
        );
    }
}

#[test]
fn media_derived_dictation_capture_accepts_validated_descriptors_request_locally() {
    let mut input = english_word_list(DictationSource::AgentOcr);
    input.input_confirmation = Some(explicit_acceptance());
    input.evidence_refs = vec![valid_evidence_ref()];

    let capture = build_dictation_capture(input).unwrap();

    assert_eq!(capture.source, DictationSource::AgentOcr);
    assert_eq!(capture.prompt_items.len(), 2);
    assert_eq!(capture.canonical_text, "because\nfriend");
    assert_eq!(capture.evidence_ref_count, 1);
    assert_eq!(capture.persistence_metadata.get("evidence_refs"), None);
    assert_eq!(capture.persistence_metadata.get("input_confirmation"), None);
}
