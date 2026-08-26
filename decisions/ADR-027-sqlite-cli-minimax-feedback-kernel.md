# ADR-027: SQLite CLI MiniMax Personal Experiment Feedback Kernel

## 状态

✅ 已接受，实施以 [#274](https://github.com/blackfaced/MemoryNexus/issues/274)
通过为前置条件。

## 背景

此前的默认路线将 MemoryNexus 定位为 Rust + Axum 的长期反馈 Engine：它拥有
`CognitiveSpace`、Namespace、Surface Gateway、Trace、SleepCycle、Adapter、PostgreSQL
和 Qdrant 等概念与运行时。这条路线保存了确认、来源、幂等、结果和 owner stop
等有价值的行为，但对当前唯一的私人自用场景过重：在 Mac mini 上持续使用需要运行多个
服务，Agent 必须了解很多内部名词，独立的定时会话也不能依赖上一段聊天历史。

当前用户已经通过运行于 Mac mini 的 MiniMax Agent 便利地使用微信输入，并可在 MiniMax
App 中查看会话。用户需要的不是另一个健康分析或 Agent memory 产品，而是一份可靠地回答
以下问题的、自己确认过的长期记录：选了什么行动、是否实际尝试、后来怎样、是否值得保留。

因此项目的默认产品方向重置为 local-first personal experiment feedback kernel。睡眠与
精力是首个十四天 dogfood 数据集，不是医疗产品或永久品类。

## 决策

### 产品差异

MemoryNexus 不再以“依据长期 Trace 自动生成成长模型和下一步行动”的通用 Engine 作为
默认产品，也不与 memory cloud、agent recall、local agent runtime 或健康顾问竞争。

它的默认价值是：在用户确认、外部建议和真实执行之间，维护可复盘的本地实验历史。MiniMax
负责自然语言理解和交互；外部工具（例如蚂蚁阿福）负责专业分析与候选建议；MemoryNexus
只保留用户确认后、足以解释选择与结果的最小事实。

### 四对象权威内核

第一版只有下列四类权威对象：

| 对象 | 权威内容 | 关键约束 |
| --- | --- | --- |
| `Observation` | 用户确认的、受限长度的事实或主观感受、发生时间、来源和确认时间 | 草稿、未确认抽取和原始聊天不进入 ledger；订正和撤回保留最小审计来源。 |
| `Recommendation` | 受限建议摘要、来源、时间，以及可选的 Observation 关联 | 来源明确区分用户、外部顾问和标记为 Agent 生成的候选；原始蚂蚁阿福对话默认临时处理。 |
| `Experiment` | 从一个 Recommendation 选择的可逆行动、起止边界、预期可观察信号和状态 | 固定 owner；第一版最多一个 active Experiment。 |
| `Outcome` | 对正确 Experiment 的执行状态和已确认结果 | 执行状态为 performed、skipped 或 not-evaluable；结果为 improved、unchanged、worse 或 unclear；订正不静默改写历史。 |

Review 只从已确认的四对象重建 Observation、建议来源、选定行动、实际执行、结果与证据缺口。
它不填补缺口，也不把生成内容当成事实。

### SQLite ledger 与 CLI seam

SQLite（WAL mode）单文件是第一版唯一权威数据库。它支持确定性迁移、JSON export、
一致备份与 restore；不新增 database abstraction、generic repository、双写或第二数据库。

编译后的本地 CLI 是**已接受的目标**主要行为 seam。完成 #274 和后续 #276–#281 后，它必须
提供稳定 JSON 输入/输出的用例级命令：`observe`、`retract`、`add-recommendation`、
`start-experiment`、`record-outcome`、`review` 和 `due`。其中 `retract` 是
Observation lifecycle 的独立用例，保留最小审计来源而不静默改写历史。

当前可运行的仍是冻结 legacy runtime；它尚未实现上述 SQLite CLI 契约。本 ADR 不把目标
命令误述为现有能力。目标 CLI 不公开通用 CRUD、表结构、任意 JSON dispatch、Surface 名称或
旧 Engine 对象。测试应跨独立 CLI 进程验证外部可见行为，而不是绑定私有 SQL 布局。

### MiniMax、微信与提醒

MiniMax Skill 将自然语言映射为上述显式 CLI 用例，并在每次权威写入前展示受限摘要、取得
明确确认。微信是首选的方便输入路径；MiniMax App 是首版提醒和独立 session 可见性的入口。

MiniMax 原生定时任务拥有 wake-up 和 delivery：独立 session 调用 `due`，从同一 SQLite
ledger 读取状态，并先在 MiniMax App 显示一条简短、上下文相关的问题。MemoryNexus 不添加
scheduler、daemon、retry worker、微信机器人或 channel framework；微信主动推送不是首版
门槛。

这一前提尚未被本仓验证。#274 必须证明普通微信会话可执行 owner-approved 本地命令并写入
共享测试状态，且独立原生定时 session 能读到该状态、结果能在 App 中看到。若任一事实不成立，
实施必须暂停并按观察到的能力改写本 ADR 与后续 ticket；不得以聊天历史代替共享权威状态。

### 健康与外部建议边界

MemoryNexus 不诊断、解读医疗报告、开具治疗建议、判断紧急情况，也不声称临床有效性。
蚂蚁阿福等外部工具只是 Recommendation 的可标记来源；完整对话、诊断、处方和医疗文档默认
不持久化。Agent 可以在用户请求时提出低风险、可逆候选，但来源必须可见且不能伪装为医疗权威。

### 兼容性与 recall projection

历史 PostgreSQL、Qdrant、embedding、vector search、Axum REST、MCP、Surface Gateway、
Sleep/Dreaming、Dictation、Thought Review 和 source Adapter runtime 不是新默认产品的一部分。
不承诺历史数据迁移、兼容 schema 或兼容 API；Git history 是被退役实现的来源。

未来第三方 memory backend 只能是从 SQLite ledger 重建的、非权威 recall projection。它不能
拥有确认、来源、Experiment 或 Outcome。只有在两个真实实现和一致性测试证明需要后，才可抽取
recall seam；不得预先加入 generic provider registry 或 backend abstraction。

### Expand–contract 与十四天删除 gate

迁移在一个明确的 expand–contract 期间进行，不双写：

1. #274 先验证 MiniMax 跨 session 本地命令与共享状态。
2. #275 记录本决策、更新公开定位，并冻结冲突的 legacy roadmap。
3. #276–#281 依次交付 SQLite Observation lifecycle（含 `retract`）、Experiment、Outcome/
   review、MiniMax Skill、`due` 和 export/backup/restore。
4. #282 在干净 Mac mini 上安装完整路径；#283 以固定十四个日历日验证它。
5. 只有 #283 以通过结论关闭后，#284 才将 SQLite/CLI/MiniMax 设为唯一默认 build、release、CI
   与文档路径；#285 与 #286 才删除旧接口和旧 storage runtime。

十四天 gate 开始前固定标准：至少十条有效 confirmed Observation；至少一个 Experiment；至少
五条 performed、skipped 或 not-evaluable 的执行更新；至少一次有证据的结果 review；fresh
MiniMax session 读到同一状态；系统故障最多一次人工干预；第十五天 owner 记录是否愿意在没有
项目测试义务时继续使用。gate 失败时先分析价值或摩擦，不继续扩张架构；成功前不删除旧运行时。

## 后果

正面：

- 默认安装不依赖 PostgreSQL、Qdrant、Docker、Axum 服务、MCP 或第三方 memory backend。
- 权威状态与 Agent session、模型记忆和外部顾问解耦，低频独立进程可稳定恢复。
- 四对象与一个 CLI seam 将实现、验证和故障定位限制在真实使用需求内。

负面：

- 旧产品、公开文档与 tracker 需要显式冻结或重写，不能让两条 ready roadmap 并存。
- 旧 runtime 中有价值的行为需要通过 CLI 合同移植，而不是通过兼容层保留。
- #274 是外部产品行为的人工验证，未通过时必须重新规划而非继续实现。

## 被取代的活动方向

ADR-014 至 ADR-026 及更早 ADR 保留为历史决策和退役 runtime 的解释材料；它们不再定义默认
产品或新的实现票。它们的有效行为经验（明确确认、来源、幂等、结果、证据缺口和 owner stop）
可在四对象 CLI 合同中重新验证。本 ADR 只取代其**活动默认方向**，不删除历史记录。

## 相关决策与规格

- [#273: parent specification](https://github.com/blackfaced/MemoryNexus/issues/273)
- [#274: MiniMax feasibility gate](https://github.com/blackfaced/MemoryNexus/issues/274)
- [ADR index](README.md)
- [Current roadmap](../docs/TODO.md)
