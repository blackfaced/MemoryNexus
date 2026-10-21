# 每周日记解读

在现有 MiniMax／微信对话里主动说“看看这周日记”或“让策展人和生活编辑看看”。
默认范围是北京时间最近七个完整日，不含今天；也可以明确指定日期。入口仍是用户
主动查询，未增加定时任务或主动推送。

## 会得到什么

- 本期范围、有效记录条数和实际记录日数。
- 策展人的短笺：哪些经历、乐趣或进展值得留下。
- 生活编辑的短笺：时间和注意力的安排是否仍被自己认可，并保留其他解释。
- 两种视角的分歧或共同关注点，以及一个回忆具体经历的追问。

有有效的往期稿和已确认的回应时，解读会接上它们。事实来自正式账本；上期解读只是
当时的解释。记录少时缩短篇幅，没有记录时不编写两封信。缺记录不代表没有经历，
没有结果记录不代表没有执行。

回答追问不自动写入账本。Agent 先展示准备记录的简短摘要，用户确认后才保存事实。
阅读稿另存为本地派生文件，不能代替正式记录，也不能代替十四天验收的实验结果回看。

## 本地实现与维护

本功能在 `codex/weekly-perspectives` 工作分支，尚未合入。规则入口是
[memorynexus-feedback/SKILL.md](../integrations/minimax/memorynexus-feedback/SKILL.md)，
内容规则是 [weekly-perspectives.md](../integrations/minimax/memorynexus-feedback/references/weekly-perspectives.md)，
读取与保存由 [weekly_review.py](../integrations/minimax/memorynexus-feedback/scripts/weekly_review.py)
完成。两个身份由同一个 Agent 顺序阅读，不依赖旧 Lens runtime。

当前实现沿用已有单一 owner 的本地配置：

- CLI：`/Users/mac/.local/bin/memorynexus-feedback-cli`
- 正式账本：`/Users/mac/.local/share/memorynexus/dogfood-2026-08-30.sqlite`
- 派生阅读稿：`/Users/mac/.local/share/memorynexus/reflections/`

账本位置必须与主 Skill 一致；不得改用测试用的 `ledger.db`。这份现有机器配置不是
通用安装器，干净 Mac mini 安装仍由 [#282](https://github.com/blackfaced/MemoryNexus/issues/282)
验收。阅读稿和测试产物不得加入 Git。

更新现有安装时，同时同步以下三个文件，并保持相对目录结构：

```text
SKILL.md
references/weekly-perspectives.md
scripts/weekly_review.py
```

本机有全局 Skill 和 Agent 专属副本，均需核对版本。`context` 每次返回当前安装的
`reading_rules`，避免旧对话沿用过期规则。不要把单个脚本的同步误认为整套规则已更新。

## 验证

在仓库根目录运行：

```sh
python3 -B -m unittest discover -s integrations/minimax/memorynexus-feedback/scripts -p 'test_weekly_review.py'
git diff --check
```

这 15 项测试使用临时文件和合成记录，覆盖北京时间日期边界、订正和撤回过滤、空白期、
过期草稿、无效引用、往期稿失效、独立重新读取，以及正式账本缺失时不创建替代账本。
工具只调用 `observation-history` 和 `review`，保存阅读稿不调用权威写命令。

每次真实阅读仍须核对当前证据和每句解释：自动校验只能检查引用及来源变化，不能保证
自然语言解释没有过度推断。曾经通过一次人工核对，不代表 MiniMax 之后每次生成都合格。

2026-09-21 的本机核查已验证现有两个 Skill 副本与工作分支的三个文件一致，实际
`context` 能取回正式记录及有效上期稿，第二期阅读稿已在本机保存。这是本机使用证据；
不是干净安装、手机微信端到端验证，也不是十四天验收通过的结论。

同日还在临时新路径恢复了一份正式账本的一致备份：`export`、`review` 和
`observation-history` 的结果均与恢复前相同，原账本读回也未改变。临时备份及恢复副本
已清理，不含正文的核查结果留在本机 `verification/` 目录。这项核查不改变 #282
其余安装验收项的状态。

本变更不修改 Rust 行为；这里的针对性验证不替代代码 PR 所需的 Format、Clippy、
Build、Test 和适用的 integration gate。备份与恢复命令见
[feedback-ledger-recovery.md](feedback-ledger-recovery.md)。
