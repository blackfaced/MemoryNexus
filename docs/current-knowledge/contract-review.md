# #318 合同审阅与冻结记录

2026-10-27，按 owner 的“审阅、无问题后冻结、拆实现 ticket”授权，由 Coordinator
对 `551b9d02dae516f1d21fc559dc75620f40f79fd7` 的合同草案完成审阅，并对最新 main
`46434b06a7304ce421aa8d020657675066f8249e` 复核。结论：下列歧义已原位修正，
**合同 v1 可作为实现依据**。冻结的是本次提交中的 `docs/current-knowledge-contract.md`；
固定提交链接由 #318 保存。改变已冻结行为须先提出合同变更，不能自行增加权威字段。

## 审阅依据与结论

| 检查 | 当前源码/合同依据 | 结论 |
| --- | --- | --- |
| 当前与历史、候选、draft | `src/task_memory/mod.rs` 的 context | 默认排除 draft；候选只有元数据进入新材料；历史与当前用途严格区分 |
| 显式订正/撤回/幂等 | `src/task_memory/lifecycle.rs`、task-memory-contract | 复用原 correct；不能追加 record、换目标或重用 key 改正文 |
| revision/incarnation | `src/task_memory/workflow/reads.rs` | v2 header 可无 workflow 读取；最终 at_revision 与 incarnation 双重检查；不直读 SQLite |
| 外部输出 | `workflow_engine.py`、`workflow_io.py`，#322 | 验证当前批准与原始 plan/receipt、唯一受管区块；不以每次历史 whole-file after_sha 校验当前多区块文件 |
| 安装边界 | `integrations/task-memory/install.py`、`launch.py`，#325 | 现有 launcher 调用 workflow；不等于客户端 material bridge；执行包必须与 binding 相同，脚本按现有 installer 平铺在 package 根 |
| 复查证据 | 新 adapter 合同 | 只接收原 read evidence；校验同目录 result 引用；check evidence 不可递归冒充原 read；保留 created_at |
| 外部副本 | 新 adapter 合同 | v1 没有外部排除证明接口，external 始终 unknown 并阻止当前材料；task 不假验 managed 文件 |
| 生命周期 | 新 adapter 合同 | 24 小时限制复用；没有后台进程不能承诺届时物理删除；限定可验证材料的命令触发清理 |
| 失败与确定性 | 新 adapter 合同 | 固定 code/exit 映射、字段顺序、UTC 时间；历史 tombstone 可读，不伪造正文 |
| 验收与范围 | C01–C12、ADR-027/028/029/030 | 三个实现切片顺序交付，最终仍需材料与客户端动作双断言；不放宽真实 gate |

文档审阅不证明 adapter 已运行。既有 22 次公开 CLI 探针保留原版本，不重标为最新源码
验收。此次仅文档修改，验证 JSON/摘要/相对链接与 diff，不需新增 Rust 测试。

## 冻结版本的路径勘误

对照 #325 installer 的 `destination_path` 映射复核后，修正了原合同把源码目录
`scripts/` 误带入安装路径的问题：运行入口是 `binding.package/current_knowledge.py`。
JSON/schema、hash 算法和权威边界不变；实现及 G2–G4 tickets 固定到此修订提交，
不建立另一套安装布局。

## #317 关闭核查

原因已定位为 PR #321 的关闭关联误触发。其原正文末句为
`This PR does not close #317 or claim deployment.`；虽然自然语言是否定句，GitHub
仍将其中的关闭关键词识别为 issue 关联。GitHub GraphQL 的
`pullRequest(number:321).closingIssuesReferences` 返回 #317，#321 合并时间为
`2026-10-26T12:06:45Z`，#317 关闭事件为 `12:06:47Z`，actor 为 `blackfaced`，
issue 的 state_reason 为 completed。该关联与时间顺序解释了关闭，不是后续能力已验收。

初查仅看 issue timeline（commit_id 为空且无评论）未能定位；补查 PR 正文与 GraphQL
关闭关联后得到上述证据。#321 的句子改为不含关闭关键词的 “Parent #317 remains open
for subsequent phases; deployment is outside this PR.”，避免继续表达错误的关闭关联。

#317 已恢复开放，保留未完成的关系治理、管理页面和真实验收。最新实现按
#327 → #328 → #329 跟踪，PR #326/#330 的源码交付不能代表父任务整体完成。
