# ADR-022: MemoryNexus Brand Semantics

## 状态
✅ 已接受

## 背景

ADR-005 选择 `MemoryNexus` 时，项目仍偏向家庭 AI 记忆中心和第二大脑方向。
随后 ADR-018 将项目定位调整为：

```text
Local-first long-term feedback engine for personal cognition and skill acquisition.
```

这带来一个命名问题：`MemoryNexus` 中的 `Memory` 容易被外部读者理解为照片、
视频、RAG recall、agent memory store 或 second brain，进而误判项目定位。

但项目并未完全离开 memory 语义。当前核心不是保存更多记忆，而是让长期 Trace、
Memory、FeedbackLoop、GrowthModel、SleepCycle 和 PracticePlan 共同形成可演化的
反馈系统。

## 决策

保留项目总名 `MemoryNexus`，但重新定义品牌语义：

```text
Memory = evolving long-term trace, feedback, and growth context.
Nexus  = the engine layer that connects traces, feedback loops, growth models,
         consolidation, and next actions.
```

中文理解：

```text
本地优先的长期反馈引擎，用 Trace 驱动复盘、成长模型和下一步行动。
```

对外描述应优先使用：

```text
Local-first long-term feedback engine for personal cognition and skill acquisition.
```

`Dictation Coach / 每日默写助手` 是第一个上游产品名，不替代 `MemoryNexus`。
面向用户的产品和面向开发者的 Engine 名称保持分层：

- `MemoryNexus`: Engine / repository / binary / MCP / release identity.
- `Dictation Coach`: first product-facing scenario and adapter experience.

## 命名边界

`MemoryNexus` 不应再被解释为：

- family photo or video memory manager;
- generic second brain;
- generic AI memory API;
- RAG profile service;
- agent memory store;
- connector platform;
- local AI runtime.

`MemoryNexus` 可以被解释为：

- memory evolution engine;
- long-term feedback engine;
- growth and practice planning engine;
- namespace-based feedback substrate;
- trace-driven learning and reflection engine.

## 后果

正面：

- 避免仓库、binary、MCP、release、文档和 issue 的全量 rename 成本。
- 保留已有 `MemoryNexus` 品牌连续性，同时修正外部误读。
- 让 Engine 名和产品名分层，避免把 Dictation Coach 写进底层架构边界。
- 与 ADR-018、ADR-019 和 ADR-020 保持一致。

负面：

- `Memory` 一词仍有误读风险，需要通过 GitHub repository description、README
  first viewport、release notes 和 docs copy 持续校准。
- 未来如果项目完全离开 Trace / Memory / FeedbackLoop 语义，可能需要再次评估总名。

## 执行要求

- GitHub repository description 不得继续使用家庭照片、视频、second brain 或泛化
  AI memory wording。
- README 和 docs 中第一次解释 `MemoryNexus` 时，应明确它是 long-term feedback
  engine，而不是 memory app。
- 产品体验可使用独立名称，例如 `Dictation Coach`，但不得把产品角色或产品边界写入
  Engine 命名。

## 相关决策

- ADR-005: 项目命名选择
- ADR-018: MemoryNexus as Long-term Feedback Engine
- ADR-019: Surfaces vs Adapters vs Engine
- ADR-020: Dictation Coach as First Upstream Product
