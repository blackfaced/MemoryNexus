# ADR-028: MiniMax 与 Codex 共用的本地任务记忆

## 状态

已接受的实施方向，2026-10-21；实现与另一台机器验收尚未完成。
用户授权先记录 ADR、发布可执行 issue，并准备远端机器接入说明。
执行真源：[总规格 #296](https://github.com/blackfaced/MemoryNexus/issues/296)，
按 #297 → #298 → #299 → #300 → #301 推进；实现前先合入本决策和合同。

## 背景

owner 在另一台机器上按目录管理拍视频、学习陪伴和游戏任务，主要使用 MiniMax，也使用
Codex，并已配置定时 review。当前问题是执行结果与 review 经验不能可靠地被下一次会话或
另一个客户端使用。第一项产品验收应是两种客户端在同一任务中双向交接经验。

讨论最初基于旧 worktree，提出 HTTP/Surface 优先。核对远端 main `c9b4373` 后确认
ADR-027 已冻结旧服务路线，且 `memorynexus-feedback-cli` 已有 SQLite、确认、幂等、
review 和 recovery 代码。因此保留稳定 JSON 与客户端无关的目标，首版采用本地 CLI，
不重新启用 Axum/Surface/PostgreSQL。无需为共用一台机器上的数据先运行服务。

## 决策

### 第一交付是目录任务的跨客户端连续性

在同一个 Rust crate、同一个 `memorynexus-feedback-cli` 可执行文件中增加 `task`
用例组。MiniMax、Codex、已有 review 调度器通过相同 JSON 请求调用它。现有个人实验
命令继续使用其原有 ledger 和确认规则。任务功能的精确合同在
[task-memory-contract.md](../docs/task-memory-contract.md)，安装与验收步骤在
[task-memory-onboarding.md](../docs/task-memory-onboarding.md)。

每个任务目录保存 `.memorynexus.json`，绑定稳定的随机 `task_id`。同一 owner 的任务使用
一个 SQLite task ledger；其路径由环境变量提供。目录改名、客户端切换与新会话不改变任务
身份。复制配置表示继续同一任务；新任务必须重新初始化。任务查询始终显式限定 task ID。

首版是同一 OS owner、同一机器的本地信任模型。任务 ID 是数据分区，不是权限边界或隔离
不可信进程的机制。机器指任务所在机器，不是当前规划会话所在机器。此切片不恢复
CognitiveSpace、Namespace、账户、子女权限体系或多机器同步；学习陪伴仅记录 owner 的
工作安排和已确认的任务结果，不扩张为儿童画像或学情系统。

### 任务记录与 review 结果有不同的证据地位

- `checkpoint`：某次执行报告的进度；不能由最后写入时间推断任务必已完成。
- `execution`：带来源的执行报告；表述为 Agent 报告，不能冒充 owner 确认的事实。
- `instruction`：owner 明确确认的目标、约束、偏好或决策。
- `lesson`：review 产生、有当前来源记录支撑的候选经验。

初始化时 owner 可以按任务授权自动保存 checkpoint/execution 和 review candidate。
这种持续授权只覆盖受限任务记录，不升级为医疗观察、私人聊天或四对象 ledger 的写入许可。
instruction、纠正、撤回仍需针对该次内容的确认。自动 review 只产生候选；系统不把模型
置信度当作确认，也不因候选内容自动执行下一项任务。输入与错误输出均使用严格、受限字段。

上下文读取返回当前 instruction、checkpoint、执行报告、单列的候选经验和来源 ID。
结果有大小上限，并明确报告省略数量。读取本身不制造新证据。纠正和撤回使依赖它的候选
失效，避免 review 不断引用自己的推测并把它放大为事实。

### 复用本地运行条件，限制后台职责

既有 MiniMax/Codex 执行器负责模型调用、任务执行、review 生成和定时唤起。MemoryNexus
负责作用域、来源、当前版本、幂等、纠正、撤回、读取和恢复。无需增加 scheduler、daemon、
provider registry、通用 connector 或后台 retry worker。

启动 hook 可调用 context；无法提供 hook 时使用通用启动/结束步骤。只靠提示词时必须标为
best effort，不宣称强制自动加载。任务产物、完整聊天、原始媒体和工具输出仍由原目录管理；
ledger 只保存受限文字和本地证据引用。已有定时 review 可以提交候选，无需新建调度器或
依赖主动消息交付。

写入失败保留同一个请求文件与幂等键，显式重试；不报告假成功。读取失败时任务可以在明确
“未加载历史”的状态下继续，但不能猜测旧状态或偷偷新建 ledger。

### SQLite、CLI 与将来的传输

任务 ledger 使用 SQLite WAL，与现有 personal experiment ledger 分开，防止改变正在
使用的健康/实验记录。两者均只有一个权威写入位置，不双写、不建立第二数据库后端或
repository abstraction。任务备份/恢复覆盖记录、任务身份、幂等 receipt 和失效关系。

稳定、版本化 JSON 是共享合同；CLI 是 v1 唯一必需传输。HTTP/MCP 可在确有跨进程环境
限制时另开 issue，调用同一用例合同。v1 不增加 HTTP server，不让旧 `memorynexus-cli`
或 legacy MCP 成为依赖，也不声称 MiniMax 与 Codex 有相同的 hook/插件配置。

## 与 ADR-027 和既有 issue 的关系

本 ADR **只为任务记忆切片**重开以下限制：仅四类领域对象、仅 MiniMax、每次操作记录都逐条
确认、第二使用场景必须等待 #283。用户本次已明确选择 MiniMax + Codex 与真实目录任务。

保留 ADR-027 的 Rust、SQLite、本地 CLI、单 owner、无额外服务、无旧 runtime 扩张原则。
#273/#282 仍描述个人实验与其安装；#283 的十四天窗口、计数、通过条件和 owner 决定均不
改变。新的双客户端 gate 不能证明 #283 通过，也不能提前授权 #284–#286 的切换或删除。
任务切片不恢复冻结的 M9/M10/M11 roadmap。

## 验收与实施顺序

1. 任务初始化、限定读写、幂等和独立进程读回。
2. 外部 review 候选、owner 纠正/撤回及依赖失效。
3. WAL 一致备份、恢复、导出及丢失响应后的重试。
4. 共用 Agent 操作说明、可验证的 CLI 接入与既有定时 review 接线。
5. 另一台机器上的真实 MiniMax 写入→新 Codex 会话读取，以及反向路径。

第 5 步需要实际客户端证据；仅把请求中的 `client` 改成两个名字不构成验收。还需证明任务
不串读、同一请求不重复、纠正后不再给出旧结论、备份恢复保留以上行为。第一轮可用无敏感
内容的任务样本。机器不可达时保持 gate 待验收，不能用本机模拟结果替代。

## 取舍

相比 HTTP-first，CLI-first 复用现有代码、减少部署步骤，但暂不支持没有本地执行能力的
客户端或远程网络访问。相比直接共享 Markdown，ledger 提供事务、幂等与来源失效语义，
代价是要维护明确的命令合同和恢复路径。确定性上下文选择先验证实际复用价值；语义搜索、
embedding、自动跨任务归纳和第三方 memory backend 均留待具体使用证据支持。

## 参考

- [ADR-027](ADR-027-sqlite-cli-minimax-feedback-kernel.md)
- [Current roadmap](../docs/TODO.md)
- [Task memory contract](../docs/task-memory-contract.md)
- [Other-machine onboarding](../docs/task-memory-onboarding.md)
