# Reference Adapter Runtime

This command is the executable reference for the L1 Source-to-MemoryNexus flow
defined by ADR-026. It is a bounded one-shot process, not another server or
scheduler.

## Source evidence lifecycle

The canonical identity is `(source_product, source_installation_id,
record_type, record_id, revision)`. One provider-native record has one current
revision. Higher revisions supersede lower revisions; same-revision identical
content replays its acknowledgement; same-revision different content and stale
lower revisions conflict.

Learning Attempt and Learning Session evidence is `contract_trusted`. Learner
Journey Summary evidence starts as `model_derived_unreviewed` and cannot create
an authoritative FeedbackLoop or update important plans by itself. Every
summary carries 1–100 exact current Attempt/Session Source Identity references;
those dependencies are persisted, and a correction or tombstone marks the
dependent summary stale. A higher
revision tombstone has no normalized content. It removes the record from the
`current_source_evidence` view and writes explicit rows to
`source_evidence_invalidations`; downstream Observation and Planning code must
read the view and honor those invalidations rather than the revision log.

The legacy #228 contract has no installation identity or source occurrence
time. Its deterministic compatibility mapping is:

- source product: `memorynexus_compat`
- installation UUID: UUID-shaped first 128 bits of
  `SHA-256("memorynexus:#228:<space UUID>:<namespace UUID>")`, with UUID v5 bits
- record type: `learning_outcome`
- record ID: the existing `source_event_id`
- revision: `1`
- occurrence and observation time: Unix epoch (unknown source time)

This mapping is fixed for each authorized Space/Namespace. It preserves #228's
original idempotency scope while preventing two Spaces that reused the same
event ID from becoming one logical Source record. The Space/Namespace
authorization scope is checked before a replay is accepted.
The mapped request then uses the same canonical fingerprint, transaction, Trace
and acknowledgement implementation as the full Source Identity contract.

## Durable Adapter ledger

`memorynexus-reference-adapter` keeps durable acquired-page state,
normalization-pending/delivery jobs, delivery attempts, safe Gateway
acknowledgements, and an expiring one-shot lease in SQLite. Raw provider records
are never stored: a normalization failure leaves only its stable delivery key
pending and reacquires the same stable source page after restart. The
acknowledged Source cursor advances only after every job in a page receives an
accepted/replayed acknowledgement with the exact expected Source Identity. On
restart an unfinished ledger page is resumed before another Source page is
acquired. A lost success response therefore retries the exact Source Identity
and consumes the Gateway's idempotent replay.

The ledger accepts only the closed typed Surface Gateway Source Evidence DTO.
Unknown/raw/media fields, secret-shaped values, oversized payloads, invalid
variant/trust combinations, and malformed identities are rejected before the
normalized payload is written.

Credentials are read from `MEMORYNEXUS_SOURCE_TOKEN`. They are never written to
the ledger or printed. The reference Source is a JSON page fixture so provider
acquisition remains an explicit replaceable seam:

```bash
MEMORYNEXUS_SOURCE_TOKEN=... cargo run --bin memorynexus-reference-adapter -- \
  sqlite://adapter-ledger.db source-pages.json \
  http://127.0.0.1:3000/api/v1/surfaces 10
```

The fixture has the closed shape `{ "pages": [{ "after_cursor": null,
"page": { "records": [...], "next_cursor": "...", "has_more": false }}] }`.
Each record contains a stable `delivery_key` and a complete normalized Surface
Gateway request under `payload`.

## Study Buddy one-shot Adapter

`memorynexus-study-buddy-adapter` is the concrete Study Buddy runner. It accepts
only the authenticated loopback `/api/integration/source-events` feed and the
loopback MemoryNexus Surface Gateway; redirects and system proxies are disabled.
The feed token and Gateway token stay in environment variables:

```bash
STUDY_BUDDY_SOURCE_TOKEN=... \
MEMORYNEXUS_SOURCE_TOKEN=... \
cargo run --bin memorynexus-study-buddy-adapter -- \
  sqlite://study-buddy-adapter-ledger.db adapter-config.json \
  http://127.0.0.1:3001/api/integration/source-events \
  http://127.0.0.1:3000/api/v1/surfaces 10
```

The configuration is a closed JSON object. It pins one Adapter runner to one
actor, one Managed CognitiveSpace, exactly one opaque source subject reference,
and the Adapter version. Content events must carry that subject in their payload;
content-free tombstones must carry it in the event envelope because configuration
alone cannot prove the subject of an installation-wide record:

```json
{
  "actor_id": "00000000-0000-4000-8000-000000000001",
  "space_id": "00000000-0000-4000-8000-000000000002",
  "allowed_subject_refs": ["00000000-0000-4000-8000-000000000003"],
  "adapter_version": "0.1.0"
}
```

The Namespace is fixed to `learning.foundation`. The mixed feed maps bounded
Learning Attempt records and only those Learning Session records that publish
an explicit supported role plus activity and success counts. Legacy sessions
without those semantics, legacy attempts that predate explicit role/correctness,
and bounded `chat_turn` references are closed-shape terminal skips before raw
content can enter the ledger. Partially upgraded or malformed records fail
closed. Provider-native `original`,
`correction`, and `reinforcement` roles map to distinct records with
provider-neutral controlled goals. Provider mistake labels also map through a
closed provider-neutral taxonomy; unknown labels fail closed. Another learner
response must have another Source Record ID; a factual edit keeps that ID and
increments Source Revision. Withdrawal produces a content-free Source
Tombstone. Unknown roles, subjects, event kinds, revisions, or cursor semantics
fail closed, and additive provider fields are never copied into the normalized
ledger record.

The Observation Surface accepts optional `window_start` and `window_end` on
`get_state_summary`. For `learning.foundation` it returns a
`foundation_learning_observation` that separates facts, deterministic recurring
error and delayed-recurrence aggregates, hypotheses, optional prompts, and
explicit evidence gaps. It reads only the current, same-Space, same-Namespace,
in-window, contract-trusted view. Typed Source Identity citations appear in the
response, while Trace metadata contains counts and gap codes only. The source
application continues to own exact correction obligations and schedules; this
L1 Adapter never writes back.
