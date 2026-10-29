# Daily helper v1: frozen public interface (#343)

Frozen before implementation at source base
`9ceb4608fcf896bcf789e3fb26272965bbe73bbd`. Owner: MemoryNexus task-memory
integration. Rust CLI, SQLite schema and repository workflows are unchanged.

## Reuse decision

| Existing capability | Daily use |
| --- | --- |
| #300 PROTOCOL.md + task context/record/review/correct/retract | Authoritative schema, policy, provenance and lifecycle |
| #304 bootstrap | Initial approved import only; its init/batch manifest is not an existing-task write interface |
| #312 install.py | Already selects all `scripts/` and `memorynexus-*` resources; source revision, binary SHA and package inventory remain separate identities |
| #327–#329 current adapter | Reuse binding/path/hash validation and bounded subprocess transport; current read/check/launch remain their separate v2, freshness-aware contract |
| #337 Codex bridge | Pinned read-only fresh-session delivery; not ordinary writes or merging an existing conversation |

The new helper is `integrations/task-memory/scripts/daily.py` in source and
`VERSION/package/daily.py` in an installation. It uses Python standard library
and existing shared modules. Always invoke the exact pinned Python:

```text
PYTHON -I -S -B DAILY verify --binding ABS
PYTHON -I -S -B DAILY read --binding ABS [--request ABS]
PYTHON -I -S -B DAILY stage --binding ABS --action record|review|correct|retract --request ABS --run-dir NEW_ABS
PYTHON -I -S -B DAILY apply --binding ABS --run-dir ABS
```

`read` defaults to `{limit:20}`; an optional 0600 request is the existing CLI
`context` object (`limit`, optional `record_ids`). No client in read requests.
`stage` takes the complete existing CLI write envelope from a private file,
including the original idempotency_key/client/run_id/time and any owner-provided
confirmation. No fields are generated or upgraded. `apply` also serves as the
**explicit** retry; no background retry, init, migration or replacement task.

## Binding and preflight

Reuse the reader binding from the [current contract](../../../../docs/current-knowledge-contract.md#binding):
`schema_version:1`, task_id, task_root, config, ledger, binary, binary_sha256,
python, package, package_sha256, state_root, timeout_seconds, client, copies;
optional repository_binding/bridge retain their existing meaning. Absolute
paths, no symlinks, private permissions, existing ledger/config, exact task ID,
Python and CLI/package hashes are checked on every call. Config remains
`task_root/.memorynexus.json`. This helper needs no repository profile, consumer,
publication or bridge. Do not remove existing copies declarations to pass a check.

`verify` additionally checks CLI `task --help` schema 1 and command capabilities,
and reads the bound task. The binary hash pins its exact bytes; record its build
source revision separately from the script package revision. Reuse an older
verified binary only if capability/preflight and public-process tests pass.
`package` is the source scripts directory or installed `VERSION/package`;
package SHA uses the existing sorted `package_inventory` + canonical digest.
Never repin drifted runtime files automatically.

## Installed resource boundary

Daily execution requires the pinned binary, the complete shared Python package,
this interface, the canonical Skill, shared protocol and referenced task/binding
contracts. Their local links and files must resolve inside the installation.
Do not fetch missing execution resources, substitute another package or use an
exception below to bypass binding/package verification.

The installer preserves document paths under `VERSION/package`, but maps
`integrations/task-memory/scripts/*.py` to `VERSION/package/*.py`. In particular,
the source evidence link to `integrations/task-memory/scripts/workflow_io.py` in
`docs/current-knowledge-plan.md` is not an installed invocation path: its runtime
module is `VERSION/package/workflow_io.py`. No duplicate module is installed.
Rust implementation links refer to the separately recorded binary build source.

Recursive document navigation reaches the following eight source-only targets
through 13 links. These links remain relative to the source checkout; they do not
resolve in an installation. Consult the recorded source revision when reviewing
them, not the latest branch or an unrelated checkout. They are not prerequisites
for an existing task's daily read/write. This is a bounded exception inventory,
not permission to ignore other missing resources.

| Source-only target (repository relative) | Referring documents (repository relative) | Boundary |
| --- | --- | --- |
| `docs/acceptance/repository-memory-workflows.md` | `docs/repository-memory-v1.md`, `docs/acceptance/current-knowledge.md`, `docs/current-knowledge-plan.md` | Prior kernel/runner acceptance evidence; does not establish this installation's acceptance |
| `docs/TODO.md` | `docs/task-memory-onboarding.md`, `decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md`, `decisions/ADR-028-shared-task-memory.md` | Repository roadmap navigation; not installed-task execution authority |
| `decisions/README.md` | `decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md` | ADR index navigation |
| `docs/handoffs/minimax-repository-memory-discovery.md` | `docs/repository-memory-workflows-issues.md`, `decisions/ADR-029-repository-memory-workflows.md` | Conditional operational dependency for repository sample discovery, not a daily-task dependency |
| `src/task_memory/lifecycle.rs` | `docs/current-knowledge-plan.md` | Implementation evidence in the binary build source |
| `src/task_memory/mod.rs` | `docs/current-knowledge-plan.md` | Implementation evidence in the binary build source |
| `src/task_memory/workflow/lifecycle_hooks.rs` | `docs/current-knowledge-plan.md` | Implementation evidence in the binary build source |
| `integrations/task-memory/scripts/workflow_io.py` | `docs/current-knowledge-plan.md` | Source evidence; installed runtime mapping stated above |

Before repository sample discovery, obtain the handoff from the explicitly
selected source revision and read its scope. If unavailable, stop discovery and
report the missing dependency; this package alone does not provide that workflow.
The handoff authorizes neither import nor schedule changes. Daily use of an
already bound task does not trigger discovery or require this handoff.

Installation tests recursively follow local Markdown links from the canonical
Skill. Every missing edge must match the exact referring-document/target pair
listed above; unknown missing edges, paths outside the package, and stale
exceptions fail. Required runtime files are checked separately. Passing this
bounded check does not mean every repository navigation link is packaged.

## Read semantics

Output preserves the CLI response (IDs, task/revision, kind/trust/state,
client/run/time, source_ids, evidence_refs, omission and truncation). Ordinary
context may contain candidate bodies in a separate list: they are proposals,
not facts. Presentation labels for owner instructions, execution self-reports,
independent acceptance and candidates do not create truth fields. Independent
acceptance is an attributed execution with an evidence reference, not a
checkpoint or invented trust value.

`context` works with v1 and v2 task ledgers. It does **not** attest to external-copy
freshness or deliver material to another client. For current-use validation and
pinned fresh-session delivery use the existing `current read/check/launch`
adapter; that path needs v2 and exposes candidate indexes only, without bodies.
A truncated ordinary read returns its bounded data with omissions and exit 3:
inspect relevant IDs before depending on missing history. A failed read returns
`history_not_loaded`, exit 2, and no partial content; stop work that depends on
history. Explicit current instructions can still be followed with the gap stated.

## Pending write and readback

`stage` validates binding and task, validates the envelope, checks a precise
correction/retraction target, then creates a new 0700 run directory **inside
state_root, outside the task Git repository**. It writes a UUID-named 0600
request and `pending.json`, both exclusive/fsynced. The note binds action,
request digest, config digest, original task/config/ledger/binary/package and
key. It is local transport state, not a second ledger. Copying the original
request into this location is staging only; dispose of the caller's draft under
its own retention policy. Never commit pending or receipt files.

`apply` locks the bound task's local helper state, validates the original bytes
and note, submits them once, and validates exit 0, one schema-1 JSON object,
accepted/replayed, task ID, revision, record UUID and allowed state. It explicitly
reads that returned ID again and preserves the observed current state. A replay
may return historical, superseded, stale or retracted evidence; it is never
presented as a new current write. Readback must not predate the receipt revision.
A successful result gives the actual record ID, receipt and fresh context.

Only after validated receipt **and readback** does the helper save a content-free
`receipt.json`, then remove request/note. Failures, timeout, nonzero exit,
truncated output, bad schema/task, lost response or readback failure keep the
same bytes and report `pending`, exit 2. Explicit retry keeps original provenance,
even from the other client; a second binding may differ only in `client`.
A validated save with cleanup failure is `saved_cleanup_pending`, exit 3. A
completed run can be re-read without resubmitting. Competing correction remains
conflict, never a new key, new target or automatic confirmation.

Diagnostics are fixed codes only: no stderr, exception text, request body or raw
chat. A binding failure before staging is `not_staged`. `verify`/`read` success
is exit 0; stage success is `staged`, exit 0. No action claims client enforcement,
installation freshness or independent acceptance merely from a CLI receipt.
