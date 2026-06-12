# AGENTS.md

本文件给 Codex 和其他代码 Agent 使用。进入仓库后请先阅读本文件，再开始修改。

## 项目主线

- MemoryNexus 是 cognitive lens memory 系统，后端主线是 **Rust-first**。
- Rust + Axum crate 位于仓库根目录，是唯一继续演进的主后端。
- 记忆归属于 `CognitiveSpace`，不归属于 Agent。
- 历史 Python/FastAPI 和空前端骨架已移除；不要重新引入双后端主线。
- Phase 4 第一版 UI 是 Rust 服务直接提供的静态 Thought Review MVP，入口在
  `web/thought_review.html`，路由在 `src/api/web.rs`。
- 不要在没有 ADR 的情况下引入 React/Vite/Next、Node dev server、BFF 或第二套
  frontend/backend 主线。
- 长期方向见 ADR-014：MemoryNexus 正在扩展为 namespace-based long-term
  feedback substrate，并引入 MemoryAtom / CognitiveScene / Lens-based
  CognitiveProjection 的 memory lifecycle；但 `CognitiveSpace` 仍然是 ownership /
  permission boundary。
- Local-first Trace Learning Runtime 见 ADR-016：MemoryNexus 会引入 Trace 记录交互、
  runtime metrics、生成对象和用户反馈，用于 local-first / trace-driven feedback
  learning；但不要把项目改成完整 local agent runtime、model catalog 或 inference
  engine。
- Sleep-based Memory Consolidation 见 ADR-017：MemoryNexus 采用 Wake / Sleep /
  Dreaming 架构。前台 Wake 路径保持低延迟并生成 Trace；后台 Sleep 路径离线整合
  Trace / Memory / FeedbackLoop；Dreaming 生成候选练习、复盘问题、场景模拟或下一步
  计划。不要把这条路线实现成模型参数训练、RL self-modification、local inference
  runtime，或每次输入都同步运行完整 cognitive pipeline。
- Supabase 接入边界见 ADR-015：Supabase 首先是托管 PostgreSQL 兼容目标，不是新的
  backend 主线。Auth / Storage / Realtime 只能作为后续 adapter 单独推进。
- EverMemOS / EverOS 可作为 memory lifecycle 的外部参考，但不要把 MemoryNexus 改成
  agent memory retrieval 系统。当前边界是：EverMemOS 偏 memory for agent reasoning；
  MemoryNexus 偏 user-owned cognitive perspective and feedback loops。

## 架构决策

- 重要决策必须写入 `decisions/`，使用 ADR 形式：`ADR-00X-short-title.md`。
- 新增 ADR 后必须更新 `decisions/README.md`。
- 不要把长期架构决策只写在 `docs/` 或对话总结里。
- 当前 Rust-first 主线见 `decisions/ADR-009-rust-first-backend.md`。
- Thought Review UI MVP 见 `decisions/ADR-013-thought-review-ui-mvp.md`。
- Namespace / FeedbackLoop 长期模型见 `decisions/ADR-014-namespace-feedback-loop.md`。
- Local-first Trace Learning Runtime 见
  `decisions/ADR-016-local-first-trace-learning-runtime.md`。
- Sleep-based Memory Consolidation 见
  `decisions/ADR-017-sleep-based-memory-consolidation.md`。

## 开发规则

- 新增 API、数据库访问、对象存储、向量检索、AI 编排默认落在 Rust 服务。
- 如果接入 Supabase，默认先验证 `DATABASE_URL` 指向 Supabase Postgres 的兼容性。
  不要绕过 Rust API 直接用 Supabase REST / PostgREST 操作核心表。
- 不要用 Supabase RLS 取代 MemoryNexus 的 `CognitiveSpace` membership / Rust 权限检查。
  Supabase Auth 如需接入，必须映射到本地 users 表并保留 Space 权限边界。
- 修改 Rust 行为时优先补单元测试或端到端验收；至少运行 `cargo test`。
- 修改静态 UI 时优先保持现有 `web/thought_review.html` 轻量实现，除非 issue/ADR
  明确要求升级前端栈。
- UI 文案优先使用用户语言：thought / perspective / review / recurring theme /
  inner tension。不要把 `Memory`、`Lens Run`、`CognitiveState` 等后端术语过早暴露为
  普通用户主入口。
- `learning.stem` UI 文案优先使用家长和学习者能懂的语言：practice / answer /
  mistake pattern / feedback / next exercise / weekly learning review。不要把
  `MemoryAtom`、`CognitiveScene`、`CognitiveProjection` 暴露为主标签。
- `Namespace` 只是 `CognitiveSpace` 内的领域分区，不是新的权限边界；不要把权限从
  Space membership 挪到 Namespace。
- `FeedbackLoop` 是长期方向，落地时应从具体 namespace 的最小验收场景反推字段，
  不要一次性做 learning / piano / chess / drawing / programming 全部产品。
- `MemoryAtom`、`CognitiveScene`、`CognitiveProjection` 是 Phase 5 lifecycle
  概念。先用 fixtures / prototype 验证 atomization、consolidation 和 Lens
  projection，不要在没有 issue/ADR 验收的情况下直接铺复杂 schema。
- Observe / projection 相关实现必须区分 `fast`、`focused`、`deep` 三种模式。
  不要让每次用户输入都同步运行 atomization、multi-lens projection、belief update
  和 contradiction detection。
- 本地没有 Rust 工具链时，可以用 Docker 验证：

```bash
docker run --rm \
  -v "$PWD:/workspace" \
  -w /workspace \
  rust:latest cargo test
```

- 不要把生成的 `*.profraw`、`target/`、临时测试产物提交进仓库。
- 不要回退用户已有改动；遇到无关脏文件时保持原样。

## 分支与 PR 规则

- 不要直接在 `main` 上开发。每个 issue / 子任务使用独立分支或 worktree。
- `main` 通过 GitHub branch protection 保护：必须走 PR，且 `Format`、`Clippy`、
  `Build`、`Test` 必须通过。
- 当前只有一个主要开发者，不要求 approving review；但 PR 仍必须通过 CI。
- 默认合并方式使用 squash 或 rebase，避免 merge commit；PR 合并后删除分支。
- 开 PR 前至少运行：

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D clippy::all
```

如果只改 Markdown 文档，可以说明未跑 Rust 测试的原因；但涉及 Rust 代码或 UI 行为时
必须跑对应验证。

## Issue / 子 Agent 工作流

- Agent 分工默认分三类：
  - **Planning / Architecture agent**：只负责路线图、ADR、issue 拆分、验收标准和架构边界；
    不直接实现产品代码，除非 issue 明确是 docs / design。
  - **Coordinator agent**：负责确认依赖顺序、创建 worktree / branch、编写子 agent
    handoff prompt、review PR、合并、关闭 issue、清理 worktree / 分支；默认不直接承接
    issue 实现。
  - **Worker agent**：只在独立 issue worktree 中干活，拥有该 issue 的实现责任和文件
    ownership；完成后提交、push、开 PR，并报告 changed files、验证命令和已知缺口。
- `main` / 主仓工作区只用于协调、review、文档规划和轻量状态检查；不要在主仓直接做
  feature implementation。每个实现 issue 必须使用独立 worktree 和同名分支，例如
  `issue-66-supabase-postgres-compat`。
- 每个 worktree 都要有自己的 issue identity / "soul"：明确 issue 编号、目标、相关
  ADR、文件 ownership、非目标、验证命令和最终交付格式。子 agent 不能假设其它未合入
  worktree 的改动存在。
- Worker agent 必须知道自己不是独占整个仓库：不要回退他人改动；遇到跨 issue 依赖、
  文件 ownership 冲突、需要修改主线架构或验收不清时，停止并报告给 Coordinator。
- Coordinator review 时只信证据：看 diff、测试、CI、issue 验收和 ADR/AGENTS 边界；
  不把 worker 的完成声明当作验收结果。
- 子 Agent 开工前必须阅读本文件、相关 issue、`README.md`、`docs/TODO.md` 和相关
  ADR。
- 子 Agent 通用 handoff 模板见 `docs/subagent-issue-workflow.md`。
- 如果 issue 描述不足，不要猜大方向；先补充 issue 评论或拆小任务。
- 每个 issue 应有明确验收标准、相关文件、非目标和验证命令。
- Phase 4 UI issue 默认基于 Rust-served Thought Review UI 继续演进，不另建前端工程。
- Phase 5 Namespace / FeedbackLoop issue 默认先做设计和最小模型/API 方案，不直接铺开
  多个垂直产品。
- STEM Learning Feedback 是第一产品 MVP 候选，产品 namespace 为 `learning.stem`。
  涉及产品入口的 Phase 5 issue 默认优先服务 parent-assisted STEM practice feedback
  loop，并以 elementary fraction word problems 作为第一验证任务；不要回到泛化学习平台。
- Phase 5 Memory Lifecycle issue 默认围绕 `Memory -> MemoryAtom -> CognitiveScene
  -> CognitiveProjection` 做小实验，不要把它实现成通用 agent retrieval engine。
- 涉及 ObserveMode 的 issue 必须明确前台低延迟行为、后台异步处理和用户主动 deep
  review 的触发条件。
- Trace / runtime metrics issue 默认先做 contract、最小 schema 或 lightweight capture。
  不要实现 OpenJarvis 式完整本地 agent runtime、模型微调、model catalog 或 inference
  backend。
- Sleep Engine / Dreaming issue 默认先读 ADR-017，并按 Trace -> SleepCycle ->
  ConsolidationResult -> DreamCandidate -> effectiveness evaluation 的顺序推进。第一版
  Dreaming 必须优先支持 deterministic / local-first 路径；不要默认接云模型，不要添加
  scheduler，不要把 Sleep 或 Dreaming 作为普通用户主入口术语。

## P0 优先级

1. Embedding -> Qdrant -> Rust search API 的语义检索闭环。
2. 注册登录、创建记忆、搜索召回、摘要生成的端到端验收。
3. Lens 最小模型：Lens 配置、Lens Run、可追溯输出。
4. 文档口径统一：README、architecture、TODO 都应以 Rust 主线和 Cognitive Space 为准。

## 当前产品入口

- Thought Review 是 reflective demo 和项目演讲入口：写下一条混乱想法，用多个
  perspective 展示 MemoryNexus 如何解释同一份 memory space。
- 第一产品 MVP 候选是 STEM Learning Feedback，产品 namespace 为 `learning.stem`；
  第一验证任务是 parent-assisted elementary fraction word problems feedback loop，
  用练习、作答、错因、反馈、下一题和周学习报告验证长期反馈价值。
- Thought Review 属于 reflective namespace，可视为 `personal.thoughts` 的 demo。
  `learning.stem` 属于 Skill Namespace，必须通过单独 issue/验收场景推进。

## 文档位置

- `README.md`：面向使用者的项目入口和快速开始。
- `docs/`：API、开发、部署、路线图等说明。
- `decisions/`：架构决策记录，所有长期决策放这里。
- `src/`：Rust 源码。
- `migrations/`：SQLx 数据库迁移。
