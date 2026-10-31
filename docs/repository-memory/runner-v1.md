# Repository memory v1 — profile, runner and file protocol

Normative R2 contract, owner-accepted at `83d85c6` on 2026-10-23; implementation tracked in #311. See
[kernel](kernel-v1.md) for shared types and CLI authority. No scheduler or model
SDK runs inside the script. Agent supplies bounded extraction/decision text.
Exact producer/consumer files and typed schemas are in [artifacts](artifacts-v1.md);
its field lists govern operation material, avoiding implicit implementation choices.

## 1. Profile and private machine binding

Core profile has exactly the following fields; JSON example is synthetic:

```json
{
  "schema_version": 1,
  "workflow_id": "40000000-0000-4000-8000-000000000001",
  "revision": 1,
  "purpose": "Retain sourced checker experience and two independent views.",
  "mechanisms": [
    {"name":"upstream_review","purpose":"Owns formal knowledge","trigger":"external morning review","read_paths":["inbox/incidents.jsonl"],"write_paths":["knowledge/checker.md"],"evidence":"AGENTS.md#review"}
  ],
  "collections": [
    {"collection_id":"50000000-0000-4000-8000-000000000001","format":"text","path":"notes/incident.md","selector":{"kind":"full"},"extractor_version":"bounded-v1","authority":"Document-reported incident; not independently verified."},
    {"collection_id":"50000000-0000-4000-8000-000000000002","format":"text","path":"knowledge/checker.md","selector":{"kind":"full"},"extractor_version":"bounded-v1","authority":"Mechanism navigation evidence."},
    {"collection_id":"50000000-0000-4000-8000-000000000003","format":"text","path":"notes/correction.md","selector":{"kind":"full"},"extractor_version":"bounded-v1","authority":"Follow-up verification status."}
  ],
  "capture": {"automatic_drafts":true,"automatic_reviews":true},
  "consumers": [
    {"consumer_id":"mechanism","lens_version":"pointer-v1","rules_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","target_id":"mechanism_doc","neutralize_stale":true},
    {"consumer_id":"lessons","lens_version":"lesson-v1","rules_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","target_id":"lessons_doc","neutralize_stale":true}
  ],
  "operation": {"workflow_name":"repository_review","trigger":"not_configured"}
}
```

Fields are required. purpose/authority/evidence/trigger and mechanism purpose
are 1–240 characters, maximum8 mechanisms, read/write paths each maximum20.
Mechanism metadata describes upstream practice, never grants tools or writes.
Collections maximum30 and consumers exactly2, unique IDs and unique targets.
UUIDs are stable across path moves; revision positive U. Lens rule digest must
match a bounded UTF-8 rule file in the pinned package. Profile paths are relative
to explicit root; no globs, traversal, URI, shell, env expansion or symlinks.
`selector` is `{kind:"full"}` or `{kind:"marked",begin:string,end:string}` with
unique exact literal lines (1–120 chars each); selected bytes are strictly between
the two markers. Missing/duplicate/reversed markers are unreadable, not empty.

For append_jsonl selector is exactly `{kind:"fields",keys:[name...]}` (1–8 unique
top-level JSON string field names); entire rows are hashed for identity, only those
text fields are shown for bounded extraction. Missing all selected fields, non-string
selected values, invalid JSON/UTF-8, embedded NUL/control bytes other than TAB/LF/CR,
or data-URI/media payload are explicit blocked units. No automatic OCR/decode.
The adapter does not advance cursor past a blocked unit. Later approved selector
correction or an upstream fix/reconciliation is required; no silent data dropping.
Record occurrence time is supplied by extraction, with attribution when unknown;
source write timestamp is never an ingestion high-water mark.

Each source file is ≤2 MiB for text and ≤64 MiB for a JSONL stream, each complete
row ≤32 KiB; selected extracted field text ≤8 KiB per row before summarization.
Oversize is needs_review, not partial truncation. At most20 new rows per prepared
batch; report remaining complete rows and any partial tail. Verify consumed prefix
on every scan; one observed append writer is not a guarantee against other writers.

Private binding is an external 0600 file in a 0700 directory:

```json
{
  "schema_version":1,
  "workflow_id":"40000000-0000-4000-8000-000000000001",
  "task_id":"10000000-0000-4000-8000-000000000001",
  "task_root":"/srv/synthetic/checker",
  "config":"/srv/synthetic/checker/.memorynexus.json",
  "ledger":"/srv/synthetic/private/task-v2.sqlite",
  "binary":"/srv/synthetic/install/v1/memorynexus-feedback-cli",
  "binary_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
  "python":"/usr/bin/python3",
  "package":"/srv/synthetic/install/v1",
  "package_sha256":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
  "profile":"/srv/synthetic/private/profile.json",
  "state_root":"/srv/synthetic/private/runs",
  "targets":[
    {"target_id":"mechanism_doc","path":"/srv/synthetic/private/mechanism.md"},
    {"target_id":"lessons_doc","path":"/srv/synthetic/private/lessons.md"}
  ],
  "timeout_seconds":60
}
```

All paths absolute, real, no symlink component; timeout integer1–300. Config must
be task_root/.memorynexus.json and declare task_id; ledger outside root; no parent
inheritance. Package/state_root outside root. Target parents must already exist,
targets may not exist until approved publication. Targets distinct, not source
full-file selections, config, ledger, package, state or their descendants. A target
may be read only through a disjoint explicit marked human region; managed blocks
and their copies are never treated as independent evidence. Whole file/region
includes a MemoryNexus marker => needs_review, rather than silently ingest output.
No target chosen from source `target` fields or model-generated path strings.

Hash in these examples is illustrative, not an installed executable assertion.
Binding digest includes exact paths and binary/package hashes. Profile/binding
changes require a new confirmed workflow-bind. Never edit task config to carry
new fields. New task uses existing bootstrap first; existing task never init/seeds.

## 2. Runner argv and operation material

Proposed entry: `integrations/task-memory/scripts/workflow.py`. Exact grammar:

```text
PYTHON workflow.py inspect --root ABS --selection ABS --run-dir ABS
PYTHON workflow.py prepare --binding ABS --workflow capture|review|publish|recovery --client LABEL --run-dir ABS
PYTHON workflow.py preview --binding ABS --prepared ABS --submission ABS --run-dir ABS
PYTHON workflow.py apply --binding ABS --plan ABS [--approve H]
PYTHON workflow.py status --binding ABS [--plan ABS]
```

No additional argv, command strings, stdin execution or plugins. Each command
prints one JSON object, diagnostics content-free on stderr. inspect selection is
`{schema_version:1,paths:[relative-path...]}` (1–30), reads at most2 MiB per file,
does not open a ledger/call any CLI/run upstream scripts. Its output inventory
lists path/hash/read_status and unknowns; Agent authors R1 conclusions separately.
Run directories are new external owner-only directories; never overwrite prior
approved material. Except inspect (which has no binding), run directories and
all referenced prior run material must be inside binding.state_root. Apply's
artifact directory is --plan's parent. umask077 and no-follow creation apply throughout.

prepare pins installed hashes, task binding and CLI capabilities, then reads
workflow-status and selected sources. It is read-only to ledger/files, writing
only private prepared.json and evidence cache. Its exact typed work items,
source-version predictions/cursor proofs and recovery classifications are defined
in artifacts §Prepared. prepare never calls source-sync merely to inspect freshness.

submission is exactly `{schema_version:1,prepared_sha256:H,items:[...]}`. Max20
items. Capture item: `{unit_id:name,text,occurred_at}`. Review item:
`{input_record_id,consumer_id,decision,reason,dependencies,output_text?,previous_decision_id?}` using
kernel fields/limits; output_text required only for proposed, ≤1,000 chars.
Publish/recovery submission has `items:[]`; use retained exact output and ledger
decisions, never ask a model to reconstruct approved prose during recovery.
prepared Work.id is a deterministic local name `u` plus decimal list index;
every submitted item must map once to a prepared item. Missing work is pending,
not processed. Unknown, repeated or scope-expanding items are invalid_input.

preview rechecks sources/profile/binding/target against preparation before writing
an immutable `plan.json`, complete human `preview.md` and any rendered target
bytes, with the complete field schema in artifacts §Submission. Plan contains no
generated CLI requests; derive requests only after its digest is fixed, so the
consumer approval's plan_sha256 cannot form a hash cycle. Authorization is `{mode:"owner"}` or
`{mode:"profile",profile_revision:U}`. Capture uses profile mode only if automatic_drafts;
review only if automatic_reviews; publish always owner except fixed neutralization.
A metadata-only capture plan with no draft items may use profile mode for source
invalidation. It grants no permission to insert replacement prose.
The plan also requires `client` (1–64 chars) copied from prepare --client. It is
provenance, not authenticated client execution; real-client acceptance is separate.
Plan digest is SHA256 of canonical plan JSON; artifacts containing it are not
recursively part of its digest. Include all decisions, proposed text, attribution,
diffs, limits, bindings and capture meaning in human preview. Gaps never enter
record text automatically. A changed source/binding/target produces stale_snapshot,
not a regenerated implicitly approved plan.

apply owner mode requires --approve equal the immutable plan digest plus actual
human confirmation recorded by the invoking Agent. Profile mode verifies active
exact stored policy; no --approve is fabricated. CLI arguments cannot prove human
consent. New plan/digest after drift always needs a new human confirmation when
owner mode applies, even if every body text is unchanged. Bootstrap keeps its own
stricter full selected-source/Git snapshot check; this incremental plan checks the
bounded selected evidence/config/target snapshot and does not depend on unrelated
Git HEAD or unselected files. No widening of old bootstrap approval is allowed.

### Phase ordering and retries

- capture: revalidate each selected collection → source-sync (metadata/invalidation)
  → source-capture bounded entries. Stage exact JSON before every CLI call. Generated
  IDs in later requests come only from validated receipts, never prediction.
- review: read current inputs/full evidence → commit consumer-review. It stores a
  proposed/deferred/skipped decision only; does not publish. Save output_text
  privately, keyed by returned decision ID/digest, for later publication preview.
- publish: prepare from a current proposed decision + exact retained output →
  preview complete file diff → human approves → selected evidence/target/decision
  preflight → consumer-approve intent → final preflight → file
  operation → publication-result. No free-text file write in capture/review phases.
- recovery: classify a retained plan against ledger intent and current file hashes;
  exact replay/receipt completion only where §4 allows. New content requires a new
  review/publish plan. Missing material returns needs_review.

Publication snapshots contain only the selected substantive decisions' expanded
dependencies and affected targets. Direct auxiliary sources and inherited origin
sources are included. Text proofs use their registered selector and digest;
JSONL proofs include the registered stream prefix and the selected registered
rows, including rows outside the next capture window. Append-only extension does
not invalidate them. Unrelated collections are neither read nor pinned. Removal
and fixed neutralization retain their owner/block guards without pinning unrelated
source files. Preparation, preview and apply check these same proofs.

A fresh publication prepare for a current approved-but-unapplied decision includes
its current substantive `previous_approval_id`. The immutable plan and human
preview expose this explicit renewal; apply requires its own owner digest and
passes that ID to consumer-approve. It does not create a new review generation or
reuse the old plan approval. Initial preflight failure writes no new intent or
receipt. A race after intent creation may still leave not_applied; retain it for
exact recovery or fresh explicit renewal. Already-applied effects are reconciled
without rewriting files, even if a later source change has invalidated authority.
New code never edits retained legacy plan bytes or invents missing recovery data.

Derive write keys `wf:<plan_id>:<action>:<item-index>:<attempt>` within128 chars,
keeping run_id=plan_id and original client label. Attempt starts0; only a known
receipted failed/conflict publication-result permits next attempt1,2,... for a
later actual retry outcome. Unknown response never advances attempt. Other actions
keep attempt0. Save `requests/<action>-<item-index>-<attempt>.json` before
calling CLI; response validated for task/incarnation/status/object IDs before a
0600 receipt is saved. Transport timeout/malformed stdout retains pending request;
retry original bytes/key. A profile/source refresh is not a retry. Capture partial
completion preserves accepted metadata/records; remaining extraction uses stored
source identity, not a new init or duplicate ingestion. prepare examines pending,
stale and unapplied work even if no source bytes changed.

On an unavailable source the owner-approved workflow permits metadata invalidation;
no replacement text is authorized by that observation. If draft retention itself
needs review, prepare/preview stops for approval before running its capture plan.
Independent consumers have independent plan items, intents and report results.
One failing branch/report cannot skip the other. One task lock under state_root
uses nonblocking OS lock, fixed workflow/task identity, not an expiring timestamp;
process death releases it. SQLite transactions/CAS remain final authority, not the
lock file. A stale lock file is not proof of a live owner.

## 3. Reports, exits and installation

Runner stdout/report schema:

```json
{"schema_version":1,"status":"partial","run_id":"60000000-0000-4000-8000-000000000001","workflow":"publish","trigger":{"kind":"manual"},"items":[{"id":"mechanism","status":"ok","receipt_ids":["70000000-0000-4000-8000-000000000001"]},{"id":"lessons","status":"failed","code":"baseline_changed","receipt_ids":[]}],"pending":["lessons"],"report_errors":[]}
```

Exactly these top-level fields; items contain id/status, optional code,
receipt_ids array (empty when none). trigger is `{kind:"manual"}` or
`{kind:"scheduler",provider:name,run_id:string,evidence_path:ABS}`; scheduler variant
requires actual supported scheduler-run evidence file, ≤64 KiB, listed hash in
run inventory. A supplied label alone never upgrades manual to scheduler.
The runner invocation accepts trigger metadata only via the approved binding's
separate run environment `MEMORYNEXUS_TRIGGER_FILE` (absolute owner-only JSON),
default manual; it cannot carry a command. The adapter validates and copies it
into operation material; absent/unverifiable evidence is manual with a gap.

| status | Exit | Meaning |
| --- | --- | --- |
| ok / noop | 0 | all requested items completed / no source change and no pending work |
| needs_review | 3 | no authorized next effect; explicit unresolved proposal or missing material |
| partial | 4 | some independent items succeeded, some failed/pending |
| failed | 2 | no successful requested effect or reporting integrity failed |
| busy | 5 | cooperative lock unavailable; no execution begun |

Item statuses are ok/noop/needs_review/failed/pending; report_errors is a list of
`{artifact:name,code:"io_error"}`. Any report write error makes aggregate partial
if any item effect succeeded, otherwise failed, never exit0. Still emit stdout and
try other branches; stdout cannot be guaranteed on a broken pipe. Per-item result
and top-level summary are durable operation evidence, not ledger authority. Compute
file size/hash only after the last process that may modify that file exits; inventory
itself does not contain a recursive self-digest. Preserve immutable approval
artifacts; corrections get a separate addendum.

Package installation uses a stable new version directory with an inventory of
relative file names and SHA256 values, sorted; package_sha256 hashes that canonical
inventory. Include workflow.py, renderer, Skills/protocol and Lens rule files.
No runtime replacement of an active version. Fixed Python path, executable/hash,
contract version, `.memorynexus.json` task ID and SQLite format are checked before
apply. Help/capability must work without opening storage; only call the pinned
task-capable binary, never guess a similarly named legacy executable.

Subprocess environment allowlist: HOME, PATH pointing only to pinned package/bin
plus /usr/bin:/bin, LANG=C.UTF-8, MEMORYNEXUS_TASK_LEDGER, and TMPDIR when explicitly
bound by the OS. No shell=True, model code or profile command evaluation. Package
rollback requires a tool supporting current DB version; no implicit DB downgrade.
Production installation, updating every pinned caller/backup entry and real cron
changes remain #312 deployment work, not R2 or maintenance-script packaging.

## 4. Managed Markdown and crash cases

UTF-8 LF renderer `markdown-block-v1` writes exactly:

```text
<!-- memorynexus:<block_id>:begin -->
<output_text ending in one LF>
<!-- memorynexus:<block_id>:end -->
```

Output must not contain either marker prefix or NUL. Normalize its terminal LF
before computing output_sha256; no other whitespace/content rewriting. New block
appends at EOF with exactly one separating LF when needed. Existing block replaces
only the prior entire matching marker region; preserve every other byte including
human text. Duplicate/missing one-sided markers are conflict. Compare both whole
file baseline and previous block body hash retained in the intent/receipt.

Proposal additionally contains `base_exists:boolean` (required); absent targets use
base_sha256=SHA256(empty) plus base_exists=false, distinct from an empty existing
file. Core approval includes this field. Parent directory must exist/be allowed;
new target is no-clobber publication. Existing target replacement uses a same-dir
temporary, fsync, atomic replace, parent fsync then ordinary file readback. Keep
owner-only mode0600. No follow of symlinks, path escape or recursive deletion.

Fixed neutral output is UTF-8 `MemoryNexus: source evidence changed; review required.\n`.
Its block ID remains unchanged. It removes old substantive body only when current
file/block match the retained publication state and the profile authorized
neutralization. An edited block needs owner review; do not erase the user's edit.
Fresh substantive output never inherits that neutralization authorization.

| Crash/recovery observation | Unique allowed next action |
| --- | --- |
| CLI intent absent, local log says success | Query/replay original request; log is not authority; no file write first. |
| Intent current, before hash/existence match, plan intact | Recheck sources/approval, exact original file operation may run. |
| After hash/existence match, intent retained | Readback + same pending publication-result or new exact completion request; no rewrite. |
| After hash matches but source/profile became stale | Record actual effect as stale, schedule authorized neutralization or needs_review. |
| Neither before nor after hash, or edited block | conflict, preserve file; new preview/approval required. |
| Plan/body missing, intent retained | uncertain/needs_review; status may classify hashes, but no reconstructed content under old approval. |
| Local receipt missing, ledger receipt present | Recover original IDs by status/exact replay; no duplicate write. |
| Ledger absent/wrong incarnation or restored | recovery_required; old plans never resume publication. |
| A succeeded, B failed, summary write failed | retain A receipt, attempt B independently, structured nonzero report; no rollback of A. |

Reading hashes and replace are not an atomic compare-and-swap against editors that
ignore the cooperative lock. Recheck immediately before replace and after it;
detectable interference reports conflict, never overwrite it on retry. The system
does not claim cross-file/SQLite atomicity or prevention of every noncooperating
editor race. Ledger can become stale before documents are neutralized; status
must expose that lag. SQLite restore does not restore documents or operation files.

## Explicit block removal

See [publication maintenance](publication-maintenance.md) for
`prepare --workflow publish --removals REQUEST.json`, exact owner approval,
byte-preserving block removal and command-level acceptance. No production
installation or binding change is implied by this extension.
