# ADR-026: Loose-Coupled Source Adapter Synchronization

## Status

Accepted

## Context

DeepTutor and Study Buddy run independently on the private Mac mini and own
their teaching, interaction, operational state, and source data. MemoryNexus
must not become their runtime, copy their databases, or make their foreground
paths depend on the long-term feedback engine.

MemoryNexus does need selected longitudinal evidence from both products:

- completed learning attempts and sessions from DeepTutor and Study Buddy; and
- bounded summaries of the child's Study Buddy conversations so the owner can
  review changes in interests, difficulties, help-seeking, persistence, and
  explicit emotional expression over time.

The existing Study Buddy integration appends MemoryNexus-shaped records to a
JSONL outbox and runs a product-local worker against an older `/memories`
contract. That shape is source-specific, is not transactional with the Study
Buddy domain write, and makes the source product understand a MemoryNexus API.
DeepTutor exposes session APIs but no durable change feed. A single universal
source protocol would force both products into the first provider's model.

This decision incorporates and replaces the unmerged SOTA Reference Adapter
draft previously recorded in commit `b1ff0ce`. The durable direction remains:
external products are replaceable reference sources, provider semantics stay
outside the Engine, and all accepted evidence enters through Surface Gateway.

## Decision

### Keep three independently operating systems

The runtime boundary is:

```text
DeepTutor / Study Buddy
  own source data and foreground product behavior
        |
        v
independent Reference Adapter Runners
  acquire, checkpoint, summarize, normalize, reconcile, and retry
        |
        v
MemoryNexus Surface Gateway
  authorizes, validates, deduplicates, traces, and records evidence
        |
        v
MemoryNexus Engine
  owns longitudinal interpretation, feedback, and bounded next actions
```

MemoryNexus owns the two Reference Adapter implementations, and the repository
keeps the Rust-first direction. Each Adapter is built and operated as an
independent process rather than as a background task inside the Axum server.
The two Adapters may share a small runtime for ledger, lease, retry,
authentication, and reconciliation behavior. This is not a plugin framework,
connector marketplace, second backend, or distributed event platform.

The source-to-Adapter seam remains provider-native. The common contract starts
only after normalization, at the Adapter-to-Surface-Gateway seam.

### Use stable Source Identity and typed Normalized Outcomes

Every source record is identified across retries, migration, correction, and
withdrawal by:

- source product;
- immutable source installation ID;
- provider-native record type;
- provider-native record ID; and
- revision.

The installation ID is generated once and migrated with the source data. It
must not be derived from hostname, IP address, filesystem path, child name, or
other mutable or identifying text.

Adapters submit a versioned envelope containing:

- Source Identity;
- occurrence and observation times;
- one typed evidence variant;
- Adapter and source provenance;
- Evidence Trust; and
- correction or tombstone semantics when applicable.

The initial evidence variants are:

- `learning_attempt`;
- `learning_session`; and
- `learner_journey_summary`.

The contract is a discriminated union, not an unrestricted metadata bag.
Unknown variants and unknown contract fields are rejected explicitly.
Provider-specific labels, scores, mastery concepts, game levels, and raw
payloads remain inside the Adapter. A provider concept becomes a shared Engine
field only after the same semantic need appears in three independent Adapters.

The Surface Gateway idempotency boundary uses the full Source Identity. An
identical revision and content returns the existing result. The same identity
and revision with different content is a conflict and never a silent overwrite.

### Treat source correction and withdrawal as authoritative

A source correction keeps the same Source Identity and advances its revision.
The new revision supersedes the old source content.

A source deletion produces a Source Tombstone. MemoryNexus removes the source
content from current evidence and future interpretation while retaining only
the minimum identity and withdrawal state required to prevent re-import.
Dependent summaries, feedback, and interpretations become stale and are
recomputed or excluded.

An owner correction inside MemoryNexus does not mutate the source product. It
is a separate, owner-authored statement with its own provenance.

### Isolate people with CognitiveSpace, not Namespace

The owner's DeepTutor evidence and the child's Study Buddy evidence live in
different CognitiveSpaces:

| Data subject | CognitiveSpace governance | Namespace |
| --- | --- | --- |
| Owner | owner-controlled private Space | `learning.self-directed` |
| Child | parent-managed private Space | `learning.foundation` |

The parent is the initial managing member of the child's Managed
CognitiveSpace. The Engine does not add parent or child product roles;
relationship mapping and age-appropriate disclosure remain Adapter concerns.
The child is told that Study Buddy conversations may contribute to private
family learning review and can pause synchronization or request correction and
deletion.

Provider names belong in provenance, never in Namespace. The Study Buddy
outcome records and Learner Journey Summaries stay together in
`learning.foundation` so a FeedbackLoop can relate observed performance to the
learner's expressed experience.

Each Adapter uses a separate least-privilege credential pinned to one
CognitiveSpace, an explicit Namespace allowlist, and allowed Surface actions.
Source payloads cannot select an arbitrary Space or Namespace. A future
multi-child Adapter must use an explicit subject-to-Space mapping and quarantine
unknown subjects instead of choosing a default.

### Make Study Buddy expose a provider-owned transactional feed

Study Buddy replaces its MemoryNexus-specific JSONL outbox with a provider-
owned, versioned source-event feed backed by its existing SQLite database.

The product writes the domain row and corresponding source-event row in the
same transaction. The event describes Study Buddy semantics and contains no
MemoryNexus token, CognitiveSpace, Namespace, Surface action, or MemoryNexus
payload.

Study Buddy exposes authenticated localhost integration APIs for:

- paginated source-event acquisition by stable cursor; and
- bounded retrieval of referenced chat turns needed to generate a Summary
  Window.

The event feed may hold stable chat references but does not duplicate chat
content. The Reference Adapter reads only the turns required for a Summary
Window and does not persist raw chat in MemoryNexus.

The Adapter never opens the Study Buddy SQLite file directly. The old
`nexus-worker`, `/memories` client, and `nexus-outbox.jsonl` path are retired
after a dry-run inventory and one-time legacy migration. The cutover forbids
dual writes. Unrecognized legacy records remain in a migration report instead
of being guessed into the new contract.

### Adapt DeepTutor without forking it in the first version

The first DeepTutor Adapter uses the product's supported localhost session
list/detail APIs. It does not read DeepTutor storage directly or add a
MemoryNexus-specific plugin.

Because the current API lacks a durable change cursor, the Adapter:

- scans recent sessions during frequent incremental runs;
- reconciles the last seven days nightly;
- builds a complete session inventory weekly;
- detects revisions using stable IDs and content fingerprints; and
- emits a tombstone only after a record is absent from two successful complete
  inventories.

A partial or failed inventory never proves deletion. If real operation shows
that supported APIs cannot provide stable correction or deletion semantics, a
provider-neutral DeepTutor export/feed extension requires a follow-up decision.
Direct database access remains forbidden.

### Use durable Adapter-owned progress and one-shot scheduling

Each Adapter owns a local SQLite operational ledger containing independent:

- source acquisition cursors;
- delivery attempts and acknowledgements;
- Summary Window jobs and generated-but-not-yet-delivered results;
- leases;
- retry state; and
- dead-letter state.

Provider-specific cursors do not enter the MemoryNexus Engine. A cursor advances
only after Surface Gateway acknowledges the corresponding outcome. Delivery is
at least once; stable Source Identity makes retry idempotent. If Surface Gateway
commits but its response is lost, retrying the same revision returns the
existing result.

`launchd` invokes bounded one-shot `run-due` jobs every five minutes and at
load. The runner acquires a lease, processes all due work, and exits. Scheduling
is only a wake-up mechanism; the ledger determines correctness and catch-up.
Missed launches, sleep, reboot, and crashes therefore leave work pending rather
than skipped.

Frequent runs acquire new structured outcomes. Nightly work generates due
Learner Journey Summaries and reconciles the recent seven-day source window.
Weekly work performs the DeepTutor full inventory. A generated summary is
retained locally when delivery fails so retry does not call the model again.

### Store bounded Learner Journey Summaries, not chat archives

Study Buddy chat remains the source of truth for raw conversation. The Study
Buddy Reference Adapter produces at most one current Learner Journey Summary
for each due Summary Window. The summary has a fixed structure:

- window and source coverage;
- themes;
- explicit expressions;
- observable learning behaviors;
- difficulties and help requests;
- changes from the previous window;
- up to three review questions;
- a short narrative;
- source session/turn references;
- generation provenance; and
- Evidence Trust.

The narrative is bounded to approximately 500 Chinese characters. It
paraphrases rather than copying child-authored text. Statements distinguish
explicit expression from observable behavior. Insufficient source evidence
produces an evidence gap, not a speculative summary. The summary excludes
direct identity details, diagnosis, personality judgment, hidden-motive claims,
and prescriptive discipline advice.

Summary generation is local-first. A cloud model may be used only through an
explicitly configured provider already authorized for this content. The
Adapter redacts direct identity details locally, sends only the current bounded
window, records provider/model/prompt/schema provenance, never logs prompts or
responses, and never silently changes provider. A model or prompt change
creates a new revision.

Contract-trusted Learning Attempts and Learning Sessions may participate in
automatic feedback. A new summary is stored as `model_derived_unreviewed`: it
may support trend display and review questions but cannot directly update an
authoritative GrowthModel or independently trigger an important plan change.
Owner confirmation upgrades it to `owner_confirmed`; an owner edit becomes
`owner_corrected`.

MemoryNexus owns the authenticated owner-review workflow for querying,
confirming, correcting, and deleting Summary Windows. A weekly
`learning.foundation` review composes the daily windows. Mavis/WeChat may send
a low-sensitivity preview and link or instruction, but only an authenticated
MemoryNexus action changes Evidence Trust or content.

### Keep the first integration one-way

The first version is L1 ingestion only. MemoryNexus observations, GrowthModel
state, plans, and owner corrections do not automatically modify DeepTutor or
Study Buddy behavior. A bounded L2 return path requires separate evidence that
the ingestion and review contract is reliable.

### Roll out with bounded backfill and health reporting

Initial import starts with a dry run that reports record counts, time ranges,
event types, subject mappings, and Summary Windows without uploading content.
The first approved backfill is:

- 30 days of completed DeepTutor learning outcomes;
- 30 days of Study Buddy structured outcomes; and
- 14 days of Study Buddy Learner Journey Summaries.

Older data requires an explicit `backfill --since` operation.

A single failure is logged and retried silently. Three consecutive failures or
more than 24 hours of lag produces one redacted alert through the existing
Mavis/WeChat path. The alert contains source, failure stage, job ID, lag, and a
safe diagnostic command; it contains no source content, child summary, raw
payload, or credential. Repeated alerts are capped at one per day, followed by
one recovery notification.

## Test Seams and Acceptance

Tests use the highest stable seams:

1. the source integration APIs exposed by Study Buddy and DeepTutor;
2. the Adapter command boundary with fake source and Surface Gateway clients
   plus a temporary operational ledger; and
3. Surface Gateway as the MemoryNexus ingestion boundary.

Implementation is not accepted without automated coverage for:

- stable retry and duplicate suppression;
- lost success responses;
- cursor advancement only after acknowledgement;
- same-revision content conflicts;
- correction and tombstone propagation;
- dependent-summary invalidation;
- Summary Window generation and delivery retry;
- reuse of generated output after delivery failure;
- lease exclusion for overlapping runs;
- redaction of logs and alerts;
- alert thresholds and recovery notification; and
- legacy Study Buddy migration without dual write.

Mac mini acceptance uses a dedicated test CognitiveSpace and synthetic data:

1. stop MemoryNexus;
2. create test records in both source products;
3. verify Adapter failure retains work and cursor position;
4. restore MemoryNexus and verify catch-up without duplicates;
5. fail one summary window and verify next-run backfill;
6. modify and delete source records and verify revision/tombstone behavior;
7. verify alerts contain operational metadata only; and
8. finish with one low-sensitivity, explicitly confirmed real-record smoke test.

Rust changes must pass formatting, unit/integration tests, and Clippy. Tests
that require the real Mac mini products or Mavis/WeChat remain a local release
gate rather than a GitHub CI dependency.

## Consequences

Positive:

- DeepTutor, Study Buddy, and MemoryNexus can start, stop, upgrade, and fail
  independently.
- MemoryNexus gets provider-neutral longitudinal evidence without copying
  source databases or raw chat.
- Source-specific change detection, pagination, and retry semantics remain
  outside the Engine.
- Separate CognitiveSpaces prevent the owner's and child's evidence from
  mixing even when provider or Namespace configuration changes.
- Durable cursors, revisions, and tombstones make failure recovery and deletion
  behavior explicit and testable.

Negative:

- Two independent Adapter binaries and their ledgers add local operational
  state.
- DeepTutor polling is less efficient than a durable source feed and requires
  periodic full reconciliation.
- Daily model-derived summaries need owner review before they can influence
  authoritative longitudinal interpretation.
- Study Buddy requires a source-event migration before the new Adapter can be
  enabled.

## Non-Goals

- No raw chat, attachment, tool trace, chain-of-thought, or unrestricted
  provider memory synchronization.
- No direct source database access.
- No provider-specific fields in the Engine contract.
- No automatic write-back to DeepTutor or Study Buddy.
- No new parent dashboard in Study Buddy.
- No family-wide CognitiveSpace or Namespace-based permission boundary.
- No scheduler, connector marketplace, dynamic plugin system, second backend,
  distributed queue, or general-purpose event platform.
- No medical or psychological diagnosis, personality model, or hidden mental
  state inference.

## Related Decisions

- ADR-009: Rust-first Backend
- ADR-014: Namespace and Feedback Loop Model
- ADR-016: Local-first Trace Learning Runtime
- ADR-017: Sleep-based Memory Consolidation
- ADR-018: Long-term Feedback Engine
- ADR-019: Surfaces, Adapters, and Engine
- ADR-021: External Media Evidence References
- ADR-024: Private Mac mini Local Lab
- ADR-025: Personal Feedback Dogfood
