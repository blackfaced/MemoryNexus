# Media Evidence Contract

This document is the field-level source of truth for provider-neutral media
evidence references. ADR-021 defines the durable architecture decision.

MemoryNexus feedback operates on user-confirmed normalized text. An evidence
reference preserves provenance to original media without requiring the Engine
to ingest, resolve, or read that media.

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
| `provider` | yes | Resolution strategy identifier. Examples include `local`, `external_drive`, `webdav`, `s3`, `oss`, and future managed storage; these are illustrative, not a closed enum. |
| `locator` | yes | Provider-specific stable locator. It must not contain credentials, tokens, secrets, or short-lived signed query parameters. |
| `media_type` | yes | MIME type or a documented provider-neutral equivalent. |
| `content_hash` | no | Stable content identity used for relocation and mismatch checks when available. Include the hash algorithm in the value or metadata. |
| `original_name` | no | Human-recognizable source name; it is not a locator. |
| `captured_at` | no | Source capture time when known. |
| `transcript` | no | OCR, ASR, or entered text retained as provenance. It is not canonical feedback input. |
| `transcript_source` | no | Provenance such as `agent_ocr`, `agent_transcribed`, or `human_entered`. It does not establish ownership. |
| `metadata` | yes | Small structured provenance. It must not contain credentials or become an unbounded media manifest. Use an empty object when no metadata is needed. |

Confirmed text in the Surface payload is canonical for feedback,
classification, reflection, and planning. `transcript` records where that text
came from and may differ before user correction; it must never silently
replace the confirmed Surface text.

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
other cognitive analysis. It does not grant authorization and must not expose a
locator when access is denied.

The v1 contract records references only. No v1 Surface operation executes a
resolver or requires media availability.

## Failure Codes

| Code | Meaning | Required behavior |
| --- | --- | --- |
| `evidence_unavailable` | Provider, path, drive, object, or resolver is temporarily or permanently unavailable. | Fail media inspection only; preserve confirmed text, Trace, and completed feedback. |
| `evidence_mismatch` | Resolved bytes do not match the recorded content hash or stable identity. | Do not present the resolved media as the original evidence; preserve completed text-based results. |
| `evidence_forbidden` | The caller is not authorized for the Space or provider resource. | Reveal neither locator details nor provider credentials. |
| `invalid_evidence_reference` | Required fields are absent, malformed, unsafe, or inconsistent. | Reject the reference at the Gateway boundary without attempting resolution. |

Evidence failure affects provenance inspection, not the validity of confirmed
text. Inaccessible media must not invalidate a completed feedback operation or
prevent later text-based feedback and planning.

## V1 Scope And Non-Goals

V1 defines documentation and request-validation semantics only. It has:

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
