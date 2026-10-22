# 仓库记忆工作流：行为合同

**行为边界已于 2026-10-22 获 owner 接受；R2 精确接口草案待 owner 审阅，增量功能尚未实现。**

[R2 精确合同](repository-memory-v1.md) 给出字段、版本、生命周期、完整请求序列及
R3/R4 命令级验收交接；其 owner acceptance 仍为 PENDING。本文件保留行为边界，
以下精确化以该草案为准，不能将文档交付当成运行时可用。
本文用于设计与验收，不是当前可运行命令说明。
架构依据见 [ADR-029](../decisions/ADR-029-repository-memory-workflows.md)，执行拆分见
[issue 索引](repository-memory-workflows-issues.md)。实现者需先读
[已有 task 合同](task-memory-contract.md)；已有字段的语义不因本稿改变。

## 1. 当前能力与缺口

| 能力 | 当前依据 | 本切片的工作 |
| --- | --- | --- |
| 首次来源盘点、预览确认、导入恢复 | [bootstrap Skill](../integrations/task-memory/memorynexus-task-bootstrap/SKILL.md) | 复用，仅补接入流程入口 |
| session 读 context、记录执行、保留候选 | [共享协议](../integrations/task-memory/PROTOCOL.md) | 接入 profile 明确何时使用 |
| 源记录纠正/撤回使 lesson 失效 | 现有 task correct/retract | 保留确认与 trust 语义 |
| 本地文件增量、已有任务导入 | 尚未提供 | 新接入脚本及必要来源合同 |
| 多 Lens 独立消费、发布回执 | 尚未提供 | 最小 CLI/SQLite 扩展 |
| 外部文件发布与恢复 | 尚未提供 | 接入脚本执行；Rust 不写 Markdown |
| 定时唤起 | 外部 Agent/调度器 | 稳定入口与独立真实验收 |

## 2. 接入步骤与完成条件

1. **发现**：显式 task root 下盘点任务目的、指令/知识/草稿来源、已有 capture/review/
   context 注入，以及已知 hooks/定时入口。仅读取选定范围，发现阶段不打开 ledger、
   调用 CLI 或安装配置。完成物是带证据路径的 inventory，未知项明确列出。
2. **制定方案**：Skill 生成 profile、首次 seed 草案、增量规则和两个 Lens 的建议。
   明确保留/替换哪个入口、来源冲突和遗漏；完成物须能被 owner 逐项审阅。
3. **确认并绑定**：确认 profile 的范围和策略。首次 seed 使用现有 bootstrap 的精确
   manifest digest；已经存在 task config 时进入单独的增量预览，不能重跑初始化。
4. **导入并读回**：使用已验证 CLI 与显式 ledger，按 receipt 确认身份/版本；部分失败
   保留原请求。只有实际读回成功才标记导入完成。
5. **手动增量闭环**：新增来源、两 Lens 评审、两份输出、重复调用、来源纠正与重试均有
   可检查结果后，才准备定时接线材料。
6. **定时接线**：由 owner 指定入口，展示配置 diff，调用同一稳定脚本。真实触发证据与
   手动执行分别记录；未触发时不报告自动化完成。

## 3. Profile 是接入配置

以下是必须表达的语义，精确 JSON schema/字段及边界在 R2 冻结后才作为实现接口。
R1 可交付 Markdown 方案与 JSON 草稿，不将其冒充当前 CLI 的合法输入。

| 部分 | 必须表达的内容 |
| --- | --- |
| 身份 | profile ID/version/digest、task root、已有/新任务、预期 task ID（存在时） |
| 绑定 | config、外部 ledger、CLI、脚本包版本/摘要；机器绑定独立保存 |
| 既有机制 | 目的、维护者、入口、读取/写入/定时方式、证据或 unknown |
| 来源 | 稳定 source ID、允许的文本路径/区域、抽取规则版本、排除项及权威性说明 |
| 首次导入 | 指向 bootstrap 草案或“已有 task，不初始化”，保留 conflicts/gaps |
| 增量 | 新增/改动/消失/移动如何处理，哪些可自动保存、哪些待确认 |
| 消费者 | consumer ID、Lens 规则与版本、输出类型/目标、批准与失效处理策略 |
| 运行 | 工作流 ID、现有调度入口或未配置、超时、产物位置、结果查询方式 |

共享模板可入版本库；机器绝对路径、正文、批准凭据、pending 和运行日志使用外部
owner-only 部署目录。profile 更新有版本和 diff；源范围、目标、授权或 Lens 改变须重新
确认，不通过修改配置扩大旧批准。

## 4. 脚本合同

首版复用 Python 标准库 adapter 方式，产品状态仍由 Rust CLI 校验和持久化。
统一入口拟为 `integrations/task-memory/scripts/workflow.py`，以下是操作语义，参数
schema 在 R2 冻结。bootstrap 保留自己的现有入口；共用脚本通过它完成首次导入。

| 操作 | 输入/输出与完成条件 |
| --- | --- |
| inspect | 显式 root 与选源范围 → inventory；只读仓库，不读 ledger |
| prepare | 已批准 profile + 工作流（增量/评审/发布恢复） → 有界工作清单；只读 ledger/来源，产物可落盘 |
| preview | Agent 提供的抽取/评审结果 → 完整提案、diff、来源版本、digest；不写 ledger/目标文件 |
| apply | 固定提案 + 对应授权 → 校验后调用 CLI/文档 writer；返回逐项 receipt、pending、冲突 |
| status | ledger 与本次运行/文件协调状态 → current/stale/pending/conflict；不隐式修复或升级 |

Agent 的一次定时会话可以按 prepare → Skill 判断 → preview → 有授权的 apply 调用。
无工作项直接结束；需要确认时留下完整预览并报告 needs_review。本地脚本不代替模型
判断，也不自行填入 owner confirmation。可以自动运行的 capture/review 范围须已被批准。

共同要求：

- 显式配置、固定解释器/CLI、受限环境，不依赖交互 shell 或默认 cwd 选库。
- 不执行 profile/模型返回的 shell 字符串、任意脚本路径或数据库 SQL。
- 单 task 协调锁；锁冲突明确返回 busy。CLI 事务和唯一约束负责最终并发保护。
- 调用前保存完整不可变请求，超时/响应丢失保留原 key、run/client、正文、时间。
- 每路独立结果；一份文档/报告落盘失败仍尝试其他独立分支并保留结构化 stdout。
- 区分 ok/noop/needs_review/partial/failed/busy；partial/failed 为非零退出；needs_review
  不是发布成功，busy 不是无新内容。确切退出码由 R2 冻结。
- 正文只出现在授权的预览/内容输出，运行摘要含身份、版本、计数、receipt 和错误码。
- 文件摘要在最后可能改变该文件的子进程退出后计算；稳定产物与报告对应。
- 安装在稳定版本目录，保留上一版便于显式回退；升级期间不更换正在运行的文件。
- 无新来源时 ledger 和正式文档不变，运行元数据可记录一次 no-op；产物保留策略需
  单独配置，首版不自动删除历史备份。

## 5. 来源、草稿与增量

核心只接收有限文本、opaque source ID/revision 和来源描述符；脚本读取文件并计算
digest。文件路径用于追溯，不充当永久身份；修改时间不充当版本或消费游标。

| 变化 | 处理 |
| --- | --- |
| 来源/抽取规则无变化 | 不重新抽取或创建记录；继续处理已有 pending/stale |
| 新来源 | 在已批准来源范围内生成有限候选，预览后按 capture 策略提交 |
| 来源内容变化 | 记下新版本；旧派生结果不再视为 current，替代内容单独抽取/确认 |
| 消失、读失败、未证明的移动 | 标记 source_unavailable/needs_review，保持原文与身份，不自动删除或撤回 |
| 经确认的删除/移动 | 更新来源映射或执行明确撤回；保留谱系，不能重新计为全新佐证 |
| 派生输出变化 | 默认排除；人工区域需要显式来源规则，不能回流自证 |

自由草稿、执行报告、owner 指令和模型候选须可区分；不能把一般笔记伪装成 execution，
也不能把文件改动当成 owner-confirmed correction。R2 选择扩展现有 record kind 还是
最小专用对象，并给出 ordinary context 的纳入规则；未确认草稿不得混成指令。

增量只覆盖已登记、可读且已扫描的来源。脱离接入脚本直接读取 ledger 时，CLI 无法知道
磁盘刚发生的改动。Skill 在使用外部文档/生成新评审前检查来源，status 明示最近检查
范围与失败项；“current”不表示监控了整个文件系统。

## 6. 独立消费与失效

R2 将原先按 entry/version 描述的消费关系精确化为
`(task_id, workflow_id, consumer_id, root_record_id)` 稳定槽位与不可变 generation。
每代固定 input ID、全部实际判定证据及其版本、Lens/profile 版本；record.revision 是
可变状态版本，不能当正文身份。同源不变而关联纠正变化也必须使受影响决定失效。
每个 consumer 的决定是 proposed/approved/skipped/deferred；有效性另记 current/stale/
retracted，发布状态另记 not_applied/applied/conflict 或恢复中。

- 两个 consumer 分别处理，同条来源可以采用不同措辞。没有来源级全局 processed。
- 同 key/同请求重放；不同请求冲突。换 run/key 后同消费键/同结果仍只有一个决定；
  改措辞须明确重新评审，撤销旧批准。
- skipped/deferred 对当前版本有效，不在每次定时运行重复生成；新版本或显式 reconsider
  才重新评审。暂缓未发布和另一消费者成功可以同时成立。
- 来源失效与旧决定失效必须同一事务；批准不能跨来源版本、Lens 版本或目标配置复用。
- source revision 和 ledger commit sequence 用于增量；发生时间仅供语义解释。
  按各 consumer 的未完成集合读取，分页或局部失败不能将任务整体标成已处理。
- 撤回清除 ledger 内对应修订链与结构依赖正文，留下最小 tombstone/receipt；
  已发布文件需独立协调/授权中性化，不承诺即时擦除文件或外部历史备份。

## 7. 文档发布属于接入脚本

首个 renderer 仅支持本地 Markdown 的受管区域，目标由 profile 绑定。默认输出建议并
确认精确 diff 后发布；不同消费者可有不同授权，首版不开放任意自由文本自动发布。

每份提案绑定来源版本、consumer/Lens/profile 版本、base 文件哈希、预期 after 哈希和
正文 digest。同一 entry/consumer 的稳定块标识保持，新版本替换对应块；人工区域保留。

脚本先持久化不可变操作计划，再重新校验当前来源和批准、获取受控写锁、检查基线，
同目录临时写入并原子替换，读回后通过 CLI 记发布回执。计划是恢复材料，只有文件读回
和 CLI 回执都成功才报告 applied。实际参数和回执字段属于 R2 的合同交付。

崩溃重试：after 哈希命中则补回执；base 哈希命中且来源/批准仍有效才重试；否则冲突。
恢复材料丢失时报告 uncertain 并要求核对，不盲目重写。来源在文件替换前后发生变化时，
回执必须如实记录曾写过旧版本并安排失效协调，不能把旧输出标成 current。

纠正后 ledger 可先 stale，文件尚未更新。若接入时已批准失效维护，脚本将未被人工修改
的旧受管文字替换为不含旧正文的中性提示；新结论仍需独立批准。没有该授权则返回待确认
移除的建议。人工改动、软链、路径越界、重复 marker 均返回冲突/错误；不吞掉修改。

两份文档分别完成，CLI 与文件系统没有跨资源事务。应用锁只能约束合作 writer，需说明
非合作编辑器并发的限制。SQLite 恢复不恢复外部文件；恢复后按现文件哈希重新协调。

## 8. 内核扩展、部署与调度

R2 必须在实现前补齐：精确 CLI action/JSON/error/限额；稳定来源与 record revision
映射；消费决定及发布回执；并发和失效事务；自动保留 policy；schema/导出/恢复兼容。
不能向旧命令偷偷添加未知字段，不能直接写 SQLite 或用运行 JSON 取代业务状态。

如果需要 schema 升级，显式备份与迁移，读取/restore 不隐式升级；保留 task ID、原
records/receipts/tombstones。部署逐一核对所有客户端和备份入口 pinned binary。

调度接线必须固定 workflow/profile/脚本版本、解释器、cwd、必要环境和结果位置。
声明来源为 scheduler 的报告需引用调度器实际 run ID/触发证据；请求参数自称 cron
不够。MCP 调用也可能只是手动触发。漏跑和 catch-up 行为按已验证调度能力说明。

首版先验证 Agent 直接调用同一脚本。后续 MCP wrapper 只转换结构化输入/输出并保留
相同策略，不提供任意 exec、不增加授权层或独立状态；采用新的最小进程，不复活旧服务。
当前 backup cron 的迁移、review cron 的创建/恢复都需要独立明确的部署范围。

## 9. 验收矩阵

| ID | 场景 | 必须观察到的结果 |
| --- | --- | --- |
| A1 | 仓库已有多种记忆机制 | inventory 有证据/unknown，profile 说明继承与改动；发现阶段零 ledger 写入 |
| A2 | 新 task 与已有 task | 前者复用 bootstrap；后者拒绝重复 init/seed，进入增量预览 |
| A3 | 相同来源、两 consumer | 一个采用、一个暂缓；随后两者不同措辞采用，关系独立 |
| A4 | 重复 run/key/client、无新输入、并发 | no-op/replay/conflict 可区分，记录和文档不重复 |
| A5 | 文件来源变更、已确认记忆纠正 | 各自遵守确认语义；旧消费失效，新版本分别处理，文件延迟可见 |
| A6 | 读失败、删除、重命名、早时间补记 | 不误撤回、不漏项、不因重命名复制来源；需要确认则明确等待 |
| A7 | 生成的文档再次被扫描 | 受管区域被排除，不产生自证循环 |
| A8 | 超时、响应丢失、文件替换后崩溃 | 原请求重放/哈希协调，无重复发布；人工变化得到 conflict |
| A9 | 一路发布失败、报告写失败 | 其他独立路仍尝试；stdout 明确 partial/failed，退出非零 |
| A10 | 备份隔离恢复 | 来源/决定/回执保留，文件需重新协调，旧批准不复活 |
| A11 | 换目标文件名、同入口两客户端 | 不改 Rust 逻辑；相同 profile/CLI 共用身份，保留不同实际 client |
| A12 | 实际外部定时触发、随后无新内容 | 有调度器 run 证据，调用固定脚本；第二次不重复写，无 catch-up 虚假承诺 |

A1–A11 需要实现证据；A11 的真实客户端部分及 A12 是独立真机 gate。原 #301 未验收
内容仍单列。所有阶段明确 passed/failed/not-run，模拟不能代替实际客户端或定时触发。
