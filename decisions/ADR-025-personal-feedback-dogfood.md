# ADR-025: Personal Feedback Dogfood

## Status

Accepted

## Context

MemoryNexus remains a local-first long-term feedback engine for personal cognition and skill acquisition. Its primary upstream learning product and evaluation fixture remains Dictation Coach (ADR-020). Before broadening product surfaces, the project needs a private self-use proof that the generic Engine can turn confirmed longitudinal evidence into a useful next adjustment for the owner of a `CognitiveSpace`.

The first proof is a deliberately narrow fourteen-day dogfood experiment in `personal.health.sleep`. It records a bounded evening sleep-and-energy check-in, uses deterministic planning only after a small evidence baseline, and evaluates usefulness through a precommitted owner-run gate. This is not a healthcare product, a clinical study, or a permanent health-product commitment.

## Decision

### Private personal-feedback priority

Personal Feedback Infrastructure is the current private self-use priority. The M9 experiment tests whether the existing Engine loop can produce evidence-backed next actions for its owner:

```text
confirmed check-in -> Trace / FeedbackLoop -> observation -> one advisory action
-> owner-reported outcome -> review
```

`personal.health.sleep` is a Namespace within the existing `CognitiveSpace`. The Space remains the ownership and permission boundary; Namespace is not a separate ACL, identity, or product-role model.

Dictation Coach remains the first upstream learning product and evaluation fixture. M9 is a generic-Engine dogfood slice, not a replacement product line.

### Bounded evidence and authority

The canonical field, correction, provenance, and retention rules are in [Personal Feedback Dogfood Contract](../docs/personal-feedback-dogfood-contract.md). The contract permits only confirmed, typed, low-sensitivity sleep timing or duration, owner-reported daytime energy, and tightly bounded lifestyle context.

OCR, screenshots, source media, and provider conversation state remain outside the Engine under ADR-021. `agent_ocr` data requires role-neutral explicit acceptance or explicit correction before it is submitted. The Engine persists only confirmed normalized values and typed provenance; unavailable source media does not invalidate confirmed canonical evidence.

Planning is deterministic. After three valid confirmed days, it may select at most one reviewed, low-risk, reversible lifestyle experiment at a time. A sparse baseline returns an explicit evidence gap. Suggestions are advisory: MemoryNexus must not execute reminders, calendars, messages, device controls, or any other external action.

Observations, correlations, hypotheses, and owner decisions are distinct. M9 makes no medical, causal, diagnostic, or efficacy claim. Local Trace, FeedbackLoop, and GrowthModel evidence remain the source of truth; M9 adds no automatic external Knowledge Refresh or health-advice lookup.

### Fixed fourteen-day gate

The owner makes one evening check-in per local calendar day. Three valid confirmed days are required before a baseline may be returned. Day seven is a preliminary review only. Day fourteen applies the fixed final gate:

- pass requires at least ten valid confirmed records, at least five tried suggestions, and at least one owner-selected adjustment worth continuing;
- fewer than ten valid records, or fewer than five tried suggestions, is
  insufficient usage / Adapter failure;
- sufficient usage (ten valid records and five tried suggestions) without a
  retained useful adjustment is Engine feedback failure.

The gate cannot be moved after results are observed. #227 is the manual fourteen-calendar-day owner/Coordinator acceptance gate, not an implementation task that closes immediately after #226.

### Adapter and deployment boundary

#130 independently proves the private Mac mini Local Lab. #220 may land without waiting for it. Only after #221 and #130 does #222 select and prove one controlled private Adapter; an authenticated private LAN web Adapter is the preferred first path. Channel, provider, browser, OCR, and deployment brands are Adapter concerns, never Engine branches.

A localhost pass for #130 does not close the independent Trial Profile issue #129.

## Consequences

Positive:

- Gives the generic Engine a falsifiable, owner-run usefulness test without changing the long-term feedback-engine positioning.
- Preserves CognitiveSpace ownership, provider neutrality, and the existing media-confirmation boundary.
- Creates a small policy seam rather than adding unconditional Dictation or health-specific fields to shared Surface responses.

Negative:

- The M9 success claim is intentionally narrow and cannot generalize to a health product, clinical benefit, or public deployment.
- #227 requires real calendar time and private owner participation; it cannot be satisfied by implementation tests alone.
- Future workers must preserve the fixed data allowlist and gate rather than making the dogfood slice an unrestricted health-history intake.

## Non-goals

- No Rust behavior, schema, API, MCP, CLI, frontend, or release implementation in this decision.
- No OCR/provider integration, raw media persistence, or resolver runtime.
- No standalone health app, public deployment, healthcare positioning, diagnosis, treatment, or clinical claim.
- No automatic external knowledge lookup or direct external action.

## Related decisions

- ADR-014: Namespace and Feedback Loop Model
- ADR-016: Local-first Trace Learning Runtime
- ADR-018: Long-term Feedback Engine
- ADR-019: Surfaces, Adapters, and Engine
- ADR-020: Dictation Coach as First Upstream Product
- ADR-021: External Media Evidence References
- ADR-023: Namespace Knowledge Refresh
