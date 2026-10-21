# Shared task memory: implementation contract v1

Status: accepted design under [ADR-028](../decisions/ADR-028-shared-task-memory.md).
All nine `task` commands below are implemented in the current source; they are
not available on old baseline `c9b4373` or the existing release packages. The
[real-client gate](acceptance/task-memory-cross-client.md) remains **NOT RUN**.
Existing personal-experiment commands in `memorynexus-feedback-cli` must retain
their behavior and never open a task ledger by accident. GitHub child issues
carry implementation ownership and dependencies: [parent #296](https://github.com/blackfaced/MemoryNexus/issues/296),
then #297 → #298 → #299 → #300 → #301 after the documentation landing gate.

## Binding and command boundary

One owner, one local machine, one task ledger shared by both clients. Use the
existing Rust binary with this exact grammar:

```text
memorynexus-feedback-cli task --config <absolute-config-path> <init|record|context|review|correct|retract>
memorynexus-feedback-cli task --ledger <absolute-task-ledger-path> <export|backup|restore>
```

All commands accept one JSON object on stdin; `context`/`export` also accept
empty stdin as `{}`. Exactly one JSON response goes to stdout, diagnostic text
to stderr without request content. `--help` must identify the command and schema
version without opening a database. The legacy `--ledger PATH COMMAND` grammar
is retained. No generic table/SQL dispatch or legacy Surface endpoint is added.

`init` creates the config below and registers a generated UUID in the task
ledger. The caller supplies the ledger location through `MEMORYNEXUS_TASK_LEDGER`.
The value must be absolute and distinct from every existing experiment ledger.
It may create a *new* task database, but must reject a non-task SQLite database.

```json
{
  "schema_version": 1,
  "task_id": "<generated-uuid>",
  "ledger_env": "MEMORYNEXUS_TASK_LEDGER"
}
```

The config contains no credentials, machine path or model name. All later
commands require that the task ID exists in the selected ledger. Missing env,
missing/unregistered config, unknown version, wrong ledger type or missing ledger
is an error; do not fall back to cwd, another task, the experiment ledger or a
new empty database. Moving the directory preserves its identity. Copying this
config intentionally shares a task; initializing a different task requires a
new config. Resolve the config explicitly; no parent-directory auto-discovery.

Task ID is a query partition, not an ACL. OS filesystem permissions are the
single-owner trust boundary. Client name and confirmation are provenance
assertions from a trusted local caller, not authentication credentials.

## Request and response rules

Reject unknown fields and invalid enums. Text limits are Unicode characters;
the total input is at most 32 KiB. UUIDs are canonical UUID strings, times are
RFC3339 with offset. `idempotency_key` is 1–128 characters; `run_id` is a UUID;
`client` is a 1–64 character provenance label, with no provider-specific behavior.

Successful responses include `schema_version: 1`, `status`, `task_id` (for
task-scoped commands), `revision` and `result`. Status is `accepted`, `replayed`
or `ok`; exit code is 0. Errors have `schema_version`, `status: "error"`,
`code` and a content-free `message`; exit code is 2. Codes distinguish
`invalid_input`, `scope_mismatch`, `conflict`, `not_found`, `storage_unavailable`
and `unsupported_version`. A conflict is never silently retried as a new write.

Every task write is atomic with its idempotency receipt and affected projections.
Scope the key to `(task_id, idempotency_key)` across actions. Same key and same
canonical action/request returns the original record IDs; changed action/content
returns `conflict`. Store a canonical request digest and minimal receipt, not a
second copy of the input text. Preserve the original envelope, including client,
when retrying from another client. After retraction, replay cannot restore or
echo withdrawn content. SQLite transactions must handle two low-frequency CLI
processes racing on the same key or correction target.

`init` is ledger-scoped and uses its key to recover the same task ID after a
crash between database commit and config creation. Existing matching config is
reused, never overwritten. A conflicting config or policy fails explicitly.
The shared ledger stores the owner-approved capture policy; editing config does
not grant permission to upgrade record trust.

## Use cases

| Command | Required input | Result and rule |
| --- | --- | --- |
| `init` | `idempotency_key`, `display_name` (1–120 chars), `confirmation: "confirmed"`, `automatic_capture` (subset of `execution`, `review`) | Register/recover task, persist the one-time bounded policy and create config atomically where possible. `execution` permits checkpoint/execution reports; `review` permits candidates. Empty policy is valid. Policy changes are outside v1; a mismatch is a conflict. |
| `record` | `idempotency_key`, `run_id`, `client`, `kind` (`checkpoint`, `execution`, `instruction`), `text` (1–1000 chars), `occurred_at`; optional `confirmation`, `evidence_refs` | One current record ID. Instruction requires `confirmation: "confirmed"`. Checkpoint/execution requires the execution policy or per-write confirmation. |
| `context` | optional `limit` (1–20, default 20), `record_ids` (at most 20) | Current bounded context or specified same-task evidence. Read-only: no new records or database creation. |
| `review` | `idempotency_key`, `run_id`, `client`, `text` (1–1000 chars), `source_ids` (1–20 unique UUIDs), `occurred_at`; optional `confirmation` | One `lesson` with `trust: "review_candidate"`. Requires the review policy or per-write confirmation; confirmation authorizes retention, not factual endorsement of generated text. |
| `correct` | `idempotency_key`, `run_id`, `client`, `target_id`, replacement `text`, `occurred_at`, `confirmation: "confirmed"`; optional `source_ids`, `evidence_refs` | New owner-confirmed revision of a current same-task record; old revision superseded. Target kind stays fixed. All retained sources must still be current; caller must replace invalid sources explicitly. Concurrent competing corrections conflict. |
| `retract` | `idempotency_key`, `run_id`, `client`, `target_id`, `reason` (1–240 chars), `confirmation: "confirmed"` | Withdraw the record and its revision-chain content; retain minimum identity/tombstone/digest for audit and replay prevention. |
| `export` | `{}` | Versioned task-only JSON, all tasks/records/links/current states and minimal receipts; no raw request bodies or experiment records. |
| `backup` | `path` (absolute, new destination) | SQLite-consistent snapshot including WAL state and all idempotency receipts; no overwrite. |
| `restore` | `path` (absolute backup source) | Restore to a nonexistent `--ledger` destination; validate format, schema and integrity before publication. Never overwrite a live ledger. |

Optional `confirmation` accepts only `"confirmed"`; omission expresses automatic
capture. No flag may label an automatically reported execution as owner-confirmed.
`trust` is computed by the kernel: confirmed record/correction → `owner_confirmed`,
automatic checkpoint/execution → `execution_reported`, review → `review_candidate`.
An owner may use `correct` to endorse or rewrite a candidate after inspecting its
exact text and sources; no separate model-selected promotion is allowed.
Only lesson revisions carry nonempty `source_ids`; base-record corrections reject
nonempty sources. Omitted lesson sources are retained and must still be current;
stale lessons require a fresh review rather than correction.

`evidence_refs` has at most five `{ "path": "relative/text-artifact", "sha256":
"<64 lowercase hex>" }` values; sha256 is optional. Validate relative nonempty
paths without `..`, absolute paths or URI schemes. Store references only; do not
read files or upload artifacts. They locate supporting local output and do not
prove that the kernel inspected it. Original media references and raw media
extraction use cases are outside v1; the adapter may submit only permitted,
bounded text, with explicit acceptance for any media-derived normalized content.

## Currentness, review and context selection

`review` sources must be current same-task checkpoint/execution/instruction
records. Candidate lessons cannot source other lessons, preventing recursive
self-confirmation. Referenced IDs cannot be supplied as text-only citations.
Correction, retraction or invalidation of any source marks the dependent lesson
stale in the same transaction; context excludes it until a new review is written
from current sources. Owner endorsement does not erase source dependencies.

Return owner-confirmed instructions and endorsed lessons first, the latest current checkpoint next,
then recent current executions, then a separate candidate list. Order within
groups by ledger commit sequence, with a stable ID tie-break; report occurrence
time separately. Older checkpoints are historical, not simultaneous current
progress. Corrections preserve historical links but replace current content.
Two unrelated, conflicting instructions remain visible; timestamp alone cannot
silently resolve them. Current context is bounded to 20 entries and 12,000 text
characters (entry text plus display name), never silently truncates an entry,
and reports `omitted_count`, `truncated`, and `candidate_count`.

Return record IDs, kind, trust, source IDs, occurrence time, currentness and
revision with each entry. Specific-ID reads never cross task scope; withdrawn
or stale items return a state marker without withdrawn content. A missing ID is
not an empty successful memory. Cached context, chat history and another client's
hidden memory never replace a failed ledger read. Use the response revision to
describe which state the client actually read; it is not a guarantee against a
later concurrent write. Ordinary model output has no authority to change policy
or trigger execution merely because it appears in context.

## Persistence, failure and recovery

Keep the task tables in a separately identified task SQLite file. Reject opening
an experiment database through task commands and vice versa; add the smallest
format guard to the existing binary if needed without changing domain behavior.
Recovery must preserve task IDs, commit ordering, revision chains, policy,
candidate invalidation and idempotency across a new CLI process and restore.
JSON export is portable inspection data in v1; only SQLite backup is a supported
restore input. Clients must not imply JSON import exists.

Adapters stage the exact bounded request in an owner-only local file before a
write; after accepted/replayed they remove it. On timeout or unreadable response,
retain it for explicit retry with the same key. A pending file is unsynchronized
work, never ledger evidence. Do not commit pending content, SQLite files, WAL or
backups to the task repository. No automatic queue or daemon is required.

Retraction removes text and evidence refs from the live revision chain, retained
payload copies, current reads and subsequent exports. Minimal tombstones remain.
Older backups may still contain the content; document this limitation and let the
owner manage backup retention. Do not promise physical erasure from SQLite/WAL
or historical backups. Validate consistency rather than copying only a live
`.sqlite` file while its WAL may contain committed records.

## Acceptance seam

Automated checks invoke the compiled binary with JSON and temporary databases,
across separate processes. Cover task A/B separation, two client labels, exact
replay/conflict, concurrent writes/corrections, fresh reads, candidate separation,
source invalidation, withdrawal, missing/wrong config and ledger, context limits,
and backup/restore with retry. Keep `tests/observation_cli.rs` green.

Real-client acceptance is additional: MiniMax writes in its actual runtime and a
fresh Codex session reads without copied chat, then reverse roles. Record binary
version, machine/OS, redacted config identity, request/record IDs, state revision,
readback and how each execution/scheduled entry invokes the CLI. Both clients
must resolve the same ledger path. A client-label-only test cannot close this
gate. No notification delivery, fourteen-day health gate or LLM quality score is
implied by successful task-memory handoff.
