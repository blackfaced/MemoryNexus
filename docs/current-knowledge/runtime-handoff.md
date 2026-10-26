# G1 最小实施 handoff（尚未发出）

状态：等待 [#318 合同](../current-knowledge-contract.md) 的固定 commit 审阅。
本文件不是已发布的 ready runtime issue。Planner/Coordinator 审阅应确认范围、失败语义、
schema 和下列验证；把审阅 commit 与结论留在 ticket/PR 后再创建实施 issue。
如发现当前 CLI 缺能力，先反馈合同，不让 worker 自行增加权威字段。

## 目标与依赖

在独立 issue worktree 实现 read/check/launch 接入，使合成 fresh child process 只收到
符合当前快照的材料；显式 correct 继续复用原 CLI。基于已合入 ADR-030/#318 合同，
复用 #310/#311 的 v2 workflow 与 #319 的测试入口。生产部署不属于这个 worker。

## 文件 ownership

- 新 `integrations/task-memory/scripts/current_knowledge.py`：合同 §3 的三个命令、
  strict request/binding、CLI 读调用、快照与文件检查、短期材料、bridge handoff。
- 新 `integrations/task-memory/tests/test_workflow_current_knowledge.py`：真实 CLI 和
  隔离合成仓库的公开进程验收，通过现有 `test_workflow*.py` discovery 自动进入 CI。
- 新 `integrations/task-memory/tests/current_knowledge_fixture.py`：固定无害 child bridge、
  同一合成根中的 A/B 两个目录、只读 marker 动作与私有 delivery 证据。
- `docs/acceptance/current-knowledge.md`：填实测 commit、命令、计数、case 结果和缺口。
- 如需把读/订正操作写入 `integrations/task-memory/PROTOCOL.md`，只链接已实现入口并明确
  prompt-only 边界；不修改所有客户端配置。不改 workflow*.py 的已有生产行为。

这是一份候选 ownership，需随实施 issue 冻结。工具拆分只能留在当前目录、相同接口之内；
要修改 Rust、迁移或既有 runner 合同先回报 Planner/Coordinator。

## 接口与约束

严格实现合同 §3–§5，示例都需保持机器可校验。支持 v2 ledger；不自动迁移。
CLI 只能调用原 context/workflow-status 等公开读接口；不直读 SQLite/导出代替 bounded
status。实际订正由原 correct 完成；该 adapter 不新增写入口或 owner 确认规则。
read 的未知注入状态可输出诊断，但不能生成可以误用的 current 材料。
launch 用 pinned bridge，以原 material 字节作 stdin；回执与实际客户端证据分开。
历史 read 不能 launch。任何漏页、来源/文件变化、恢复 incarnation 变化必须失败。

默认 fixture 的 bridge 是可审计确定性进程：读取 stdin、保留输入摘要、从 current entry
选择 A/B、只打开相应 marker 文件，写出实际动作证据，返回精确 ack。它只证明模拟交付。
不得让测试预先把 B 答案写进 bridge、环境、启动参数或 user prompt。

## 验证矩阵与命令

必须实现 [C01–C12](../acceptance/current-knowledge.md) 全部合成场景：材料+动作双断言，
严格 schema、task 隔离、两条读路径、候选/draft、失效/历史/撤回/恢复、并发/TOCTOU、
失败/丢回执、source/file/profile/approval drift、private/no-symlink、limit/空/截断。
不能以 `client:"codex"` 或 mocked subprocess 代替真实 CLI 进程。

```text
cargo build --locked --bin memorynexus-feedback-cli
MEMORYNEXUS_TEST_BINARY=ABS_TO_BUILT_CLI PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s integrations/task-memory/tests -p 'test_workflow*.py' -v
MEMORYNEXUS_TEST_BINARY=ABS_TO_BUILT_CLI PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_task_memory_bootstrap.py
git diff --check
```

缺 CLI 或 skipped 必须失败；由 #319 同一 Test job 执行，保留其他 checks。
不修改 Rust 时不新增 Rust 行为测试；若批准后的范围确需 Rust，再按 AGENTS 运行相关检查。
记录 Linux CI 与 macOS 合成进程结果。必要时注入可控并发写入验证读快照重试和 check 拒绝，
不要靠 sleep 猜 race，不改变生产路径安全规则以迁就 fixture。

## 非目标与完成边界

不做语义去重、关系治理、UI、索引/检索、HTTP/MCP、数据库扩展、模型 SDK、调度器、
私人 ledger 操作、真实客户端账号接入或安装/迁移。不得改变 #283 的 gate。
真实 bridge/hook 安装与注入盘点交给 #312；实际 MiniMax/Codex/scheduler 证据回填
#301/#313，保留它们既有部分通过结果。实现 worker 不替 owner 作继续使用决定。

交付：commit/PR、changed files、每条 case 的证据、验证命令/结果、未验证项和合同偏差。
公开附件只留合成材料；真实私有会话和绝对私人路径留在目标机器。
