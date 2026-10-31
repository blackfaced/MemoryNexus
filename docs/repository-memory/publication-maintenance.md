# Publication removal and selection

Issue #338 defines explicit removal; #339 adds selection independently. Neither
authorizes installation or production execution. Existing CLI and SQLite schema
v2 publication/attempt JSON carry the additional operation; history is retained.

## Removal contract (#338)

`workflow.py prepare --workflow publish --removals REQUEST.json ...` accepts
`{"schema_version":1,"items":[{"approval_id":"UUID","reason":"deduplicate"}]}`.
One or two distinct previously applied substantive approvals, at most one per
target. This explicit mode never falls back to normal publish/neutralize work.
An intact block must match that approval's output digest. A later applied
generation owns the block instead and rejects removal through an older approval.

The prepared/plan item has `purpose: remove`, `previous_approval_id`, `reason`,
`prior_block_sha256`, and empty output (SHA-256 of empty bytes). Preview retains
the full rendered file and unified diff. Removal deletes exactly the matched
marker region including its trailing LF, preserving bytes outside it. The target
file remains even when empty. Missing, duplicate or edited markers/body conflict.

The public `consumer-approve` request retains the existing envelope/proposal and
adds `maintenance: remove`, `previous_approval_id`, `reason` (1–240 characters),
`prior_block_sha256` and `confirmation: confirmed`. The previous approval must be
same-task/workflow/decision/target/block, substantive and applied. The current
incarnation/profile gate still applies; source validity is not deletion authority.
The runner always requires `--approve SHA256_CANONICAL_PLAN`; automatic review or
neutralization policy never authorizes removal. `publication-result` records the
ordinary applied/conflict/failed outcome. Both receipts and removal metadata are
included in export/backup/restore. No record or historical publication is deleted.

An applied removal suppresses automatic publication/neutralization of that block
through the removed generation; a later explicit review generation can publish.
File baseline and block identity are rechecked immediately before replacement.
The retained plan handles response loss and recovery exactly as publication does;
a restored ledger requires reconciliation and renewed authorization.

## Renewing a conflicted removal (#344)

A removal approval with a latest `publication-result` outcome of `failed` or
`conflict` does not reserve the block forever. Prepare a fresh `--removals` run
against the current intact block and file baseline, preview its complete diff,
and obtain the new canonical plan digest from the owner. Apply with that digest
creates a distinct approval and attempt when the proposal differs. Preserve all
earlier approvals, attempts and receipts; do not rewrite them as successful.

An approval without an attempt is pending and blocks a different proposal. An
applied removal also blocks a different proposal. Check every matching removal
approval in the same decision/incarnation/profile revision before reusing an old
failed proposal, so it cannot bypass a newer pending or successful removal.
The exact same proposal and removal metadata may reuse the same approval. An
exact original request/key replays its existing receipt; changed bytes on the
same key conflict. New plans use new keys and require fresh owner approval.
Ordinary publication and neutralization approval rules are unchanged. Existing
block ownership, intact-body, target-baseline and recovery gates still apply.

Command acceptance: publish a fixture block, preview removal, append human text
outside its block, then apply to record `conflict` without overwriting the file.
Reprepare/preview; no digest or the old digest must refuse without writes. The
new digest must remove only the block and retain the human text, using a new
approval/attempt while every historical failure and receipt remains. Repeat with
multiple conflicts and a recorded failed outcome; pending and applied approvals
must reject a competing proposal. Original-key replay and changed-key-body
conflict remain distinct. Run the existing restored-ledger/newer-owner and
publication-selection regressions as well. All fixtures are isolated.

Reproduce against a fixed compiled binary (do not rebuild it during the tests):

```sh
cargo build --locked --bin memorynexus-feedback-cli
MEMORYNEXUS_TEST_BINARY="$PWD/target/debug/memorynexus-feedback-cli" \
  PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s integrations/task-memory/tests -p 'test_workflow_removal_reapproval.py' -v
```

## Publication selection (#339)

### Selected evidence and unapplied intent renewal (#349)

Substantive publication scans only its expanded dependencies. Text uses the
registered selector; JSONL preserves the registered stream prefix and selected
registered rows, not the next 20-row capture window. A changed unrelated collection
does not invalidate that selected publication. Source/decision/target preflight
precedes any new substantive approval and is repeated before replacement. Removal
keeps its established approval/conflict-attempt history semantics. Preflight cannot eliminate
races with external editors; approved-but-unapplied remains an explicit state.

Fresh prepare/preview on that state carries `previous_approval_id` in the publish
item. Following a fresh owner digest, the runner sends ordinary `consumer-approve`
with its existing envelope, exact decision/proposal/confirmation, plus:

```json
{"previous_approval_id":"70000000-0000-4000-8000-000000000001"}
```

This is an explicit replacement of the pending substantive intent, not a removal
or a new review generation. Its output must be unchanged and its predecessor
current in the same scope, without an applied attempt. The predecessor becomes
stale with `superseded_by`; the new intent retains `previous_approval_id`. Failed
attempts, old requests and receipts remain. Two competing successors cannot both
renew the same intent. Old plan/key replay cannot restore retired authority.
Applied intent, changed output, restored incarnation, stale profile or missing
recovery bytes require their existing reconciliation/review paths. Installation
does not migrate old authorization or activate production automatically.

Command-level regression (isolated real CLI/runner, no production paths):

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest discover \
  -s integrations/task-memory/tests -p 'test_workflow_publication_preflight.py' -v
```

The tests check unrelated/related drift, no-intent after-byte confusion, per-item
auxiliary evidence, >20 registered JSONL row proofs and append/prefix identity,
fresh digest renewal, competing/applied/invalid predecessors, replay/conflict,
human text preservation and retired plans producing no new effect.

`prepare --workflow publish --publish-selection SELECT.json` accepts strict JSON
`{"schema_version":1,"items":[{"decision_id":"UUID","purpose":"publish"}]}`.
One or two unique decisions, purpose `publish` or `neutralize`, at most one per
target. Empty selections, unknown decisions, unsupported purposes, conflicting
targets and ineligible work fail explicitly. This flag is mutually exclusive
with `--removals` and is rejected for other workflows. Omission retains FIFO.

Selection filters candidates before target allocation and truncation; it cannot
add fabricated prepared items or change their order. Unselected eligible work
remains `unselected` in remaining (with total count when truncated), without any
ledger decision change. The immutable prepared snapshot and canonical plan bind
exactly the selected effects. Preview still requires `items: []`; it is not a
second selection mechanism. Publish/remove always require owner digest. Selected
pure neutralization alone retains its existing explicit profile policy.

Command acceptance: create two targets headed by neutralization with later
publishable decisions; prepare with their decision IDs must select those later
publishes only, preview/apply with owner digest updates them while preserving the
old blocks and publication history. Unknown/duplicate/same-target/empty requests
fail without effects. Missing output and stale snapshot cannot be bypassed.
Default preparation remains unchanged. Following an approved #338 removal,
fresh default preparation can publish successors without recreating removed
blocks. No production deployment is part of either issue.

## Command-level acceptance

Use synthetic files, two consumers and the compiled CLI in separate processes.
Capture/review/publish a block, then prepare removal with its real approval ID.
Preview with the raw prepared-file digest and `items: []`. Applying without the
canonical plan digest must fail with no file effect; applying with it removes
only the selected block. Export must retain the old approval and record the
reason/link and applied result. Repeat apply must not add attempts. A subsequent
default publish prepare must not resurrect the removed generation.

Repeat with external manual text, last-block empty file, modified body, duplicate
markers, missing block, stale baseline, wrong approval scope, newer published
generation, changed request on the same key, and restored incarnation. Reject
unsafe effects; preserve history. Simulate response loss after replacement and
recover using the exact retained request, never an unrecorded manual deletion.
