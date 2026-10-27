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
| 安装边界 | `integrations/task-memory/install.py`、`launch.py`，#325 | 现有 launcher 调用 workflow；不等于客户端 material bridge；执行包必须与 binding 相同 |
| 复查证据 | 新 adapter 合同 | 只接收原 read evidence；校验同目录 result 引用；check evidence 不可递归冒充原 read；保留 created_at |
| 外部副本 | 新 adapter 合同 | v1 没有外部排除证明接口，external 始终 unknown 并阻止当前材料；task 不假验 managed 文件 |
| 生命周期 | 新 adapter 合同 | 24 小时限制复用；没有后台进程不能承诺届时物理删除；限定可验证材料的命令触发清理 |
| 失败与确定性 | 新 adapter 合同 | 固定 code/exit 映射、字段顺序、UTC 时间；历史 tombstone 可读，不伪造正文 |
| 验收与范围 | C01–C12、ADR-027/028/029/030 | 三个实现切片顺序交付，最终仍需材料与客户端动作双断言；不放宽真实 gate |

文档审阅不证明 adapter 已运行。既有 22 次公开 CLI 探针保留原版本，不重标为最新源码
验收。此次仅文档修改，验证 JSON/摘要/相对链接与 diff，不需新增 Rust 测试。

## #317 关闭核查

GitHub issue 显示 `state_reason=completed`，关闭事件时间为
`2026-10-26T12:06:47Z`（北京时间 20:06:47），actor 为 `blackfaced`；事件没有
关联 commit、GitHub App 或理由，issue comments 为 0。只能确认关闭记录，**没有证据
说明为何判定完成**，不能推断是特定 PR 自动关闭或后续能力已经交付。

#317 仍包含第二阶段关系治理、第三阶段页面及真实验收未完成项，因此恢复为开放的父级
跟踪，更新已合入的 #321/#324/#323 状态并链接实现任务。关闭原因若以后取得新证据，
在同一跟踪中核实；本记录不猜测操作者动机。
