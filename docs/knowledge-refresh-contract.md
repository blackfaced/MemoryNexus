# Namespace Knowledge Refresh Contract

Namespace Knowledge Refresh defines the V1 contract from
[ADR-023](../decisions/ADR-023-namespace-knowledge-refresh.md) for using
external knowledge as scoped context for a `CognitiveSpace` and Namespace.

External Skills, Agents, and Adapters may discover sources, fetch them, extract
candidate claims, and prepare summaries. MemoryNexus does not do that work in
V1. The Engine validates scoped submissions, approval state, provenance,
quality, freshness, privacy opt-in, and downstream links before approved
context can be considered by bounded downstream consumers such as manual
Surface-triggered SleepCycle / deterministic Dreaming.

This contract started docs-first and now has bounded Rust persistence plus
Capture / Observation Surface paths. It is still not a frontend UI, crawler,
provider integration, search adapter, scheduler, corpus store, or Knowledge
Surface.

## Goals

- Define the shared contracts for `KnowledgeSourceCandidate`,
  `KnowledgeSourcePolicy`, `KnowledgeContext`, and `AcquisitionTrace`.
- Keep every object scoped to one `CognitiveSpace` and one Namespace.
- Separate external discovery and extraction from Engine validation.
- Preserve provenance, quality, freshness, expiry, and privacy consent signals.
- Keep external knowledge distinct from user Memory, GrowthModel, and
  PracticePlan state.

## Non-Goals

- Do not implement network fetching, crawling, RSS, web search, provider APIs,
  search adapters, extraction agents, or schedulers.
- Do not persist full external articles, raw provider payloads, or full
  corpora.
- Do not add a Knowledge Surface in V1.
- Do not directly wire `KnowledgeContext` into `GrowthModel` or
  `PracticePlan`.
- Do not use `KnowledgeContext` in SleepCycle / Dreaming except as approved,
  non-expired, same-Space and same-Namespace candidate-only context with
  explicit `knowledge_context_id` citations.
- Do not change Engine ownership: `CognitiveSpace` remains the permission
  boundary and Namespace remains a partition inside it.

## Boundary

```text
External Skill / Agent / Adapter
  discover source
  fetch source
  extract candidate claims
  prepare evidence snippets
  prove opt-in when private context was used
      |
      v
MemoryNexus
  validate CognitiveSpace and Namespace scope
  validate source approval state and policy state
  validate AcquisitionTrace provenance
  validate quality, freshness, expiry, privacy, and downstream links
  store only V1 bounded KnowledgeContext fields
```

Rules:

- `space_id` is required on every object and remains the ownership and
  permission boundary.
- `namespace_id` is required on every object. Knowledge Refresh is scoped to a
  specific Namespace inside the owning Space, not to the whole Space by
  default.
- Linked objects must belong to the same `CognitiveSpace`.
- Cross-Namespace links are invalid unless a future contract explicitly allows
  them with a recorded reason.
- External submissions require `AcquisitionTrace`.
- Private Trace-derived discovery is invalid when `private_context_used = true`
  and no opt-in proof is present.

## V1 Storage Boundary

MemoryNexus may store:

- structured claims;
- source and extraction provenance;
- approval, policy, and downstream link identifiers;
- quality signals;
- expiry and freshness signals;
- short evidence snippets needed to inspect why a claim was accepted;
- redacted validation diagnostics.

MemoryNexus must not store by default:

- full external articles;
- full webpages;
- raw provider responses;
- complete RSS entries when they include full article bodies;
- full transcripts or corpora;
- crawler state;
- external search indexes;
- secret-bearing source locators, credentials, signed URLs, cookies, tokens, or
  unredacted private payloads.

Short evidence snippets are inspection aids, not corpus storage. A V1
implementation should bound snippet length, remove secrets, and keep enough
provenance to reacquire or re-evaluate the source outside the Engine when an
authorized Adapter can do so.

## KnowledgeSourceCandidate

`KnowledgeSourceCandidate` represents a proposed source that may be useful for
one Namespace. It is not approved context by itself.

Answers:

```text
Which external source is being proposed for this Space and Namespace, and why?
```

### Conceptual Shape

```text
KnowledgeSourceCandidate {
  id
  space_id
  namespace_id
  state
  proposed_source
  proposed_use
  proposer
  acquisition_trace_id
  private_context_used
  opt_in_proof?
  provenance
  quality_signals
  freshness
  expiry
  downstream_link_candidates
  decision?
  created_at
  updated_at
  metadata
}
```

### Fields

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable ID for the candidate. |
| `space_id` | yes | Owning `CognitiveSpace`; all permission checks remain Space-based. |
| `namespace_id` | yes | Namespace partition this source would inform, such as `child.english.spelling`. |
| `state` | yes | `proposed`, `approved`, `rejected`, or `expired`. |
| `proposed_source` | yes | Structured descriptor such as canonical URL, feed entry, citation, local file handle, package docs reference, or provider-neutral source handle. Must not include secrets. |
| `proposed_use` | yes | Short reason the source is relevant to the Namespace. |
| `proposer` | yes | External Skill, Agent, Adapter, or human/developer label that proposed it. |
| `acquisition_trace_id` | yes | Required `AcquisitionTrace` that records how the proposal was discovered or extracted. |
| `private_context_used` | yes | True when private Trace, Memory, GrowthModel, PracticePlan, or user-specific context shaped discovery or ranking. |
| `opt_in_proof` | conditional | Required when `private_context_used = true`; must identify explicit opt-in scope and time. |
| `provenance` | yes | Source origin, discovery method, extractor/version labels, and retrieval time when known. |
| `quality_signals` | yes | Structured reliability, relevance, confidence, and review status signals. |
| `freshness` | yes | Source publish/update time when known plus observed-at time. |
| `expiry` | yes | Expiration time or review-by time; stale candidates must become `expired` or be revalidated. |
| `downstream_link_candidates` | no | Candidate links to future source policy, KnowledgeContext, review task, or issue/decision references. |
| `decision` | no | Approval/rejection decision metadata, reviewer, reason, and decided-at time. |
| `created_at` | yes | Candidate creation time. |
| `updated_at` | yes | Last candidate update time. |
| `metadata` | no | Small structured extension point, not raw provider payload storage. |

### Candidate State

```text
proposed
approved
rejected
expired
```

- `proposed`: submitted but not approved for context generation.
- `approved`: accepted as a source candidate and may be used to create or link
  a `KnowledgeSourcePolicy`.
- `rejected`: not allowed for this Space and Namespace. Rejected payloads that
  contain secrets must not be persisted except as redacted diagnostics.
- `expired`: no longer fresh enough for use without revalidation.

Approval of a candidate does not approve every future claim extracted from that
source. `KnowledgeContext` still requires its own validation and must link to an
active `KnowledgeSourcePolicy`.

## KnowledgeSourcePolicy

`KnowledgeSourcePolicy` represents the approved use rules for a source within
one Space and Namespace.

Answers:

```text
What approved source may inform this Namespace, under which limits, and until when?
```

### Conceptual Shape

```text
KnowledgeSourcePolicy {
  id
  space_id
  namespace_id
  state
  source_candidate_id
  source_descriptor
  allowed_use
  disallowed_use
  privacy_policy
  refresh_policy
  quality_thresholds
  freshness_requirements
  expiry
  approved_by
  approved_at
  revoked_or_paused_reason?
  metadata
}
```

### Fields

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable policy ID. |
| `space_id` | yes | Owning `CognitiveSpace`; must match the source candidate. |
| `namespace_id` | yes | Namespace this policy applies to; must match the source candidate. |
| `state` | yes | `active`, `paused`, `revoked`, or `expired`. |
| `source_candidate_id` | yes | Approved candidate that produced this policy. |
| `source_descriptor` | yes | Sanitized source descriptor; no credentials or raw provider payloads. |
| `allowed_use` | yes | Allowed extraction and context uses, such as claim support, trend context, rubric update candidate, or review-question input. |
| `disallowed_use` | yes | Explicit limits, including no direct GrowthModel or PracticePlan updates. |
| `privacy_policy` | yes | Whether private context can be used for refresh, and which opt-in proof is required when it is. |
| `refresh_policy` | yes | Manual-only, adapter-triggered, or future scheduler-compatible intent. V1 does not implement schedulers. |
| `quality_thresholds` | yes | Minimum source reliability, extraction confidence, review status, or contradiction handling requirements. |
| `freshness_requirements` | yes | Maximum age, review interval, observed-at requirements, and stale handling. |
| `expiry` | yes | Policy expiry or review-by time. |
| `approved_by` | yes | Human/developer/system actor authorized to approve within the Space. |
| `approved_at` | yes | Approval timestamp. |
| `revoked_or_paused_reason` | no | Required when transitioning to `paused` or `revoked`. |
| `metadata` | no | Small structured extension point. |

### Policy State

```text
active
paused
revoked
expired
```

- `active`: source may be used for validated `KnowledgeContext` submissions.
- `paused`: temporarily disabled; no new `KnowledgeContext` may be accepted
  under this policy until it returns to `active`.
- `revoked`: permanently disabled for this policy identity.
- `expired`: no longer valid because the policy expiry or review window passed.

`KnowledgeContext` submissions under `paused`, `revoked`, or `expired` policies
are invalid.

## AcquisitionTrace

`AcquisitionTrace` records how an external Skill, Agent, Adapter, or human
workflow discovered, ranked, fetched, extracted, or submitted a source or
context object.

Answers:

```text
How did this external knowledge submission come into MemoryNexus?
```

### Conceptual Shape

```text
AcquisitionTrace {
  id
  space_id
  namespace_id
  submitted_by
  acquisition_kind
  discovery_method
  extraction_method
  private_context_used
  private_context_basis?
  opt_in_proof?
  source_handles
  source_observed_at
  extraction_run_id?
  tool_or_adapter_version?
  validation_summary
  redacted_diagnostics
  created_at
  metadata
}
```

### Fields

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable ID for the acquisition trace. |
| `space_id` | yes | Owning `CognitiveSpace`. |
| `namespace_id` | yes | Namespace partition for the acquisition. |
| `submitted_by` | yes | Skill, Agent, Adapter, CLI, dashboard, developer, or human workflow label. |
| `acquisition_kind` | yes | `source_candidate`, `source_policy_review`, `knowledge_context`, or `revalidation`. |
| `discovery_method` | yes | How the source was found, such as manual entry, user-provided link, external search, RSS reader, package docs lookup, or private Trace-derived suggestion. |
| `extraction_method` | yes | Human summary, deterministic parser, external extractor, model-assisted extractor, or none for proposal-only traces. |
| `private_context_used` | yes | True when private user/Space data influenced discovery, ranking, extraction, filtering, or submission. |
| `private_context_basis` | conditional | Required when private context was used; records the class of private context without copying raw private payloads. |
| `opt_in_proof` | conditional | Required when `private_context_used = true`; identifies explicit opt-in scope, actor, method, and timestamp. |
| `source_handles` | yes | Sanitized handles for the source material. Handles must not include secrets or raw full payloads. |
| `source_observed_at` | yes | Time the external source was observed or extracted. |
| `extraction_run_id` | no | External run ID useful for debugging, when safe to store. |
| `tool_or_adapter_version` | no | Version label for repeatability and provenance. |
| `validation_summary` | yes | Engine validation outcome or expected validation target. |
| `redacted_diagnostics` | no | Bounded diagnostics with secrets removed. |
| `created_at` | yes | Trace creation time. |
| `metadata` | no | Small structured extension point. |

### Privacy Rule

Any submission with `private_context_used = true` and missing `opt_in_proof` is
invalid input.

Private context includes Trace-derived discovery, Memory-derived search terms,
GrowthModel-derived ranking, PracticePlan-derived gaps, user-specific namespace
history, and any private Space data used to decide which external source to
fetch or how to extract it.

`opt_in_proof` must be explicit and scoped. It should record:

- consenting actor or authorization handle;
- consent method such as explicit acceptance, explicit correction, or settings
  toggle;
- consent timestamp;
- allowed namespace scope;
- allowed private context categories;
- expiry or revocation behavior.

The proof must not include raw private Trace, Memory, or secret-bearing payloads.

## KnowledgeContext

`KnowledgeContext` is the bounded, validated set of external structured claims
that may be considered as context by future issues. It is not user Memory and
does not directly mutate Engine growth or planning state.

Answers:

```text
Which approved external claims can this Namespace consider, with what provenance and limits?
```

### Conceptual Shape

```text
KnowledgeContext {
  id
  space_id
  namespace_id
  source_policy_id
  source_candidate_id
  acquisition_trace_id
  state
  context_type
  structured_claims
  provenance
  quality_signals
  freshness
  expiry
  evidence_snippets
  private_context_used
  opt_in_proof?
  downstream_links
  conflict_notes
  created_at
  updated_at
  metadata
}
```

### Fields

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable KnowledgeContext ID. |
| `space_id` | yes | Owning `CognitiveSpace`; must match policy, candidate, and acquisition trace. |
| `namespace_id` | yes | Namespace partition; must match policy, candidate, and acquisition trace. |
| `source_policy_id` | yes | Active `KnowledgeSourcePolicy` that allows this context. |
| `source_candidate_id` | yes | Source candidate behind the policy. |
| `acquisition_trace_id` | yes | Required acquisition trace for the submitted context. |
| `state` | yes | `candidate`, `valid`, `rejected`, or `expired` for context validation lifecycle. |
| `context_type` | yes | `reference_claims`, `rubric_context`, `practice_context`, `trend_context`, `contradiction_context`, or `review_context`. |
| `structured_claims` | yes | Bounded list of extracted claims with claim text, type, confidence, and claim-level provenance. |
| `provenance` | yes | Source descriptor, author/publisher when known, observed-at, extracted-at, extractor label/version, and citation handles. |
| `quality_signals` | yes | Reliability, relevance, extraction confidence, review status, contradiction status, and known limitations. |
| `freshness` | yes | Published/updated/observed timestamps when known plus stale-after or refresh-by date. |
| `expiry` | yes | Expiration or review-by timestamp. Expired context must not be used as active context. |
| `evidence_snippets` | yes | Short, bounded snippets supporting claims. These are not full source storage. |
| `private_context_used` | yes | Mirrors whether private context influenced discovery, extraction, or selection. |
| `opt_in_proof` | conditional | Required when `private_context_used = true`; must match the acquisition trace and policy. |
| `downstream_links` | yes | Explicit candidate links to future consumers such as `SleepCycle`, `DreamCandidate`, review question, issue, decision, or validation task. Links are not automatic mutations. |
| `conflict_notes` | no | Known conflicts with local Trace/GrowthModel evidence or other KnowledgeContext. |
| `created_at` | yes | Context creation time. |
| `updated_at` | yes | Last context update time. |
| `metadata` | no | Small structured extension point, not raw corpus storage. |

### Structured Claims

Each claim should be bounded and independently inspectable:

```text
StructuredClaim {
  claim_id
  claim_type
  text
  confidence
  source_fragment_ref
  evidence_snippet_ids
  limitations
}
```

Claim text should be a concise assertion, rule, rubric item, fact, trend, or
context note. It should not copy a full article, lesson, transcript, or provider
payload.

### Quality Signals

Quality signals should include enough information for validation and later
manual review:

- source reliability or trust tier;
- relevance to the Namespace;
- extraction confidence;
- whether a human reviewed the extraction;
- contradiction or conflict status;
- known limitations or missing context;
- freshness score or stale risk;
- policy compliance status.

### Freshness And Expiry

`freshness` and `expiry` are required because external knowledge can become
stale even when local user evidence remains valid.

V1 should distinguish:

- when the source was published or last updated, if known;
- when the source was observed by the external workflow;
- when the claims were extracted;
- when the context must be reviewed again;
- when the context becomes invalid for active use.

## Validation Rules

A submission is invalid when any of these are true:

- missing `space_id` or `namespace_id`;
- any linked candidate, policy, acquisition trace, or context belongs to a
  different `CognitiveSpace`;
- any linked candidate, policy, acquisition trace, or context belongs to a
  different Namespace;
- `KnowledgeSourceCandidate.state` is `rejected` or `expired`;
- `KnowledgeSourcePolicy.state` is not `active`;
- `KnowledgeContext` has no `source_policy_id`;
- `KnowledgeContext` has no `AcquisitionTrace`;
- `private_context_used = true` and `opt_in_proof` is missing or out of scope;
- source locators, metadata, snippets, diagnostics, or downstream links include
  secrets;
- `structured_claims`, provenance, quality signals, freshness, or expiry fields
  are missing from `KnowledgeContext`;
- the payload includes full source documents or raw provider payloads instead
  of bounded claims and snippets.

Invalid secret-bearing payloads must be rejected as whole references. Redaction
is only for diagnostics and log messages; rejected raw payloads and secrets must
not enter Trace, metadata persistence, KnowledgeContext, or any other
persistence.

## Relationship To Memory And Planning

External knowledge is not user Memory.

`KnowledgeContext` may be considered as candidate context for manual
SleepCycle or deterministic Dreaming work when the context is approved,
non-expired, and scoped to the same `CognitiveSpace` and Namespace. Even when
that path is wired:

- external claims do not become Memory automatically;
- external claims do not directly update `GrowthModel`;
- external claims do not directly update or obsolete `PracticePlan`;
- local Trace, FeedbackLoop, and GrowthModel evidence remain higher-priority
  evidence about the user's actual behavior;
- conflicts should become review questions, hypotheses, experiment candidates,
  or DreamCandidate inputs, not silent model overwrites.

Any future Planning or Sleep/Dreaming use must cite `knowledge_context_id` and
preserve the candidate nature of externally sourced knowledge.

## V1 State Flow

```text
KnowledgeSourceCandidate(proposed)
  -> approved
  -> KnowledgeSourcePolicy(active)
  -> KnowledgeContext(candidate)
  -> valid
```

Alternate paths:

```text
KnowledgeSourceCandidate(proposed) -> rejected
KnowledgeSourceCandidate(proposed) -> expired
KnowledgeSourcePolicy(active) -> paused
KnowledgeSourcePolicy(active) -> revoked
KnowledgeSourcePolicy(active) -> expired
KnowledgeContext(candidate) -> rejected
KnowledgeContext(valid) -> expired
```

The state flow is conceptual only. It does not imply routes, persistence,
schedulers, or UI.
