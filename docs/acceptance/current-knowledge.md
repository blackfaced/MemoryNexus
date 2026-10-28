# Current knowledge acceptance

Status: **CONTRACT v1 FROZEN (2026-10-27); G2 task read/check, G3 repository
read/check and G4 bridge launch are implemented and tested with synthetic public
processes. Actual MiniMax/Codex and scheduler acceptance remains separate.**
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
adapter contract. The G4 tests run separate CLI, adapter and child bridge processes;
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

Implemented tests are `test_workflow_current_knowledge.py`,
`test_workflow_current_knowledge_repository.py` and
`test_workflow_current_knowledge_delivery.py` under `integrations/task-memory/tests/`.
Build/discovery commands and ownership are in the
[runtime handoff](../current-knowledge/runtime-handoff.md). The G4 matrix below
combines their simulated coverage; it does not establish real-client acceptance.

## Separate execution columns

Fill a row per immutable version and client run; never roll simulated and real
success into one total. Missing input visibility is UNKNOWN, not pass.

| Evidence | Simulated public processes | Real MiniMax | Real Codex | Actual scheduler |
| --- | --- | --- | --- | --- |
| Pinned binary/package/bridge SHA and binding | PASS synthetic fixture | NOT RUN | NOT RUN | NOT RUN |
| Current material bytes/IDs/R/I/source observations | PASS | NOT RUN | NOT RUN | NOT RUN |
| Input delivery trace and fresh session/run identity | PASS fresh synthetic child | NOT RUN | NOT RUN | NOT RUN |
| B marker action, without A/B answer in prompt/env/argv | PASS | NOT RUN | NOT RUN | NOT RUN |
| Drift/withdrawal prevents covered launch of old material | PASS | NOT RUN | NOT RUN | NOT RUN |
| Remaining injected copies and bypass coverage | Declared copies tested; bypass outside coverage | NOT RUN | NOT RUN | NOT RUN |
| No-op/retry and actual scheduler trigger proof | Replay/claim PASS; scheduler NOT RUN | n/a | n/a | NOT RUN |

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


## G2 task read/check implementation (2026-10-27)

Implementation source: `f1f13dd555a4c42ce1f89e6d0435b0a596d1db1d` (#327), based on
frozen contract `c09a6120f4601befaaa290eab8a41fe334d34afa` (#318 / PR #326).
New files are `scripts/current_knowledge.py`, `current_knowledge_task.py`,
`current_knowledge_artifacts.py`, `tests/current_knowledge_fixture.py` and
`test_workflow_current_knowledge.py` under `integrations/task-memory/`.
No Rust, workflow runner behavior, installer or private client configuration changed.

The real CLI was built offline on macOS from unchanged Rust source at
`46434b06a7304ce421aa8d020657675066f8249e`; tests use a separate stripped copy with
SHA-256 `3ff8ad8c4ae9d398ccde82e82bdff10f86010d408376e6ec9a639d883a275b23`.
The original build artifact was retained. Every fixture copies and hashes the
actual adapter package, initializes two synthetic tasks before migration, and
invokes separate CLI/adapter processes with explicit ledger/config paths.

| Scope | Evidence | Remaining |
| --- | --- | --- |
| C01/C02 task correction | Current B excludes A; exact replay, stale-target conflict and concurrent competing corrections; old read check fails | Repository outputs and child marker action |
| C03/C04 | Metadata-only candidates; candidate-only current read empty; corrected draft stays out; superseded history/tombstones and history check refusal | Repository trust attribution |
| C05/C09 | Missing/v1 ledger, wrong task/config, strict/oversize input, empty/count/character/material-byte limits, bounded process pipes/timeout | Repository paging |
| C06 | Real CLI barrier produces deterministic revision races: one retry succeeds, a second drift stops; check refuses changed revision and actual restored incarnation at unchanged revision | External source/publication races |
| C08/C12 | Unknown/missing external copies; symlink/private-mode refusal; package/material/result tampering; original evidence identity, expiry, bounded private cleanup; unrelated contradictory records remain current | Real harness coverage, repository copies and launch |
| C10 | Retraction removes new current/history/export chain body; public post-withdrawal backup/restore changes incarnation; failed check removes original material | Repository and delivery combinations |

The barrier forwards real CLI responses; it adds a controlled pause at context
for concurrent writes. Separate fault modes deliberately time out or overflow a
pipe to prove refusal; no fabricated successful CLI response is used. Restore
checks use a publicly restored SQLite file in disposable paths, not direct SQL.

Validation at the implementation commit:

- 17 new public-process task tests passed with no skips.
- 22 task bootstrap tests passed with no skips.
- Full workflow regression and remote PR checks are tracked on #327/its PR.
- Python source compilation, `cargo fmt --all -- --check` and `git diff --check` passed.

Commands use `MEMORYNEXUS_TEST_BINARY=ABS_TO_STRIPPED_TEST_COPY` and
`PYTHONDONTWRITEBYTECODE=1`, then the unittest discovery/bootstrap commands in the
handoff. No new Rust behavior tests were added because Rust did not change;
required Clippy/Build/Test gates still apply to the PR.

At the G2 baseline, repository mode, repository_binding and launch returned
`unsupported_version`. G3 below implements repository read/check and binding
verification. G4 below adds launch after the accepted G3 dependency. No bridge was installed, no
actual client received material, and the real-client/scheduler table above stays
NOT RUN for this adapter. Existing #301 PARTIAL evidence retains its own scope.
This is a partial implementation of frozen v1, with no change to authority or
confirmed-content rules and no claim to cover #317's second or third phase.


## G3 repository read/check implementation (2026-10-27)

Implementation source: `fafc6e78700cdfc8c17b32d02b636841219b777d` (#328), based on
accepted G2/main `4dd046d61c36cfc5e973814f07a48ee139754475`. Frozen v1 semantics
remain `c09a612`; this slice adds no CLI command, schema or authority write.

The reader pins all six workflow views, explicit dependency reads and original
kernel receipt lookups to one revision/incarnation. It validates the common
installation, workflow profile/binding, full typed dependency closure, actual
source selections/JSONL consumed prefixes, original publication plan and exact
request/receipt identities. It compares each current block with original output
bytes and records/rechecks whole target hashes, so later blocks in the same file
and replacement generations work without trusting historical whole-file hashes.
All selected targets are automatically included as managed copies. External copies
remain unknown. Read/check never sync, confirm, publish, neutralize or change human
regions. Missing retained proof prevents delivery; it is never reconstructed.

Files: new `scripts/current_knowledge_repository.py`,
`current_knowledge_publication.py` and
`tests/test_workflow_current_knowledge_repository.py`, plus the existing reader,
evidence/transport helpers and fixture. The only existing G2 test expectation
changes replace unsupported-repository results with missing/mismatched-binding
refusals; #328 ownership explicitly records this transition. Existing workflow
runner behavior, Rust and production installations are unchanged.

| Scope | Public-process evidence | Remaining |
| --- | --- | --- |
| C01/C03 | Both consumers, source-reported drafts and complete auxiliary record/source/candidate closure; candidate body excluded and trust retained; correction invalidates old authority | Fresh child action in G4 |
| C06 | Every page uses R/I; actual writes between input pages exhaust the single allowed retry; check rejects changed fingerprints and incarnation | Finite observation race after final check remains |
| C07 | Edited/deleted/selected sources, JSONL prefix changes/new complete rows/partial tail, profile changes, edited/unknown/duplicate target blocks; no read-side sync or file repair | Actual repository/client gate |
| C08/C09 | Automatic target copies, whole-file hashes, exact managed dedup/conflict, external unknown, mismatched task/binding and unknown consumer refusal | Actual injected-input inventory |
| C10 | Correction/withdrawal prevent stale output; actual backup/restore at unchanged revision invalidates old publication; failed check purges original material | Combined bridge cases in G4 |
| C12 | Exact retained plan/request/receipt/rendered bytes, kernel receipt digest, source/target symlinks, strict request fields and multi-page inputs | No cross-ID semantic dedup claim |

Validation uses the same isolated stripped CLI and unchanged Rust source/hash
recorded in G2. The final affected read/check set passes 34 tests (17 task, 17
repository), including one additional candidate-closure test after the initial
33-test run. Full workflow regression, bootstrap and exact-head CI results are
recorded in the #328 PR. Local bootstrap: 22 tests, no skips. Python parsing,
`cargo fmt --all -- --check` and `git diff --check` pass. Required remote
Clippy/Build/Test and PostgreSQL integration remain PR gates.

Retained proof lookup examines at most 1,000 direct workflow-state entries and
1,000 receipt entries per selected publication; exceeding the bound is incomplete,
not permission to ignore old files. Status paging uses the frozen 128-page/view
and 16 MiB aggregate limit. These bounds do not establish semantic freshness of
human regions or unobserved native caches. Material remains subject to the frozen
24-hour lifecycle and explicit check-before-use limitation.

This is repository adapter acceptance only. No bridge, real MiniMax/Codex session
or scheduled trigger ran, and the separate real-client columns remain NOT RUN for
this adapter. #312/#301/#313 and the existing #283 gate retain their own scope.


## G4 bridge delivery implementation (2026-10-27)

Implementation source: `09b12285cf226dcb9a3d0a266db4ae6a2b04ecec` (#329). G3 was
reviewed and accepted at `fd05036f73ce7ed08d6b9f1d6b5cedacfcd85750` in #331,
after all required CI checks passed. G4 uses the same frozen v1 contract and
unchanged Rust CLI binary/source/hash recorded above.

`launch` uses the complete task/repository check, reconstructs the original bytes,
checks the pinned executable, and creates/fsyncs an exclusive no-follow claim in
the original read directory before spawning one bridge. The child receives only
those bytes on stdin, no shell or extra argv, with the bound cwd and ledger.
Bounded ack/timeout/pipe handling separates acknowledged from unknown delivery.
An existing claim blocks later attempts even under another run directory. Neither
a failed spawn nor a missing ack authorizes automatic retry. Evidence stores no
authentication environment, stderr or material body; material is purged after
an attempt. Following a crash, the next adapter invocation purges identifiable
claimed material while preserving the claim. A crashed run without delivery.json
is unresolved, never an implied successful or safe-to-repeat launch.

Changed files: new `scripts/current_knowledge_delivery.py` and
`tests/test_workflow_current_knowledge_delivery.py`, plus the existing entry,
transport/evidence helpers, test fixture and the obsolete unsupported-launch test
expectation. No existing workflow runner behavior, Rust/schema, installer or real
client hook changed. The runtime does not create a scheduler or model client.

### Combined synthetic C01–C12 evidence

| Case | Result and precise scope |
| --- | --- |
| C01 | PASS: one two-task fixture records A, reviews a candidate, publishes both consumers, corrects the exact current ID to B, rejects both old snapshots, and republishes with fresh approval. Separate task and repository launches receive the exact new bytes and actually open B's marker; neither covered child opens A. |
| C02 | PASS: G2 exact replay/competing correction/race tests establish one current revision. G4 never replaces a stale snapshot during launch. |
| C03 | PASS: candidates remain metadata-only in task input; draft/auxiliary record/source/candidate dependencies retain original trust in repository output. No candidate body is delivered. |
| C04 | PASS: explicit superseded history remains audit material; history launch refuses before claim/spawn. |
| C05 | PASS: G2 strict/empty/overflow/missing-ledger/timeout refusals produce no usable original evidence; G4 missing/changed bridge and invalid original evidence cannot spawn. |
| C06 | PASS: real task/page revision races, final source/file rechecks, immutable check/launch comparison and concurrent delivery claims. A post-check external write remains a documented finite-observation limit. |
| C07 | PASS: G3 source/JSONL/profile/target drift cases; G4 actual upstream source mutation and injected stale target block stop launch before claim. Human regions remain unchanged. |
| C08 | PASS within declared-file scope: external old cache blocks covered reads/launch; independent cache contents are preserved. An explicit direct-bridge bypass test can still read independently cached A bytes, with no adapter claim/delivery record. That is outside coverage, not freshness success. Native injection visibility remains a real-client gate. |
| C09 | PASS: task/config/repository mismatch and cross-task context tests; task X in the combined fixture receives no A/B instruction body. |
| C10 | PASS: task correction/retraction/backup/restore and repository restored-incarnation tests; old material cannot launch and is purged. Prior independent backups/copies remain outside chain erasure. |
| C11 | PASS: exact stdin hash, actual marker read/hash, fresh child PID/session receipt, bound cwd/ledger and inherited synthetic auth. Wrong ID/hash, extra/duplicate fields, oversized ack/session, stderr overflow, nonzero exit, lost ack, timeout and spawn failure produce unknown; claim prevents repeat. |
| C12 | PASS: G2/G3 strict input/path/artifact/paging bounds and G4 claim symlink/crash/concurrent-process cases. Cross-ID contradictory records and bypass remain explicit limits, not semantic dedup or whole-machine enforcement. |

The A/B selection comes only from material text: neither marker choice is supplied
in argv, environment or a separate prompt. The bridge code performs a bounded
synthetic marker selection, opens that relative file in the bound task root and
records its digest. This proves the adapter's controlled test path, not model
comprehension or any installed MiniMax/Codex bridge. The deliberate bypass case
is separate from the covered C01 trace and is never counted as a compliant action.

Validation at the implementation source: 48 affected current-knowledge tests
passed (17 task, 17 repository, 14 delivery; the initial 46-test run plus two
boundary tests), 22 bootstrap tests passed, no skips. The full workflow regression
and exact-head required CI are recorded in the #329 PR. Python source parsing,
`cargo fmt --all -- --check` and `git diff --check` passed. No new Rust tests were
added because Rust behavior did not change; remote Format/Clippy/Build/Test and
PostgreSQL integration remain mandatory. The optional external-service Acceptance
job is not a substitute for the real-client columns above.

### Actual-install handoff

[#312](https://github.com/blackfaced/MemoryNexus/issues/312) must install a versioned
package/CLI and an explicitly selected, pinned client bridge on the target machine.
Its existing #325 workflow launcher invokes workflow operations and is not a
client bridge. Keep the reader state root separate from workflow runs, ledger and
package; store the strict current binding owner-only. A package/binary/binding
change also requires the existing confirmed workflow-binding/reconciliation path;
do not silently reuse previous publication authority after changing its pin.

The bridge must establish a fresh real session, deliver the exact stdin bytes and
return the frozen ack only after real delivery. Pin and record the actual client
version/configuration as well as the bridge hash. Missing client capability is
bridge_unavailable/unknown, never a fabricated ack. Inventory AGENTS/skills/native
memory/cache inputs: declared external copies remain unknown under v1. A successful
synthetic bridge cannot justify deleting an inventory entry or promising the old
chat history is erased.

[#301](https://github.com/blackfaced/MemoryNexus/issues/301) retains the actual
MiniMax↔Codex handoff and owner decision. [#313](https://github.com/blackfaced/MemoryNexus/issues/313)
retains the repository workflow and actual scheduler trigger/no-op/retry evidence.
Record client input trace, fresh real session IDs, B-marker action, hashes and
unknown injected inputs in those existing private acceptance records. Do not retry
an unknown delivery by deleting its claim; reconcile client evidence first.
No real client, native hook, scheduler, production ledger or private repository
was changed by G4. #317 remains the parent for real-use feedback and later phases;
#283 and the legacy contraction gates remain unchanged.
