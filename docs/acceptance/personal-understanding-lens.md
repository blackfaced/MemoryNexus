# “理解与疑问”Lens：验收记录（#346）

## 先看这几条

- 已在现有日常 Skill 中加入按需调用的 Lens，没有新增服务、数据库或独立 Skill。
- 初版四组合成行为检查通过；你随后反馈“无法判断”，原因是“都是英文太长了我看不懂”。这项展示问题单独保留，没有改写成“有用”。
- 已改为按用户语言展示：中文请求默认短中文，四项各一句，详细证据按需查看。
- 修订版 A/B 比较、订正后重读和简短中文展示均通过复核。源规则与证据随本次提交交付；未更新私人安装，没有修改私人 ledger。

## 本次改了什么

[日常 Skill](../../integrations/task-memory/memorynexus-task-memory/SKILL.md) 只在明确提出理解审阅时加载
[理解与疑问规则](../../integrations/task-memory/memorynexus-task-memory/references/understanding-lens.md)。
普通“接上”沿用原流程。

输出保留四项：你解释或尝试过什么；哪些仍未验证；疑问和变化；一个下一步验证问题。
首屏用简短来源标记连接判断与原记录，完整 ID、版本、类别、可信来源、状态、时间和引用核验信息保留，按需在会话展开，不自动写详细报告。

只处理一个任务、一个主题和已授权的有界材料。上下文读取按状态、类别、提交顺序选取，不是主题语义搜索。
用户自述、助手产物、执行报告和待核实推测保持各自来源；确认表述不等于证明掌握。
来源订正后重新读取；旧观察不能自证或恢复撤回的正文。没有增加自动晋升、评分或新的内核行为。

## 合成输入与行为结果

合成任务 ID 为 `34600000-0000-4000-8000-000000000001`，主题为“响应丢失后重试”。
十九个原始文件是按现有字段构造的测试快照，不是 CLI 抓取，也不证明真实作者、真实学习或工具执行。
有界记录按样本来源顺序提供，不声称重现原生 CLI 的提交选择顺序。

| 输入 | 变化 |
| --- | --- |
| [A](fixtures/personal-understanding-lens/sample-a/input.json) | 用户自述、有来源的解释和预测、助手答案、缺失执行引用、待核实结论；另一任务作为干扰项 |
| [B](fixtures/personal-understanding-lens/sample-b/input.json) | 只替换更正确、更完整的助手答案及对应哈希；人的记录完全不变 |
| [C](fixtures/personal-understanding-lens/sample-c/input.json) | 来源订正、练习撤回和派生结论失效后的新快照与历史标记 |

每份样本故意缺少同一执行日志。C 的撤回材料没有正文和引用。
C 在提供给评估会话前已修正候选数量、历史发生时间和首次失效版本；初始未使用副本保留在本地测试目录。没有替换或隐去失败的行为运行。

| 检查 | 初版结果 | 短中文版结果 |
| --- | --- | --- |
| 判断有来源及限制，不凭空认定掌握 | 通过 | A/B/C 通过 |
| 自述、个人解释、助手产物、执行报告分开 | 通过 | A/B/C 通过 |
| 助手答案更好，不提高人的理解判断 | A/B 比较通过 | A/B 比较通过 |
| 订正、撤回后旧观察退出；另一任务不混入 | C 通过 | C 通过 |
| 普通“接上”不自动运行 Lens | 通过 | 本轮未改该路由，复用初版结果 |
| 只展示，不写 ledger 或自动保存结论 | 所供快照范围内通过 | 所供快照范围内通过 |

初始评估会话不继承本聊天，只有固定 Skill、单份原始样本和短问题，没有预期答案或本验收文档。
A/B 使用独立新会话；C 接续 A，以验证收到变化后重新读取。一次样例通过不代表所有模型会话都可靠。
原生依赖失效机制复用 #317 的[既有证据](current-knowledge.md) C01/C03/C10，本票不重做或重新声称该机制验收。

初版完整原始输出：[A](fixtures/personal-understanding-lens/outputs/response-a.md)、
[B](fixtures/personal-understanding-lens/outputs/response-b.md)、
[C](fixtures/personal-understanding-lens/outputs/response-c.md)、
[接上](fixtures/personal-understanding-lens/outputs/response-resume.md)。
初版[输入、包和输出清单](fixtures/personal-understanding-lens/outputs/acceptance-manifest.json)保留原固定字节与精确短问题，属于展示修订前的证据，不冒充最终规则版本。

短中文版原始输出：[A](fixtures/personal-understanding-lens/outputs/response-brief-a.md)、
[B](fixtures/personal-understanding-lens/outputs/response-brief-b.md)、
[C](fixtures/personal-understanding-lens/outputs/response-brief-c.md)；
[最终输入、包、输出和断言清单](fixtures/personal-understanding-lens/outputs/brief-acceptance-manifest.json)。
独立复核确认 A/B/C 的中文正文约 175/193/195 个汉字；7/6/6 个来源链接均为已存在的绝对路径，缺失日志没有伪链接。

中间展示修订曾失败：[A 原输出](fixtures/personal-understanding-lens/outputs/presentation-v2-a.md)、
[B 原输出](fixtures/personal-understanding-lens/outputs/presentation-v2-b.md)、
[失败记录](fixtures/personal-understanding-lens/outputs/presentation-v2-result.json)。
其 A/B 语义比较通过，但相对链接、长句、完整任务 UUID 和缺失日志链接仍妨碍阅读；这轮没有标为完整验收通过。
最终规则进一步压缩首屏、将有效引用解析为绝对路径，再使用新固定源重验。

## 固定版本与资源验证

源码基线：`3c0d82b4b4d59657f500dd7c5d3dbeac3fe4ff51`，包含已合入的 #343。
#343 原验收源 `71f2d7e04a639fef728adb9ecbd2335179d66065` 保持独立，不追溯改写其结果。

| 项目 | 记录 |
| --- | --- |
| 初版临时包快照 | `1ae4a7bda92c0825b270a0ccdc885ebd0d7f504b` |
| 中间展示修订临时包快照 | `249b259d5029c3d7476d4896efeb42b4c5fb29a0` |
| 最终短中文版临时包快照 | `444f4887119c0b95c7fa5b29c0f483be24ae9c10` |
| 最终规则 SHA-256 | `e476cc7b026f8340e81151fc83d82c8fc2c3dc0ca0ccb2a876e67a4442893bf2` |
| 本地证据根目录 | `/private/tmp/memorynexus-issue346-review-r5l6hixi`，最终短中文版位于其 `language-v3/` |

这些临时 Git 快照只用于隔离测试，不是发布提交。当前源码与报告的发布身份、合入状态以所属 Git 历史和 PR 为准。
安装器本来就选择整个 canonical Skill 子树；新增规则的安装位置为
`VERSION/package/integrations/task-memory/memorynexus-task-memory/references/understanding-lens.md`。
只引用同包内 `daily-interface.md`，本验收文档和样本不进入运行依赖，无须改安装器。

已执行：Skill frontmatter 校验；七份快照的现有字段校验；可用引用哈希核验；A/B 仅助手材料变化核验；`git diff --check`。
各版均用固定临时源执行 `install.collect`，物化 67 个选中文件（其中 64 个在 `package/`），再用既有 `Installation.assert_daily_resources` 检查递归资源边界，均通过。
这只验证包选择和引用闭包，不等于完整安装、二进制能力、解释器权限或运行时回归。
Rust、UI、CLI/schema 和 helper 均未修改，因此没有重建或运行 Rust 测试。

## 2026-11-06 提交前核对

模式：update。集成基线为 `76128eb2f63d1ecfddfce03e26367c11f2d1ceb2`；新增的上游 #350 不触及本票两个规则文件。
本轮未重跑模型会话：最终冻结清单的21个输入和9份历史输出逐项哈希匹配，复用既有行为证据；不把本轮静态核对称为新的独立 QA。
7份快照字段、9次可用引用哈希、3个预期缺失日志及 A/B 仅助手材料变化均通过核对。

既有 `install.collect` 对当前基线选包，再覆盖本票两个规则文件，物化68个文件后通过未修改的 `Installation.assert_daily_resources`。
首次核对器未解析 macOS 临时目录别名，导致路径边界检查受阻；只将核对器临时根目录改为真实路径后通过，生产代码和断言没有变更。
选包清单摘要、原始结果与适用范围见 [提交前核对清单](fixtures/personal-understanding-lens/outputs/closeout-check.json)。提交后的固定 Git 选包还须匹配同一摘要，结果在 PR 交付记录中回写。

历史答复保留原运行时的绝对路径。此次重查发现19处临时链接已不可读，原始输出没有改写，也不将该项记为当前可点击性通过。
对应目标全部能映射到本次发布的同字节样本，以下仓库链接供后续复核，不依赖本机临时目录：

| 样本 | 原引用对应的版本化材料 |
| --- | --- |
| A | [解释](fixtures/personal-understanding-lens/sample-a/sources/owner-explanation.md)、[预测](fixtures/personal-understanding-lens/sample-a/sources/owner-practice.md)、[助手答案](fixtures/personal-understanding-lens/sample-a/sources/agent-answer.md)、[记录](fixtures/personal-understanding-lens/sample-a/task-context.json) |
| B | [解释](fixtures/personal-understanding-lens/sample-b/sources/owner-explanation.md)、[预测](fixtures/personal-understanding-lens/sample-b/sources/owner-practice.md)、[助手答案](fixtures/personal-understanding-lens/sample-b/sources/agent-answer.md) |
| C | [订正](fixtures/personal-understanding-lens/sample-c/sources/owner-correction.md)、[助手答案](fixtures/personal-understanding-lens/sample-c/sources/agent-answer.md)、[当前记录](fixtures/personal-understanding-lens/sample-c/task-context.json)、[历史标记](fixtures/personal-understanding-lens/sample-c/history.json) |

身份合同 `applicable=false`：本票仅解释规则、合成材料和验收文档；不改变认证、绑定、资源归属或写入接口，原有单 owner 与内容确认合同继续适用。
当前阶段是源码交付；私人安装、完整安装及跨客户端验收未在本轮执行。Rust 源码、CLI/schema 与 helper 未修改，本机不运行 Rust rebuild/test，PR 继续执行仓库既有五项必需 CI；结果与合入指针以 PR 为准。

## 你的实际试用反馈

2026-10-30，你选定“订正记录后的旧结论失效”，并授权查看已有记录。
只读检查找到既有 #155 隔离联验任务：固定日常 helper 读取版本 12 的四条当前记录，并按两个指定 ID 查看替代链；无遗漏、无当前候选，三个报告引用的哈希匹配。
这是真实 CLI 读取一个已有的合成验收任务，不是个人真实学习数据。没有跨任务搜主题，也没有写 ledger。
读取正文和 binding 保持私有，不复制到仓库。

当时可追溯的观察是：已有记录展示直接指令替代，但没有你的机制解释，也没有派生候选失效样本。
随后你在本聊天的提示场景中说明：L 需要重新审阅，先前确认不应阻止重新检查，并提出删除、保留反例或拆分观点等后续选择。
这补充了一段有来源、可纠正的局部解释；不是独立完成任务或已掌握的证明，也没有授权执行这些后续选择。

精确合同补充：L 必须实际依赖 A，且 A 经既有订正机制变成 B；满足此前提时，L 自动失效并退出默认有效读取。
先前确认不会消除来源依赖。时间流逝或另写一条无关 B 本身不触发失效；后续需从当前来源重新审阅。

**你的评价：无法判断。理由：都是英文太长了我看不懂。**
初版没有得到“有用”的反馈；短中文修订不能追溯改变这次评价。真实个人学习记录及后续使用效果仍未验证。
