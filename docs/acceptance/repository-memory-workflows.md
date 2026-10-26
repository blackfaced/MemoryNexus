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

Reviewed implementation commit: `bcdd5b6d7eeec315b0b8f9972c3bb998c49b8112`.
Independent review has no unresolved findings. The exact final commit passed all
35 runner tests, with no skips, using the real compiled CLI. Existing bootstrap
regression tests passed 22 cases with no skips.

```text
MEMORYNEXUS_TEST_BINARY=ABS_TO_COMPILED_CLI python3 -m unittest discover -s integrations/task-memory/tests -p 'test_workflow*.py'
MEMORYNEXUS_TEST_BINARY=ABS_TO_COMPILED_CLI python3 scripts/test_task_memory_bootstrap.py
```

The tested CLI SHA-256 is
`d437b93d12ea775e0ac61da56662a64df3757df760ca3994bee91f253c92f322`.
Tests use synthetic private repositories and real init/backup/migrate/bind calls.

Coverage includes all five commands, two independent Lens/document publications,
no duplicate publication, selective auxiliary-evidence invalidation, JSONL 20+2
batching/prefix/blocked/partial-tail/EOF refresh, source-unavailable review gates,
immutable plan approval and exact request replay, crash recovery, permanent stop
after baseline conflict, preservation of human text, result files per attempt,
and independent branch/report-write failures.

Trigger metadata is bounded, validated and retained privately. Unsupported or
unverifiable scheduler proof remains manual with a visible pending gap. No real
scheduler-provider execution is claimed. Python bytecode creation is disabled
before imports so ordinary execution preserves the pinned package inventory.

## CI integration (#319)

The existing `Test` job now runs the complete `test_workflow*.py` discovery
against the absolute `MEMORYNEXUS_TEST_BINARY` built from the checkout. A missing
executable, zero discovered cases, any skipped case, or test failure fails the
job. Counts are reported by unittest, not pinned to the historical 35-case count.
The bootstrap step and other required checks remain in place.

The two process fixtures use `Path(tempfile.gettempdir()).resolve()` instead of
`/private/tmp`. Resolving the system temporary root accommodates macOS aliases
and Linux paths without changing production no-symlink or private-mode checks.

Local macOS verification, 2026-10-26, against the #319 patch on
`955f1c53ea1f5fb14cad03fb1e6c31aa8005662c`:

```text
CARGO_TARGET_DIR=/private/tmp/memorynexus-319-target cargo build --locked --offline --bin memorynexus-feedback-cli
MEMORYNEXUS_TEST_BINARY=/private/tmp/memorynexus-319-target/debug/memorynexus-feedback-cli PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s integrations/task-memory/tests -p 'test_workflow*.py' -v
MEMORYNEXUS_TEST_BINARY=/private/tmp/memorynexus-319-target/debug/memorynexus-feedback-cli PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_task_memory_bootstrap.py
git diff --check
```

Results: build passed; 35 workflow cases passed in 43.612s with no skips;
22 bootstrap cases passed in 16.229s with no skips; whitespace check passed.
Compiled CLI SHA-256:
`747358b6b29ed3be3ab79d74eac797c7cf7e59fe64651af69045f96e595e5d57`.
The exact CI shell body was also checked with a missing binary, an empty
synthetic test directory and a skipped synthetic case; all exited nonzero.
No Rust source changed, so the local checks target the modified process suite;
the existing full Rust checks still run in CI.

The first Ubuntu run at `a247f0e6df9f5a50faf485ccb0f64cabf98090a3` discovered all
35 cases but failed fixture initialization because Linux debug sections made the
CLI exceed the runner's 128 MiB file-read limit. CI now strips only debug sections
from its checkout-built executable before workflow tests; production limits and
runner behavior remain unchanged. The fixture pins the resulting binary hash.

Ubuntu evidence must be read from [PR #323](https://github.com/blackfaced/MemoryNexus/pull/323)'s
Actions run for its exact head commit. The corrected run is pending at the time
of this record; macOS results alone do not establish Ubuntu success.
These runs use isolated synthetic repositories
and ledgers, with no model, scheduler or private installation access.

## Separate deployment gates

Production installation/migration and all pinned callers remain #312 work.
Real dual-Lens, real MiniMax/Codex clients and actual scheduler execution remain
separate acceptance work. These synthetic tests do not close #301 or #283.
MCP wrappers and maintenance-script packaging remain deferred.
