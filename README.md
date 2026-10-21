# MemoryNexus

MemoryNexus keeps local task experience across MiniMax and Codex sessions, and
confirmed personal experiment history for one owner. Rust CLI + stable JSON +
SQLite remain the implementation direction.

The current priority is [shared task memory, ADR-028](decisions/ADR-028-shared-task-memory.md):
read a task's context, record execution, retain sourced review candidates, and
continue in another client. See the [contract](docs/task-memory-contract.md),
[other-machine onboarding](docs/task-memory-onboarding.md) and [issues](docs/TODO.md).
The source implements `task` init/record/context/review/correct/retract/export/backup/restore
and a simulated CLI preflight. The [real-client gate](docs/acceptance/task-memory-cross-client.md)
has **not run**; #301 and parent #296 remain open.

The existing personal-experiment path helps one owner keep a confirmed record of
what they observed, which advice they chose, what reversible adjustment they tried,
and what happened later. It
is not a generic AI memory store, a medical-analysis product, a cloud service,
or an Agent runtime.

The personal-experiment contract remains [ADR-027](decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)
and its parent [spec #273](https://github.com/blackfaced/MemoryNexus/issues/273).
[#274](https://github.com/blackfaced/MemoryNexus/issues/274) has passed: a real
MiniMax cross-session tracer proved shared local state. It also established that
cron output is owner-queryable only, not a reliable proactive WeChat or MiniMax
App notification.

## The feedback kernel

```text
confirmed Observation
  -> sourced Recommendation
  -> one active, reversible Experiment
  -> confirmed Outcome
  -> evidence-backed review
```

- `Observation`: a confirmed, bounded fact or subjective report.
- `Recommendation`: a bounded suggestion from the owner, an external advisor,
  or explicitly marked Agent wording.
- `Experiment`: the one selected reversible action, its period, and expected
  observable signal.
- `Outcome`: whether it was performed, skipped, or not evaluable, plus the
  confirmed result.

SQLite in WAL mode is the authoritative store. At baseline `c9b4373`, the
compiled `memorynexus-feedback-cli` contains JSON commands for `observe`, `retract`,
`add-recommendation`, `start-experiment`, `record-outcome`, `review`, and
`due`, `end-experiment`, `observation-history`, `export`, `backup` and `restore`.
Source presence is not clean-machine acceptance: #282 and #283 remain separate
installation and owner gates. Task records use a separate task ledger;
they do not change the existing experiment database or its confirmation rules.

## MiniMax and health boundary

The owner can use an existing MiniMax Agent through WeChat for convenient input.
MiniMax Skill turns natural-language intent into explicit local CLI use cases
and asks for confirmation before every authoritative write. A native scheduled
task can invoke `due` in an independent session; the owner retrieves its
result by querying the existing MiniMax conversation. The SQLite ledger, not
chat history, provides continuity. Proactive WeChat or MiniMax App delivery is
not a first-version capability.

MemoryNexus does not diagnose, interpret medical documents, prescribe treatment,
or make clinical claims. Ant Afu and other tools are external Recommendation
sources. Their raw conversations, reports, diagnoses, and prescriptions remain
ephemeral by default; only an owner-confirmed bounded summary is retained.

## Transition

The former Rust + Axum, PostgreSQL/Qdrant, Surface Gateway, MCP, Sleep/Dreaming,
Dictation, Thought Review, and source Adapter runtime is frozen during the
replacement. There is no data migration, dual write, compatibility promise, or
new generic backend abstraction. Git history remains the record of the retired
implementation.

The old runtime is deleted only after the fixed fourteen-calendar-day owner
dogfood gate passes. Until then, this repository contains the frozen legacy
implementation while the new SQLite/CLI path is added and validated. A future
third-party memory backend may only be a rebuildable, non-authoritative recall
projection from the SQLite ledger.

## Roadmap and documentation

- [Shared task memory decision](decisions/ADR-028-shared-task-memory.md)
- [Shared task memory onboarding](docs/task-memory-onboarding.md)
- [Shared client protocol](integrations/task-memory/PROTOCOL.md)
- [Real-client acceptance TODO](docs/acceptance/task-memory-cross-client.md)
- [Personal-experiment decision](decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)
- [Current roadmap and tracker reconciliation](docs/TODO.md)
- [Current architecture](docs/architecture/README.md)
- [Vision](docs/vision.md)
- [All architecture decisions](decisions/README.md)
- [GitHub Issues](https://github.com/blackfaced/MemoryNexus/issues)

The older documents under `docs/architecture/` describe the frozen legacy
runtime. They remain available for historical context and must not be treated
as the supported product path.

## Verification

From a source checkout containing the task implementation:

```sh
cargo build --locked --bin memorynexus-feedback-cli
python3 scripts/task-memory-smoke.py --binary target/debug/memorynexus-feedback-cli
cargo test --test task_memory_cli
cargo test --test observation_cli
```

The smoke uses temporary synthetic data and fresh CLI processes; its JSON report
is explicitly `simulated_cli`, with real-client acceptance `not_run`. Installation
uses a pinned source build: current release packages omit this binary. Follow
[onboarding](docs/task-memory-onboarding.md) for commit/hash/help checks and the
actual MiniMax, fresh Codex and existing scheduled-review gate. These checks do
not replace the independent #283 fourteen-day personal-experiment gate.

## License

MIT. See [LICENSE](LICENSE).
