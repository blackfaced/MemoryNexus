# Current knowledge G1 acceptance

Status: **CONTRACT v1 FROZEN (2026-10-27); new adapter/runtime acceptance NOT RUN.**
Issue [#318](https://github.com/blackfaced/MemoryNexus/issues/318) delivers the
[contract](../current-knowledge-contract.md), examples and implementation handoff.
It does not install a bridge or complete real-client acceptance.

## Evidence that already exists

The [cross-client record](task-memory-cross-client.md) remains **PARTIAL**:
it reports real MiniMax sessions and recurring review on earlier task-memory
versions; the complete Codex path and owner decision are outstanding.
The [repository workflow record](repository-memory-workflows.md) preserves
#310/#311 kernel and runner process tests. Neither record proves this new
read/check/launch adapter is installed. Do not reset them to NOT RUN or count
their old client labels as a new G1 run. #301, #312, #313 and #283 retain their
own gates and outcome authority.

## Public-process test specification

The implementation test must create private synthetic task A and task X,
directories `workspace-a` and `workspace-b` with distinct harmless marker files,
isolated ledgers, and a real CLI compiled from a recorded immutable commit.
Initialize both tasks in v1 with explicit synthetic fixture confirmation, then migrate via a
verified backup to v2, and bind the existing two-consumer workflow. No real
ledger, model, private repository, hook or scheduler is modified.

[task-sequence.json](../current-knowledge/examples/task-sequence.json) contains
supported stdin examples, with target/source IDs explicitly replaced from CLI
responses. [read-request.json](../current-knowledge/examples/read-request.json)
and [material.json](../current-knowledge/examples/material.json) use the frozen
adapter contract. The test runs separate CLI, adapter and child bridge processes;
no direct database mutation, fake CLI response, or model label stands in for one.

| Case | Setup/action through public interfaces | Required assertions |
| --- | --- | --- |
| C01 explicit A→B | record confirmed instruction for workspace-a; create a review candidate and two consumer outputs depending on it; obtain fixture confirmation and correct exact current ID to workspace-b | Old record superseded; old lesson/decisions/approvals invalid; task current material contains B and no old A body. Fresh child receives those exact bytes, selects B from them and opens B's marker; trace proves it never opened A |
| C02 replay/competition | Replay exact saved correct JSON; then new-key correction against the old target; race two different corrections against one current target | First replay adds no revision/record; losing request returns conflict; only one new current revision; adapter never changes target or falls back to record |
| C03 candidate/draft | Review an execution; capture source draft; explicitly correct a current draft; publish a separately approved consumer output | Candidate body excluded from task current material; all drafts absent even owner_confirmed. Repository material has attributed consumer output and complete dependencies, never silently promotes source trust |
| C04 history | Explicit task history read with old/new IDs after C01; attempt check/launch | History labels retained, superseded A only for audit; history cannot launch. Current requests reject record_ids; stale/retracted have no body |
| C05 transport/gaps | Missing/unreadable ledger, wrong task binding, invalid/oversize JSON, process timeout, empty context, >20 current records or >12,000 characters | No client launch, replacement ledger, implicit init or hidden-memory fallback; explicit error/empty/incomplete; no truncated current material reported successful |
| C06 concurrent read/use | Change task revision between status and context/page; change selected source/file between observations; restore before final header; mutate after read before check/launch | At most one complete read retry; persistent change fails. check/launch reject old R/I/fingerprints. No mixed pages, changed approval or silent snapshot replacement; exact post-check race limitation documented |
| C07 source/target/profile drift | Change/delete source or JSONL prefix; change bound profile or managed target block after publication; leave stale block in an injected file | needs_review, no stale current material. Original authorized workflow invalidates/reviews/neutralizes or stays pending. Human region unchanged. Old publication authority cannot publish B without new exact approval |
| C08 copies/entry | Inject an external old file or unobservable native cache; bypass wrapper; exercise unknown copy and history-only input | Unknown/stale shown and covered launch blocked; bypass counted outside coverage, not as a pass. No claim old session content was deleted; approved managed block alone cannot bless whole injected file |
| C09 cross-task | Query/correct task A record through X config; mismatched repository binding; foreign record ID in dependency/history | scope/not-found failure as specified by existing CLI; no bodies leak into X and no changes to either task |
| C10 withdrawal/restore | Retract the corrected chain; export; backup **after withdrawal**; restore to isolated new destination; query/retry old requests/try old publication plan | No withdrawn chain body in new context/history/export/replay; dependent current material unavailable; new incarnation, recovery gate and old authorization unusable. Old external cached artifacts cannot be delivered by new entry. Older backups and independently retained derived records are outside this chain-erasure assertion |
| C11 actual delivery | Child records exact stdin hash and reads chosen synthetic marker; compare retained material hash with receipt; force bridge timeout or missing/wrong ack | Byte hash and actual harmless action both match. Unknown delivery never auto-retries launch; neither process name nor client string alone establishes real client acceptance |
| C12 limits/safety/known boundary | Unknown/duplicate JSON keys, symlink/temp collision, modified executable/package/artifact, overflow/page cap, and two separately recorded contradictory instructions | Unsafe/incomplete input fails, no partial material or permission weakening; material retention/purge follows contract. Both cross-ID facts may remain current: record as known phase-1 limit, not automatic dedup success |

Run C01 through task current first. Then recapture/review and obtain the exact
new publication approval to exercise repository current through both consumers.
One consumer failure must not be relabeled as both ready. For C07, assert file
bytes outside managed blocks are identical. No whole-file deletion is authorized.
For C10, search synthetic sentinel bodies in newly generated outputs, not merely
in JSON state labels. Historical pre-withdrawal backup contents are not a failure
of the post-withdrawal restore assertion and must not be silently deleted.

Expected future test location:
`integrations/task-memory/tests/test_workflow_current_knowledge.py`.
Exact build/discovery commands and ownership are in the
[runtime handoff](../current-knowledge/runtime-handoff.md). All C01–C12 remain
NOT RUN for the frozen adapter until that implementation exists.

## Separate execution columns

Fill a row per immutable version and client run; never roll simulated and real
success into one total. Missing input visibility is UNKNOWN, not pass.

| Evidence | Simulated public processes | Real MiniMax | Real Codex | Actual scheduler |
| --- | --- | --- | --- | --- |
| G1 installed binary/package/bridge SHA and binding | NOT RUN | NOT RUN | NOT RUN | NOT RUN |
| Current material bytes/IDs/R/I/source observations | NOT RUN | NOT RUN | NOT RUN | NOT RUN |
| Input delivery trace and fresh session/run identity | NOT RUN | NOT RUN | NOT RUN | NOT RUN |
| B marker action, without A/B answer in prompt/env/argv | NOT RUN | NOT RUN | NOT RUN | NOT RUN |
| Drift/withdrawal prevents use of old material | NOT RUN | NOT RUN | NOT RUN | NOT RUN |
| Remaining injected copies and bypass coverage | NOT RUN | NOT RUN | NOT RUN | NOT RUN |
| No-op/retry and actual scheduler trigger proof | NOT RUN | n/a | n/a | NOT RUN |

Each private run record must include date/operator, source commit, OS, binary/
package/bridge hashes, task and installation binding digest, client version and
entry type, evidence/delivery digests, case outcomes, real session/trigger IDs,
unknowns and redacted artifact references. Publish only synthetic material or
bounded redacted results. Real transcript, source paths and source text stay private.
The owner continuation decision belongs to the existing gates; this template
does not start a new fixed-length trial or change #283's fourteen-day criteria.

## Contract verification on 2026-10-26

The supported sequence was exercised with **22 real CLI calls** in a disposable
private directory on macOS, using the CLI built from the unchanged Rust source at
`955f1c53ea1f5fb14cad03fb1e6c31aa8005662c`; binary SHA-256:
`747358b6b29ed3be3ab79d74eac797c7cf7e59fe64651af69045f96e595e5d57`.
All assertions passed: current B, superseded A, stale candidate with no context
body, exact replay, competing correction conflict, cross-task refusal, withdrawn
chain export, post-withdrawal backup/restore and changed incarnation.

The first probe tried to register task X after migration and received
unsupported_version: init currently supports v1 only. The final fixture registers
both tasks before migration, as specified above. No production code was changed.
Existing export retains stale derived records' own text; the chain-erasure
assertion checks withdrawn record IDs, not semantic copies throughout the ledger.

JSON examples, the material byte digest, relative links and `git diff --check`
are documentation checks. No Rust source changed, so no new Rust test suite is
required here. These checks do not execute the proposed adapter or establish any
of C01–C12 as a completed end-to-end client case.
