# 在另一台机器接入共享任务记忆

目标：同一个任务目录里，MiniMax 写入，新的 Codex 会话读回并使用；反向也成立。
依据 [ADR-028](../decisions/ADR-028-shared-task-memory.md) 和
[v1 命令合同](task-memory-contract.md)。

**当前状态：源码已实现九个 task 命令、共用协议与模拟预检；另一台机器的真实验收
尚未运行。** 本文面向包含 #297–#300 的源码版本，旧 baseline `c9b4373` 不含 task 功能。
实施顺序见 [TODO](TODO.md)，待办见 [#301 验收记录](acceptance/task-memory-cross-client.md)。
这份说明可以直接交给任务机器上的 Agent；机器访问暂不可用不等于验收通过。

## 1. 先做只读盘点

记录操作系统/架构、一个无敏感内容的任务目录、MiniMax 的实际执行器、Codex 的执行位置，
以及现有定时 review 如何启动。确认两种执行器都能运行本地程序、传入 stdin、读取 JSON
stdout，并能访问同一个本地目录。模型名称或客户端品牌本身不能证明这些能力。

若一个执行器位于容器、沙盒或远端主机，先证明它看到的是同一份本机 ledger。不能满足时
记录阻碍并停止实际接入；不要各建一个同名数据库，或假装已通过。HTTP/MCP 网络桥接是
独立后续工作，不是这份 v1 安装说明的隐含步骤。

完成条件：记录两边的真实本地命令执行能力、任务路径和 review 入口；不读取私人聊天，
不修改原来的调度频率，不动 existing personal-experiment ledger。

## 2. 从固定源码版本安装同一版 CLI

当前 release 工作流和 one-click 包**不包含** `memorynexus-feedback-cli`。从包含任务实现的
明确 commit 做干净源码 checkout，记录该 commit；不要用旧 release binary 猜测能力。
下面命令在该 checkout 运行。若工作区有变更，先保留并处理它们，不把未知源码当作安装版本。

```sh
git rev-parse HEAD
git status --short
rustc --version
cargo --version
uname -sm
cargo build --release --locked --bin memorynexus-feedback-cli --target-dir target/task-memory
```

构建成功后安装到单独的、带 commit 的 owner 目录。先确认该目的版本路径没有已有 binary，
不要覆盖已验证版本；父目录必须由 owner 控制。示例只安装当前机器本地构建的 binary：

```sh
umask 077
TASK_BUILD_COMMIT=$(git rev-parse HEAD)
TASK_INSTALL_DIR="$HOME/.local/lib/memorynexus-task/$TASK_BUILD_COMMIT"
mkdir -p "$TASK_INSTALL_DIR"
export MEMORYNEXUS_BIN="$TASK_INSTALL_DIR/memorynexus-feedback-cli"
test ! -e "$MEMORYNEXUS_BIN" || exit 1
install -m 700 target/task-memory/release/memorynexus-feedback-cli "$MEMORYNEXUS_BIN"
python3 -c 'import hashlib, os; print(hashlib.sha256(open(os.environ["MEMORYNEXUS_BIN"], "rb").read()).hexdigest())'
"$MEMORYNEXUS_BIN" task --help
python3 scripts/task-memory-smoke.py --binary "$MEMORYNEXUS_BIN"
```

记录 commit、工具链、OS/CPU、SHA-256 和 help JSON。CLI **没有 `--version` 命令**；Cargo
包版本不足以标识任务能力。预检只用临时合成数据、每次新 CLI 进程，不访问真实任务库、
不调用 MiniMax/Codex 模型、不安装调度。stdout 是一个 JSON；成功退出 0，失败非 0。
报告必须同时有 `mode: "simulated_cli"`、`status: "passed"` 和
`real_client_acceptance: "not_run"`；失败时先处理报告中的检查项。合成通过后才进入真实接入。

把该二进制放到两种执行器都能访问的位置。新安装路径不要覆盖仍用于个人实验的已验证
二进制，直到既有命令的回归检查通过。通过 `"$MEMORYNEXUS_BIN" task --help`
核对 schema version 1 和 init/record/context/review/correct/retract/backup/restore/export
九条必需能力；合法的额外命令（例如 repository-memory 的命令）不影响预检。
缺少任何所需命令或响应格式非法就停止，不回退到 legacy REST/MCP 文档。

若第 0 步曾误报 `missing_commands`，保留现有 worktree、失败 JSON 和 binary 摘要，
从包含 #333 修复的固定 commit 运行上述 smoke，并记录 smoke 脚本的 commit/SHA-256
及实际 CLI 的源码 commit/SHA-256。Python-only 修复可以复用原已编译 CLI，但必须实测
并记录这个版本组合；不要把新脚本标作旧 commit，也不要静默覆盖旧安装。
五组检查全通过后，由原试点操作会话按原授权继续同一预留 config/ledger 的初始化，
并读回 instruction。已有状态不得删除或覆盖；本修复不授权写入真实试点 ledger。

task 功能运行时无需启动 PostgreSQL、Qdrant、Docker 或 MemoryNexus daemon。
源码构建仍受当前 Cargo 依赖影响；这里不承诺安装耗时或所有平台都有预编译制品。

## 3. 绑定一个试点目录

已有目录第一次接入且希望整理当前进度时，先用
[初始化 Skill](../integrations/task-memory/memorynexus-task-bootstrap/SKILL.md)
生成完整 Markdown/JSON 导入草稿。复制完整 Skill 目录（含 scripts/references），并保留
共用协议的可读路径；不假设 MiniMax/Codex 的全局安装位置。草稿阶段不需要 binary/ledger，
也不创建配置。owner 确认精确草稿、绑定和 policy 后再 apply；它复用下列 CLI 初始化流程，
因此不要提前执行本节的手动 init。已存在配置的任务不会自动重新导入。状态目录必须在任务
仓库外且 owner-only；批准的 ledger 父目录需明确创建并核对权限。详见 Skill 的 manifest
说明。这一步不能替代 #301 的真实客户端及定时 review 验收。

两个执行器及原来的 review 任务必须显式得到同一环境变量。例如在任务机器上：

```sh
umask 077
mkdir -p "$HOME/.local/share/memorynexus"
export MEMORYNEXUS_TASK_LEDGER="$HOME/.local/share/memorynexus/task-memory.sqlite"
export TASK_CONFIG="/absolute/path/to/pilot/.memorynexus.json"
```

不要将此路径设为现有健康/个人实验 ledger，也不要依赖定时任务继承交互式 shell 的环境。
选择试点目录，令 `TASK_CONFIG` 指向该目录的 `.memorynexus.json` **绝对路径**。
每次调用都显式传该路径。配置只含稳定 task ID、schema version 和环境变量名称。

先验证已有 ledger 父目录是 owner-only，试点目录已经存在；替换上面的示例绝对路径。
按[共用协议的 pending 步骤](../integrations/task-memory/PROTOCOL.md#durable-pending-request-lifecycle)
创建 owner-only 请求文件，令 `PENDING_REQUEST` 指向它。以下每份写入 JSON 都先独立暂存，
保留 action/config/ledger 绑定，再调用命令；响应校验成功后才清理，不用 shell 拼接正文。

由 owner 确认该任务允许自动保留哪些内容，再准备 stdin JSON（本例幂等键仅用于临时试点；
正式任务生成自己的 key，重试保留原值）：

```json
{
  "idempotency_key": "pilot-init-2026-10-21",
  "display_name": "共享任务记忆试点",
  "confirmation": "confirmed",
  "automatic_capture": ["execution", "review"]
}
```

```sh
"$MEMORYNEXUS_BIN" task --config "$TASK_CONFIG" init < "$PENDING_REQUEST"
```

检查返回 task ID 与生成配置一致。重复执行原请求应恢复同一绑定；不能擦除已有数据。
另一个无关任务应生成不同 task ID。同一目录搬家保留配置；新任务不复制旧 task ID。

## 4. 安装同一份操作协议

使用[共用协议](../integrations/task-memory/PROTOCOL.md)、
[MiniMax 入口](../integrations/minimax/memorynexus-task-memory/SKILL.md)和
[Codex 入口](../integrations/codex/task-memory.md)。MiniMax 按实际执行器支持的 Skill/指令
入口引用协议；Codex 在确认运行时会读取任务 AGENTS.md 后追加同一协议引用。
不要覆盖已有配置；协议放在两边都能读取的绝对路径，或明确版本的本地副本中。

协议要求：

1. 开工先调用 `context`；依据返回的 task ID、revision 和来源说明已加载什么。
2. 执行结束，用 `record` 写入受限 checkpoint/execution；明确的 owner 要求以经确认的
   instruction 写入。Agent 自己推测的成功不得冒充 owner 确认。
3. 既有定时 review 先读取 context，再提交带 `source_ids` 的 `review` 候选。
4. 每次写前把 JSON 暂存为 owner-only 请求文件；参数中不拼接自由文本。失败保留同一
   文件和幂等键，accepted/replayed 后清理；pending 文件不提交 Git。
5. 用户纠正或撤回时使用对应命令，下一次读取检查旧结论与依赖候选是否已退出当前上下文。

支持启动 hook 的执行器可在 hook 中调用 context；支持包装启动的使用显式前置步骤。
只有提示词约定的标记为 **best effort**。定时 review 内必须显式读取配置和 ledger 环境，
不能依赖 cwd、上一会话或模型的隐藏记忆。实际客户端配置格式要在该机器核对，不照抄
另一品牌的配置，也不宣称计划任务结果会主动推送手机。

## 5. 双向交接验收（真实客户端，当前 NOT RUN）

先用如下无敏感内容样本，所有文本均为演示任务事实：

```json
{
  "idempotency_key": "pilot-checkpoint-1",
  "run_id": "50b928e6-b195-4098-9e14-53fc9ab76916",
  "client": "minimax",
  "kind": "checkpoint",
  "text": "演示任务：已完成三段素材的编号，下次从选择开场片段继续。",
  "occurred_at": "2026-10-21T10:00:00+08:00"
}
```

替换为实际时间与一次生成的 run ID，将 JSON 独立暂存在新的 owner-only 请求文件中，
更新 `PENDING_REQUEST`。让 **实际 MiniMax 会话**
调用以下命令，再在 **全新 Codex 会话**读取。不能只改 JSON 里的 client 标签。

```sh
"$MEMORYNEXUS_BIN" task --config "$TASK_CONFIG" record < "$PENDING_REQUEST"
"$MEMORYNEXUS_BIN" task --config "$TASK_CONFIG" context < /dev/null
```

| 验收 | 必须保留的证据 |
| --- | --- |
| MiniMax → Codex | 实际写入 receipt，fresh Codex 的同 task ID/context revision/记录 ID，以及依据该记录继续任务的回答；不复制旧聊天。 |
| Codex → MiniMax | Codex 写入新的执行报告，fresh MiniMax 读取到对应 ID 并使用。 |
| 定时 review | 由已有 review 入口执行一次，记录其读到的 task ID、来源 ID 和新候选 ID；另一个客户端看到候选标记。手动模拟只算预检。 |
| 隔离与限额 | 任务 B 看不到任务 A 内容；context 返回省略数量；候选不显示为已确认指令。 |
| 丢失响应与重试 | 原请求第二次调用为 replay，同 key 改文案为 conflict，没有重复事实。 |
| 纠正与撤回 | owner 确认后订正来源，旧内容及依赖候选不再用于回答；撤回后当前查询/新导出没有原文字。 |
| 故障 | 隐藏 env 或临时指向不存在 ledger，读取失败明确呈现；不自动创建替代 ledger，不报告“没有历史”。 |
| 恢复 | 一致备份恢复到新文件，切换两边的同一 env 后读回相同身份/状态；重放备份前请求仍不重复。 |

## 6. 备份、故障与停用

包含 #299 的源码版本提供以下恢复命令；安装和真实客户端验收仍按前面的步骤单独执行。

```sh
"$MEMORYNEXUS_BIN" task --ledger "$MEMORYNEXUS_TASK_LEDGER" export < /dev/null > task-export.json
"$MEMORYNEXUS_BIN" task --ledger "$MEMORYNEXUS_TASK_LEDGER" backup < backup-request.json
"$MEMORYNEXUS_BIN" task --ledger "$NEW_TASK_LEDGER" restore < restore-request.json
```

backup/restore 请求都是 `{ "path": "<absolute-path>" }`。backup 的 path 是新目的文件；
restore 的 path 是源 SQLite 备份，`NEW_TASK_LEDGER` 必须是不存在的新文件路径。目的父目录
必须已经存在且可写。已有文件、目录、符号链接，以及目的路径旁残留的 `-wal`、`-shm` 或
`-journal` 都会被拒绝；不要为了重试自动删除它们。恢复不覆盖当前 ledger，不复制任务配置。
保留 `.memorynexus.json`，验证恢复后再一致地更改两种执行器的同一 ledger 环境变量。

backup 使用 SQLite `VACUUM INTO` 生成包含已提交 WAL 内容的一致快照。restore 重新生成
完整快照，检查 task 格式、v1 schema、SQLite integrity 与外键，再以不覆盖方式发布新文件。
任务 ID、自动捕获策略、记录顺序、修订/来源关系、失效状态、撤回 tombstone 和 receipt
均保留。恢复后先比对 context 的 task ID、revision、记录 ID，再用原始请求验证 replay；
同 key 改内容仍应 conflict。备份之后的新写入不会出现在较早的快照中。

### 导出格式 v1

export 接受空 stdin 或 `{}`，输出一个 `schema_version: 1`、`status: "ok"` 的 JSON。
`result.format` 为 `memorynexus-task-ledger`，`result.version` 为 `1`。导出包含同一读取
快照中的全部任务，字段如下：

| 字段 | 含义 |
| --- | --- |
| `tasks` | 稳定 ID、display_name、automatic_capture 策略与各任务 revision。 |
| `records` | 按 commit_sequence 排序的记录、来源 client/run/time、kind/trust/state/revision；未撤回的文字及 evidence_refs。撤回记录省略 text 和 evidence_refs。 |
| `record_sources` | record_id、source_id 和来源顺序 position；包含失效经验的来源关系。 |
| `record_revisions` | record_id 与其 supersedes_id；包括已撤回修订链。 |
| `receipts` | task_id、idempotency_key、action、SHA-256 digest、record_id 和原接受 revision。 |
| `init_receipts` | 初始化 key、digest、task_id 和原接受 revision。 |
| `commit_sequence` | 已分配的全局记录顺序上界；恢复后的新记录继续递增。 |

外层 `revision` 是此快照中各任务 revision 的总和，用于标识整份 ledger 的状态；任务
context 仍使用各自的 revision。备份/恢复成功响应的 revision 取自所生成快照，result
返回目的 path、format 和 version。导出保留历史状态，不能把 stale/superseded/historical
记录当作当前上下文。它不含原始请求、撤回原因副本或 personal-experiment 数据。
JSON export 仅用于检查，**v1 不支持 JSON import**；恢复必须使用 SQLite 备份。

### 失败与保留策略

读失败：显式说明未加载历史，必要时仅根据当前指令继续。写失败：保留原请求文件，报告未
同步状态。权限、格式或 schema 错误停止写入；不删除库、不切换到实验库、不创建新库
掩盖故障。目的父目录缺失或不可写会返回 storage_unavailable；已有目的返回 conflict。
失败重试前检查目的路径：若进程在完成发布后、返回成功前中断，那里可能已有完整快照；
先用 task export/context 检查，再选择另一个新路径，不能覆盖或假定上次失败意味着文件为空。
未完成发布的随机临时文件不构成可用备份；正常错误会清理本次临时文件，崩溃遗留交由 owner
检查。SQLite 一致备份必须通过命令完成，不只复制正在使用的主文件。

撤回清除 live ledger 修订链的文字和证据引用，后续导出及备份保留最小 tombstone。
**历史备份可能仍含后来撤回的内容**，owner 自行管理保留或删除；这里不承诺 SQLite/WAL、
文件系统快照或物理磁盘的安全擦除。导出和备份同样含 owner 数据，不提交到任务 Git 仓库。

停用时移除该目录的接入指令/启动 hook 和 review 提交步骤，保留 ledger 与备份供 owner
处理；原任务与原定时 review 可以独立继续。只有确认不再需要后才删除本地请求或数据。

## 交付报告

将逐项结果填写到[真实验收记录](acceptance/task-memory-cross-client.md)，并向
[#301](https://github.com/blackfaced/MemoryNexus/issues/301) 提交脱敏证据。当前各项都为
NOT RUN；#301 和父票 #296 保持打开。#283 十四天个人实验 gate 独立且标准不变。

报告版本/commit、OS/CPU、两种实际执行器及连接方式、脱敏后的 task ID 与 ledger 路径、
逐项通过/失败/未验证、对应 request/record IDs、启动是否强制或 best effort、pending
请求状态，以及备份恢复结果。机器不可访问或客户端不可执行时，真实 gate 保持未通过；
不以本地单元测试、品牌标签或这份文档的存在替代验收。

## 已有任务的日常接入（#343）

一个短意图加绝对 binding 路径即可交接，例如“接上任务，入口 `/absolute/private/binding.json`”。
使用 [canonical Skill](../integrations/task-memory/memorynexus-task-memory/SKILL.md)；
精确 `verify/read/stage/apply`、完整 envelope、pending/receipt 与失败语义以
[冻结接口](../integrations/task-memory/memorynexus-task-memory/references/daily-interface.md) 为准。
helper 不代填确认、不 bootstrap/migrate，不要求 repository profile/consumer/publication。

部署仍使用固定 source 的 install.py，scripts 会进入 `VERSION/package/`；Skill 及所有
相对文档保留包内原目录。日常入口为 `PYTHON -I -S -B VERSION/package/daily.py`，
不是 `launch.py daily`，也不是不存在的 Rust `task read/check`。安装已有 workflow
包时沿用其 rule-file 参数；该参数不要求为日常任务创建 workflow binding。
安装前核验资源清单含 canonical Skill、daily.py、共用模块、协议及 LICENSE。

handoff 应分别记录 script source 的完整 SHA、canonical 路径、包内文件清单/摘要、
CLI build source SHA 与 binary SHA、Python 路径/版本、兼容 CLI schema 1/ledger v1/v2。
脚本升级可复用经过能力检查和 public-process 测试的旧 binary；Rust 改动则重建。
不要把脚本 source SHA 当成旧 binary 的构建来源，不要把合入等同于本机更新。
新版本先安装到新目录，核对新 manifest 后由 owner 切换绑定；旧 pending 继续使用原
binding/脚本/binary，不改 pin 来让它通过。

普通 context 包含单列候选正文；v2 current adapter 的候选只有索引，且另做已知复制件
检查。不要删除 external copies 声明、静默降级或把候选当成已用于当前事实的正文。

客户端证据分开：本机 CLI 真实进程测试是协议验证；Codex/Traex 的实际执行能力、
原 Codex Desktop 下一轮显式重读、AIWB #155 的独立验收否决→修复→重验及协调成本
需分别记录。prompt-only 不承诺强制加载，fresh bridge 不承诺旧聊天历史合并。
本票不自动更改全局客户端、私人 ledger 或任何已有调度。
