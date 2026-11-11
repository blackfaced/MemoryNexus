# ADR-031: 跨机任务接力与记忆同步的边界

## 状态与决策

**Proposed，2026-11-12；选型决策，D 级文档变更。** 本文及[接力合同](../docs/task-memory-cross-host-contract.md)是 [blackfaced/MemoryNexus#347](https://github.com/blackfaced/MemoryNexus/issues/347) 的设计交付，不表示多机 runtime 例外已获接受或实现。核查基线为 `891deb936c811a2e91d804b6ee28610b5210b0f8`；设计交付随本次文档 PR，不修改 Rust、SQLite 或既有 CLI。

推荐方向是：各设备本地保存已授权任务记忆，按任务同步；**一个任务的实现执行权仍须显式交接，同一时刻只有一个已确认执行者**。同步到记录不授予执行权，停止执行也不禁止 owner 在另一设备离线记一条独立观察。当前没有跨机同步实现，跨机自动接管暂缓；条件不齐时只展示已确认材料和缺口。

这是对 [ADR-028](ADR-028-shared-task-memory.md)“同机、单权威写入位置”的**拟议有界扩展**：只涉及同一 owner 显式配对设备上的选定任务。接受、实现及真实往返各自留证；[ADR-027](ADR-027-sqlite-cli-minimax-feedback-kernel.md) 的四对象确认和十四天 gate、[ADR-029](ADR-029-repository-memory-workflows.md) 的发布职责、[ADR-030](ADR-030-current-knowledge-maintenance.md) 的订正/撤回与实际读取保证均保留。

## 核查事实与备选项

[hancheng.hc/dev-tracker#661](https://code.byted.org/hancheng.hc/dev-tracker/issues/661) 记录 2026-11-11 色卡实验：Mac/Linux 使用同源码 `46434b06a7304ce421aa8d020657675066f8249e` 的原生 CLI、各自 SQLite，从相同四条记录开始离线各写一条。仅恢复连接不会补齐。人工交换原请求后两端正文达到六条，但只有四个基线 ID 相同，默认 checkpoint 恰好相反；同机请求重放幂等，不能替代跨机记录复制。它没有验证同步实现、执行权交接或代码往返。

| 选项 | 适用条件、成本与风险 | 取舍 |
| --- | --- | --- |
| A：常在线机器持有唯一 ledger，客户端走既有受限远端执行通道 | 少一套复制状态；需 owner 选定主机、授权通道及验证可达性。断网端无法向唯一账本提交。现有 SSH 样例不等于已批准日常远端执行 | 可作为明确接受离线不能写的替代方案；不满足本轮双端离线记忆目标，不默认启用 |
| B：源端停写后整体一致快照移交，核对收据再换写入者 | 复用 backup/restore；整库可能含其他任务，restore 只到新路径。丢失回执、pending 与恢复世代需核验，不能合并两个已前进的账本 | 仅适合 owner 明确批准的完整迁移/隔离演练；不作为日常按任务接力 |
| C：设备各自 SQLite、任务级变更与持久化回执，另行显式交出执行权 | 保留离线读写；需要原 ID 保真、并发进度展示及确认位置，均未实现 | 推荐后续方向。先完成下述小切片，再验证跨机接力；本票只冻结合同 |

## 决定性变量与翻转条件

决定性变量是**是否必须双端离线保存记忆**，以及接班时能否取得可核验材料与明确执行授权。若取消离线写要求且批准了可靠常在线端，可重开 A；若双方必须错开在线接班，可另评估受信加密暂存节点。未发出的尾部始终未知，不能以中继或 P2P 承诺消除。

## 复用、代价与退出条件

- 记忆仍复用 [task contract](../docs/task-memory-contract.md)、Rust CLI、SQLite、来源/确认、修订链和撤回。`task export` 的 task-only 指区别于四对象实验数据，仍可包含多个 task；JSON 是检查材料，不是已有 import。`backup/restore` 提供一致恢复，不提供并发副本合并。
- 代码准备/取回复用 AIWB 固定版本 `54d62413d7b38042f17b576cddddd84b02bc6aae` 的 [handoff-to-dev](https://github.com/blackfaced/ai-workbench/blob/54d62413d7b38042f17b576cddddd84b02bc6aae/skills/handoff-to-dev/SKILL.md) 与 [collect-from-dev](https://github.com/blackfaced/ai-workbench/blob/54d62413d7b38042f17b576cddddd84b02bc6aae/skills/collect-from-dev/SKILL.md)。它们不提供记忆复制或分布式执行租约；Botmux 只复用既有消息/会话路由。
- 本次不新增持久化模型、API、命令、schema 或部署单元。合同字段是交接证据要求，不是可调用接口。新同步格式/事务边界由实现票另行冻结。
- 可逆性：本次撤销文档即可回到上述本地基线，无数据迁移。未来来源 ID/世代/序号影响跨机引用，必须先冻结兼容合同；不能用重新生成 ID 作为无损回退。
- [#672](https://code.byted.org/hancheng.hc/dev-tracker/issues/672) 先保留来源 ID；其后 [#673](https://code.byted.org/hancheng.hc/dev-tracker/issues/673) 给出并发 checkpoint 接手视图、[#674](https://code.byted.org/hancheng.hc/dev-tracker/issues/674) 提供按方向的连续持久确认。执行交接消费这些结果，不替它们实现同步。
- 设计退出：合同逐项覆盖 #347 七组问题和八类故障，每项区分现有能力、人工步骤及未实现依赖。实现退出另验：原 ID、并发当前视图、双向回执、代码快照与真实原聊天读回均有证据，才能声称跨机接力成立。

## 非目标与未决项

永不由本接力层承担：模型/IDE/机器人平台、账户体系、全聊天复制或替 owner 制造内容授权。本期不实现、由既有票后续交付：任务同步及缓存失效、订正/撤回传播、可靠路由去重、机器验收。待定：中继、第三台设备、自动接管或技术 fencing；须有独立合同与授权，P2P/DHT/CRDT 不作为前置条件。

首次真实试用前由 owner 选定一台开发机、目标项目/主机身份、允许的 harness、现有传输/凭据通道、Botmux 群/话题和有界任务。执行 owner 核验固定包、接线及恢复规则，缺项则停在准备态。本 ADR 不选定具体主机/IP、不复用私人发布批准、不恢复暂停试点或定时任务。八类推演与后续小切片统一见[接力合同](../docs/task-memory-cross-host-contract.md)，运行状态继续由 Issue 管理。
