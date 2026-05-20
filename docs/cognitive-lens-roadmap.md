# Cognitive Lens Memory Roadmap

> MemoryNexus 的产品方向从普通 agent memory 调整为 cognitive lens memory。

## Phase 0 TODO: 理论内核入仓

目标：把 cognitive memory 的理论基础固化为项目文档，作为后续 Rust 重构的稳定依据。

- [x] 新增 [Cognitive Manifesto](cognitive-manifesto.md)，明确项目不是普通 agent memory，而是探索 cognition formation。
- [x] 新增 [Cognitive Concepts](cognitive-concepts.md)，解释 Memory、Reflection、Concept、Belief、Relation、Contradiction、CognitiveSpace、Lens、CognitiveEvent、CognitiveState。
- [x] 新增 [Cognitive Architecture](cognitive-architecture.md)，确定 Functional Core + Imperative Shell 的 Rust-first 工程形态。
- [x] 新增 cognitive lens ADR，固化 Memory Space/Cognitive Space 不属于 Agent 的长期决策。
- [ ] 后续实现前，将旧 README、architecture、TODO 中的普通 memory app 口径逐步收敛到 cognitive memory 口径。

## 定位

MemoryNexus 不是给单个 Agent 追加一块私有长期记忆，而是构建一个可被多人、多 Agent、多应用共同使用的 **Cognitive Space**。Cognitive Space 保存事实、事件、材料、上下文、关系和派生解释；Agent 只是临时执行者，不拥有 memory。

在这个模型里，**Lens** 是认知视角或 interpretation strategy。Lens 决定如何读取、筛选、组织和解释 Cognitive Space 中的内容，例如“家庭成长记录”“学习复盘”“健康观察”“项目上下文”“风险审查”。同一份 memory 可以被多个 Lens 解释出不同的结构和意义。

## 核心原则

1. **Cognitive Space 是核心所有权边界**
   - 记忆归属于空间，而不是 Agent。
   - 空间可以代表家庭、个人、项目或组织。
   - 权限、检索、摘要、派生洞察都以 Cognitive Space 为入口。

2. **Agent 不拥有 memory**
   - Agent 可以写入、读取、检索和生成派生内容。
   - Agent 的身份、模型和 prompt 可以替换，不影响 Cognitive Space 的连续性。
   - Agent 输出必须能追溯到空间内的原始 memory、检索结果或 Lens 配置。

3. **Lens 是解释策略**
   - Lens 不复制 memory，也不成为新的存储孤岛。
   - Lens 定义关注点、过滤规则、排序方式、摘要风格、冲突处理和输出结构。
   - Lens 的结果可以缓存或持久化为派生视图，但必须保留来源和可重算性。

4. **Rust-first 后端承载主路径**
   - 新增 API、数据库访问、向量检索、AI 编排默认落在 Rust + Axum 服务。
   - 语义检索闭环仍是 P0 基础能力：Embedding -> Qdrant -> Rust search API。

## 概念模型

```text
Cognitive Space
  ├── Raw Memory
  │   ├── text / image / audio / video
  │   ├── metadata
  │   └── source provenance
  ├── Semantic Index
  │   ├── embeddings
  │   ├── vector payload
  │   └── search filters
  ├── Relationships
  │   ├── tags
  │   ├── entities
  │   ├── time
  │   └── references
  └── Lens Views
      ├── interpretation strategy
      ├── query plan
      ├── summary style
      └── derived insights

Agent
  ├── uses Cognitive Space
  ├── applies Lens
  └── produces traceable output
```

## Phase 1 TODO: Cognitive Space 基础闭环

目标：把“记忆归属于空间”落成最小可用模型，并完成语义检索闭环。

- [x] 定义 Cognitive Space 数据模型：空间 ID、成员、权限、默认 Lens、创建者和审计字段。
- [x] 将现有 memory CRUD 明确绑定到 Cognitive Space，不再以 Agent 作为归属主体。
- [x] 完成 Embedding -> Qdrant upsert -> Rust search API -> 召回结果返回的端到端路径。
- [x] 为向量 payload 补齐 `space_id`、`memory_id`、`source_type`、`created_at`、`visibility` 等过滤字段。
- [ ] 补充注册登录、创建记忆、语义搜索召回、摘要生成的端到端自动化验收。
  - [x] register -> space -> memory add -> keyword search -> semantic search 的本地 acceptance test。
  - [ ] 摘要生成路径验收。
- [x] 更新 API 文档，明确 memory 创建、搜索和摘要接口都以 Cognitive Space 为核心上下文。

### Phase 1A TODO: Cognitive Space 最小落地

目标：先让 Space 成为可创建、可列出、可用于 memory/search 的真实边界。

- [x] 新增 `cognitive_spaces` 和 `cognitive_space_members`。
- [x] 注册用户时自动创建默认 personal Cognitive Space。
- [x] `memories` 增加 `space_id`，创建、列表、搜索默认使用用户默认空间。
- [x] Space REST API 支持 create/list/get。
- [x] CLI 支持 `space create/list`，并支持 `memory add/list --space`、`search --space`。
- [x] Qdrant payload 和 filter 加入 `space_id`。
- [x] 本地 smoke test 覆盖迁移后的 register -> space list -> memory add -> search。
- [x] 完成 `source_type`、`created_at`、`visibility` 等更完整的向量 payload 字段。

### Phase 1B TODO: Semantic Index 基础闭环

目标：让 Cognitive Space 内的 memory 能被稳定写入 Semantic Index，并通过 Rust search API 语义召回。

- [x] 新增 deterministic local embedding provider，用于本地和 CI 友好的语义链路验证。
- [x] Rust 服务启动时根据 `QDRANT_URL` 和 `QDRANT_COLLECTION` 自动确保 collection 存在。
- [x] memory create 使用应用级 embedder，不在接口路径临时创建 provider。
- [x] memory embedding upsert 写入 `space_id`、`memory_id`、`source_type`、`created_at`、`visibility` 等 provenance 字段。
- [x] semantic search filter 使用 `space_id`，保持 Cognitive Space 级隔离。
- [x] 增加 register -> space -> memory add -> semantic search 的自动化端到端测试。
- [ ] 将摘要生成接入同一 Space-scoped semantic recall 结果。

### Phase 1C TODO: 自动化验收闭环

目标：把可手动验证的 CLI MVP 固化为可重复运行的本地验收。

- [x] 新增 ignored acceptance test：`tests/phase1c_acceptance.rs`。
- [x] 验收覆盖 API 启动、注册、Space 创建、memory 创建、keyword search、semantic search。
- [x] 使用 `MEMORYNEXUS_EMBEDDING_PROVIDER=local`，不依赖外部 embedding API。
- [ ] 摘要生成路径纳入验收。
- [ ] 使用独立 test database 和可清理的 Qdrant collection，降低本地状态污染。

### Phase 1.5 TODO: CLI MVP 试用入口

目标：在前端完成前，用一个很薄的 CLI 先验证后端 API 和最小记忆闭环。

- [x] 新增 `memorynexus-cli` Rust binary，作为现有 REST API 的无状态客户端。
- [x] 支持 `health`、`auth register/login`、`memory add/list/get/delete`、`search --semantic`。
- [x] 默认输出 JSON，便于人类调试，也便于 Agent 调用和解析。
- [x] 使用 `MEMORYNEXUS_API_URL` 和 `MEMORYNEXUS_TOKEN` 配置，不在第一版持久化 token。
- [x] CLI 支持 `space`、`memory --space`、`search --space`，Phase 2A 已追加 Lens create/list/get。

## Phase 2 TODO: Lens 最小模型

目标：把 Lens 从 prompt 概念收敛为可配置、可复用、可审计的解释策略。

- [x] 定义 Lens 最小数据模型：名称、适用空间、策略、检索模式、输出格式、创建者和审计字段。
- [x] 新增 `lenses` 持久化表，并预留 `lens_runs` 作为后续运行 provenance 的存储边界。
- [x] 新增 Lens repository 与 REST API：create/list/get，访问控制仍以 Cognitive Space membership 为准。
- [x] 新增 CLI 命令：`lens create`、`lens list`、`lens get`。
- [x] 端到端验收覆盖 CLI 驱动的 Lens create/list/get。
- [ ] 支持内置 Lens：默认回顾、家庭成长、学习复盘、项目上下文。
- [ ] 在 search/summarize 路径中引入 `lens_id`，由 Lens 决定检索过滤、排序和摘要风格。
- [ ] 记录 Lens 运行 provenance：输入 query、命中的 memory、使用的策略版本、生成时间和输出。
- [ ] 区分 Lens 配置与 Lens 运行结果，避免把派生解释误当作原始 memory。
- [x] 为 Lens CLI 解析、API serde 和 repository 增加基础测试。
- [ ] 为 Lens Run execution 增加单元测试和端到端验收。

## Phase 3 TODO: Cognitive Lens 工作流

目标：让 Lens 能组织多步认知流程，而不把流程状态绑定到某个 Agent。

- [ ] 增加 Lens Run 概念，表示一次可追踪的解释过程。
- [ ] 支持多步骤解释：检索、聚类、冲突检查、摘要、行动建议。
- [ ] 支持多 Lens 对同一 Cognitive Space 给出不同解释，并展示差异来源。
- [ ] 增加派生洞察存储：结论、置信度、引用 memory、生成策略版本。
- [ ] 增加人工校正入口：用户可以接受、隐藏、修正或标记 Lens 解释。
- [ ] 将 Agent 执行日志与 Cognitive Space/Lens Run 关联，而不是保存为 Agent 私有记忆。

## Phase 4 TODO: 多主体协作与开放接口

目标：让 Cognitive Space 成为家庭、应用和外部 Agent 的长期认知底座。

- [ ] 支持外部 Agent 通过开放 API 接入 Cognitive Space，但只能基于权限读写空间。
- [ ] 增加空间级策略：谁可以写入、谁可以运行 Lens、哪些 Lens 可以产生持久化洞察。
- [ ] 支持 Lens marketplace 或模板库，允许用户复用解释策略。
- [ ] 增加跨空间检索和授权分享，保持默认隔离。
- [ ] 完善审计与安全：访问日志、派生内容来源、删除和导出能力。
- [ ] 建立生产监控：Lens 成本、检索质量、摘要质量、运行延迟和失败率。

## 非目标

- 不把 MemoryNexus 做成某个 Agent 框架的私有记忆插件。
- 不让 Agent 身份成为 memory 的所有权边界。
- 不在 Lens 中复制完整 memory 形成新的数据孤岛。
- 不重新引入第二套后端主线。

## 验收口径

- 用户创建的 memory 必须能明确归属到 Cognitive Space。
- 任意 Agent 可被替换，已有 memory、索引、Lens 配置和派生视图仍可继续使用。
- Lens 输出必须能说明它使用了哪些 memory、策略和时间范围。
- 语义搜索、摘要和洞察生成默认通过 Rust 服务主路径完成。
