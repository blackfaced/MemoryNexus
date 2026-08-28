# AGENTS.md

本文件给在 MemoryNexus 工作的 Agent 使用。先读本文件、相关 GitHub issue、
[README](README.md)、[当前路线图](docs/TODO.md) 和
[ADR-027](decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)。

## 当前默认方向

ADR-027 与 [#273](https://github.com/blackfaced/MemoryNexus/issues/273) 是当前产品
方向：MemoryNexus 是运行在 Mac mini 上的 local-first personal experiment feedback
kernel。

目标路径为：

```text
MiniMax Skill / WeChat input / owner-initiated conversation query
                    |
                    v
          compiled local CLI (stable JSON)
                    |
                    v
     SQLite WAL single-file authoritative ledger
                    |
                    v
 Observation -> Recommendation -> Experiment -> Outcome
```

- 权威内核只保留 `Observation`、`Recommendation`、`Experiment` 和
  `Outcome`。所有权威写入都需要 owner 的明确确认。
- 目标 CLI 是唯一主要行为 seam：`observe`、`retract`、
  `add-recommendation`、`start-experiment`、`record-outcome`、`review` 和
  `due`。不要以通用 CRUD、任意 JSON dispatch 或数据库表作为产品接口。
- SQLite（WAL mode）单文件是第一版唯一权威 ledger；必须支持确定性迁移、JSON export、
  一致备份和 restore。
- MiniMax 负责自然语言理解、澄清、未确认草稿和 native scheduled-task wake-up。SQLite
  ledger，而不是 Agent chat history，提供跨 session 连续性。
- 微信是方便输入路径；native scheduled task 的结果由 owner 主动查询既有 MiniMax
  conversation。首版没有可靠的 MiniMax App 或微信主动提醒。MemoryNexus 不实现
  scheduler、daemon、retry worker、微信机器人或 channel framework。
- 不诊断、解读医疗文件、开具治疗建议或声称临床有效性。蚂蚁阿福等工具只是显式来源的
  Recommendation 候选；原始对话、报告、诊断和处方默认不持久化。
- 第三方 memory backend 只能是从 SQLite ledger 可重建的、非权威 recall projection；
  在两个真实实现和一致性测试证明需要前，不增加抽象、provider registry 或第二 backend。

## 已接受目标与当前可运行代码

ADR-027 记录的是**目标契约**，不是现有功能声明。当前可运行的仍是冻结的 legacy Rust
runtime；它尚未实现 SQLite CLI/MiniMax 产品路径。不要把目标命令写成已可运行能力。

[#274](https://github.com/blackfaced/MemoryNexus/issues/274) 已通过：普通微信 MiniMax
session 能执行 owner-approved 本地命令并写入共享状态，独立 native scheduled session
能读到它。cron output 没有 channel context，故结果只能由 owner 主动查询既有
conversation，不能作为可靠主动提醒。不得以聊天历史或 hidden provider memory 替代
ledger，也不得把主动 delivery 加入 #276+ 的实现假设。

## Expand–contract 纪律

执行顺序固定为：

1. #274 已验证 MiniMax 跨 session 本地命令与共享状态，并确认 owner-initiated pull 语义。
2. #275 记录决策、对齐公开定位和冻结旧 roadmap。
3. #276–#281 交付四对象 SQLite CLI、MiniMax Skill、`due` 与恢复路径。
4. #282 在干净 Mac mini 验证安装；#283 运行固定十四天 owner dogfood gate。
5. 仅当 #283 通过后，#284 才切换默认 build/release/CI/docs；#285 与 #286 才可删除
   legacy 接口和 storage runtime。

十四天 gate 开始前固定标准：至少十条有效 confirmed Observation、一个 Experiment、五条
执行更新、一次 evidence-backed review、fresh session 一致读回、最多一次人工故障介入，
以及第十五天 owner 的继续使用决定。失败时先分析价值或摩擦，不扩张架构。

不得做历史数据迁移、dual write、永久 compatibility layer 或预先抽象的 repository/backend
接口。Git history 保存被退役实现。

## Frozen legacy runtime

以下内容在 expand–contract 期间只作为 legacy runtime 的维护、验证或最终删除对象：

- Rust + Axum、PostgreSQL、Qdrant、embeddings/vector search、REST、MCP、Surface Gateway；
- `CognitiveSpace`、Namespace、Trace、FeedbackLoop、GrowthModel、Sleep/Dreaming、
  Lens、Thought Review、Dictation/STEM 与 source Adapter 产品路线；
- M9、M10、M11、Reference Adapter、Study Buddy 和 DeepTutor 的旧 roadmap。

不要为这些路线添加功能、扩张 schema、创建新 issue、标为 ready，或将它们作为 #276+
实现前提。只允许：

- 修复阻止安全运行、审阅或收缩的缺陷；
- 保留对确认、来源、幂等、订正、撤回、执行结果和 evidence gap 的可移植行为证据；
- 在 #283 通过后按 #284–#286 删除旧路径。

已由 `47fa3e0` 交付的 #239/#240 Reference Adapter capability 是冻结历史实现；
[reference-adapter-runtime.md](docs/reference-adapter-runtime.md) 只作历史参考，不能成为
新产品路线或继续扩张的授权。

## 工程与交付规则

- 不在 `main` 上实现；每个实现 issue 使用独立 worktree/branch。
- 不回退他人变更；遇到跨 issue 依赖、文件 ownership 冲突或 ADR/issue 冲突时停止并报告。
- 默认不 commit、push、创建或合并 PR、关闭 issue，除非用户明确要求。
- Issue 是可执行任务的真源。不要用 markdown 镜像取代 Issue；`docs/TODO.md` 只记录
  当前路线与只读 reconciliation 建议。
- Worker 开始前必须读本文件、相关 issue、README、TODO、ADR-027，以及与其 ticket
  明确相关的历史 ADR/legacy contract。
- Worker 最终报告：changed files、验证命令和结果、未验证项、与 ADR-027/issue 的偏差。
- 修改 Rust 行为时至少运行相关测试；docs-only 变更可不跑 Rust 测试，但必须说明原因。
  修改代码的 PR 仍需满足 Format、Clippy、Build、Test 和适用的 integration gate。
- 不提交 `target/`、`*.profraw` 或临时测试产物。

## 当前优先级

1. #275 的 ADR、公开文档与 tracker reconciliation，包含 #274 的 owner-initiated pull
   constraint。
2. #276–#281 的最小 SQLite CLI path。
3. #282 clean-install 与 #283 fixed fourteen-day owner gate。
4. 通过 gate 后的 cutover 和 contraction（#284–#286）。

## 文档位置

- `README.md`：当前用户与仓库入口。
- `decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md`：当前架构决策。
- `docs/TODO.md`：当前依赖顺序和 live tracker 的只读处置建议。
- `docs/architecture/` 与其他旧 runtime 文档：冻结历史参考；不作为当前实现指令。
- `decisions/ADR-001` 至 `ADR-026`：历史决策与可移植行为依据；不定义默认产品路线。
