# 当前有效知识维护：实施规划

状态：2026-10-26，owner 接受方向与阶段顺序；#318 的
[精确接入合同](current-knowledge-contract.md) 已形成提案，待固定版本审阅。
长期决策见 [ADR-030](../decisions/ADR-030-current-knowledge-maintenance.md)，执行真源为
[#317](https://github.com/blackfaced/MemoryNexus/issues/317)。本文记录差距与顺序，不镜像
全部 issue 验收条件，不表示生产接入已获授权或已经完成。

## 1. 已有能力与实际缺口

核查源码：`955f1c53ea1f5fb14cad03fb1e6c31aa8005662c`（2026-10-23 main）。

| 需求 | 已实现的依据 | 本轮需要补齐的部分 |
| --- | --- | --- |
| 明确订正替代旧版本 | [correct 与依赖失效](../src/task_memory/lifecycle.rs)，task_memory_cli 的订正/竞争测试 | Agent 必须找到准确当前目标并走 correct；追加另一条 record 仍可产生矛盾 |
| 默认读取过滤失效内容 | [context](../src/task_memory/mod.rs) 选择 current，候选分列、限制大小 | 证明实际会话调用了入口；历史读取不混作当前事实 |
| 来源与下游失效 | [workflow lifecycle](../src/task_memory/workflow/lifecycle_hooks.rs)，task_workflow_consumers_cli 的选择性失效测试 | 来源只在被观测后判定；定义读取前漂移复查和多个快照的一致性 |
| 受管文档替换 | [render/publish](../integrations/task-memory/scripts/workflow_io.py) 替换稳定 block，校验摘要 | 核验所有选定的外部输入副本；区块外的旧说法不能被假定已修正 |
| 不同 consumer 独立采纳 | 已有稳定槽位、generation、dependencies、发布回执 | 不等于语义去重；另一 ID 的同义或矛盾事实仍需关系合同 |
| 来源草稿进入实际工作 | source-capture 保留 draft/source_reported；普通 context 始终排除 draft | 冻结 repository consumer 输出的读取路径，不悄悄提升 trust 或改 context 合同 |
| 客户端接入 | [共享协议](../integrations/task-memory/PROTOCOL.md)、[Codex 指针](../integrations/codex/task-memory.md) | 区分强制 hook / wrapper / 提示词，核验实际注入而非只看账本 |
| 回归保护 | #311 有 35 项 runner 测试的验收记录 | CI 尚未执行 test_workflow*.py；两处临时根硬编码 /private/tmp |

## 2. 执行顺序

| 顺序 | 跟踪 | 交付与依赖 |
| --- | --- | --- |
| 第一阶段合同 | [#318](https://github.com/blackfaced/MemoryNexus/issues/318) | 文档任务：冻结订正、两类读取、实际材料证据、并发/失败行为及最小实施 handoff |
| 独立回归保护 | [#319](https://github.com/blackfaced/MemoryNexus/issues/319) | 既有 workflow suite 接入 Test CI，修复测试临时路径的跨平台问题；不依赖 #318 |
| 第一阶段实现 | #318 审阅后再拆 ticket | 仅实现冻结后的最小接入；不提前为未知接口发运行时任务 |
| 固定安装 | [#312](https://github.com/blackfaced/MemoryNexus/issues/312) | 复用其安装/接线工作；新接入部署需要其对应版本及绑定核验，不阻断现有 R5 的独立推进 |
| 真实验证 | [#301](https://github.com/blackfaced/MemoryNexus/issues/301)、[#313](https://github.com/blackfaced/MemoryNexus/issues/313) | 在既有 gate 中补新行为证据；分别保留原标准、结果与 owner 决定 |
| 跨记录整理 | #317 第二阶段 | 第一阶段价值复盘后，再冻结关系身份、范围、提案/批准/撤销与下游影响合同 |
| 管理页面 | #317 第三阶段 | 在读取及关系合同之上展示时间线、局部关系和预览；首版运行方式另定 |

#318 交付合同、[合成验收矩阵](acceptance/current-knowledge.md) 与
[最小实施 handoff](current-knowledge/runtime-handoff.md)；新 adapter 接口仍是提案，
尚未实现或安装。审阅完成前不将对应 runtime ticket 标成 ready。#319 可独立推进。
本规划没有新建另一套真实客户端或调度验收，没有关闭或改写 #301/#313 的既有 gate。

## 3. 第一阶段的核心验收场景

以下使用合成内容；是交付目标，不是已经执行的证据。

1. 在 task A 保存“项目工作目录为目录 A”，保留 ID，并由该记录生成候选及两份 consumer
   输出。候选、事实报告和确认内容保持原 trust。
2. owner 明确确认“改为目录 B”，以原记录当前 ID 调用 correct。旧记录 superseded，
   依赖旧版本的候选/决定/批准失效；并发更改或重放保持现有 conflict/幂等语义。
3. 重新生成外部输出时复用现有 review、精确发布批准及受管区块替换。未获批准、文件冲突
   或来源不可读时报告待处理；不能把 ledger 更新称为外部文档已经同步。
4. 新的实际会话经声明的入口读取，保留有界证据，按新内容在合成目录 B 做无害读取或检查。
   不向该会话粘贴迁移结论，不靠旧聊天提示答案。验证输入材料及实际动作两者。
5. 历史读取仍可解释 A→B；默认当前用途不能使用 A。显式 draft 路径按 #318 合同单独测试。
6. 覆盖读取失败、空/截断、跨 task、来源/文件漂移、旧批准、撤回与隔离恢复。恢复使用
   已含撤回状态的备份，不能复活正文或旧发布授权；不声称更早的备份已被清除。
   失败不新建替代 ledger、不回退隐藏记忆、不从当前历史/缓存重新暴露已撤回正文。

纯 CLI 测试证明内核；模拟客户端证明接入机制；真实 MiniMax/Codex 会话证明实际路径。
实际 scheduler 运行单列，其 run ID 和 no-op 证据不能由手动测试代替。
旧会话已读入内容不能被承诺消失；保证应写明入口、检查时点和覆盖的副本。

## 4. 第一阶段合同必须解决的问题

- 普通 context 与 repository consumer 输出如何分别进入当前工作；draft 保持来源报告
  身份，消费/发布不自动等于 owner-confirmed 事实。
- 目标 ID 选择不明确时如何停留在提案；历史审阅与当前消费如何区分。
- 实际读取证据的最小字段、界限、私有存放和清理；task revision 与适用的 incarnation、
  来源版本、材料摘要及多次读取间变化如何处理。读取日志不成为第二个权威库。
- 哪些注入副本能观察；unknown/stale 如何呈现；当前来源观测的时间边界是什么。
- hook/wrapper 的真实运行能力与绕过范围；confirmed 的可信调用者假设如何明确呈现。

精确命令/JSON 以 #318 交付的合同为准；本规划没有增加 task CLI 字段或更改 schema。

## 5. 价值验证与继续投入的依据

在 owner 选定的一个仓库进行有边界的试用。开始前记录试用范围、会话入口、观察窗口与
案例清单；不照搬或改变 #283 的十四天门槛。本规划不代 owner 选私人目录或宣布开始试用。

| 观察项 | 记录方式 |
| --- | --- |
| 订正后旧说法再次影响工作 | 记录发生次数、可核验的订正后使用机会数与脱敏 case 引用 |
| 用户重复解释/纠正 | 区分新事实、原订正未传播、Agent 未走入口及普通任务澄清 |
| 实际读取与账本一致 | 记录已验证会话数/总观察会话数，unknown 单列；不能把不可观察计为通过 |
| 维护负担 | 记录每次确认、冲突处理和手工恢复的次数及耗时，区分安装成本与日常成本 |
| 是否值得继续使用 | owner 明确选择继续、调整或停止，并说明依据 |

这些是拟用于试用的记录口径，不是已经接受的数值阈值。分母为零或证据缺失时记未评估，
不得制造成功率。只保留脱敏结果及必要引用，不为统计保存原始私聊或完整提示词。
先判断是否减少重复纠正且维护成本可接受，再决定第二、三阶段的投资与范围。

## 6. 已有验证与边界

- 2026-10-26 只读核查：main 的 Format/Clippy/Build/Test 通过；9 项 workflow 文件处理
  单测在合成临时目录重跑通过。这不是第一阶段新接入的验收。
- [R3/R4 验收](acceptance/repository-memory-workflows.md) 保存此前 Rust/runner 实现证据。
- [真实客户端记录](acceptance/task-memory-cross-client.md) 为 PARTIAL，已报告部分真实
  MiniMax 与定时运行；完整 Codex 路径和 owner 决定未完成。这份记录不证明新功能已部署。
- #312/#313 在规划核查时仍开放；#283 与 legacy contraction gate 均不变。
