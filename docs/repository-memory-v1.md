# Repository memory v1: exact contract for owner review

**R2 (#309): ACCEPTED on 2026-10-23. Runtime implementation: #310 and #311 in PR #316.**
The owner accepted fixed contract commit
`83d85c6c0216b6ae4089c83db03dd0a473235851`; #315 merged as
`a16aa41a97ee35941b57d9fee420a78ac35afc80`. Command-level acceptance is recorded
in #310/#311 before implementation. The new commands are available when built from PR #316; [implementation acceptance](acceptance/repository-memory-workflows.md) records their checks. This does not authorize production deployment. [ADR-029](../decisions/ADR-029-repository-memory-workflows.md)
and the [behavior contract](repository-memory-workflows-contract.md) remain the
scope authority. The existing nine [task v1 commands](task-memory-contract.md) retain their inputs alongside the new workflow commands.

The R1 [Skill](../integrations/task-memory/memorynexus-repo-onboard/SKILL.md) and
[pilot walkthrough](../integrations/task-memory/memorynexus-repo-onboard/references/pilot-walkthrough.md)
provide the requirements: useful experience rather than only pointers, missing
targets, nonmonotonic timestamps, source corrections, immutable approval, and
independent consumers. Examples here are synthetic, not private pilot data or
evidence of real-client acceptance.

## 1. What memory means

A fact claim concerns the world or what evidence says. A memory is a reusable
record of that claim, experience, constraint or candidate, retaining attribution,
evidence and applicability. Neither retention nor owner confirmation proves
objective truth. A short experience can preserve the observation, result,
conditions and uncertainty without copying the source document in full.

| Dimension | Meaning | Does not imply |
| --- | --- | --- |
| trust | source-reported, execution-reported, owner-confirmed, or review candidate | factual truth, source verification, permission to execute |
| currentness | usable against the versions actually observed by this workflow | the filesystem/world has not changed since the scan |
| consumer decision | one Lens proposed, approved, skipped or deferred an output | any other Lens made the same decision or source trust increased |
| publication | a particular file effect was read back and receipted | the underlying claim is true or still current |
| context selection | a bounded set shown to the client | omitted records were deleted, forgotten or processed |

| Event | Lifecycle effect |
| --- | --- |
| New checkpoint | Previous checkpoint becomes historical; dependents stale. |
| Confirmed correct | Previous record superseded; new owner-confirmed revision, dependent decisions/lessons stale. |
| Observed source/config change | Affected derived records/decisions stale atomically; no confirmation of replacement content. |
| Confirmed retract | Ledger revision-chain body/refs and structurally dependent text scrubbed; source identity blocked; minimum tombstones retained. Published files remain pending coordination/authorized neutralization; old backups are not erased. |
| Source unavailable | Retain identity/content for audit, mark derived use stale; no inferred deletion or owner retraction. |
| Context budget limit | Omitted count only; no mutation. |
| Old/unused but otherwise valid | No age-only purge or automatic trust promotion. |

## 2. Boundary and version names

The same Rust `memorynexus-feedback-cli` and task SQLite remain authoritative.
The adapter uses Python's standard library, selected local text and managed
Markdown regions. No HTTP/Lens service, extra database, direct SQL writer, model
SDK, scheduler, MCP wrapper or maintenance-script migration is included.

| Version | Frozen value / rule |
| --- | --- |
| Current task config and existing command envelopes | 1, unchanged |
| New workflow CLI envelopes / request contract | `schema_version: 1`, `workflow_contract: 1` |
| Task SQLite / export format after explicit migration | 2, application_id unchanged (`1296979027`) |
| Profile / machine binding / runner / operation plan | `schema_version: 1`, separate types |
| Bootstrap manifest/helper | Existing v1, unchanged exact snapshot and approval semantics |

See [kernel fields and commands](repository-memory/kernel-v1.md),
[profile, runner and publication protocol](repository-memory/runner-v1.md), and
[complete walkthroughs and acceptance](repository-memory/walkthrough-v1.md).
These linked references are normative. They resolve the earlier behavior draft's
open field/schema choices; no new runtime behavior is claimed.

## 3. Reuse and minimum new state

`task_records` remains the sole normalized memory-body store. A source-captured
item has `kind: draft`, `trust: source_reported`; an actual execution continues
to use `record`, and a sourced inference uses existing `review`. Normal context
excludes drafts even after owner correction; workflow input inspection exposes
them explicitly. Current commands retain their confirmation semantics.

| Addition | Why existing structures are insufficient |
| --- | --- |
| Approved workflow profile/binding | Existing init policy covers execution/review, not arbitrary draft capture or document targets. |
| Source collection cursor and source versions | evidence_refs only locate artifacts; they cannot identify stream occurrences or observe revisions. |
| Source-to-record mapping | Links one extracted body to its source version and prevents re-extraction duplicates. |
| Consumer decisions and complete dependency set | Existing lesson source IDs do not represent independent adoption, auxiliary file dependencies or publication authorization. |
| Publication intents/receipts | A record receipt cannot assert an external file effect or coordinate recovery. |
| Ledger incarnation and typed receipt result | Restore must invalidate old workflow approvals; receipt IDs can refer to non-record use cases without fake memory records. |

Original source bodies remain upstream. An exact document proposal's rendered
bytes remain private operation material, not another authoritative memory store.
SQLite keeps its digest, scope, dependencies, approval and receipt. Losing the
bytes has an explicit recovery result; it never authorizes reconstructing a new
proposal under an old approval.

## 4. Acceptance and implementation handoff

- R1 is a discovery/proposal Skill, not production ingestion. Existing bootstrap
  is still new-task-only; existing tasks use the future incremental path.
- The fixed contract is owner-accepted. No source-sync/consumer command is
  available merely because its name appears here; implementation evidence is separate.
- #310 implements the kernel cases in walkthrough §5. #311 implements the
  runner/file cases in §6 after #310; both reference the accepted fixed commit.
- #312 owns installation/deployment and #313 real workflow acceptance. #301 and
  #283 retain their distinct gates. Manual success is not scheduled success.
- Shared repository files contain synthetic fixtures only. Production paths,
  content, IDs, hashes, approvals and pending requests remain owner-only.
