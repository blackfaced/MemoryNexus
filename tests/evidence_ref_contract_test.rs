use memorynexus::domain::evidence::{
    validate_evidence_request, EvidenceValidationCode, InputConfirmation, InputConfirmationMethod,
    InputConfirmationStatus,
};
use serde_json::{json, Value};

fn valid_ref() -> Value {
    json!({
        "provider": "agent_ocr",
        "locator": "s3://study/archive/token-guidelines.pdf",
        "media_type": "image/png",
        "content_hash": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "original_name": "dictation.png",
        "captured_at": "2026-06-18T03:10:00Z",
        "transcript": "because friend enough",
        "transcript_source": "agent_ocr",
        "metadata": {
            "page": 2,
            "label": "weekly review",
            "source": "teacher notes"
        }
    })
}

fn valid_refs() -> Vec<Value> {
    vec![valid_ref()]
}

fn expect_invalid(reference: Value, code: EvidenceValidationCode, path: &str) {
    let err = validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap_err();
    assert_eq!(err.code, code);
    assert_eq!(err.path, path);
}

fn explicit_acceptance() -> InputConfirmation {
    InputConfirmation {
        status: InputConfirmationStatus::Confirmed,
        method: InputConfirmationMethod::ExplicitAcceptance,
    }
}

#[test]
fn evidence_refs_are_optional_and_empty_lists_are_accepted() {
    validate_evidence_request(None, None, None).unwrap();
    validate_evidence_request(Some("typed"), Some(&explicit_acceptance()), Some(&[])).unwrap();
}

#[test]
fn valid_provider_neutral_descriptor_is_accepted() {
    validate_evidence_request(
        Some("agent_ocr"),
        Some(&explicit_acceptance()),
        Some(&valid_refs()),
    )
    .unwrap();
}

#[test]
fn media_derived_sources_require_confirmed_input_confirmation() {
    for source in ["agent_ocr", "agent_transcribed", "mixed"] {
        let err = validate_evidence_request(Some(source), None, Some(&valid_refs())).unwrap_err();
        assert_eq!(err.code, EvidenceValidationCode::InputConfirmationRequired);
        assert_eq!(err.path, "input_confirmation");
    }

    let correction = InputConfirmation {
        status: InputConfirmationStatus::Confirmed,
        method: InputConfirmationMethod::ExplicitCorrection,
    };
    validate_evidence_request(Some("mixed"), Some(&correction), Some(&valid_refs())).unwrap();
}

#[test]
fn input_confirmation_rejects_unconfirmed_status_and_invalid_method() {
    for raw in [
        json!({"status": "pending", "method": "explicit_acceptance"}),
        json!({"status": "confirmed", "method": "implicit_acceptance"}),
    ] {
        let confirmation: InputConfirmation = serde_json::from_value(raw).unwrap();
        let err =
            validate_evidence_request(Some("agent_ocr"), Some(&confirmation), Some(&valid_refs()))
                .unwrap_err();
        assert_eq!(err.code, EvidenceValidationCode::InvalidInputConfirmation);
        assert_eq!(err.path, "input_confirmation");
    }
}

#[test]
fn caller_supplied_ownership_fields_are_rejected() {
    for field in ["id", "space_id"] {
        let mut reference = valid_ref();
        reference[field] = json!("caller-owned");
        let err = validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap_err();
        assert_eq!(err.code, EvidenceValidationCode::CallerOwnershipFieldDenied);
        assert_eq!(err.path, format!("evidence_refs[0].{field}"));
    }
}

#[test]
fn required_fields_are_enforced() {
    for field in ["provider", "locator", "media_type", "metadata"] {
        let mut reference = valid_ref();
        reference.as_object_mut().unwrap().remove(field);
        let err = validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap_err();
        assert_eq!(err.code, EvidenceValidationCode::RequiredFieldMissing);
        assert_eq!(err.path, format!("evidence_refs[0].{field}"));
    }
}

#[test]
fn provider_and_transcript_source_use_closed_identifier_syntax() {
    for value in ["a".to_string(), format!("a{}", "2".repeat(63))] {
        let mut reference = valid_ref();
        reference["provider"] = json!(value.clone());
        reference["transcript_source"] = json!(value);
        validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap();
    }

    for (field, value) in [
        ("provider", "A".to_string()),
        ("provider", "_agent".to_string()),
        ("provider", format!("a{}", "2".repeat(64))),
        ("transcript_source", "Agent".to_string()),
        ("transcript_source", format!("a{}", "2".repeat(64))),
    ] {
        let mut reference = valid_ref();
        reference[field] = json!(value);
        let err = validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap_err();
        assert_eq!(err.code, EvidenceValidationCode::InvalidIdentifier);
        assert_eq!(err.path, format!("evidence_refs[0].{field}"));
    }
}

#[test]
fn locator_limits_encoding_and_credential_bearing_parts() {
    let long = "a".repeat(4097);
    for (locator, code, path) in [
        (
            "",
            EvidenceValidationCode::LocatorEmpty,
            "evidence_refs[0].locator",
        ),
        (
            long.as_str(),
            EvidenceValidationCode::LocatorTooLarge,
            "evidence_refs[0].locator",
        ),
        (
            "https://example.test/a\nb",
            EvidenceValidationCode::ControlCharacterDenied,
            "evidence_refs[0].locator",
        ),
        (
            "https://user:secret@example.test/media/1",
            EvidenceValidationCode::LocatorUserinfoDenied,
            "evidence_refs[0].locator",
        ),
        (
            "data:image/png;base64,AAAA",
            EvidenceValidationCode::InlineBytesDenied,
            "evidence_refs[0].locator",
        ),
        (
            "https://example.test/media?X-Amz-Signature=fixture",
            EvidenceValidationCode::LocatorQueryDenied,
            "evidence_refs[0].locator.query.X-Amz-Signature",
        ),
        (
            "https://example.test/media#access_token=fixture",
            EvidenceValidationCode::LocatorQueryDenied,
            "evidence_refs[0].locator.fragment.access_token",
        ),
        (
            "https://example.test/media?note=%ZZ",
            EvidenceValidationCode::InvalidPercentEncoding,
            "evidence_refs[0].locator.query.note",
        ),
        (
            "https://example.test/media?note=Bearer%20secret",
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].locator.query.note",
        ),
        (
            "https://example.test/media#note=eyJmaXh0dXJlIjoxfQ.cGF5bG9hZA.c2lnbmF0dXJl",
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].locator.fragment.note",
        ),
    ] {
        let mut reference = valid_ref();
        reference["locator"] = json!(locator);
        let err = validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap_err();
        assert_eq!(err.code, code);
        assert_eq!(err.path, path);
        assert!(!err.to_string().contains("fixture-password"));
        assert!(!err.to_string().contains("Bearer secret"));
        assert!(!err.to_string().contains("fixture"));
    }

    let mut accepted = valid_ref();
    accepted["locator"] =
        json!("https://example.test/api_key/access_token_notes.txt?version=3#page=2");
    validate_evidence_request(Some("typed"), None, Some(&[accepted])).unwrap();
}

#[test]
fn locator_policy_enumerates_closed_signed_keys_value_prefixes_and_percent_decoding() {
    for key in [
        "password",
        "passwd",
        "secret",
        "client_secret",
        "token",
        "access-token",
        "refresh.token",
        "api_key",
        "authorization",
        "cookie",
        "session",
        "credential",
        "private_key",
        "mount-secret",
        "x-amz-algorithm",
        "X-Amz-Credential",
        "x_amz_signature",
        "x.amz.security.token",
        "x-goog-algorithm",
        "X-Goog-Credential",
        "x_goog_signature",
        "signature",
        "sig",
        "expires",
    ] {
        for separator in ['?', '#'] {
            let mut reference = valid_ref();
            reference["locator"] = json!(format!(
                "https://example.test/media{separator}{key}=fixture"
            ));
            let component = if separator == '?' {
                "query"
            } else {
                "fragment"
            };
            expect_invalid(
                reference,
                EvidenceValidationCode::LocatorQueryDenied,
                &format!("evidence_refs[0].locator.{component}.{key}"),
            );
        }
    }

    for value in [
        "Bearer%20secret",
        "-----BEGIN%20PRIVATE%20KEY-----%20fixture",
        "eyJmaXh0dXJlIjoxfQ.cGF5bG9hZA.c2lnbmF0dXJl",
        "sk-test",
        "AKIAfixture",
        "AIzafixture",
        "ghp_fixture",
    ] {
        let mut reference = valid_ref();
        reference["locator"] = json!(format!("https://example.test/media?note={value}"));
        expect_invalid(
            reference,
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].locator.query.note",
        );
    }

    let mut double_encoded = valid_ref();
    double_encoded["locator"] = json!("https://example.test/media?X%252DAmz%252DSignature=fixture");
    validate_evidence_request(Some("typed"), None, Some(&[double_encoded])).unwrap();

    let mut invalid_utf8 = valid_ref();
    invalid_utf8["locator"] = json!("https://example.test/media?note=%FF");
    expect_invalid(
        invalid_utf8,
        EvidenceValidationCode::InvalidPercentEncoding,
        "evidence_refs[0].locator.query.note",
    );

    let mut serialized_too_large = valid_ref();
    serialized_too_large["locator"] = json!("\\".repeat(4096));
    expect_invalid(
        serialized_too_large,
        EvidenceValidationCode::LocatorTooLarge,
        "evidence_refs[0].locator",
    );
}

#[test]
fn media_type_hash_name_timestamp_and_transcript_rules_are_enforced() {
    let cases = [
        (
            "media_type",
            json!("Image/PNG"),
            EvidenceValidationCode::InvalidMediaType,
        ),
        (
            "media_type",
            json!("image/png; charset=utf-8"),
            EvidenceValidationCode::InvalidMediaType,
        ),
        (
            "content_hash",
            json!("sha1:abc"),
            EvidenceValidationCode::InvalidContentHash,
        ),
        (
            "content_hash",
            json!("sha256:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef"),
            EvidenceValidationCode::InvalidContentHash,
        ),
        (
            "original_name",
            json!("folder/image.png"),
            EvidenceValidationCode::InvalidOriginalName,
        ),
        (
            "original_name",
            json!("bad\nname.png"),
            EvidenceValidationCode::ControlCharacterDenied,
        ),
        (
            "captured_at",
            json!("2026-06-18T11:10:00+08:00"),
            EvidenceValidationCode::InvalidTimestamp,
        ),
        (
            "transcript",
            json!("x".repeat(65_537)),
            EvidenceValidationCode::TranscriptTooLarge,
        ),
    ];

    for (field, value, code) in cases {
        let mut reference = valid_ref();
        reference[field] = value;
        let err = validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap_err();
        assert_eq!(err.code, code);
        assert_eq!(err.path, format!("evidence_refs[0].{field}"));
    }
}

#[test]
fn field_boundary_values_are_enforced() {
    let mut reference = valid_ref();
    reference["locator"] = json!("a".repeat(4096));
    validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap();

    let mut reference = valid_ref();
    reference["media_type"] = json!(format!("{}/{}", "a".repeat(127), "b".repeat(127)));
    validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap();

    let mut reference = valid_ref();
    reference["media_type"] = json!(format!("{}/{}", "a".repeat(128), "b".repeat(127)));
    expect_invalid(
        reference,
        EvidenceValidationCode::InvalidMediaType,
        "evidence_refs[0].media_type",
    );

    let mut reference = valid_ref();
    reference["original_name"] = json!("a".repeat(255));
    validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap();

    let mut reference = valid_ref();
    reference["original_name"] = json!("a".repeat(256));
    expect_invalid(
        reference,
        EvidenceValidationCode::InvalidOriginalName,
        "evidence_refs[0].original_name",
    );

    let mut reference = valid_ref();
    reference["transcript"] = json!("x".repeat(65_536));
    validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap();

    let mut metadata_string = String::new();
    while serde_json::to_vec(&json!({"value": metadata_string}))
        .unwrap()
        .len()
        < 16_384
    {
        metadata_string.push('a');
    }
    let mut accepted_metadata = metadata_string.clone();
    while serde_json::to_vec(&json!({"value": &accepted_metadata}))
        .unwrap()
        .len()
        > 16_384
    {
        accepted_metadata.pop();
    }
    let mut reference = valid_ref();
    reference["metadata"] = json!({"value": accepted_metadata});
    validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap();

    let mut rejected_metadata = metadata_string;
    while serde_json::to_vec(&json!({"value": &rejected_metadata}))
        .unwrap()
        .len()
        <= 16_384
    {
        rejected_metadata.push('a');
    }
    let mut reference = valid_ref();
    reference["metadata"] = json!({"value": rejected_metadata});
    expect_invalid(
        reference,
        EvidenceValidationCode::MetadataTooLarge,
        "evidence_refs[0].metadata",
    );
}

#[test]
fn metadata_shape_size_depth_key_and_secret_value_policy_are_enforced() {
    let denied = [
        (
            json!({"outer":[{"Client-Secret":"fixture-value"}]}),
            EvidenceValidationCode::SecretKeyDenied,
            "evidence_refs[0].metadata.outer[0].Client-Secret",
        ),
        (
            json!({"note":"Bearer secret"}),
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].metadata.note",
        ),
        (
            json!({"value":"sk-test"}),
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].metadata.value",
        ),
        (
            json!({"outer":[{"token_value":"eyJmaXh0dXJlIjoxfQ.cGF5bG9hZA.c2lnbmF0dXJl"}]}),
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].metadata.outer[0].token_value",
        ),
        (
            json!({"outer":[{"key_material":"-----BEGIN PRIVATE KEY----- fixture"}]}),
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].metadata.outer[0].key_material",
        ),
        (
            json!({"a":{"b":{"c":{"d": "too deep"}}}}),
            EvidenceValidationCode::MetadataTooDeep,
            "evidence_refs[0].metadata.a.b.c.d",
        ),
    ];

    for (metadata, code, path) in denied {
        let mut reference = valid_ref();
        reference["metadata"] = metadata;
        let err = validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap_err();
        assert_eq!(err.code, code);
        assert_eq!(err.path, path);
        assert!(!err.to_string().contains("Bearer secret"));
        assert!(!err.to_string().contains("sk-test"));
        assert!(!err.to_string().contains("PRIVATE KEY"));
        assert!(!err.to_string().contains("eyJmaXh0dXJl"));
    }

    for metadata in [
        json!({"page":2,"label":"weekly review","source":"teacher notes"}),
        json!({"note":"authorization awareness lesson","value":"sketch test","nested":{"summary":"safe string"}}),
        json!({"percent%5Ftoken": "sk%2Dtest"}),
    ] {
        let mut reference = valid_ref();
        reference["metadata"] = metadata;
        validate_evidence_request(Some("typed"), None, Some(&[reference])).unwrap();
    }
}

#[test]
fn metadata_policy_enumerates_closed_denied_keys_and_value_prefixes() {
    for key in [
        "password",
        "PASSWD",
        "secret",
        "Client-Secret",
        "token",
        "access_token",
        "refresh.token",
        "api-key",
        "authorization",
        "cookie",
        "session",
        "credential",
        "private_key",
        "mount.secret",
    ] {
        let mut reference = valid_ref();
        reference["metadata"] = json!({"outer": [{"safe": {key: "fixture-value"}}]});
        expect_invalid(
            reference,
            EvidenceValidationCode::SecretKeyDenied,
            &format!("evidence_refs[0].metadata.outer[0].safe.{key}"),
        );
    }

    for value in [
        "Bearer secret",
        "-----BEGIN PRIVATE KEY----- fixture",
        "eyJmaXh0dXJlIjoxfQ.cGF5bG9hZA.c2lnbmF0dXJl",
        "sk-test",
        "AKIAfixture",
        "AIzafixture",
        "ghp_fixture",
    ] {
        let mut reference = valid_ref();
        reference["metadata"] = json!({"outer": [{"safe": value}]});
        expect_invalid(
            reference,
            EvidenceValidationCode::SecretValuePatternDenied,
            "evidence_refs[0].metadata.outer[0].safe",
        );
    }
}
