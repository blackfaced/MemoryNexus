use std::fmt;

use chrono::{DateTime, SecondsFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputConfirmationStatus {
    Confirmed,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputConfirmationMethod {
    ExplicitAcceptance,
    ExplicitCorrection,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputConfirmation {
    pub status: InputConfirmationStatus,
    pub method: InputConfirmationMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceValidationError {
    pub code: EvidenceValidationCode,
    pub path: String,
}

impl EvidenceValidationError {
    fn new(path: impl Into<String>, code: EvidenceValidationCode) -> Self {
        Self {
            path: path.into(),
            code,
        }
    }
}

impl fmt::Display for EvidenceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid_evidence_reference path={} code={}",
            self.path, self.code
        )
    }
}

impl std::error::Error for EvidenceValidationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceValidationCode {
    CallerOwnershipFieldDenied,
    ControlCharacterDenied,
    InlineBytesDenied,
    InputConfirmationRequired,
    InvalidInputConfirmation,
    InvalidContentHash,
    InvalidIdentifier,
    InvalidMediaType,
    InvalidOriginalName,
    InvalidPercentEncoding,
    InvalidTimestamp,
    LocatorEmpty,
    LocatorQueryDenied,
    LocatorTooLarge,
    LocatorUserinfoDenied,
    MetadataNotObject,
    MetadataTooDeep,
    MetadataTooLarge,
    RequiredFieldMissing,
    SecretKeyDenied,
    SecretValuePatternDenied,
    TranscriptTooLarge,
}

impl fmt::Display for EvidenceValidationCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CallerOwnershipFieldDenied => "caller_ownership_field_denied",
            Self::ControlCharacterDenied => "control_character_denied",
            Self::InlineBytesDenied => "inline_bytes_denied",
            Self::InputConfirmationRequired => "input_confirmation_required",
            Self::InvalidInputConfirmation => "invalid_input_confirmation",
            Self::InvalidContentHash => "invalid_content_hash",
            Self::InvalidIdentifier => "invalid_identifier",
            Self::InvalidMediaType => "invalid_media_type",
            Self::InvalidOriginalName => "invalid_original_name",
            Self::InvalidPercentEncoding => "invalid_percent_encoding",
            Self::InvalidTimestamp => "invalid_timestamp",
            Self::LocatorEmpty => "locator_empty",
            Self::LocatorQueryDenied => "locator_query_denied",
            Self::LocatorTooLarge => "locator_too_large",
            Self::LocatorUserinfoDenied => "locator_userinfo_denied",
            Self::MetadataNotObject => "metadata_not_object",
            Self::MetadataTooDeep => "metadata_too_deep",
            Self::MetadataTooLarge => "metadata_too_large",
            Self::RequiredFieldMissing => "required_field_missing",
            Self::SecretKeyDenied => "secret_key_denied",
            Self::SecretValuePatternDenied => "secret_value_pattern_denied",
            Self::TranscriptTooLarge => "transcript_too_large",
        })
    }
}

pub fn validate_evidence_request(
    input_source: Option<&str>,
    input_confirmation: Option<&InputConfirmation>,
    evidence_refs: Option<&[Value]>,
) -> Result<(), EvidenceValidationError> {
    if let Some(confirmation) = input_confirmation {
        if confirmation.status != InputConfirmationStatus::Confirmed
            || !matches!(
                confirmation.method,
                InputConfirmationMethod::ExplicitAcceptance
                    | InputConfirmationMethod::ExplicitCorrection
            )
        {
            return Err(EvidenceValidationError::new(
                "input_confirmation",
                EvidenceValidationCode::InvalidInputConfirmation,
            ));
        }
    }

    if matches!(
        input_source,
        Some("agent_ocr" | "agent_transcribed" | "mixed")
    ) && input_confirmation.is_none()
    {
        return Err(EvidenceValidationError::new(
            "input_confirmation",
            EvidenceValidationCode::InputConfirmationRequired,
        ));
    }

    for (index, reference) in evidence_refs.unwrap_or_default().iter().enumerate() {
        validate_reference(index, reference)?;
    }

    Ok(())
}

fn validate_reference(index: usize, reference: &Value) -> Result<(), EvidenceValidationError> {
    let root = format!("evidence_refs[{index}]");
    let object = reference.as_object().ok_or_else(|| {
        EvidenceValidationError::new(root.clone(), EvidenceValidationCode::MetadataNotObject)
    })?;

    for field in ["id", "space_id"] {
        if object.contains_key(field) {
            return Err(EvidenceValidationError::new(
                format!("{root}.{field}"),
                EvidenceValidationCode::CallerOwnershipFieldDenied,
            ));
        }
    }

    let provider = required_str(object.get("provider"), &format!("{root}.provider"))?;
    validate_identifier(provider, &format!("{root}.provider"))?;
    let locator = required_str(object.get("locator"), &format!("{root}.locator"))?;
    validate_locator(locator, &format!("{root}.locator"))?;
    let media_type = required_str(object.get("media_type"), &format!("{root}.media_type"))?;
    validate_media_type(media_type, &format!("{root}.media_type"))?;

    if let Some(value) = object.get("content_hash") {
        validate_content_hash(
            required_str(Some(value), &format!("{root}.content_hash"))?,
            &format!("{root}.content_hash"),
        )?;
    }
    if let Some(value) = object.get("original_name") {
        validate_original_name(
            required_str(Some(value), &format!("{root}.original_name"))?,
            &format!("{root}.original_name"),
        )?;
    }
    if let Some(value) = object.get("captured_at") {
        validate_timestamp(
            required_str(Some(value), &format!("{root}.captured_at"))?,
            &format!("{root}.captured_at"),
        )?;
    }
    if let Some(value) = object.get("transcript") {
        let transcript = required_str(Some(value), &format!("{root}.transcript"))?;
        if transcript.len() > 65_536 {
            return Err(EvidenceValidationError::new(
                format!("{root}.transcript"),
                EvidenceValidationCode::TranscriptTooLarge,
            ));
        }
    }
    if let Some(value) = object.get("transcript_source") {
        validate_identifier(
            required_str(Some(value), &format!("{root}.transcript_source"))?,
            &format!("{root}.transcript_source"),
        )?;
    }

    let metadata_path = format!("{root}.metadata");
    let metadata = object.get("metadata").ok_or_else(|| {
        EvidenceValidationError::new(
            metadata_path.clone(),
            EvidenceValidationCode::RequiredFieldMissing,
        )
    })?;
    validate_metadata(metadata, &metadata_path)?;

    Ok(())
}

fn required_str<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> Result<&'a str, EvidenceValidationError> {
    value.and_then(Value::as_str).ok_or_else(|| {
        EvidenceValidationError::new(path, EvidenceValidationCode::RequiredFieldMissing)
    })
}

fn validate_identifier(value: &str, path: &str) -> Result<(), EvidenceValidationError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_lowercase()
        || !bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
    {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidIdentifier,
        ));
    }
    Ok(())
}

fn validate_locator(locator: &str, path: &str) -> Result<(), EvidenceValidationError> {
    if locator.is_empty() {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::LocatorEmpty,
        ));
    }
    if locator.len() > 4096
        || serde_json::to_string(locator).map_or(true, |value| value.len() > 8192)
    {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::LocatorTooLarge,
        ));
    }
    if locator.chars().any(char::is_control) {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::ControlCharacterDenied,
        ));
    }
    if locator.to_ascii_lowercase().starts_with("data:") || locator.contains(";base64,") {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InlineBytesDenied,
        ));
    }
    if has_uri_userinfo(locator) {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::LocatorUserinfoDenied,
        ));
    }

    validate_locator_pairs(locator, path, true)?;
    validate_locator_pairs(locator, path, false)?;
    Ok(())
}

fn has_uri_userinfo(locator: &str) -> bool {
    let Some(scheme_end) = locator.find("://") else {
        return false;
    };
    let authority_start = scheme_end + 3;
    let authority_end = locator[authority_start..]
        .find(['/', '?', '#'])
        .map_or(locator.len(), |offset| authority_start + offset);
    locator[authority_start..authority_end].contains('@')
}

fn validate_locator_pairs(
    locator: &str,
    path: &str,
    query: bool,
) -> Result<(), EvidenceValidationError> {
    let Some(raw) = locator_component(locator, query) else {
        return Ok(());
    };
    let label = if query { "query" } else { "fragment" };
    for pair in raw.split('&').filter(|pair| !pair.is_empty()) {
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode_utf8(raw_key).map_err(|_| {
            EvidenceValidationError::new(
                format!("{path}.{label}.{raw_key}"),
                EvidenceValidationCode::InvalidPercentEncoding,
            )
        })?;
        let value = percent_decode_utf8(raw_value).map_err(|_| {
            EvidenceValidationError::new(
                format!("{path}.{label}.{key}"),
                EvidenceValidationCode::InvalidPercentEncoding,
            )
        })?;
        let compact = compact_key(&key);
        if denied_locator_key(&compact) {
            return Err(EvidenceValidationError::new(
                format!("{path}.{label}.{key}"),
                EvidenceValidationCode::LocatorQueryDenied,
            ));
        }
        if has_secret_value_pattern(&value) {
            return Err(EvidenceValidationError::new(
                format!("{path}.{label}.{key}"),
                EvidenceValidationCode::SecretValuePatternDenied,
            ));
        }
    }
    Ok(())
}

fn locator_component(locator: &str, query: bool) -> Option<&str> {
    if query {
        let start = locator.find('?')? + 1;
        let end = locator[start..]
            .find('#')
            .map_or(locator.len(), |offset| start + offset);
        Some(&locator[start..end])
    } else {
        Some(&locator[locator.find('#')? + 1..])
    }
}

fn percent_decode_utf8(input: &str) -> Result<String, ()> {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(());
            }
            let high = hex_value(bytes[index + 1]).ok_or(())?;
            let low = hex_value(bytes[index + 2]).ok_or(())?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).map_err(|_| ())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn denied_locator_key(compact: &str) -> bool {
    denied_metadata_key(compact)
        || matches!(
            compact,
            "xamzalgorithm"
                | "xamzcredential"
                | "xamzsignature"
                | "xamzsecuritytoken"
                | "xgoogalgorithm"
                | "xgoogcredential"
                | "xgoogsignature"
                | "signature"
                | "sig"
                | "expires"
        )
}

fn validate_media_type(value: &str, path: &str) -> Result<(), EvidenceValidationError> {
    if value.len() > 255
        || !value.is_ascii()
        || value.contains(';')
        || value.chars().any(char::is_uppercase)
    {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidMediaType,
        ));
    }
    let Some((media_type, subtype)) = value.split_once('/') else {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidMediaType,
        ));
    };
    if !valid_media_token(media_type) || !valid_media_token(subtype) || subtype.contains('/') {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidMediaType,
        ));
    }
    Ok(())
}

fn valid_media_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                )
        })
}

fn validate_content_hash(value: &str, path: &str) -> Result<(), EvidenceValidationError> {
    let digest = value.strip_prefix("sha256:").ok_or_else(|| {
        EvidenceValidationError::new(path, EvidenceValidationCode::InvalidContentHash)
    })?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidContentHash,
        ));
    }
    Ok(())
}

fn validate_original_name(value: &str, path: &str) -> Result<(), EvidenceValidationError> {
    if value.len() > 255 || value.contains('/') || value.contains('\\') {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidOriginalName,
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::ControlCharacterDenied,
        ));
    }
    Ok(())
}

fn validate_timestamp(value: &str, path: &str) -> Result<(), EvidenceValidationError> {
    if !value.ends_with('Z') {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidTimestamp,
        ));
    }
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| {
        EvidenceValidationError::new(path, EvidenceValidationCode::InvalidTimestamp)
    })?;
    let canonical = parsed
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    if canonical != value {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::InvalidTimestamp,
        ));
    }
    Ok(())
}

fn validate_metadata(metadata: &Value, path: &str) -> Result<(), EvidenceValidationError> {
    if !metadata.is_object() {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::MetadataNotObject,
        ));
    }
    if serde_json::to_vec(metadata).map_or(true, |bytes| bytes.len() > 16_384) {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::MetadataTooLarge,
        ));
    }
    validate_metadata_value(metadata, path, 1)
}

fn validate_metadata_value(
    value: &Value,
    path: &str,
    depth: usize,
) -> Result<(), EvidenceValidationError> {
    if depth > 4 {
        return Err(EvidenceValidationError::new(
            path,
            EvidenceValidationCode::MetadataTooDeep,
        ));
    }
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if denied_metadata_key(&compact_key(key)) {
                    return Err(EvidenceValidationError::new(
                        child_path,
                        EvidenceValidationCode::SecretKeyDenied,
                    ));
                }
                validate_metadata_value(child, &child_path, depth + 1)?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                validate_metadata_value(child, &format!("{path}[{index}]"), depth + 1)?;
            }
        }
        Value::String(value) if has_secret_value_pattern(value) => {
            return Err(EvidenceValidationError::new(
                path,
                EvidenceValidationCode::SecretValuePatternDenied,
            ));
        }
        _ => {}
    }
    Ok(())
}

fn compact_key(value: &str) -> String {
    value
        .bytes()
        .filter_map(|byte| match byte {
            b'-' | b'_' | b'.' => None,
            b'A'..=b'Z' => Some((byte + 32) as char),
            _ if byte.is_ascii() => Some(byte as char),
            _ => Some(byte as char),
        })
        .collect()
}

fn denied_metadata_key(compact: &str) -> bool {
    matches!(
        compact,
        "password"
            | "passwd"
            | "secret"
            | "clientsecret"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "apikey"
            | "authorization"
            | "cookie"
            | "session"
            | "credential"
            | "privatekey"
            | "mountsecret"
    )
}

fn has_secret_value_pattern(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("bearer ")
        || value.starts_with("-----BEGIN PRIVATE KEY-----")
        || value.starts_with("sk-")
        || value.starts_with("AKIA")
        || value.starts_with("AIza")
        || value.starts_with("ghp_")
        || is_jwt_like(value)
}

fn is_jwt_like(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    let Some(third) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && [first, second, third].iter().all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
}
