# MemoryNexus Roadmap

> Updated: 2026-10-23 (R1 merged; R2 exact contract accepted; R3 then R4 implementation)
> Executable tracker source of truth: [GitHub Issues](https://github.com/blackfaced/MemoryNexus/issues).
> The shared-task-memory issues below are published. The older reconciliation
> section remains a historical recommendation, not a claim that its issues were closed.

## Accepted plan: repository onboarding and repeatable memory workflows

[ADR-029](../decisions/ADR-029-repository-memory-workflows.md), the
[behavior contract](repository-memory-workflows-contract.md), and
[R1–R6 issue index](repository-memory-workflows-issues.md) define the accepted
behavior plan. Skill-led discovery and bootstrap lead to incremental
scripts, independent Lens consumption and external document updates. The existing
SQLite/CLI stays authoritative; document names and scheduler bindings live in the
integration profile. R2 freezes exact interfaces before kernel implementation;
MCP and maintenance-script packaging remain deferred, without new tickets.
Issues #308 → #309 → #310 → #311 → #312 → #313 are published. R1's
[onboarding Skill](../integrations/task-memory/memorynexus-repo-onboard/SKILL.md)
and synthetic pilot walkthrough are in source. The [R2 exact contract](repository-memory-v1.md)
was accepted at `83d85c6c0216b6ae4089c83db03dd0a473235851` on 2026-10-23;
#315 merged and #310/#311's command-level acceptance is recorded. Implement
the incremental CLI in #310, then the runner in #311. Production deployment requires
its own explicit scope. Existing #301/#283 gates retain their own status.
For a real sample now, use the [MiniMax read-only discovery handoff](handoffs/minimax-repository-memory-discovery.md).

## Current direction

[ADR-028](../decisions/ADR-028-shared-task-memory.md) and
[shared task-memory spec #296](https://github.com/blackfaced/MemoryNexus/issues/296)
are the current priority: MiniMax and Codex on the task machine share bounded
task context through the same Rust CLI/JSON and local SQLite task ledger.
The task ledger is separate from existing experiment data. HTTP/MCP transport,
legacy Surface work, model SDKs and a new scheduler are not prerequisites.

The architecture/contract gate landed in [#302](https://github.com/blackfaced/MemoryNexus/pull/302)
(`7ae3d67`). #297–#300 have source implementation on the integration branch in
[#303](https://github.com/blackfaced/MemoryNexus/pull/303); live issue/PR status is
the closure authority. The target machine is unavailable, so #301 and parent
#296 remain open. A `ready-for-agent` label means specified, not dependency acceptance.

| Order | Issue | Outcome / dependency |
| --- | --- | --- |
| Plan | [#296](https://github.com/blackfaced/MemoryNexus/issues/296) | ADR-028, contract, docs and complete mirrored specification; tracks the real-client gate. |
| T1 | [#297](https://github.com/blackfaced/MemoryNexus/issues/297) | Task init/config, scoped record/context, strict JSON and idempotency; after the documentation gate. |
| T2 | [#298](https://github.com/blackfaced/MemoryNexus/issues/298) | Review candidates, owner correction/retraction and source invalidation; after #297. |
| T3 | [#299](https://github.com/blackfaced/MemoryNexus/issues/299) | WAL-consistent backup/restore/export preserving task identity and receipts; after #298. |
| T4 | [#300](https://github.com/blackfaced/MemoryNexus/issues/300) | Shared protocol, thin MiniMax/Codex entry instructions, existing-review integration and simulated smoke; after #299. |
| T5 | [#301](https://github.com/blackfaced/MemoryNexus/issues/301) | Owner/Coordinator real MiniMax↔Codex and scheduled-review acceptance on the other machine; after #300. |

Read [the exact contract](task-memory-contract.md) before implementation and
[the onboarding guide](task-memory-onboarding.md) before target-machine work.
T5 requires actual client execution and an owner decision; it remains open when
the target machine is unavailable. Complete the [real-client TODO and evidence
record](acceptance/task-memory-cross-client.md) when access is supplied. The local
simulated preflight does not establish deployment or an actual scheduled run.

## Existing personal-experiment track

[ADR-027](../decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md) and
[parent spec #273](https://github.com/blackfaced/MemoryNexus/issues/273) reset
the default product to a local personal experiment feedback kernel:

```text
MiniMax Skill -> local CLI -> SQLite authoritative ledger
Observation -> Recommendation -> Experiment -> Outcome
```

The legacy Rust + Axum / PostgreSQL / Qdrant / Surface / Adapter roadmap is
frozen while replacement is validated. It is historical context, not a second
active product roadmap. No historical data migration, dual write, or
compatibility layer is planned.

## Personal-experiment dependencies retained

| Order | Issue | Required outcome |
| --- | --- | --- |
| 1 | [#274](https://github.com/blackfaced/MemoryNexus/issues/274) | **Completed:** normal WeChat MiniMax and an independent native scheduled session share local state. Cron output is owner-queryable through the existing conversation, not a proactive App/WeChat reminder. |
| 2 | [#275](https://github.com/blackfaced/MemoryNexus/issues/275) | Record ADR-027, align positioning, and prepare legacy-roadmap reconciliation. |
| 3 | [#276–#281](https://github.com/blackfaced/MemoryNexus/issues/276) | Feedback CLI/Skill/recovery source exists at `c9b4373`; installation and owner gates are separate. |
| 4 | [#282](https://github.com/blackfaced/MemoryNexus/issues/282) | Clean-Mac-mini installation acceptance. |
| 5 | [#283](https://github.com/blackfaced/MemoryNexus/issues/283) | Fixed fourteen-calendar-day owner dogfood gate. |
| 6 | [#284–#286](https://github.com/blackfaced/MemoryNexus/issues/284) | Only after a passed gate: cut over defaults, then remove legacy interfaces and storage. |

#274 closed as passed on 2026-08-28. It proved cross-session local-state
continuity and recorded a delivery constraint: cron output lacks channel context,
so the owner must query the existing MiniMax conversation. Proactive MiniMax App
or WeChat delivery is not a first-version capability and must not become an
implementation assumption.

## Fourteen-day deletion gate

ADR-028 authorizes only its bounded task-memory slice ahead of this gate. It
does not change the following criteria, substitute #301 for #283, or authorize
legacy deletion early. Future #284–#286 must preserve the new task path.

The old runtime remains until #283 passes. The fixed gate requires at least ten
valid confirmed Observations, one Experiment, five execution updates
(performed, skipped, or not-evaluable), one evidence-backed result review,
consistent fresh-session recovery, no more than one manual system intervention,
and a day-fifteen owner continuation decision. A failure stops expansion and
requires analysis of value or friction; it does not justify more infrastructure.

## Live tracker reconciliation — recommendations only

These were read from live GitHub on 2026-08-26. “Freeze” means remove
`ready-for-agent`/active-roadmap treatment while preserving the issue and its
history. “Supersede” means close only with a reference to #273/#275 after
Coordinator approval. “Rewrite” means retain the issue only if it becomes a
post-gate, evidence-backed extension. None of these actions have been executed.

### M9 Personal Feedback Dogfood

| Issue | Current conflict | Recommended action |
| --- | --- | --- |
| [#222](https://github.com/blackfaced/MemoryNexus/issues/222) | Requires a legacy MCP/Surface/API private Adapter path and `CognitiveSpace`. | Freeze now; supersede after #275 review. #274 completed the replacement tracer and recorded pull-only delivery. |
| [#226](https://github.com/blackfaced/MemoryNexus/issues/226) | Builds legacy seven-/fourteen-day Engine review through Surface Gateway. | Freeze now; supersede with #283’s four-object review gate. |
| [#227](https://github.com/blackfaced/MemoryNexus/issues/227) | Runs the old M9 Surface/Adapter dogfood gate. | Freeze now; supersede with #283; preserve its fixed-gate lessons as history. |

### M10 upstream learning feedback

| Issue | Current conflict | Recommended action |
| --- | --- | --- |
| [#229](https://github.com/blackfaced/MemoryNexus/issues/229) | Requires parent-authorized learning Observation through the legacy Engine and Surface. | Freeze now; supersede after #275 unless a passed #283 identifies a real need to rewrite it as a separate post-gate product decision. |

### M11 reference Adapter synchronization

Every remaining open M11 issue expands the frozen provider-neutral source
evidence, Reference Adapter, Surface Gateway, PostgreSQL, launchd, or Study
Buddy path. Freeze each now; after Coordinator approval, supersede it with #273
rather than deleting history. No M11 work resumes before a passed #283 and a
new evidence-backed spec. #239 and #240 are different: their implementation
landed in `47fa3e0` and should be closed as completed, then retained as frozen
historical capability rather than described as unfinished supersession work.

| Issues | Individual live issue titles | Recommended action |
| --- | --- | --- |
| [#239](https://github.com/blackfaced/MemoryNexus/issues/239) | Accept versioned source evidence with revision and tombstone semantics | Implemented by `47fa3e0`; close as completed, then freeze as historical legacy capability. |
| [#240](https://github.com/blackfaced/MemoryNexus/issues/240) | Build durable Reference Adapter runtime and operational ledger | Implemented by `47fa3e0`; close as completed, then freeze as historical legacy capability. |
| [#241](https://github.com/blackfaced/MemoryNexus/issues/241) | Implement the Study Buddy Reference Adapter | Freeze, then supersede. |
| [#242](https://github.com/blackfaced/MemoryNexus/issues/242) | Generate bounded Learner Journey Summaries with owner review | Freeze, then supersede. |
| [#243](https://github.com/blackfaced/MemoryNexus/issues/243) | Cut over and accept Study Buddy L1 ingestion on the Mac mini | Freeze, then supersede. |
| [#251](https://github.com/blackfaced/MemoryNexus/issues/251) | Deliver one Study Buddy Learning Attempt end to end | Freeze, then supersede. |
| [#252](https://github.com/blackfaced/MemoryNexus/issues/252) | Add lease, retry backoff, and dead-letter controls | Freeze, then supersede. |
| [#253](https://github.com/blackfaced/MemoryNexus/issues/253) | Synchronize Study Buddy Sessions, revisions, and withdrawals | Freeze, then supersede. |
| [#254](https://github.com/blackfaced/MemoryNexus/issues/254) | Retain generated artifacts and expose redacted health state | Freeze, then supersede. |
| [#255](https://github.com/blackfaced/MemoryNexus/issues/255) | Run content-free inventory and reconciliation jobs | Freeze, then supersede. |
| [#256](https://github.com/blackfaced/MemoryNexus/issues/256) | Acquire bounded Study Buddy Summary Windows | Freeze, then supersede. |
| [#257](https://github.com/blackfaced/MemoryNexus/issues/257) | Reconcile Study Buddy and report legacy migration safely | Freeze, then supersede. |
| [#258](https://github.com/blackfaced/MemoryNexus/issues/258) | Generate and deliver one deterministic unreviewed Journey Summary | Freeze, then supersede. |
| [#259](https://github.com/blackfaced/MemoryNexus/issues/259) | Add explicitly authorized cloud summary generation | Freeze, then supersede. |
| [#260](https://github.com/blackfaced/MemoryNexus/issues/260) | Query and confirm a Journey Summary | Freeze, then supersede. |
| [#261](https://github.com/blackfaced/MemoryNexus/issues/261) | Correct or delete a Journey Summary with owner provenance | Freeze, then supersede. |
| [#262](https://github.com/blackfaced/MemoryNexus/issues/262) | Compose a weekly review from current trusted evidence | Freeze, then supersede. |
| [#263](https://github.com/blackfaced/MemoryNexus/issues/263) | Prepare Mac mini release, launchd, and cutover preflight | Freeze, then supersede. |
| [#264](https://github.com/blackfaced/MemoryNexus/issues/264) | Verify redacted alerts and operational recovery | Freeze, then supersede. |
| [#265](https://github.com/blackfaced/MemoryNexus/issues/265) | Run the synthetic L1 failure and recovery matrix | Freeze, then supersede. |
| [#266](https://github.com/blackfaced/MemoryNexus/issues/266) | Cut over bounded Study Buddy L1 backfill | Freeze, then supersede. |
| [#267](https://github.com/blackfaced/MemoryNexus/issues/267) | Run one owner-authorized real-record acceptance | Freeze, then supersede. |

| Issue | Current conflict | Recommended action |
| --- | --- | --- |
| [#269](https://github.com/blackfaced/MemoryNexus/issues/269) | Parent for Study Buddy correction/reinforcement semantics through the Reference Adapter and Surface Gateway. | Freeze, then supersede; preserve as historical source-integration design. |
| [#270](https://github.com/blackfaced/MemoryNexus/issues/270) | Maps correction and reinforcement attempts in the Study Buddy Adapter. | Freeze, then supersede. |
| [#271](https://github.com/blackfaced/MemoryNexus/issues/271) | Reports long-term correction patterns through legacy weekly Observation. | Freeze, then supersede. |

### Other live legacy deployment work

| Issue | Current conflict | Recommended action |
| --- | --- | --- |
| [#129](https://github.com/blackfaced/MemoryNexus/issues/129) | Hosted Trial API plus MCP/REST token path conflicts with local SQLite/CLI-only first version. | Freeze; rewrite only after a passed #283 proves a concrete need for a second channel. |

The live M9, M10, M11, and Support milestones still advertise the prior
Surface/Adapter/PostgreSQL roadmap. Do not close or delete milestones as part of
this documentation task. A Coordinator should first apply the issue-level
freeze/supersession decisions, then update or close milestones with a durable
comment linking #273 and ADR-027. For #239/#240, record completed delivery
before freezing the legacy milestone material.

## Historical material

The prior roadmap, ADRs, architecture documents, and runtime remain available
for review and behavioral reference during contraction. They are not deleted by
this change. Git history remains the durable record after removal of the legacy
runtime.
