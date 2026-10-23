//! Strict wire validation shared by the workflow use cases.
use super::*;
use std::collections::BTreeSet;
pub(super) fn object(v: &Value, required: &[&str], optional: &[&str]) -> Result<(), Error> {
    let o = v.as_object().ok_or_else(Error::invalid)?;
    if required.iter().any(|k| !o.contains_key(*k))
        || o.keys()
            .any(|k| !required.contains(&k.as_str()) && !optional.contains(&k.as_str()))
    {
        return Err(Error::invalid());
    }
    Ok(())
}
pub(super) fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str, Error> {
    v[k].as_str().ok_or_else(Error::invalid)
}
pub(super) fn number(v: &Value, k: &str) -> Result<i64, Error> {
    let n = v[k].as_u64().ok_or_else(Error::invalid)?;
    if n > 9_007_199_254_740_991 {
        return Err(Error::invalid());
    }
    Ok(n as i64)
}
pub(super) fn boolean(v: &Value, k: &str) -> Result<bool, Error> {
    v[k].as_bool().ok_or_else(Error::invalid)
}
pub(super) fn array<'a>(v: &'a Value, k: &str, max: usize) -> Result<&'a Vec<Value>, Error> {
    let a = v[k].as_array().ok_or_else(Error::invalid)?;
    if a.len() > max {
        return Err(Error::invalid());
    }
    Ok(a)
}
pub(super) fn name(s: &str) -> Result<(), Error> {
    if s.is_empty()
        || s.len() > 64
        || !s.as_bytes()[0].is_ascii_lowercase()
        || !s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    {
        return Err(Error::invalid());
    }
    Ok(())
}
pub(super) fn hash(s: &str) -> Result<(), Error> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::invalid());
    }
    Ok(())
}
pub(super) fn hash_value(v: &Value) -> String {
    hash_bytes(v.to_string().as_bytes())
}
pub(super) fn hash_bytes(v: &[u8]) -> String {
    format!("{:x}", Sha256::digest(v))
}
pub(super) fn time(v: &Value, k: &str) -> Result<(), Error> {
    chrono::DateTime::parse_from_rfc3339(string(v, k)?).map_err(|_| Error::invalid())?;
    Ok(())
}
pub(super) fn relative(s: &str) -> Result<(), Error> {
    validate_evidence_refs(&[EvidenceRef {
        path: s.to_owned(),
        sha256: None,
    }])?;
    if s.contains(['$', '*', '?', '[', ']', '`']) {
        return Err(Error::invalid());
    }
    Ok(())
}
pub(super) fn literal(v: &Value, k: &str, s: &str) -> Result<(), Error> {
    if string(v, k)? != s {
        return Err(Error::invalid());
    }
    Ok(())
}
pub(super) fn optional_confirm(v: &Value) -> Result<(), Error> {
    if v.get("confirmation").is_some() {
        literal(v, "confirmation", "confirmed")?;
    }
    Ok(())
}
pub(super) fn profile(v: &Value) -> Result<(), Error> {
    object(
        v,
        &[
            "schema_version",
            "workflow_id",
            "revision",
            "purpose",
            "mechanisms",
            "collections",
            "capture",
            "consumers",
            "operation",
        ],
        &[],
    )?;
    if number(v, "schema_version")? != 1 || number(v, "revision")? == 0 {
        return Err(Error::invalid());
    }
    canonical_uuid(string(v, "workflow_id")?)?;
    bounded(string(v, "purpose")?, 240)?;
    for m in array(v, "mechanisms", 8)? {
        object(
            m,
            &[
                "name",
                "purpose",
                "trigger",
                "read_paths",
                "write_paths",
                "evidence",
            ],
            &[],
        )?;
        name(string(m, "name")?)?;
        for k in ["purpose", "trigger", "evidence"] {
            bounded(string(m, k)?, 240)?;
        }
        for k in ["read_paths", "write_paths"] {
            for p in array(m, k, 20)? {
                relative(p.as_str().ok_or_else(Error::invalid)?)?;
            }
        }
    }
    let mut ids = BTreeSet::new();
    for c in array(v, "collections", 30)? {
        object(
            c,
            &[
                "collection_id",
                "format",
                "path",
                "selector",
                "extractor_version",
                "authority",
            ],
            &[],
        )?;
        let id = string(c, "collection_id")?;
        canonical_uuid(id)?;
        if !ids.insert(id) {
            return Err(Error::invalid());
        }
        relative(string(c, "path")?)?;
        name(string(c, "extractor_version")?)?;
        bounded(string(c, "authority")?, 240)?;
        let sel = &c["selector"];
        match (string(c, "format")?, string(sel, "kind")?) {
            ("text", "full") => object(sel, &["kind"], &[])?,
            ("text", "marked") => {
                object(sel, &["kind", "begin", "end"], &[])?;
                for k in ["begin", "end"] {
                    bounded(string(sel, k)?, 120)?;
                    if string(sel, k)?.contains(['\n', '\r']) {
                        return Err(Error::invalid());
                    }
                }
                if sel["begin"] == sel["end"] {
                    return Err(Error::invalid());
                }
            }
            ("append_jsonl", "fields") => {
                object(sel, &["kind", "keys"], &[])?;
                let keys = array(sel, "keys", 8)?;
                if keys.is_empty() {
                    return Err(Error::invalid());
                }
                let mut seen = BTreeSet::new();
                for key in keys {
                    let k = key.as_str().ok_or_else(Error::invalid)?;
                    if !seen.insert(k) {
                        return Err(Error::invalid());
                    }
                }
            }
            _ => return Err(Error::invalid()),
        }
    }
    object(
        &v["capture"],
        &["automatic_drafts", "automatic_reviews"],
        &[],
    )?;
    boolean(&v["capture"], "automatic_drafts")?;
    boolean(&v["capture"], "automatic_reviews")?;
    let cs = array(v, "consumers", 2)?;
    if cs.len() != 2 {
        return Err(Error::invalid());
    }
    let mut ids = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for c in cs {
        object(
            c,
            &[
                "consumer_id",
                "lens_version",
                "rules_sha256",
                "target_id",
                "neutralize_stale",
            ],
            &[],
        )?;
        for k in ["consumer_id", "lens_version", "target_id"] {
            name(string(c, k)?)?;
        }
        hash(string(c, "rules_sha256")?)?;
        boolean(c, "neutralize_stale")?;
        if !ids.insert(string(c, "consumer_id")?) || !targets.insert(string(c, "target_id")?) {
            return Err(Error::invalid());
        }
    }
    object(&v["operation"], &["workflow_name", "trigger"], &[])?;
    name(string(&v["operation"], "workflow_name")?)?;
    bounded(string(&v["operation"], "trigger")?, 240)?;
    Ok(())
}
pub(super) fn envelope(v: &Value, fields: &[&str], optional: &[&str]) -> Result<(), Error> {
    let mut req = vec![
        "idempotency_key",
        "run_id",
        "client",
        "workflow_id",
        "profile_revision",
        "incarnation",
    ];
    req.extend(fields);
    object(v, &req, optional)?;
    bounded(string(v, "idempotency_key")?, 128)?;
    bounded(string(v, "client")?, 64)?;
    for k in ["run_id", "workflow_id", "incarnation"] {
        canonical_uuid(string(v, k)?)?;
    }
    if number(v, "profile_revision")? == 0 {
        return Err(Error::invalid());
    }
    Ok(())
}
