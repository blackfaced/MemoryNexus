# Repository profile proposal template

Use Markdown for R1. These sections express the behavior contract's eight
required parts, **not a current CLI JSON schema**. Replace the prompts with
bounded findings, evidence and explicit unknowns. Do not add these fields to
`.memorynexus.json` or bootstrap JSON. Exact interfaces are owned by R2 (#309).

## 1. Identity

- Proposal/profile ID and revision; distinguish these from the future task ID.
- Exact root and snapshot timestamp; Git HEAD/status if available.
- New task or existing config; existing declared task ID, without assuming it
  was validated against the ledger. Copy/move semantics and intended scope.
- Profile content digest in a companion inventory (no self-referential hash).

## 2. Binding — private companion, not shared rules

Record absolute config, external ledger, pinned CLI path/build/hash, bootstrap
bundle version and future script version or `not implemented`. For each binding
label evidence as proposed, file inspected, capability verified, or unverified.
Separate binary capability from this task's identity/ledger verification. Avoid
embedding secrets or guessing another client's environment.

Record config tracking/ignore status and evidence of ledger backup coverage.
Propose required setup separately; neither is changed by producing this profile.

## 3. Existing mechanisms

| Mechanism | Purpose and maintainer | Trigger / reads / writes | Evidence | Keep/change/propose |
| --- | --- | --- | --- | --- |
| Capture and inbox | | | | |
| Human/Agent review and formal knowledge | | | | |
| Task/context injection and Agent memory | | | | |
| Hooks/schedules | | | | |

Separate authority by subject, source assertions, actual observations and owner
decisions. Preserve constraints on upstream time, output paths and actions.
Do not infer “24 configurations = 24 healthy jobs”, or replace an old unknown
count with a guessed definition. Report configured items, planned trigger points
and evidenced successful runs as distinct snapshot metrics.

## 4. Sources

For each selected source propose an opaque identity, allowed text file/region,
extractor rule/version, exclusions, authority scope, provenance and unavailable
evidence. Paths locate content; they are not durable identity. Record whether
identity is native or unresolved and whether actual revisions are available.
Hashing/listing a file is not reading it as evidence.

- Missing `target` means unresolved routing, not unusable or globally consumed.
  A source may support multiple consumer outputs without upstream backfilling.
- Nonmonotonic time, no stable IDs, incomplete lines, duplicate rows, rewrites,
  deletion and movement require explicit R2 handling; do not use event time or
  mtime as a high-water cursor. One append-only writer does not prove all writers
  append or that rotation never occurs.
- Exclude secrets, raw chat/media/log dumps and binary payloads. A text container
  can contain binary content. Media-derived normalized text still requires the
  existing explicit acceptance rule; record a gap instead of ingesting raw bytes.
- Generated/managed regions are excluded by default. Outputs outside the root
  reduce one route of re-ingestion but do not guarantee no feedback loop; check
  every selected source range, retain original lineage and reject self-proof.

## 5. First import

New task: reference a private bootstrap draft and full preview, with conflicts,
gaps and exact manifest digest. Summarize proposed kinds and continuing capture
policy without replacing the complete reviewable text. State which entries carry
concrete experiences and which only provide navigation. Each factual claim needs
read evidence; lesson base entries must actually support its inference.

Existing task: `already initialized; no init/seed`. State the incremental proposal
and that ledger duplicates/currentness are unverified during discovery. Do not
create a replacement task to bypass this boundary.

## 6. Incremental capture and lifecycle handoff

Describe proposed triggers and what should happen for new, changed, missing,
unreadable or moved sources. Separate automatic retention from owner-confirmed
instructions/corrections/retractions; external file edits are not confirmations.
An arbitrary note must remain an external draft until a supported contract
exists. Do not claim current `record` ingests all source kinds.

Include the following lifecycle map, naming the affected proposal entries:

| Event | Existing task behavior / R2 requirement |
| --- | --- |
| New checkpoint | Existing: predecessor becomes historical; it is not deletion. |
| Owner correction | Existing: old revision superseded; dependent lessons stale. |
| Source record invalidated/retracted | Existing: dependent lessons stale, including endorsed ones; fresh review needed. |
| Owner retraction | Existing: live revision-chain text/refs removed; minimal tombstones remain; old backups may still contain text. |
| Context budget omission | Existing: record still stored; inspect omissions and explicit IDs. |
| External file changes | R2/later implementation: detect known source revisions and invalidate affected dependencies; current CLI refs do not watch files. |
| Old but still useful experience | No automatic age-based purge/TTL exists; propose rechecking applicability, not blind deletion. |

## 7. Consumers

Define two consumer IDs and their distinct questions, Lens rule/version, output
target/region, adoption policy, publication approval and invalidation policy.
Use at least one shared source example with independent adopted/deferred outcomes,
clearly labeled hypothetical until actual ledger decisions exist.

For **each** decision list the full evidence dependency set (identity + version)
and separately list navigation-only links. Explain why each evidence item matters.
An auxiliary correction can invalidate only the affected consumer while the
common source stays unchanged. Repeated unchanged runs should produce no duplicate
record/output; deferred/skipped decisions must awaken when relevant evidence or
rules change. Publishing a pointer does not certify the linked mechanism or
upgrade source trust. Show bounded useful summaries without copying parameter
tables; R2 must define version/decision/publication keys before implementation.

## 8. Operation

Name workflow and proposed external trigger, or `not configured`; list the known
upstream time budget, fixed entry/version, result location, timeout and owner query
path. State which parts are current, proposed or unverified. Use one future script
entry consistently across clients; do not create it or a new cron during R1.

Propose manual acceptance before scheduling: one useful sourced experience, two
independent consumers, unchanged rerun, relevant source correction and selective
reassessment. Real scheduled evidence requires an actual scheduler run ID and
trigger source; a manual run or requested client label is not that evidence.
MCP, maintenance packaging and production schedule changes remain separate.
