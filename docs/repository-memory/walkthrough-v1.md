# Repository memory v1 — request walkthrough and acceptance handoff

**Synthetic contract review only. No runtime or real-client acceptance has run.**
The [JSON fixture](examples/sequence-v1.json) contains complete request objects,
profile/binding, source text/digests and rendered outputs. Its IDs stand for returned
values in an execution; illustrative decision/plan/binary/package/backup/source
AGENTS hashes are explicitly not approvals or measured artifacts. Before an actual
request, use validated response identities and actual artifact hashes. The fixture
is not an executable script and must never be pointed at a production ledger.

## 1. Command and response conventions

For every `requests.<name>` in the fixture, action selects exactly the kernel
command and input is the complete JSON stdin object (no outer fixture wrapper):

```text
BIN task --config CONFIG ACTION < request.json
BIN task --ledger LEDGER export|backup|restore|migrate < request.json
```

Use --ledger only for those four ledger operations; all other fixture actions use
--config. Set MEMORYNEXUS_TASK_LEDGER explicitly to the selected existing path.
Current v1 uses task-v1.sqlite; migration creates task-v2.sqlite; restore target
is a third nonexistent restored-v2.sqlite. Switching the env/binding is an explicit
owner-approved cutover after all pinned callers are compatible, never fallback.

Generated task/incarnation/record/decision/approval IDs are aliases in the fixture,
not preselected UUIDs a real init may be forced to use. Dynamic decision hashes
come from consumer-review; plan hash comes from canonical immutable preview;
backup hash is ordinary final file SHA256. Source and output digests in the fixture
are actual hashes of the included synthetic strings. Agent must see/confirm each
required exact artifact in the real workflow; copied `confirmation` text is not
proof of consent.

Representative source-capture response:

```json
{"schema_version":1,"workflow_contract":1,"status":"accepted","task_id":"10000000-0000-4000-8000-000000000001","revision":7,"incarnation":"20000000-0000-4000-8000-000000000001","result":{"record_id":"80000000-0000-4000-8000-000000000001","root_record_id":"80000000-0000-4000-8000-000000000001","source_id":"50000000-0000-4000-8000-000000000001","source_revision":1,"state":"current","kind":"draft","trust":"source_reported"}}
```

All example revisions are derived for the stated path; an existing task's prior
revision is retained, never reset to the new-task example. Errors use the fixed
kernel envelope and exit2, for example:

```json
{"schema_version":1,"workflow_contract":1,"status":"error","code":"stale_snapshot","message":"workflow snapshot is no longer current"}
```

## 2. Complete new-task path

1. R1 inspect/profile and existing bootstrap preview: show exact seed, sources,
   task binding and continuing execution/review policy. Source/Git drift means a
   new preview **and human confirmation**. Do not use R2 to weaken bootstrap.
2. Bootstrap executes fixture `init` then `seed`, using an actual approved helper
   manifest's request envelopes. This illustrates their legal current CLI shape;
   helper-generated bootstrap keys/run ID replace example keys. Read context to
   validate the generated task ID, policy, seed text and trust. No existing config.
3. `backup_v1`: new snapshot, final SHA and isolated restore/logical comparison
   before upgrade. Stop all writers; inventory pinned clients and backup callers.
4. `migrate` with actual backup hash, then explicit new ledger binding. `header`
   is now legal and returns v2 incarnation. Before migration only existing export/
   context are valid reads. Approve and run `bind` with real profile/binding hashes.
5. `observe_incident`, `observe_mechanism`, `observe_correction`: three available
   text sources, revision1 each. `capture` creates one bounded actionable draft;
   input does not label the source `verified` or owner-confirmed. Ordinary context
   still shows the seed, not the free draft; explicit inputs shows the draft.
6. `review_a` proposes an attributed mechanism pointer using record+incident+index.
   `review_b` defers using record+incident+correction. Both refer to the same root;
   decision identities/generations are independent. No document has changed yet.
7. Publish prepare/preview A: freeze exact target existence, before/after hashes,
   block/output bytes and plan digest. Obtain owner approval. `approve_a` saves
   intent; write/readback exactly fixture outputs.a; `publish_a` saves result.
   A is approved/current/applied; B is deferred/current/not_applied.
8. Repeat unchanged sources/capture/reviews with new run/key: business noop (A
   reports actual approved state), no new record/generation/document write. Same
   original key/envelope gives replayed. Changed text on the same tuple conflicts.
   Unchanged prepare with no pending work yields runner noop/exit0, no model call.
9. Change only correction text to fixture correction_v2. `observe_correction_v2`
   creates Y/r2 and atomically stales B's deferred generation; S/r1 and A remain
   current. `review_b_v2` names the stale predecessor and new dependency. A does
   not rerun or publish; B becomes proposed generation2.
10. New exact publish preview/owner approval B → `approve_b` → file operation →
    `publish_b`. Both independent outputs now exist, with different wording.
    `backup_v2` preserves origins, decisions, receipts and task identity. This is
    a manual workflow result, not scheduler or #301 acceptance.

`new_task_sequence` names the complete CLI requests in that order; it omits the
non-CLI human/file operations described above rather than pretending core writes
Markdown. The runner phases deterministically generate the same command shapes.

## 3. Complete existing-task path with partial failure

Use the `existing_v1_task_sequence`; initial `inspect_v1` is existing export {},
not unsupported workflow-status. Validate config/task/ledger and actual binary;
never init or replay a seed. If already v2, omit backup_v1/migrate and read header;
otherwise backup/migrate/cut over as in §2, preserving all previous task records.
The sequence includes lost-response replay and a receipted transient failure then
successful retry. `publish_b_conflict` is an alternate branch at that failure
point, not a request to run after B is already applied. That alternate branch
stops at conflict: do not follow publish_b_after_failure or reuse its absent-target
approval. Continue only with the new generation/plan/approval in step4.

1. Confirm bind, observe the three sources and capture one experience as above.
   Simulate response loss after capture commit: keep exact request. `receipt_query`
   finds accepted ID; exact capture replay returns that ID/current state. Never
   use a new key to resolve uncertainty. New-key identical capture later is noop.
2. Review/publish A and defer B as §2. Change correction Y and propose B generation2.
3. Approve B's exact new plan, then introduce a target file whose bytes match
   neither approved absent baseline nor expected after hash. Runner must not
   overwrite it. Send publication-result with B approval ID, outcome conflict,
   actual observed_sha256 and error_code baseline_changed, with attempt0 key.
   A remains applied; B is conflict/pending; structured result is partial/exit4
   when the run also completed A, otherwise failed/exit2 for its B-only request.
4. Owner reviews the conflict. Changed target baseline requires explicit consumer
   reconsideration/new generation and a new exact publish plan/approval, not a
   retry that edits the old plan. Once resolved, B is applied independently.
5. For a transient I/O failure where the baseline/approved plan is unchanged,
   publication-result failed/io_error attempt0 is retained. Subsequent successful
   operation uses attempt1/new key. A timeout on either attempt retries that exact
   attempt bytes/key; it must not increment the attempt without a known receipt.
6. Backup v2. Restore only to a new path; observe new incarnation and all workflows
   recovery_required. Old A/B plan approvals cannot execute. Preserve record/replay
   identities and history; reconcile source/file states and known later withdrawals,
   then obtain a new profile/binding confirmation with recovery_acknowledged=true.
   `recovery_sequence` gives its CLI objects; before bind_after_restore, inspect
   sources/files read-only and reconcile known later evidence. Use recovery_binding
   and the actual new incarnation; the fixture's renewed confirmation is illustrative.

An existing config that cannot be validated is a stopping condition, not license
to initialize another task. The profile's two consumers do not own upstream
target fields or its existing review scheduler.

## 4. A3–A10 have one allowed result

| ID / fixture mutation | Required result |
| --- | --- |
| A3: §2 steps6–10 | Same entry, A approved and B deferred; later B adopts different prose. Separate generation/receipt/block; no global processed bit. |
| A4: repeat exact key; repeat same business content new key; change body; race same expected version | replayed original ID; noop existing ID/current validity; conflict; one winning transition and other replay/noop or conflict according to exact equality. No duplicate content/block. |
| A5a: S unchanged, only Y/r1→r2 | B deferred/approved becomes stale; A unaffected. B must name predecessor; no source-global noop. |
| A5b: confirmed correct of input record | Original superseded, owner-confirmed new ID; dependent decisions/lessons stale atomically, trust not retroactively changed; new decisions use current ID/root. |
| A5c: extractor/selector changed under confirmed profile, bytes identical | New extractor_version and next source observation revision; old captures stay stale; fresh capture tuple extends same root, no old approval reuse. JSONL at EOF revises all known occurrences after prefix verification, keeping IDs/cursor and exposing paginated recapture. |
| A6a: first text scan missing, later appears | Collection unavailable with no fake source/digest; first available source revision1; no deletion/retraction. |
| A6b: known source unreadable/deleted | Unavailable revision and dependent stale; retain content/identity for audit. Only explicit confirmed retract scrubs it. |
| A6c: renamed stream same prefix, profile-approved path change | Same collection/source identities; changed profile/definition freshness requires rescan; no new independent event. Unproven move/truncate/rewrite conflicts. |
| A6d: earlier timestamp appended; identical second JSONL row | Both new byte offsets are captured, irrespective of time/body equality. Same offset/prefix retry noop. Partial/binary row holds cursor, visible blocked work. |
| A7: source region includes managed output or copied derivative | Exclude/block region; never independent corroboration. Outside-root output alone is no proof of absence of a loop. |
| A8a: lost CLI stdout | Exact retained request replay/receipt query; same IDs, no new key/content. |
| A8b: after file replace, before result; profile meanwhile changes | After hash recognized; retained same-incarnation intent records applied **stale** effect under original profile, no fresh write authorization. |
| A8c: before/after mismatch or plan missing | Conflict or uncertain/needs_review respectively; no blind overwrite/reconstruction. |
| A9: one target fails; writing its result/summary also fails | Other target still attempted. Applied receipt preserved; report_errors populated, stdout structured, exit4/2 (never0); source not globally completed. |
| A10a: v2 isolated restore after approvals | New incarnation; old workflow approvals unusable, history/records/receipts preserved; fresh source/file reconciliation and confirmation required. |
| A10b: snapshot predates a later retract | Cannot infer the lost withdrawal; old body can exist in restored history. Explicitly report limitation, reconcile retained later evidence; never assert no resurrected truth from the new ID alone. |

Additional withdrawal check: fixture `retract` clears the full source-derived
refresh/correct chain, every dependent lesson revision-chain body and affected
consumer reason; decisions retracted, source blocked. Exact old write replay gives
identity/state only. New capture **and direct source dependency** are rejected,
even with a new key. External output can remain until authorized neutralization;
status exposes that lag, and old backups/source originals are not claimed erased.

## 5. R3 (#310) command-level acceptance handoff

Owner accepted fixed R2 commit `83d85c6` on 2026-10-23. These references are now
copied into the GitHub issue; this is an acceptance map, not a second implementation tracker.

- **Format/compatibility:** exact supported v1→new-v2 migrate with verified backup;
  all old IDs/digests/order remain. Wrong backup, future schema, overwrite/sidecars
  and old task binary on v2 reject before mutation. Existing task v1 commands and
  experiment CLI retain their behavior. Config/envelope version remains1.
- **Binding/source/capture:** workflow-bind strict schema/confirmation/CAS; exact
  repeat/new-key noop order; source-sync file/JSONL identity, first-unavailable,
  append/prefix conflict, rule refresh and atomic invalidation; source-capture
  bounded draft/trust/origin/one-body/chain/dedupe. Context separates drafts.
- **Consumption:** consumer-review full dependencies, proposed/approved equality,
  deferred/skipped wake-up, explicit predecessor/reconsider and selective A5.
  consumer-approve pins exact diff digests/profile/target/incarnation; no trust promotion.
- **Effect/reporting:** publication-result current/stale/withdrawn intent results,
  old-profile effect exception, original-key replay vs new known attempt, rejected
  cross-task/unknown IDs. workflow-status projections/pagination/cursor conflict.
- **Lifecycle/recovery:** existing checkpoint/correct/retract reach new dependent
  decisions atomically; deterministic derivative scrub/blocked source recapture;
  v2 export includes all state; backup/restore new incarnation/gate preserves
  original replay identities and never reactivates old publication authority.

Use future compiled CLI seams and temporary synthetic ledgers only. #309 itself
adds/runs no product tests; examples are statically walked against the contract.

## 6. R4 (#311) command-level acceptance handoff

- inspect zero ledger/CLI/upstream execution; exact profile/binding/selection/
  prepared/submission/plan/artifact schemas and version/hash/permission checks.
- prepare/preview/apply for capture → review → publish using §2 and §3, including
  new-vs-existing task routing, complete source coverage and remaining/pending sets.
- New exact digest after source refresh needs new human consent; `--approve`
  proves byte match only. No plan/request hash cycle or reconstructed envelope.
- At most one publication item per target/plan; later entries remain queued and
  get a fresh baseline plan, never dropped or marked consumed.
- File before/after/crash/plan-loss scenarios in A8, strict managed region/symlink/
  ownership constraints, separate source/currentness and file-effect completion.
- Actual attempt numbering, locked immutable staged request/receipt/report files;
  failures in either document/report do not suppress independent branches.
- Final hashes after last possible mutation, fixed exit/report schema, pinned
  installation path/version, explicit client provenance; no false scheduler proof.

After both implementation tickets pass, R5/R6 perform actual client and scheduler
acceptance. This contract does not deploy the private pilot, update backup cron,
package maintenance scripts, enable MCP or close the older gates.
