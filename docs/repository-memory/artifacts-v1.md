# Repository memory v1 — operation artifact schemas

Normative R2 contract, owner-accepted at `83d85c6`; not runtime. Completes [runner](runner-v1.md) JSON grammar.
All objects reject unknown/duplicate keys/null. Optional means omitted. Shared
types/limits are in [kernel](kernel-v1.md). This file is schema documentation,
not a second tracker or an implementation.

## Fixed filenames and envelope

For each fresh --run-dir, commands create only their phase's files below. stdout
uses runner §3; callers derive artifact paths from --run-dir and these fixed names.
`artifacts.json` contains `{schema_version:1,files:[{path,bytes,sha256,mode}]}` after
final writes/chmod; path is relative, mode is literal `0600`. The inventory excludes
itself. No missing required artifact may be reported as ok.

| Command | Required artifacts |
| --- | --- |
| inspect | inventory.json, summary.json, artifacts.json |
| prepare | prepared.json, evidence/ files actually read, summary.json, artifacts.json |
| preview | plan.json, preview.md, rendered/ files for publish, summary.json, artifacts.json |
| apply | append-only requests/, receipts/, outputs/ when retaining review text, result-<item-id>-<attempt>.json, summary-<attempt>.json, inventory-<attempt>.json |
| status | stdout only; no implicit repair or output directory |

Apply writes under the parent directory of --plan, within binding.state_root.
It never rewrites plan/preview/rendered bytes or a prior request/receipt/result.
Repeated known result uses its existing file; changed attempt writes
result-<item-id>-<attempt>.json. Report filename attempt is the next unused positive
integer chosen under the task lock; it is operational identity, not a new business
key. Initial filenames use attempt0. Summary content follows runner §3. Files are
owner-only and source/document bodies stay out of diagnostic summary.

ArtifactRef is exactly `{path:relative-path,sha256:H,bytes:U}`; resolved under the
artifact's run directory, no traversal/symlink. Referenced bytes must match before
use. A retained prior run uses `{run_dir:ABS,artifact:ArtifactRef}` and must resolve
inside binding.state_root. Arbitrary absolute attachment paths are not executable
or trusted operation material.

## Inspect inventory

`inventory.json` is `{schema_version:1,root:ABS,observed_at,files:[...]}`. Each file:
`{path:relative-path,status:"read"|"missing"|"unreadable"|"excluded",sha256?,bytes?,
reason?}`. sha256/bytes required only for read; reason is `not_found`, `io_error`,
`unsafe_path`, `oversize`, `binary` or `invalid_text`. No filesystem mutation or
mechanism authority inferred from filenames. Agent's R1 analysis remains separate.

## Prepared snapshot and typed work

`prepared.json` exactly:

```text
{schema_version:1, prepared_id:UUID, created_at, workflow, client, snapshot,
 items:[Work], remaining:[{id:name,reason:Reason}], evidence:[Evidence]}
```

workflow is capture/review/publish/recovery. client comes from prepare --client.
Reason is awaiting_extraction/awaiting_review/awaiting_approval/target_queued/
partial_tail/source_unavailable/blocked_unit/plan_missing/restore_gate.
Max20 items; remaining max20 plus optional top-level `remaining_count:U` when more
exist. Count never declares them processed; next prepare queries authoritative
pending sets again. Entries in remaining are diagnostics, not completion cursors.

Snapshot exactly: `{task_id,workflow_id,incarnation,profile_revision,
profile_sha256,binding_sha256,ledger_revision,scans:[Scan],dependencies:[Dependency],
targets:[Target]}`. Empty arrays are valid.

- Scan is exactly the source-sync action fields, without common write envelope:
  collection_id/expected_scan/observed_at/availability/units/cursor?/reason?. It
  describes this observed attempt, not a committed source revision. At most30.
- Dependency has the kernel's typed record/source form; includes only existing
  committed evidence. New source versions are named by capture work below, never
  pretended to be current ledger dependencies.
- Target is `{target_id,exists:boolean,sha256:H,prior_block_sha256?:H}`. Absent
  target hash is SHA256(empty) and exists=false. At most2 distinct targets.
- Evidence is `{source_id,source_revision:U,sha256:H,artifact:ArtifactRef}`; a new
  source's source_revision=0 here only. File cache bytes follow selected-source
  limits; source_sha is original selected bytes, artifact SHA is the cache bytes
  (which may contain only selected JSONL fields). The distinction is explicit.

Work is one of these exact variants:

| kind | Required fields beyond id:name and kind |
| --- | --- |
| capture | collection_id, source_id, source_revision (positive proposed next/current revision), extractor_version, source_sha256, evidence_index:U |
| review | consumer_id, input_record_id, root_record_id, input_text (≤1,000), dependencies, previous_decision_id? (stale only), previous_validity? (literal stale) |
| publish | consumer_id, decision_id, decision_sha256, target_id, block_id, output (prior-run ArtifactRef wrapper), output_sha256, purpose(publish/neutralize), previous_approval_id? |
| recovery | consumer_id, approval_id, original_plan (prior-run ArtifactRef wrapper), item_id:name, classification(before/after/conflict/uncertain/restored), pending_request? (prior-run ArtifactRef wrapper) |

For substantive publish, optional `previous_approval_id` explicitly identifies the
unapplied intent being renewed under a fresh owner plan (#349). It does not refer
to an older generation's applied block owner. The selected publication snapshot
pins its expanded dependencies and source proofs, not unselected collections.
Publication proof scans allow up to 40 units per collection (two decisions with
at most 20 expanded dependencies each). Capture/review scans retain their 20-unit
limit; no ingestion or cursor advancement occurs in publication preparation.

Review includes all evidence that the Agent declares it will use; Agent may add
only source dependencies present in prepared.evidence/current snapshot. If further
evidence is needed, rerun prepare with an explicitly approved profile change where
scope expands. It may remove unused auxiliary dependencies, never required input
origin. A still-current settled decision is not a review work item. Explicit
reconsideration is CLI-only in v1; runner never infers it from different wording.

Publish/recovery selects **at most one item per target per plan**, oldest pending
decision generation's workflow_sequence first, ID tie-break. All remaining items
are target_queued. Finish/reconcile that item, then prepare a fresh plan against
the new file baseline. This keeps A/B independent and avoids two proposed writes
sharing an incompatible whole-file baseline. No target batching is implied.

## Submission and immutable plan

`submission.prepared_sha256` is the SHA256 of exact prepared.json bytes (not
artifacts.json). Capture/review submission fields are runner §2, with review also
accepting optional previous_decision_id; it must equal the prepared stale
predecessor. `reconsider` is rejected by runner. Publish/recovery use empty items.
Capture submission.unit_id equals its prepared Work.id exactly.
Owner-mode approval maps to optional confirmation on source-capture/consumer-review
only after actual exact plan approval; profile mode omits it. This authorizes
retention, not owner_confirmed source trust.

`plan.json` exactly:

```text
{schema_version:1,plan_id:UUID,created_at,workflow,client,prepared_sha256:H,
 snapshot,items:[PlanItem],authorization:{mode:owner|profile,profile_revision?:U}}
```

PlanItem carries id/kind and:

| kind | Required fields |
| --- | --- |
| capture | source_id, source_revision, extractor_version, text, occurred_at |
| review | consumer_id, input_record_id, decision, reason, dependencies, previous_decision_id?, output_text? |
| publish | consumer_id, decision_id, decision_sha256, target_id, block_id, base_exists, base_sha256, after_sha256, output_text, output_sha256, rendered:ArtifactRef, purpose, previous_approval_id? |
| recovery | same fields as prepared recovery variant, with its original immutable plan reference |

The source-sync requests are generated from snapshot.scans. Capture versions are
predicted from observed bytes/current version using the fixed source rules; apply
must compare actual source-sync receipt identities/revisions with the plan before
submitting capture. Mismatch stops pending work, never rewrites plan fields.

**Acyclic hashes:** plan contains no generated CLI request bodies, no its-own hash,
and no new approval ID or publication receipt that this plan has yet to create.
Recovery may reference an already existing approval_id, and neutralization may
reference previous_approval_id; these are inputs, not self-references. Hash canonical plan first. Only then
derive CLI envelopes/keys and inject that hash into consumer-approve.proposal.
Rendered file digest depends on block/body/baseline, never plan digest or approval
ID. Request files are staged afterward and immutable; response IDs come from
validated receipts, not template guesses. Owner must approve this same plan hash.

For review output_text, normalize terminal LF once in preview and freeze those
bytes; compute output_sha for the CLI. Accepted review retains
`outputs/<decision_id>.txt`, whose bytes equal frozen output_text. If creating that
file fails, ledger proposed decision may exist while publish is pending; recover
from the exact immutable plan and its CLI receipt, not model paraphrase. Missing
both plan and output is plan_missing and requires a new proposal/generation.

## Requests and receipts

Each `requests/<action>-<index>-<attempt>.json` contains only the full exact CLI
stdin object defined by kernel (no outer wrapper). Its sibling receipt is
`receipts/<action>-<index>-<attempt>.json` containing exactly
`{request_sha256:H,action,task_id,incarnation,response}`. response is the validated
CLI envelope; no textual reconstruction of a missing response is permitted.
Request hash/action/key must agree with authoritative receipt query or exact replay.
At most one unresolved request exists for an action/item; on uncertainty it is
retried byte-for-byte. Only a known completed failed/conflict publication attempt
permits the next attempt number and newly observed result, as runner §2 defines.

No local result file is authority for source revision, approval or applied status.
Losing operation material can prevent resuming a file effect; losing ledger state
requires supported restore/reconciliation. Either loss is visible, not replaced
by hidden Agent memory or a different key.
