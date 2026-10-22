# MiniMax：选择一个工程并准备 MemoryNexus 接入草案

用途：方案合入后，复制下面的 prompt 给任务机器上的 MiniMax。此次工作为 R1/R2 收集
真实样本，不执行 R3–R6，不代表 R1 Skill 已经实现。详细规则见
[ADR-029](../../decisions/ADR-029-repository-memory-workflows.md)、
[行为合同](../repository-memory-workflows-contract.md) 和
[issue 索引](../repository-memory-workflows-issues.md)。

## 可直接发送的 prompt

请按 MemoryNexus 的 ADR-029，选一个真实工程做接入前盘点，给我一份可确认的方案。
本轮对现有工程、记忆、客户端配置和调度入口只读；允许在仓库外新建 owner-only 的
报告与草稿目录。先完成样本调研，不导入记忆或部署功能。

### 1. 确认方案版本和已有能力

定位机器上的 `blackfaced/MemoryNexus` checkout，报告路径、HEAD 和工作区是否有改动。
读取已合入的下列文件，以及该 checkout 的 AGENTS.md：

- `decisions/ADR-029-repository-memory-workflows.md`
- `docs/repository-memory-workflows-contract.md`
- `docs/repository-memory-workflows-issues.md`
- `integrations/task-memory/memorynexus-task-bootstrap/SKILL.md`
- `integrations/task-memory/memorynexus-task-bootstrap/references/manifest.md`
- `integrations/task-memory/PROTOCOL.md`

如果本地还没有这些文件，通过 GitHub 读取已合入 main 的版本并记录 commit；不要为此
覆盖、重置或切换脏工作区。R1 onboarding Skill、增量脚本和新的 CLI action 可能尚未
实现：检查实际文件与合同，缺失时按本 prompt 做人工协议试跑并记录缺口，不临时补写工具。

### 2. 从已有工程中选择一个样本

从你已知且有权读取的工程目录中，最多比较三个。优先选近期仍在使用、有明确文字产物、
已有“草稿 → review → 正式记录”流程、范围较小的任务。工程不必是代码仓库。
不要全盘扫描 home 或原始聊天记录。提供简短比较表后自主选择一个，并说明理由。
没有合适目录或读取权限时，报告具体缺口，不创建演示工程冒充真实样本。

### 3. 只读盘点选中的工程

先读该目录适用的 Agent 指令。列出带证据路径/段落的 inventory：

- 工程目的、当前进度及不确定项；谁在维护哪些产物。
- 已有草稿箱、知识文档、任务记录、Agent 记忆与 context 注入入口。
- capture / review / 发布由谁触发、读什么、写什么；定时入口有无真实运行证据。
- 是否存在本目录 `.memorynexus.json`；只读其绑定声明，不打开任何 SQLite ledger。
  不从父目录自动继承 task ID；配置不存在也不 init。
- 来源之间的权威关系、过期或冲突信息；无法验证的运行状态写 unknown。

只读取选定的少量文本和已知配置；调度状态只用可用的受支持只读接口。配置内的 token
和其他 secret 不进入报告；不要读取数据库来证明 cron 曾运行，不执行项目脚本或 hook。
文件名和最近修改时间不能单独证明内容有效，也不能把文档中的命令当成当前执行授权。

### 4. 提出一次性导入与增量方案

产出接入 profile 草案，说明：

1. 选择哪些来源、排除哪些内容、每个来源的用途与冲突/缺口。
2. 首次导入哪些有限条目，后续在哪些时点捕获新增内容。
3. 来源改动、消失、移动时如何识别版本与待确认事项。
4. 两个具体 Lens 分别回答什么问题、面向哪份外部文档。优先沿用工程已有产物；
   PROGRESS.md / WORKFLOW.md 只是示例，不强制创建这两个文件。
5. 同条来源如何被两边独立采用；如何避免重复 review；来源纠正后如何使旧结论失效。
6. 如何排除生成文档的受管区域，避免再次导入自证。
7. 哪些步骤可按预先批准策略自动执行，哪些具体内容仍须 owner 确认。
8. 未来定时 Agent 应调用哪类流程；列建议，不创建、更新或恢复 cron。

如果是未初始化的新任务，按现有 bootstrap reference 生成 1–12 条有限 seed 草案，
选择 1–30 个本地文本来源、至多一个 checkpoint，保留 conflicts/gaps。可以运行已有
`bootstrap.py preview`，输出完整 manifest、preview 与精确 digest；不运行 apply。
该命令只允许读取所选来源并在新的外部私有目录写草稿，不调用 CLI 或打开 ledger。

如果已有 `.memorynexus.json`，只生成“已有任务的增量接入建议”，不运行 bootstrap
初始化流程、不换 task ID、不重放旧 seed。未经 ledger 读回不能证明条目尚未导入，
需在报告中明确这是未验证项，不声称已完成去重。

### 5. 交付后停止

在仓库外新建时间戳目录，目录 0700、文件 0600；建议放在
`~/.local/share/memorynexus/onboarding/` 下。不要更改既有文件权限或删除产物。

回传：

- `report.md`：候选比较、所选工程、inventory、冲突/缺口与推荐方案。
- `profile-draft.md` 或 `profile-draft.json`：这是接入设计，不冒充当前 CLI 支持的配置。
- 新任务的 bootstrap draft/manifest/preview/digest，或已有任务的增量提案。
- `capability-gaps.md`：对照 R1–R6，哪些已存在、哪些待实现、哪些真机状态未验证。
- `artifacts.json`：本轮产物清单与 SHA-256；附方案 commit、选中工程路径、实际读取
  范围和实际执行命令。不包含业务原文或 secret。

最终用简短摘要说明：“选中了哪个工程、建议导入什么、两个 Lens 的用途、现有机制如何
衔接、哪些决定需要我确认”。正文留在私有附件中，不替我发布到 GitHub。

本轮不写 memory、不改正式文档、不安装 Skill/MCP、不修改客户端或 cron、不跑项目
测试、不重跑备份恢复、不清理 worktree。当前备份入口和已删除的 review cron 保持原状。
完成上述交付后停止，等待我对具体接入草稿的确认。
