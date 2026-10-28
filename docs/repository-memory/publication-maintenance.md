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
