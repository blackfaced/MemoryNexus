# Repository memory v1 — kernel, schema and recovery

Normative R2 contract, owner-accepted at `83d85c6` on 2026-10-23; implementation tracked in #310. Entry:
[contract](../repository-memory-v1.md). Runtime transport is the existing CLI.

## 1. Common types, requests and results

`U` is a nonnegative JSON integer ≤ 2^53−1; `UUID` is lowercase canonical UUID;
`H` is 64 lowercase SHA-256 hex digits. Text bounds count Unicode scalar values.
`name` is `[a-z][a-z0-9_-]{0,63}`. Times are RFC3339 with offset. Paths supplied to
core are provenance locators, not files it opens. Relative paths reject empty,
absolute, URI, NUL and `..` components. All JSON objects reject unknown/duplicate
keys and null; omit optional fields. Total stdin ≤32 KiB, stdout ≤128 KiB. No
unbounded metadata bag, arbitrary JSON dispatch or SQL interface is accepted.

New actions use this grammar (one JSON stdin object):

```text
memorynexus-feedback-cli task --config ABS workflow-bind
memorynexus-feedback-cli task --config ABS source-sync
memorynexus-feedback-cli task --config ABS source-capture
memorynexus-feedback-cli task --config ABS consumer-review
memorynexus-feedback-cli task --config ABS consumer-approve
memorynexus-feedback-cli task --config ABS publication-result
memorynexus-feedback-cli task --config ABS workflow-status
memorynexus-feedback-cli task --ledger ABS migrate
```

Every workflow write contains `idempotency_key` (1–128 chars), `run_id: UUID`,
`client` (1–64 chars), `workflow_id: UUID`, `profile_revision: U` (>0), and
`incarnation: UUID`, plus exactly the action fields below. `workflow-bind` also
uses this envelope; its profile_revision is the requested new revision.
`workflow-status` is read-only and uses its own fields. Existing nine commands
are unchanged inputs; do not send them new fields.

Success is exit 0, exactly one stdout object:

```json
{"schema_version":1,"workflow_contract":1,"status":"accepted","task_id":"10000000-0000-4000-8000-000000000001","revision":12,"incarnation":"20000000-0000-4000-8000-000000000001","result":{"object_id":"30000000-0000-4000-8000-000000000001"}}
```

`status` is accepted (new state), replayed (same transport request), noop (same
business state under a different request), or ok (read). Action result fields
replace the example result, as listed below. `revision` is the task's current
revision at the transaction/read, not the original write's revision. A replay
returns the original object IDs plus their current validity, never withdrawn
body text. `incarnation` identifies the active ledger instance, not the task ID.

Errors exit 2 with `{schema_version:1,workflow_contract:1,status:"error",code,
message}`; message is fixed, content-free. Existing codes retain meaning.
Additional codes: `stale_snapshot` (expected source/profile/incarnation no longer
matches), `approval_required` (missing/invalid current exact authorization),
`recovery_required` (restored workflow not reauthorized). `conflict` covers changed
request on a key, CAS failure, incompatible result or competing generation.
`storage_unavailable` includes SQLite lock timeout; it is not a no-op. Invalid
inputs, scopes, versions or authorization produce zero business writes.

### Replay, no-op and transaction order

1. Validate shape/scope/version. Within BEGIN IMMEDIATE, look up task-wide key.
2. Same key + canonical action/request returns replayed, even if the result has
   become stale/retracted. Changed action/request returns conflict. Replay is
   evidence of a past write, never renewed publication authority.
3. A new key validates incarnation, active workflow, current profile and applicable
   policy. Check equivalent-current business state before expected-version CAS;
   identical content gives noop, different content on the same generation gives
   conflict. Then check expected versions for a new transition. Blocked sources
   and recovery gates cannot be bypassed by business noop.
4. Atomically save changed objects, invalidations, task revision and receipt;
   commit before stdout. No branch may commit half of a source invalidation.

Publication-result alone has the historical-effect exception in §6; it does not
authorize a new file mutation and can report an invalidated retained intent.

Transport digest is SHA256 of canonical `{action,request}`. Canonical JSON is
UTF-8, object keys sorted by Unicode code point, separators `,`/`:`, no optional
whitespace, no ASCII escaping, strings NFC-unmodified, no floats. Arrays preserve
order except dependency arrays sorted by `(type,id,revision_or_0,sha256)`; record
dependencies use revision_or_0=0, source revisions start at1. Duplicate typed
identity+revision is invalid_input even if hashes match; two versions of one source
in one decision are also invalid_input. Dependency expansion unions matching
already-declared dependencies rather than duplicating them.
Do not rewrite run/client/time when retrying. A business noop returns existing
object IDs and creates no receipt/revision; exact repeated noop is still noop.
Retain a lost-response request until authoritative read/replay classifies it.

## 2. Profile binding

`workflow-bind` fields:

| Field | Required type / meaning |
| --- | --- |
| `expected_profile_revision` | U; 0 for new workflow, otherwise current revision |
| `profile` | Exact core profile from runner §1; profile.revision equals envelope profile_revision, normally expected+1 |
| `profile_sha256` | H of canonical profile |
| `binding_sha256` | H of canonical private machine binding |
| `confirmation` | literal `confirmed`; exact profile, binding and policies shown to owner |
| `recovery_acknowledged` | optional literal true, required after restore |

Result: `workflow_id`, `profile_revision`, `profile_sha256`, `binding_sha256`,
`state:"active"`. First compare the exact stored profile object (including its
revision), profile/binding hashes and incarnation: equality is noop before checking
expected_profile_revision, so a new-key retry of accepted r1/expected0 stays noop.
This path is disabled while recovery_required. Otherwise require expected current
revision and requested revision=expected+1. Workflow update
requires expected current revision; invalidates all its current decisions and
publication approvals. Changed source selector/extractor additionally stales its
derived draft chains and lessons. Existing unchanged source IDs/cursors survive.
Removed collection/consumer definitions are retained as inactive, never deleted.
An inactive source cannot be captured until a confirmed profile includes it again.
Changing a selector or extraction rule requires a different extractor_version
in that confirmed profile. Each source observation also pins the canonical whole
collection-definition digest. On the next source-sync, a changed definition adds
a new source revision even when selected bytes are identical; no-op requires both
content/availability and definition digest unchanged. This yields a new capture
tuple and keeps the same source/root identity, rather than reactivating a stale
old extraction. Profile bind itself does not claim the new selector was scanned.

The stored profile is the authority for automatic **draft retention** and neutral
stale-output replacement. It does not supersede existing execution/review capture
or authorize code/browser/scheduler actions. Owner confirmation and caller labels
remain trusted local assertions; CLI does not authenticate a human or inspect UI.

## 3. Source identity, revisions and cursors

Profile collections have stable UUID IDs. For `text`, its source ID equals its
collection ID. For `append_jsonl`, an occurrence ID is UUIDv5(namespace=collection
UUID, name=UTF-8 decimal start-byte offset). Locator is not identity. A target or
event time is never a key. Identical rows at distinct offsets are distinct events.
The collection's identity is never reused for a truncated/replaced stream.
An observation locator is exactly `{path,selector}` for text (profile-relative
path and its strict selector), or `{path,start,end}` for JSONL. Source identity
retains collection ID and byte range, while each observation records its then-current
path. Neither path changes nor mutable record.revision redefine that identity.

V1 supports selected UTF-8 text (whole file or exact marker interval) and verified
append-only JSONL. Stream rewrite/truncation/rotation is a reconciliation conflict;
no automatic restart, reset or new collection ID. A confirmed profile may relocate
the same collection to a new path only after its previous complete consumed prefix
matches. A new unrelated stream requires a new owner-approved collection, with
overlap reported rather than silently re-counted as independent evidence.

`source-sync` handles **one collection** per request, with fields:

| Field | Required type / meaning |
| --- | --- |
| `collection_id` | active profile collection UUID |
| `expected_scan` | U, collection CAS counter; initially 0 |
| `observed_at` | RFC3339, scan time, never ingestion cursor |
| `availability` | `available` or `unavailable` |
| `units` | 0–20 unit objects described below |
| `cursor` | JSONL only: `{offset:U,prefix_sha256:H}`; exact consumed-byte prefix |
| `reason` | unavailable only: `missing`/`unreadable`/`identity_uncertain` |

Text available: exactly one unit `{sha256:H}`; digest covers selected UTF-8 bytes
without newline/Unicode normalization. Text unavailable: no units/cursor.
JSONL available: each unit `{start:U,end:U,sha256:H}` covers one complete raw line,
including its newline, in contiguous byte order starting at previous cursor.offset
(initial 0). Its digest includes all original bytes; the adapter may extract only
approved fields. Cursor ends at last unit.end and its digest covers bytes [0,end).
At EOF with no complete new line, units empty and cursor exactly retained. An empty
prefix uses SHA256 of empty bytes. Partial final line stays pending, not skipped.
Unavailable JSONL: no units, retain previous cursor. Core validates range arithmetic,
stored IDs and expected_scan; adapter validates actual bytes/prefix. A syntactically
valid caller assertion is not proof core read the source.

Every source has an immutable sequence of observation revisions starting at 1.
A content or availability transition adds a revision; returning from unavailable
adds one even if bytes match an older version. Unchanged available digest is noop.
Each revision has content digest (last known digest retained while unavailable),
availability, locator, observed_at and extractor version. No source body is stored.
Ordinary JSONL append creates only new occurrence sources: it does not revise earlier rows.
Collection unavailability stales dependent uses of all its known sources; returning
available requires verifying the retained prefix and creates fresh available
revisions for those previously unavailable sources, without duplicating identities.
For a changed JSONL collection definition, an available source-sync first verifies
the entire retained prefix. It creates a definition-bound next revision for **all**
known occurrences, including an EOF request with empty units, and stales their
old derived uses in the same transaction. IDs and cursor stay unchanged; new units,
if present, are then appended. Each occurrence needs only one revision when both
availability and definition change. Paginated sources/inputs expose pending
recapture, and the adapter rereads the known byte ranges for the new extractor.
This does not replay old rows as new events or bypass a blocked source identity.

Changed source revision and stale marks on derived records/lessons/consumer
decisions/approvals commit together. Availability changes preserve raw record text
for explicit audit; ordinary active context and publication exclude stale results.
No read failure is a deletion. Confirmed deletion uses explicit existing `retract`
on the affected draft chain; a missing file alone cannot trigger it.

Before the first successful content read, unavailable is stored on the collection
only (`availability`, `reason`, `observed_at`), with no source/version/digest invented.
Sources result is empty. First later available content creates source revision1;
the collection scan CAS counter already reflects the unavailable observation.
For known sources, unavailable versions retain their last real digest. Repeated
same availability/reason/content/cursor is noop; observed_at alone never forces
a write. JSONL same previously accepted units+hashes+cursor is checked before
CAS/contiguity against the new cursor and is noop; changed overlapping bytes conflict.

Result: `collection_id`, `scan` (CAS counter), `sources` array of
`{source_id,source_revision,sha256,availability}`, `cursor` when JSONL,
`invalidated:{records:U,decisions:U,approvals:U}`. Max 20 **new units** per call;
known-source unavailability invalidation is internal and reported by counts.
The sources array contains only requested units (none for an unavailable scan),
not every affected historical source, so large stream history cannot overflow it.
For returning availability or definition changes, read paginated status to inspect
affected old sources and recapture work; this internal fanout is not limited to20.
The scan counter increments once for a changed collection transaction, not per
unit; no change leaves ledger untouched. A latest unchanged scan is evidenced by
the adapter's run report, not a fabricated ledger update timestamp.

## 4. Capturing a bounded source draft

`source-capture` fields: `source_id:UUID`, `source_revision:U` (>0),
`extractor_version:name`, `text` (1–1,000 chars), `occurred_at`, optional
`confirmation:"confirmed"`. New source draft requires the approved profile's
`automatic_drafts:true` or this exact retention confirmation. It **always** creates
`kind:draft, trust:source_reported`; observed upstream `verified` flags and
retention confirmation cannot raise trust. Structured event/result/conditions/
uncertainty are expressed in bounded text, not a free metadata bag.

One normalized draft per `(source_id,source_revision,extractor_version)`. Same
tuple/text/time returns existing ID noop; different text/time conflicts. Store
only that body's record, provenance ref and source mapping. The first capture
creates a revision-chain root. Later source/extractor refresh appends to that same
chain; the previous derived record stays stale, never becomes owner-confirmed.
The action receipt distinguishes automatic source refresh from confirmed correct.
Current source version and extractor must match and be available. Withdrawn source
identity is permanently blocked from capture in v1, even with a new key/run.

Result: `record_id`, `root_record_id`, `source_id`, `source_revision`,
`state:"current"`, `kind:"draft"`, `trust:"source_reported"`.
Those state/trust values describe a new capture. Replay/noop reports the mapped
record's actual state/trust, including superseded/stale/retracted; it never
reactivates an old extraction. Inputs inspection returns the current chain head.
`correct` on a current draft retains kind, origin and dependencies, creating an
owner_confirmed revision as today; drafts remain outside ordinary context.
Existing `review` may use a current draft as a base source and still only produces
a lesson candidate. Stale drafts need a fresh source-capture, not correct.

Ordinary `context` keeps its v1 fields, grouping/limits and excludes every draft.
Explicit record_ids inspection may return drafts labeled by kind/trust. It never
silently maps them to executions. Source navigation refs are not dependencies;
existing records gain no monitored source dependency from a matching path alone.

## 5. Consumer review and complete dependencies

`consumer-review` fields:

| Field | Meaning |
| --- | --- |
| `consumer_id` | name in active profile |
| `input_record_id` | current same-task base record (draft/checkpoint/execution/instruction); no lesson input |
| `decision` | `proposed`, `skipped`, or `deferred` |
| `reason` | 1–240 chars, bounded rationale; no source/body duplication |
| `dependencies` | 1–20 exact dependency objects |
| `output_sha256` | H, required only for proposed; UTF-8 proposed managed-block body |
| `previous_decision_id` | optional UUID; required when replacing an existing slot |
| `reconsider` | optional literal true; required to replace a still-current decision |
| `confirmation` | optional literal confirmed; retention approval when automatic_reviews is false |

Dependency is exactly `{type:"record",id:UUID,sha256:H}` or
`{type:"source",id:UUID,revision:U,sha256:H}`. Record digest covers its exact text;
record IDs identify immutable content, unlike mutable record.revision (last state
change). Include input and all evidence actually used, including auxiliary
corrections. Core expands source origins and existing base-record links, verifies
currentness, and stores the sorted union. It cannot detect undeclared reads: Skill
must declare them, and walkthroughs enforce that seam. Navigation-only links are
not in dependencies and do not support factual claims.

Consumer review needs automatic_reviews from the confirmed profile or exact
retention confirmation. A blocked source, including one reached through expansion,
is conflict; it is excluded from pending input work. Neither a source dependency
nor a new key can bypass confirmed withdrawal. V1 has no unblock/reapprove-source
action. Unavailable or noncurrent dependency is stale_snapshot.

A consumer slot is `(task_id,workflow_id,consumer_id,root_record_id)`. Source
refresh and owner correct preserve root, so they replace the same document block.
Each slot has immutable numbered generations and exactly one current generation.
Same input/dependencies/Lens/profile/original proposed-decision/reason/output
returns existing noop, including a different run/key. Store the immutable original
proposal separately from adoption state: repeating `proposed` after approval is
noop and returns actual `decision:approved`, not a new proposal. Different content with no explicit predecessor is
conflict. Reconsidering current state invalidates the old approval; no silent edit.
Stale predecessor can be replaced without reconsider=true, but must be named.

Result: `decision_id`, `generation`, `root_record_id`, `decision`,
`validity` (actual current/stale/retracted), `dependency_sha256`, `decision_sha256`, `block_id`.
dependency_sha256 is SHA256 of the canonical expanded dependency array.
decision_sha256 is SHA256 of canonical `{task_id,workflow_id,consumer_id,
root_record_id,generation,input_record_id,proposed_decision,reason,dependencies,
output_sha256?,profile_revision,incarnation,lens_version,rules_sha256,target_id}`.
Omit output_sha256 when absent. This immutable digest excludes transport
key/run/client, adoption state, validity and publication; preserve it as an audit
digest after withdrawal scrubs reason. Block ID is lowercase SHA256 of canonical
`{task_id,workflow_id,consumer_id,root_record_id}` and remains stable across generations.
Skipped/deferred remain settled until dependency/rule change or explicit reconsider;
they are not regenerated each run. No source-global processed state exists.

Every source transition, checkpoint historicalization, correct and retract must
invalidate all dependent consumer decisions/approvals in the same transaction as
the record/source change, including approved or deferred decisions. Complete
dependencies give selective invalidation: changed auxiliary evidence for B does
not invalidate A if A did not use it. Unchanged source is not whole-workflow noop
when stale decisions or pending publication intents remain.

## 6. Exact publication approval and result

`consumer-approve` fields: `decision_id:UUID`, `decision_sha256:H`,
`proposal:{target_id:name,block_id:H,base_exists:boolean,base_sha256:H,after_sha256:H,output_sha256:H,
plan_sha256:H}`, `confirmation:"confirmed"`.
Only a current proposed decision with matching profile/Lens/dependencies may be
approved. All digests and exact before/after diff must be shown to the human.
Core checks output/block/target identity, not file bytes. Approval creates an
immutable publication intent; result is `approval_id`, `decision_id`,
`decision:"approved"`, `validity:"current"`, `publication:"not_applied"`, and
the exact proposal. One identical approval is noop. Changed output requires a new
consumer decision generation. An unchanged output may renew an unapplied intent
using optional `previous_approval_id:UUID` and a fresh owner-confirmed plan digest
(#349). The predecessor must be the current substantive intent for this exact
decision/incarnation/profile, with no applied attempt; pending, failed or conflict
intents may be explicitly replaced. The new intent stores `previous_approval_id`;
the predecessor becomes stale and records `superseded_by` without deleting any
proposal, attempt or receipt. A competing successor, stale/superseded predecessor,
changed output, applied predecessor or different scope is conflict. Exact original
key/bytes still replay history but never restore authority. Superseded intents
remain in publication history but not actionable pending counts/recovery work. Missing predecessor
is not_found. No automatic free-text publish or automatic renewal authorization.

`publication-result` fields: `approval_id:UUID`, `outcome:"applied"|"conflict"|"failed"`,
`observed_at`, optional `observed_sha256:H`, optional `error_code`.
Applied requires observed_sha256 equal intent.after_sha256 and no error_code.
Conflict requires observed_sha256; error_code is `baseline_changed` or
`managed_region_changed`. Failed requires error_code from `io_error`,
`source_unavailable`, `plan_missing`; observed_sha256 is optional. No paths or
private exception text in error_code. A retry after failed/conflict can append a
new attempt with a new transport key only against the same still-valid intent.

Result: `approval_id`, `attempt_id`, `outcome`, `validity` (current/stale/retracted),
`publication` (applied/conflict/failed). If invalidated between file replacement
and CLI result, record the actual applied effect as **stale**, do not reject away
the evidence or claim current. An exact repeated applied result is noop. A
retracted decision never returns body; receipt can record its outstanding external
effect. A different incarnation is recovery_required, not a resumed old approval.

Historical-effect exception: publication-result may retain an effect against its
existing same-task intent even if the request's profile revision is now inactive
or newer source/profile state invalidated the intent. Validate exact retained
intent identity, original profile revision, same current incarnation, task scope
and result hashes; bypass only the current-profile/active-decision gate. Save the
effect as stale/retracted. This endpoint cannot grant authority to write a file,
modify a proposal or reactivate a decision. Restore incarnation gate still applies.

Neutral stale-output replacement is authorized only by profile policy
`neutralize_stale:true`. Its intent uses the same endpoint with
`confirmation` omitted and `maintenance:"neutralize"` instead; this is the sole
variant of consumer-approve, valid only for a stale/retracted previously published
decision. Output bytes must equal renderer's fixed neutral block (runner §4),
`output_sha256` must match those bytes, and existing managed block must be intact.
This creates a maintenance intent without changing decision back to approved or
upgrading source trust. Any new substantive wording still needs owner confirmation.

## 7. Reads, limits and durable schema

`workflow-status` input: `workflow_id:UUID` optional (omit for ledger/task header),
`view` one of summary/collections/sources/inputs/decisions/publications/receipts (default
summary), `limit:U` 1–20 default20, optional `at_revision:U`, `after:U`,
`ids:[UUID]` ≤20 or `keys:[string]` ≤20 (receipts only). after defaults0; ids/keys
cannot combine with after. First page returns current task revision; subsequent
pages must send it as at_revision. Any change returns conflict; restart, never
skip using an outdated page cursor. Result contains `items`, `next_after` only
when more exist, and `complete` boolean. Collections/sources/decisions/publications
sort by their immutable workflow_sequence; inputs by record commit_sequence;
receipts by original task revision (one task write per revision). `after` and
next_after use that view's sequence, never wall time. Explicit ids/keys return in
request order with complete=true and no next_after. Summary is one item with current profile/binding, incarnation,
recovery_required, counts by state, known last source transitions and pending
intents. Body text appears only in explicit inputs view; diagnostics contain counts.
No reads create a DB, upgrade schema, observe files or repair state.

Omitting workflow_id is valid only for summary; header item is
`{task_id,incarnation,ledger_version:2,workflow_contract:1,workflows:[UUID...]}`.
An explicit missing ID/key is not_found; an existing cross-task ID is scope_mismatch.
Duplicate IDs/keys are invalid_input. Exact item projections are:

| view | Item fields (optional fields marked ?) |
| --- | --- |
| summary | workflow_id, profile_revision, profile_sha256, binding_sha256, state(active/recovery_required), counts `{sources,inputs,current_decisions,stale_decisions,pending_publications}`, last_transition_at? |
| collections | collection_id, format, scan, availability(unknown/available/unavailable), reason?, observed_at?, cursor? (JSONL), active |
| sources | source_id, collection_id, source_revision, sha256, availability, observed_at, locator, extractor_version, blocked, root_record_id?; current observation per source |
| inputs | v1 record fields id/kind/trust/occurred_at/run_id/client/state/revision/source_ids, root_record_id, text/evidence_refs only when current, origin? `{source_id,source_revision,extractor_version}`; current base chain heads by default |
| decisions | decision_id, consumer_id, root_record_id, generation, input_record_id, proposed_decision, decision, validity, dependencies, dependency_sha256, decision_sha256, output_sha256?, block_id, reason? (absent when scrubbed), profile_revision |
| publications | approval_id, decision_id, incarnation, profile_revision, proposal, maintenance?, previous_approval_id?, superseded_by?, validity, publication(not_applied/applied/conflict/failed), attempts `[ {attempt_id,outcome,observed_at,observed_sha256?,error_code?} ]` |
| receipts | idempotency_key, action, digest, result_type, result_id, revision(original commit), result (original identities plus current validity, no text) |

Old source revisions and complete attempt history remain in export. Publications
view returns only the latest attempt in its attempts array (or empty), preventing
unbounded result growth. Core snapshots determine current validity even for a
replay/noop; clients must not equate accepted IDs with fresh authorization.

Schema v2 keeps task IDs, records and existing source/revision links. New record
kind/trust strings above and source-refresh lineage are supported. Add the following
logical tables with these keys/constraints; JSON columns use only the strict types
defined here, not extensibility bags:

| Table | Key and retained fields |
| --- | --- |
| `task_ledger_meta` | singleton=1, incarnation UUID, workflow_sequence U counter; schema version remains PRAGMA user_version |
| `task_workflows` | PK(task_id,workflow_id); profile_revision, canonical profile, profile/binding digests, incarnation, active/recovery_required |
| `task_source_collections` | PK(task_id,workflow_id,collection_id); scan, cursor, availability/reason/observed_at, active definition digest, workflow_sequence |
| `task_sources` | PK(task_id,source_id); workflow/collection FK, root_record_id optional, current_revision, blocked boolean, JSONL immutable start/end when applicable, workflow_sequence |
| `task_source_versions` | PK(task_id,source_id,revision); digest, availability, observed_at, locator, collection_definition_sha256 and extractor version |
| `task_source_extractions` | unique(source_id,source_revision,extractor_version); record_id FK unique, root follows task_record_revisions |
| `task_consumer_decisions` | ID PK; slot/generation unique; typed fields/dependencies/digests from §5, immutable proposed_decision plus adoption decision/validity, workflow_sequence |
| `task_publications` | approval_id PK; decision FK, incarnation, immutable proposal, maintenance flag, validity, workflow_sequence |
| `task_publication_attempts` | attempt_id PK; approval FK, outcome/hash/time/error, workflow_sequence |

Existing `task_receipts` gains `result_type` (record/source_sync/capture/decision/
approval/publication/workflow) and `result_id`; record_id becomes optional only for
non-record actions. Preserve old record_id and digest verbatim. For source_sync,
the result ID identifies its retained minimal receipt result containing collection
scan/source IDs, not raw content. Store minimal action-specific response identity
in `result_json` (no text), FK/check constraints for each type, and preserve one
task-wide PK(task_id,idempotency_key) across old/new commands. Init receipts remain
unchanged. Workflow sequence increments the singleton counter inside the same
transaction for new collection/source/decision/intent/attempt objects; state changes retain it and advance task
revision. No-op never allocates sequence or revision.

Export v2 result has `format:"memorynexus-task-ledger",version:2`, existing v1
arrays and commit_sequence, plus arrays named for the nine new tables (without
`task_` prefix), `workflow_sequence`. Every row above is exported; canonical JSON
objects are decoded, withdrawn text absent. This is inspection, not JSON import.
Foreign keys scope every linked object to its task/workflow. Max 30 collections,
2 consumers, 20 source units per sync, 20 dependencies per decision after expansion.
Exceeding an expanded limit is invalid_input, never truncation. Source and receipt
history is not automatically pruned.

### Retraction extension with existing confirmation meaning

Existing correct/retract still require confirmation for that exact request. On
retract, source-captured refresh/correction revisions belong to one chain and all
are scrubbed; mark its source identity blocked to prevent recapture under a new
key. Existing non-source record chains keep their meaning. Traverse source links
to every affected lesson and its full revision chain: scrub all of their text and
evidence refs; mark each current lesson stale, retain older states and all links.
Core does not guess which words reveal withdrawn input. Affected consumer decisions
and approvals become retracted and their reason text is scrubbed; output bytes in private pending plans are quarantined for
deletion/neutralization by the adapter, never returned from CLI. Publication hashes
and tombstones remain. Subsequent exports cannot expose scrubbed live bodies.
Source edits only stale, not scrub: this stronger behavior requires owner retract.

## 8. Migration, backup and restore

`migrate` reads stdin `{path:ABS,backup_path:ABS,backup_sha256:H,expected_version:1,target_version:2,
confirmation:"confirmed"}`. --ledger is the existing v1 source; path is a new v2
destination, not an in-place overwrite. Require a separately verified v1 backup,
all writers stopped, exact supported v1 schema/integrity/FKs, no destination or
sidecars. Snapshot consistently, add v2 schema to a temporary sibling, preserve
all old identities/receipts/policies/body/order, assign a new incarnation, validate,
close DELETE mode/fsync/no-clobber publish. Validate backup's final hash, exact v1
schema/integrity and full logical export equality to the migration snapshot;
mismatch is conflict. Backup/source/destination paths must differ. Destination is not activated or bound
automatically. Response retains schema_version1, workflow_contract1, status ok,
revision=sum task revisions, result `{path,format,version:2,incarnation}`. Source
unchanged. Crash before publication leaves no destination; crash after publication
leaves complete v2 to inspect. Destination exists on retry => conflict, inspect
then use it; never overwrite. No down migration and no source dual writes.

| Tool/operation | v1 ledger | v2 ledger |
| --- | --- | --- |
| Current v1 task binary | existing behavior | unsupported_version before mutation |
| New binary existing commands | existing behavior, no upgrade | same legacy command contract plus lifecycle hooks |
| New workflow commands | unsupported_version | this contract |
| New init | creates v1 (bootstrap compatibility) | existing matching init receipt only; never new schema implicitly |
| New export/backup | version1 | version2, complete new state |
| New restore | restore as v1 | restore as v2 with recovery gate below |
| migrate | explicit v1→new v2 path | unsupported_version; no repeated implicit migration |
| Unknown schema / wrong ledger | reject unchanged | reject unchanged |

Keep application_id, config version1 and existing command response version1
independent of DB user_version2. Normal reads, help and restore never migrate.
Backup retains original incarnation and complete receipts; a v2 restore assigns
a new incarnation, marks every workflow recovery_required and invalidates every
old publication authorization. Historical approval/receipt rows remain readable,
never executable. Existing record trust/replay identities remain; restore is not
a new fact endorsement. New workflow writes require full source/file reconciliation
and workflow-bind with exact renewed confirmation + recovery_acknowledged=true.
Old-incarnation requests may retrieve a retained replay but cannot authorize a new
effect. Pending file plans cannot resume across that gate.

An old snapshot cannot reconstruct later unbacked withdrawals or corrections.
Old bodies may reappear on restore; report the backup point and reconcile known
later receipts/tombstones before enabling writes. A new incarnation prevents old
publication approval reuse; it does not prove recovered claims are current or
erase unavailable future history. Manual file rollback bypassing supported restore
is unsupported; single-ledger storage cannot detect every such replacement.

Before activating a migrated/restored path, inventory **every** pinned binary in
sessions, Skills, scheduled calls and backup/export/restore entries. Upgrade or
disable incompatible callers and verify exact hash/help/schema support. A current
task-aware experiment binary rejects the unchanged application_id; older binaries
predating that guard cannot be assumed safe. File permissions are the owner trust
boundary, not protection against an obsolete trusted writer. No reverse cutover
to a pre-v2 tool against the v2 file; retain v1 backup separately.

## Owner-approved publication removal

The bounded extension in [publication maintenance](publication-maintenance.md)
adds `maintenance: remove` to `consumer-approve`, retaining the existing envelope,
publication tables and result command. Removal is always owner-confirmed and
records its reason and previous applied approval; it never retracts a source or
uses automatic neutralization authority.
