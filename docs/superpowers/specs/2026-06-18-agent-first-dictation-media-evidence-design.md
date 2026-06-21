# Agent-First Dictation And Media Evidence Design

## Status

Approved in conversation on 2026-06-18. This document defines the documentation
and architecture changes that should precede implementation.

ADR-021 and `docs/media-evidence-contract.md` are the canonical sources if this
design wording ever differs from the approved media evidence contract.

## Goal

Make Dictation Coach testable through a chat agent before building a dedicated
web or mobile app. The first useful loop must let an agent:

1. record a dictation word list;
2. submit the learner's result;
3. review deterministic mistakes and recurring patterns;
4. generate the next short practice;
5. observe change over time.

MemoryNexus remains the long-term feedback Engine. The agent is an Adapter that
handles conversation, OCR, speech-to-text, and user confirmation before calling
MemoryNexus Surfaces.

## Architecture Boundary

```text
Image / audio / video
  -> Agent or App OCR / ASR
  -> user-confirmed normalized text
  -> Surface Gateway(text + optional EvidenceRefInput)
  -> Trace / FeedbackLoop / GrowthModel / PracticePlan
```

MemoryNexus does not perform OCR, ASR, or raw-media interpretation in the first
slice. Its analysis and planning operate on normalized text. Images, audio, and
video may be linked as provenance so a person can later inspect the original
source.

The ownership and permission boundary remains `CognitiveSpace`. A media
reference must belong to the same Space as the Trace or Surface request that
uses it. Namespace remains a domain partition, not a media permission boundary.

## Repository Boundary

The MemoryNexus repository owns:

- Surface Gateway and generic Surface actions;
- Trace and long-term feedback objects;
- the provider-neutral media evidence contract;
- the MCP transport adapter needed to call Surfaces;
- generic evidence-reference validation and provenance.

A future standalone Dictation Coach repository should own:

- parent and learner product language;
- product-specific agent prompts or skills;
- OCR and ASR provider selection;
- media acquisition and local or remote file management;
- a dedicated web or mobile experience;
- orchestration of MemoryNexus Surface calls.

The standalone app must not directly access Engine tables or internal domain
objects. The first agent test does not depend on that repository existing.

## Media Evidence Contract

The core abstraction is an evidence reference, not a storage SDK:

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
```

Field rules:

- `provider` identifies a resolution strategy such as `local`,
  `external_drive`, `webdav`, `s3`, or `oss`.
- `locator` is provider-specific and must not contain credentials or a
  short-lived signed URL.
- `content_hash` provides stable identity and relocation checks when available.
- `media_type` is a MIME type or a documented provider-neutral equivalent.
- `transcript` is optional provenance. Surface payloads still carry the
  confirmed normalized text used by the Engine.
- `transcript_source` records values such as `agent_ocr`,
  `agent_transcribed`, or `human_entered` without making the Agent an owner.
- `metadata` is small structured provenance and must not become an unbounded
  media manifest.

The first implementation accepts an inline `EvidenceRefInput` in a Surface
request. These descriptors remain ephemeral in this slice; there is no
`EvidenceRef` persistence, repository, or schema.
`EvidenceRefInput` omits `id` and `space_id`; Surface Gateway assigns ownership
from the authorized request context. Any future persisted `EvidenceRef` is
governed by ADR-021 and `docs/media-evidence-contract.md` and requires a
dedicated implementation issue with explicit lifecycle and permission
acceptance criteria.

## Resolver Boundary

`EvidenceResolver` is an optional integration abstraction. A resolver may:

- determine whether a reference is currently available;
- return a readable location or stream to an authorized caller;
- relocate a reference after a drive mount point or provider path changes;
- verify a content hash when available.

A resolver must not:

- perform OCR, ASR, classification, reflection, or planning;
- grant access outside `CognitiveSpace` authorization;
- put provider credentials into Trace, Surface payloads, or `EvidenceRef`;
- make media availability a prerequisite for text-based feedback.

The first Dictation Coach slice validates ephemeral `EvidenceRefInput`
descriptors but does not persist, resolve, or read them.

## Agent-First Surface Flow

The MCP/chat Adapter should expose product-friendly tools backed by generic
Surface Gateway actions:

| Agent intent | Surface | Gateway action |
| --- | --- | --- |
| Record today's list | Capture | `capture_observation` |
| Submit dictation result | Performance | `submit_attempt` |
| Review mistakes | Reflection | `review_evidence` |
| Generate tomorrow practice | Planning | `generate_next_task` |
| Show recent change | Observation | `get_state_summary` |

The agent may accept text, images, audio, or video from its own environment. For
non-text input it must:

1. perform OCR or ASR outside MemoryNexus;
2. require explicit user acceptance or correction of every media-derived
   normalized payload; confidence or uncertainty scores do not substitute for
   this confirmation;
3. send only the explicitly accepted or corrected text to the relevant Surface;
4. include an optional `EvidenceRefInput` when the original media should remain
   traceable.

Agent-facing tool names may be dictation-specific, but their implementation
must call generic Surface actions instead of mutating Engine objects directly.

## Failure And Security Semantics

- An unavailable drive, expired external link, missing provider, or failed
  resolver returns `evidence_unavailable` for media inspection.
- `evidence_unavailable` does not invalidate confirmed text, existing Trace, or
  a completed deterministic evaluation.
- A content hash mismatch returns `evidence_mismatch` and prevents presenting
  the resolved file as the original source.
- Missing media permission returns `evidence_forbidden` without revealing the
  locator.
- OCR or ASR uncertainty is resolved by the Agent and user before submission.
  MemoryNexus must not invent text from a reference it cannot inspect.
- Any credential, token, mount secret, signed authentication locator, or secret
  in `locator` or `metadata` makes the whole `EvidenceRefInput` invalid and it
  must be rejected.
- Redaction applies only to diagnostics and logs. Diagnostics expose the
  offending field or path and an error code, never the raw value.
- Rejected raw payloads and secret values must not enter logs, Trace, or any
  persistence.

## Documentation Changes

The implementation plan should update documentation in this order:

1. Add `ADR-021-external-media-evidence-references.md` as the durable decision.
2. Add `docs/media-evidence-contract.md` as the detailed field, lifecycle,
   resolver, security, and failure contract.
3. Update `ADR-002-storage-abstraction.md` to state that its S3/MinIO abstraction
   applies only when MemoryNexus is the managed media provider. ADR-021 governs
   external evidence references and is not replaced by a storage SDK.
4. Update `decisions/README.md` with ADR-021.
5. Update `docs/dictation-coach-mvp.md` so OCR and ASR are Adapter capabilities,
   not globally prohibited capabilities. Add optional `evidence_refs` and
   `agent_ocr` / `agent_transcribed` provenance.
6. Update `docs/architecture/surfaces-and-adapters.md` with the Agent -> text ->
   Surface flow and the resolver boundary.
7. Update `docs/trace-contract.md` so Trace can link media evidence without
   owning media bytes.
8. Update `docs/agent-integration.md` with confirmation, submission, and
   sensitive-locator rules.
9. Update `docs/architecture/README.md` so S3/MinIO is an optional managed
   provider rather than the required media path.
10. Update `AGENTS.md`, `docs/TODO.md`, and `docs/issues.md` with the durable
    implementation boundary and MCP/chat-first Dictation Adapter priority.
11. Update the matching GitHub issues after the repository documentation is
    merged so issue acceptance criteria use the same language.

`README.md` should receive at most a short link or boundary sentence. Detailed
contract language belongs in the ADR and contract document.

## Validation

The documentation change is complete when:

- no document says MemoryNexus itself must perform OCR or ASR for Dictation MVP;
- no document implies S3/MinIO is the only valid media provenance path;
- the distinction between text evidence and original media evidence is clear;
- failures to access media do not break the text-based feedback loop;
- Agent, Surface, Resolver, Engine, Space, and Namespace responsibilities are
  consistent across all touched documents;
- the Dictation MVP explicitly prioritizes MCP/chat Agent testing before a
  dedicated App;
- Markdown links resolve and `git diff --check` passes.

Because the first change is documentation-only, Rust tests are not required.
Implementation issues that alter Rust behavior must run the repository's normal
format, test, and Clippy checks.

## Non-Goals

- Implementing `EvidenceRef`, `EvidenceResolver`, or database migrations.
- Uploading, downloading, moving, or deleting media.
- Choosing a single object-store or network-drive provider.
- Adding OCR, ASR, image understanding, or video understanding to MemoryNexus.
- Creating the standalone Dictation Coach repository.
- Building a web or mobile UI.
- Supporting multi-child product roles in the Engine.
