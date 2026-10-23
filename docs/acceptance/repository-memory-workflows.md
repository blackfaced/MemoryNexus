# Repository memory workflow implementation acceptance

Accepted exact contract: `83d85c6c0216b6ae4089c83db03dd0a473235851`.
The contract merged through #315 as `a16aa41a97ee35941b57d9fee420a78ac35afc80`.
Implementation PR: [#316](https://github.com/blackfaced/MemoryNexus/pull/316).

## Kernel (#310)

Reviewed implementation commit: `e50f4f8139d01816fab2dd55c17c7679bbef76ce`.
Independent review has no unresolved findings. One test used the wrong export
argv; corrected to the public `--ledger` interface and independently rerun.

| Contract behavior | Public CLI evidence |
| --- | --- |
| Strict scope, identity, JSON, concurrency and bounded status | `tests/task_workflow_cli.rs`: 11 cases |
| Independent consumers, complete dependencies, selective invalidation, exact publication approval and historical effects | `tests/task_workflow_consumers_cli.rs`: 14 cases |
| Explicit verified-backup migration, full export/backup, new restore incarnation and authorization gates | `tests/task_workflow_recovery_cli.rs`: 6 cases |
| Existing task behavior | `tests/task_memory_cli.rs`: 31 cases |

Required checks passed: `cargo fmt --check`, full `cargo test`, and
`cargo clippy --all-targets --all-features -- -D clippy::all`.
The first sandboxed full run could not bind loopback listeners for three legacy
adapter tests. The complete rerun with listener permissions exited 0.

The actual preserved baseline v1 binary rejected migrated v2 ledgers for export,
backup, context and record; source bytes and logical exports stayed unchanged.
Synthetic fixtures and isolated ledgers were used throughout.

After restore, historical publication authorization remains unexecutable.
After reconciliation and exact renewed binding, the current profile may authorize
a **new** fixed-neutral maintenance intent referencing previously published stale
history. It cannot approve the old substantive output or resume the old file plan.

## Runner (#311)

Implementation and acceptance are in progress. No runner completion is claimed.

## Separate deployment gates

Production installation/migration and all pinned callers remain #312 work.
Real dual-Lens, real MiniMax/Codex clients and actual scheduler execution remain
separate acceptance work. These synthetic tests do not close #301 or #283.
MCP wrappers and maintenance-script packaging remain deferred.
