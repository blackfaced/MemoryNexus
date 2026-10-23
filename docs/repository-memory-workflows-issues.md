# 仓库记忆接入与工作流：GitHub issue 索引

owner 于 2026-10-22 接受行为方案并授权发布 R1–R6。GitHub Issues 是执行真源；
本文件仅记录依赖和入口，不维护任务正文的第二份镜像。

- 架构：[ADR-029](../decisions/ADR-029-repository-memory-workflows.md)。
- 行为与 A1–A12 验收：[行为合同](repository-memory-workflows-contract.md)。
- R2 精确接口与 memory 生命周期：[已接受合同](repository-memory-v1.md)，固定 commit `83d85c6`，2026-10-23。
- 真实样本：[MiniMax 只读盘点 prompt](handoffs/minimax-repository-memory-discovery.md)。

## 执行顺序

| 阶段 | Issue | 前置条件 / 交付 |
| --- | --- | --- |
| R1 | [#308 盘点现有仓库并生成接入方案](https://github.com/blackfaced/MemoryNexus/issues/308) | 方案合入；接入 Skill、profile、bootstrap 预览示例，不写生产记忆 |
| R2 | [#309 冻结通用来源、独立消费与脚本接口](https://github.com/blackfaced/MemoryNexus/issues/309) | R1 样本；精确 CLI/JSON/schema/runner 合同及 owner 审阅 gate |
| R3 | [#310 最小 task CLI 持久化扩展](https://github.com/blackfaced/MemoryNexus/issues/310) | R2 接受并合入；来源、消费、失效、幂等与恢复 |
| R4 | [#311 共用脚本打通增量、Lens 与外部文档](https://github.com/blackfaced/MemoryNexus/issues/311) | R3；复用 bootstrap、配置化 Lens、文件应用及故障恢复 |
| R5 | [#312 版本化安装与外部定时接线](https://github.com/blackfaced/MemoryNexus/issues/312) | R4 手动闭环；稳定脚本包、MiniMax/Codex 接入与调度交接 |
| R6 | [#313 一个真实仓库的验收 gate](https://github.com/blackfaced/MemoryNexus/issues/313) | R5 与 owner 指定目录/入口；真实客户端、两次定时触发与继续使用决定 |

依赖链为 R1 → R2 → R3 → R4 → R5 → R6，同时记录在 GitHub 原生 blocked-by 关系和
issue 正文中。最新状态以 GitHub 为准。R1 在方案合入后可执行；R2 是设计交付，
R3–R5 在精确合同及各自前置未通过前不能标为 ready-for-agent。R6 需要真实机器与
owner 参与；模拟结果不能关闭该 gate。#301/#283 继续保留各自标准与状态。

当前源码含 R1 Skill 和脱敏 walkthrough；已接受的 R2 合同含完整字段/序列及
[R3/R4 命令级验收交接](repository-memory/walkthrough-v1.md#5-r3-310-command-level-acceptance-handoff)。
owner 已接受固定 R2 commit `83d85c6`，验收已写回 GitHub issue；先实现 #310，再实现 #311。

方案合入不等于功能实现，也不授权生产部署。每个实现 issue 使用独立分支/worktree，
引用所依据的合同 commit；需要改变行为边界时先更新合同并接受审阅。

## 后续方向（本批不建 issue）

- **MCP 薄封装**：在脚本入口稳定且实际客户端需要时，再冻结工具合同。只包装同一
  实现的结构化参数/结果，保留批准、scope、幂等和错误；不暴露任意 exec 或恢复旧服务。
- **维护脚本收编**：以在用 `mn-backup-all.py` 等完整源码、配置和真机证据为输入，
  单独评估版本化、部署切换、权限、最终哈希和分路失败隔离。不得顺带改当前 cron、
  恢复已删除 review 或开启 prune；它不依赖多 Lens 功能。

两项均不阻塞 R1–R6。本批不创建父 issue 或后续实现 issue，避免新增平行 tracker。
