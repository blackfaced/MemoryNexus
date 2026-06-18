# Media Evidence Contract

This document is the field-level source of truth for provider-neutral media
evidence references. ADR-021 defines the durable architecture decision.

MemoryNexus feedback operates on user-confirmed normalized text. An evidence
reference preserves provenance to original media without requiring the Engine
to ingest, resolve, or read that media.

This is a documentation and future validation contract. It introduces no
current runtime Surface, persistence, resolver execution, upload, download,
OCR, ASR, or other media handling capability.

## Canonical Shapes

```text
EvidenceRef {
  id
  space_id
  provider
  locator
  media_type
  content_hash?
  original_name?
  captured_at?
  transcript?
  transcript_source?
  metadata
}

EvidenceRefInput {
  provider
  locator
  media_type
  content_hash?
  original_name?
  captured_at?
  transcript?
  transcript_source?
  metadata
}
```

`EvidenceRefInput` is the caller-facing shape. Surface Gateway creates the
stable `id` when persistence exists and derives `space_id` from the authorized
request context. A caller cannot provide or claim evidence ownership.

## Field Rules

| Field | Required | Contract |
| --- | --- | --- |
| `id` | stored reference only | Stable identifier assigned by MemoryNexus. |
| `space_id` | stored reference only | Owning `CognitiveSpace`, derived by Surface Gateway from authorized context. |
| `provider` | yes | ASCII identifier matching `^[a-z][a-z0-9._-]{0,63}$` (maximum 64 bytes). Examples include `local`, `external_drive`, `webdav`, `s3`, `oss`, and future managed storage; these are illustrative, not a closed enum. |
| `locator` | yes | Non-empty provider-specific stable locator. The decoded string is limited to 4,096 UTF-8 bytes and its serialized JSON string to 8,192 bytes. It must not contain control characters, credentials, tokens, secrets, or short-lived signed query parameters. |
| `media_type` | yes | Normalized lowercase MIME `type/subtype` without parameters, maximum 255 ASCII bytes. Both tokens must match `[a-z0-9][a-z0-9!#$&^_.+-]*`. |
| `content_hash` | no | V1 encoding must match exactly `^sha256:[0-9a-f]{64}$`. It provides stable content identity for relocation and mismatch checks. |
| `original_name` | no | Human-recognizable basename, maximum 255 UTF-8 bytes, with no control characters or `/` or `\\` path separators; it is not a path or locator. |
| `captured_at` | no | RFC 3339 timestamp normalized to UTC with the `Z` designator, for example `2026-06-18T03:10:00Z`. Numeric offsets are not canonical input. |
| `transcript` | no | Valid UTF-8 OCR, ASR, or entered text, maximum 65,536 bytes. It is provenance, not canonical feedback input. |
| `transcript_source` | no | ASCII identifier matching the `provider` syntax and 64-byte limit, such as `agent_ocr`, `agent_transcribed`, or `human_entered`. It does not establish ownership. |
| `metadata` | yes | JSON object, maximum 16,384 serialized UTF-8 bytes and maximum nesting depth 4, counting the root object as depth 1. It must not contain credentials or become an unbounded media manifest. Use an empty object when no metadata is needed. |

All limits apply before persistence or resolver execution. Values that violate
required presence, syntax, encoding, normalization, size, nesting, or safety
rules map to `invalid_evidence_reference`.

Confirmed text in the Surface payload is canonical for feedback,
classification, reflection, and planning. `transcript` records where that text
came from and may differ before user correction; it must never silently
replace the confirmed Surface text.

A transcript difference from confirmed Surface text records the Adapter and
user-confirmation path. It is not `evidence_mismatch` and does not require the
Engine to read or semantically analyze the media.

## Ownership And Authorization

- `CognitiveSpace` is the ownership and permission boundary.
- Namespace is a domain partition within a Space, not a separate media ACL.
- Surface Gateway derives `space_id` from the authenticated, authorized request
  context. `EvidenceRefInput` intentionally has no `space_id`.
- Evidence linked to a Trace or Surface operation must belong to the same Space.
- Authorized media resolution does not grant access to other Engine objects or
  other Spaces.

## Locator Safety

A locator identifies media without carrying authority. It may contain a stable
object key, provider-relative path, opaque provider ID, or local path understood
by an authorized integration. It must not contain:

- access keys, passwords, bearer tokens, cookies, or embedded credentials;
- short-lived signed URL query parameters;
- mount secrets or provider session material;
- data URLs or inline media bytes.

Surface Gateway must reject a malformed or sensitive locator as
`invalid_evidence_reference`. Provider credentials belong in the authorized
resolver integration and are never persisted in EvidenceRef, Trace, or Surface
payload provenance.

## EvidenceResolver Boundary

`EvidenceResolver` is an optional integration abstraction. Its operations are
limited to:

1. checking current availability;
2. resolving a readable location or stream for an authorized caller;
3. relocating a reference when a provider path or mount point changes;
4. verifying `content_hash` when available.

A resolver does not perform OCR, ASR, classification, reflection, planning, or
other cognitive analysis. Every resolver must:

- canonicalize the locator according to provider-specific rules before access;
- confine resolution to explicitly configured roots, buckets, and hosts;
- reject path traversal, symlink escape after canonicalization, unsupported
  schemes, and arbitrary URL or file resolution;
- independently authorize access to the provider resource after
  `CognitiveSpace` authorization succeeds;
- avoid exposing the locator when either authorization layer denies access.

Space authorization and provider-resource authorization are both required.
Neither one grants the other.

## Future Relocation Lifecycle

Relocation is a future resolver/persistence capability, not a v1 operation. A
future implementation must:

1. authorize the actor for the existing reference's `CognitiveSpace` and keep
   the relocated reference in that same Space;
2. prove stable identity using the recorded `content_hash`, or a documented
   provider-stable immutable identity when no content hash is available;
3. preserve an audit trail containing the prior locator, replacement locator,
   actor, reason, and timestamp;
4. update the locator only after identity verification succeeds;
5. create a new evidence reference instead of silently rewriting historical
   provenance when stable identity cannot be proven.

Relocation cannot move ownership between Spaces or reinterpret transcript
content.

The v1 contract records references only. No v1 Surface operation executes a
resolver or requires media availability.

## Failure Codes

| Code | Meaning | Required behavior |
| --- | --- | --- |
| `evidence_unavailable` | Provider, path, drive, object, or resolver is temporarily or permanently unavailable. | Fail media inspection only; preserve confirmed text, Trace, and completed feedback. |
| `evidence_mismatch` | Resolved content does not match the recorded hash or documented provider-stable immutable identity. | Do not present the resolved media as the original evidence; preserve completed text-based results. Transcript/confirmed-text differences do not use this code. |
| `evidence_forbidden` | The caller is not authorized for the Space or provider resource. | Reveal neither locator details nor provider credentials. |
| `invalid_evidence_reference` | Required fields are absent, malformed, unsafe, or inconsistent. | Reject the reference at the Gateway boundary without attempting resolution. |

Evidence failure affects provenance inspection, not the validity of confirmed
text. Inaccessible media must not invalidate a completed feedback operation or
prevent later text-based feedback and planning.

## V1 Scope And Non-Goals

V1 defines documentation and future request-validation semantics only. It has:

- no current runtime Surface accepting these shapes;
- no media upload or download;
- no resolver execution;
- no `EvidenceRef` database schema or repository;
- no OCR or ASR;
- no provider SDK requirement;
- no requirement to copy external evidence into MemoryNexus-managed storage.

Future persistence, resolver execution, provider integrations, and managed
storage require separately scoped implementation issues with explicit Space
authorization and lifecycle acceptance criteria.

## Related Documents

- [ADR-021: External Media Evidence References](../decisions/ADR-021-external-media-evidence-references.md)
- [ADR-002: Storage Abstraction](../decisions/ADR-002-storage-abstraction.md)
- [Surfaces and Adapters](architecture/surfaces-and-adapters.md)
- [Trace Contract](trace-contract.md)
