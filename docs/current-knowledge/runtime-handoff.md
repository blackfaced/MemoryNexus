# 当前知识 v1 实施 handoff

状态：#318 于 2026-10-27 完成 [审阅](contract-review.md)，[合同](../current-knowledge-contract.md)
已冻结。执行真源是 #317 链接的三个实现 issue；每个 issue 固定 #318 冻结 commit，独立
worktree/branch，按下列依赖顺序实施。本文定义分工，不替代 issue 验收。

## G2：task read/check

实现 task current/history read、同一 R/I 快照、候选元数据、原 read 证据校验与 check、
安装/配置/hash 检查、严格输入、有界 CLI transport、private artifacts 与生命周期。
只调用原 context/workflow-status。repository read 与 launch 返回明确 unsupported_version，
不伪造交付能力。保留 frozen binding/request schema；external 或 task managed copy 为 unknown。

ownership：新 `scripts/current_knowledge.py`、必要的同目录 `current_knowledge_*.py`，
新 `tests/current_knowledge_fixture.py`、`tests/test_workflow_current_knowledge.py`，
及 `docs/acceptance/current-knowledge.md`。脚本与 tests 均相对 `integrations/task-memory/`。
不改 Rust、已有 workflow*.py 行为、安装器或客户端配置。

公开进程验收覆盖 C01/C02 的 task 订正/重放/冲突、C03 候选和 corrected draft、C04 历史、
C05 失败/空/截断、C06 task 并发与使用前复查、C08 未知副本、C09 隔离、C10 撤回/恢复、
C12 严格 schema/包与材料漂移/权限/到期。C01 的 child 动作和 repository 部分留给 G4/G3；
不得以这些局部通过宣称整条 C01 或 C01–C12 完成。

## G3：repository read/check（依赖 G2）

扩展同一入口的 repository 模式。所有分页固定 R/I；核验完整 typed dependencies、实际
source selector/JSONL prefix、profile/binding、原 plan/receipt、当前 applied publication
与唯一受管区块。自动纳入所有选中 target copies，失败保留人工区块、不 sync 或 neutralize。
复用既有 runner 规则，不更改其生产行为；必要 helper 拆分仍限 current_knowledge_*。

ownership：上述 current_knowledge 脚本、fixture 与新增
`tests/test_workflow_current_knowledge_repository.py`，验收记录。覆盖两 consumer 的
C01/C03/C06/C07/C08/C09/C10/C12；包括 #322 同块新 generation 与多区块 whole-file 情况。
实现和验收不能越过 G2 已接受接口；不以 target 文件反推批准。

## G4：pinned bridge launch（依赖 G2、G3）

实现 launch 使用前完整复查、精确字节 stdin、有界 ack、持久 O_EXCL delivery claim、未知
交付禁止自动重试、交付后清理。复用 G2/G3；合成 bridge 从材料选择 A/B 并实际打开 marker，
禁止在 env/argv/prompt 预塞答案。验证 C01–C12 剩余组合与 C11 字节/动作双断言。

ownership：current_knowledge 脚本、fixture、新
`tests/test_workflow_current_knowledge_delivery.py`、验收记录；必要时 PROTOCOL.md 只链接
已实现入口并标明 prompt-only 边界。不安装生产 hook，不修改现有 launcher/安装器合同。
#325 的 workflow launcher 不等于本 client bridge；实际版本 pin/接线由 #312 承接，真实
MiniMax/Codex/scheduler 由 #301/#313 验收，保留已有 PARTIAL 记录。

## 共用验证与交付

```text
cargo build --locked --bin memorynexus-feedback-cli
MEMORYNEXUS_TEST_BINARY=ABS_TO_BUILT_CLI PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s integrations/task-memory/tests -p 'test_workflow*.py' -v
MEMORYNEXUS_TEST_BINARY=ABS_TO_BUILT_CLI PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_task_memory_bootstrap.py
git diff --check
```

已有 Test CI 执行 workflow discovery。缺 binary 或 skipped 必须失败；使用临时 strip 副本，
保留构建产物。记录 immutable source commit、实际测试计数和每项适用范围。代码 PR 满足
Format/Clippy/Build/Test 与适用 integration gate；不改 Rust 时不新增 Rust 行为测试。

均不做语义去重、关系治理、UI、检索、HTTP/MCP、数据库扩展、模型 SDK、新调度器、私人
ledger 修改或真实部署；不改变 #283。需要新增权威字段、改 runner 或范围外文件时先报告
合同冲突。交付 changed files、命令/结果、未验证项、合同偏差和 PR。
