# Shared task memory: implementation contract v1

Status: accepted design under [ADR-028](../decisions/ADR-028-shared-task-memory.md).
All nine `task` commands below are implemented in the current source; they are
not available on old baseline `c9b4373` or the existing release packages. The
[real-client gate](acceptance/task-memory-cross-client.md) remains **NOT RUN**.

The [repository-memory R2 proposal](repository-memory-v1.md) specifies a separately
gated DB v2 and new workflow actions. It is pending owner acceptance and not
implemented; the current v1 commands and confirmation rules below remain valid.
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
| `record` | `idempotency_key`, `run_id`, `client`, `kind` (`checkpoint`, `execution`, `instruction`), `text` (1–1000 chars), `occurred_at`; optional `confirmation`, `evidence_refs`, `merge_checkpoint_ids` | One current record ID. Instruction requires `confirmation: "confirmed"`. Checkpoint/execution requires the execution policy or per-write confirmation. |
| `context` | optional `limit` (1–20, default 20), `record_ids` (at most 20) or `checkpoint_handoff: true` for current context | Current bounded context or specified same-task evidence. Read-only: no new records or database creation. |
| `review` | `idempotency_key`, `run_id`, `client`, `text` (1–1000 chars), `source_ids` (1–20 unique UUIDs), `occurred_at`; optional `confirmation` | One `lesson` with `trust: "review_candidate"`. Requires the review policy or per-write confirmation; confirmation authorizes retention, not factual endorsement of generated text. |
| `correct` | `idempotency_key`, `run_id`, `client`, `target_id`, replacement `text`, `occurred_at`, `confirmation: "confirmed"`; optional `source_ids`, `evidence_refs` | New owner-confirmed revision of a current same-task record; old revision superseded. Target kind stays fixed. All retained sources must still be current; caller must replace invalid sources explicitly. A local edit against a superseded target returns conflict. Paired peer corrections and explicit resolution are described below. |
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

Return owner-confirmed instructions and endorsed lessons first, current checkpoints next,
then recent current executions, then a separate candidate list. Order within
groups by ledger commit sequence, with a stable ID tie-break; report occurrence
time separately. A checkpoint retires its causal predecessors. Independent children remain current.
For a paired task, `checkpoint_handoff: true` orders current checkpoints by original ID
and adds `origin_device` and `checkpoint_parents` where relay provenance exists.
Its `checkpoint_progress` counts all local current checkpoints before truncation;
`peer_completeness: "unconfirmed"` makes no claim about unseen peer writes.
A checkpoint replacing multiple local heads requires `confirmation: "confirmed"`
and `merge_checkpoint_ids` naming the exact set the owner approved. A changed set
conflicts; an unchanged retry of an accepted write retains its original receipt. Corrections preserve historical links but replace current content.
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

### 显式配对任务的记录传递

两个独立账本可从同一任务和已核对的基线记录开始。owner 为这一对设备生成两个设备 UUID、一个新的共享 epoch UUID 和一个随机的 32 字节密钥。通过已授权的私有通道向两端提供同一密钥的 64 位十六进制串，使用环境变量 `MEMORYNEXUS_TASK_PAIR_KEY`。密钥只用于 relay 调用。

入口为 `task relay --config <绝对路径> <动作>`，沿用绑定和 `MEMORYNEXUS_TASK_LEDGER`。每次调用从 stdin 读取一个 JSON 对象。

| 动作 | 输入及结果 |
| --- | --- |
| `pair` | 传入 `device`、`peer`、`epoch`、`confirmation: "confirmed"`。另一端对调 device 与 peer。配对在当前任务生效。账本格式 v1/v2 分别升为 3/4，后续访问使用支持配对的 CLI。 |
| `export` | `{"record_ids":["<来源记录 UUID>"]}` 显式选择 1–20 条本端在此次配对后创建的记录。将输出的 `result` 原样传给对端。基线及所依赖的前序须已存在，或一同选中。 |
| `receive` | 输入导出的签名批次。整批在一个事务中核对任务、设备、世代、签名和变更身份，按来源 ID 写入。返回本次新增及重放的 ID，以及此方向、此世代已接收的唯一变更数。该计数用于核对重放。 |

普通 `record`、`review`、`correct` 仍产生本机 ID 和写入回执。配对后，这些写入一并保存来源设备、世代、序号和前序引用。原账本被撤回或已失效的正文不能导出。恢复副本后须以新世代重新配对。

#### 配对后的撤回

使用原 `task retract` 入口逐次确认撤回。写入事务同时撤回整条修订链、使依赖结论失效，并为本次撤回分配独立的变更 ID 与来源序号。本机回执仍返回目标记录 ID。下次刷新通过已有配对传递撤回标记，载荷只含目标 ID 和确认，不携带原文或撤回理由。对端在同一事务更新普通读取与 Lens 的来源状态并记录已收位置。重传同一撤回或已经处理过的旧批次保持幂等。

若未确认的创建或订正在发送前已失效，刷新页以原序号、原始变更摘要和无正文的记录身份补齐该位置。已保存的旧批次随后重放时按原始摘要去重。显式按记录 ID 导出失效正文仍返回冲突。首次配对撤回或接收相应标记时，账本在原事务内升至 v11（基础账本）或 v12（工作流账本）；两端使用支持此格式的 CLI。

#### 同一修订的并发订正

两台已配对设备离线订正同一版本时，接收端保留每条订正的原 ID、来源设备及共同前序。当前 `context` 用 `pending_corrections` 和 `pending_correction_count` 单列待取舍分支。每条分支的状态为 `pending_confirmation`，不计入当前结论或当前 checkpoint；仍可按 ID 查询正文、`origin_device` 和 `supersedes_id`。读取、重放和双向收齐回执均保留待取舍状态。

owner 确认具体替换文本后，使用原 `correct` 入口，把一个待确认 ID 放入 `target_id`，同链其余待确认 ID 放入 `resolve_with`，同时给出 `text` 和 `confirmation: "confirmed"`。本机在写事务中核对完整分支集合，遗漏、异链或已失效的选择返回冲突。新订正按原同步路径传递，双方都能按新 ID 读到选定内容及全部直接前序 `supersedes_ids`；原分支保留各自来源，可按 ID 查阅。

首次接收合法分叉时，状态和修订边在同一事务升级到 v9（基础账本）或 v10（工作流账本）；较早的配对账本在普通写入与读取时沿用原格式。已升级副本需使用支持该格式的 CLI。

#### 手动刷新与连续回执

对同一对已绑定的设备，使用 `python3 integrations/task-memory/scripts/refresh_peer.py --local /绝对路径/local-relay --peer /绝对路径/peer-relay` 立即刷新。两个可执行入口分别固定本端和对端的已授权配置，接受一个动作参数，从 stdin 读 JSON，把对应 CLI 的 JSON 写到 stdout。传输与配对密钥由已有私有通道提供。每页至多 20 条。

在上述命令末尾加 `--auto`，可在一台协调设备的前台监听这对端点，退出即停止。任一方向积满 8 条，或最老待发变更入队满 2 秒，就启动一次同样的双向刷新。两方向分别计数。后续写入不重置等待时间；原命令随时可用于接班前立即刷新。同一协调机通过临时目录下的配对锁串行提交刷新轮，手动和自动调用共用端点配置及临时目录。在线监听的两次查询之间最多休眠 200 毫秒，实际开始发送和对端读回还要计入端点调用耗时。

`status` 可选传 `{"include_batch_age":true}`，由每个来源设备在本机计算 `outgoing.oldest_age_ms`。入队时间随本机变更在同一事务记录。待发数为零时年龄为 `null`；旧版本留下的待发变更没有时间戳，按积压立即补发。传输不可达时保留待发，并在退避后重试，重连有积压即可刷新。每个完成的刷新输出一行 JSON，含触发原因及两方向实际冻结的目标范围 `(after, through]`；成功仍以原有持久回执为准。


| relay 动作 | 约定 |
| --- | --- |
| `refresh` | 本端以 `{}` 开始一轮，在事务内冻结自己的来源流上界，返回签名声明。对端以 `{"round":"<同一 UUID>"}` 冻结其上界。重复当前轮沿用原目标。已配对的旧账本在首次新增本机变更或开启新刷新轮时升级到 v7（基础账本）或 v8（工作流账本）。使用该副本的 CLI 需支持相应版本。 |
| `page` | 以 `{"round":"<UUID>"}` 取已确认位置之后、冻结上界以内的下一批。可指定 `after` 和 `limit`。 |
| `receive-page` | 输入对端的页，或仅传 `{"offer":<对端声明>}` 交换上界。以同一事务验证并保存记录、去重结果与连续位置。返回的 `receipt` 仅覆盖目标内的连续前缀。缺少前序依赖的变更须在前序到达后重投。 |
| `ack` | 输入对端返回的签名 `receipt`，核对设备、任务、配对世代、当前轮及目标后持久化确认位置。同轮旧位置不能回退进度。 |
| `status` | 传 `{}`。本机的 `outgoing` 是对端已回执的位置，`incoming` 是本机已持久接收的位置。`covered` 仅证明本轮冻结的目标，`unconfirmed_range` 是尚未连续证明的区间。省略 `target` 表示该方向尚未声明上界。 `outgoing.pending_count` 是本机对此配对任务、对端及世代尚无连续回执的变更总数，包含冻结上界后的新写入。首次刷新前也可读取；本轮 `covered` 为真时仍可能大于零。 |

刷新中断后可以重新发起。上一轮可证的位置保留，新一轮需要自己的有效回执。签名回执遗失时重投同一批；接收方按原身份去重并重新签发。接手读取在同一 SQLite 快照查看两个方向和当前 checkpoint，已收齐的任务仍可能有多个待确认的 checkpoint。对端在已声明上界之后的尾部始终未知。

### 恢复已配对任务

把备份恢复到新路径后，每个曾配对任务独立进入待核对状态。普通读取（包括按 ID 读）、日常接班、Lens、写入和对外发送均受限；同账本中未配对的任务仍可使用。整库 JSON 导出在存在待核对任务时返回 `recovery_required`。可以保留 SQLite 备份。

用 `task relay --config <绝对路径> recovery-status` 查看旧世代、最后可证的发出和接收位置、未知尾部。此查询无需配对密钥。只剩旧备份时保留这个状态。旧 CLI 已恢复并停用配对的副本，可以用当前 CLI 再恢复到一个新路径，取得本次恢复专属的 `challenge`。未配对账本沿用原恢复行为。

在已授权私有通道上联系原对端。以下调用使用原配对密钥，只作用于选中的任务。恢复会将配对账本升级至 v13（基础）或 v14（工作流）；两端需使用支持该版本的 CLI。

1. 将 `challenge` 原样交给原对端的 `recovery-proof`，取得该时点两条来源流的连续位置、回执位置和有效记录摘要。
2. 从原对端调用 `recovery-page`，输入 `{"challenge":…,"origin":"<来源设备 UUID>","after":0}`。对两个来源分别按返回的 `through` 取下一页，直至没有 `batch`。将每个批次交给恢复端的 `receive`，沿用原标识去重。遇到缺失前序时先补另一条流，再重试该页。
3. owner 确认恢复原配对后，各次 `reconcile` 都传 `{"proof":…,"confirmation":"confirmed"}`。先在恢复端提交第 1 步的证明，再把返回的证明交给原对端，最后把原对端返回的完成证明交回恢复端。两端逐次核对任务、方向、世代、连续位置、撤回状态和来源依赖。恢复端在最后一次事务提交后开放读取和发送。确认响应丢失可以重交相同证明。
4. 新世代从位置零开始，按原方式立即刷新。对端在核对期间写入导致证明失效时，重新从第 1 步开始。若一端独有的记录无法在对端核实，或缺失前序无法补齐，保留隔离副本与待核对状态。
