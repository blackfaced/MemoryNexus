# ADR-018: MemoryNexus as Long-term Feedback Engine

## 状态
✅ 已接受

## 背景

MemoryNexus 早期定位经历过几个阶段：

- cognitive lens memory system;
- AI thought organizer;
- user-owned cognitive memory layer;
- namespace-based feedback substrate.

这些方向都保留了有价值的部分，但如果产品继续被描述成普通 AI memory app、second
brain、agent memory store、RAG profile 或 connector platform，就会和
Supermemory / Mem0 / OpenJarvis 等生态混在一起。

MemoryNexus 真正要回答的问题不是：

```text
AI 如何记住更多？
```

而是：

```text
系统如何基于长期轨迹，持续生成更好的反馈和下一步行动？
```

## 决策

MemoryNexus 的项目定位调整为：

```text
A local-first, namespace-based long-term feedback engine for personal cognition
and skill acquisition.
```

中文理解：

```text
MemoryNexus 是一个本地优先、基于 namespace 的长期反馈引擎，用来把人的经验、表现、
练习和对话转化为记忆、复盘、成长模型与下一步计划。
```

生态分层：

- OpenJarvis: Local Personal AI Runtime。
- Supermemory / Mem0: Memory Runtime / Memory Cloud。
- MemoryNexus: Memory Evolution / Feedback / Growth Engine。

MemoryNexus 应聚焦：

- Trace-driven learning。
- Feedback loop。
- GrowthModel。
- Namespace-based domain spaces。
- Sleep-based consolidation。
- Planning / next-action generation。
- Long-term skill acquisition and personal cognition。

MemoryNexus 不优先竞争：

- 通用 memory API。
- connector 数量。
- RAG 检索质量榜。
- agent profile cloud。
- local model runtime。
- agent framework。

## 架构影响

核心 Engine 对象：

- `Namespace`
- `Trace`
- `MemoryAtom`
- `CognitiveScene`
- `FeedbackLoop`
- `GrowthModel`
- `SleepCycle`
- `PracticePlan` / `DreamCandidate`
- `Lens`

核心闭环：

```text
Trace -> FeedbackLoop -> GrowthModel -> PracticePlan -> next Trace
```

## 后果

正面：

- 项目差异化更清晰，避开普通 memory/RAG 竞争。
- `learning.stem`、Dictation Coach、Thought Review 都能作为 Engine 的不同上游产品或 demo。
- Trace、SleepCycle、GrowthModel 和 PracticePlan 成为一条连贯路线。
- Local-first 不再意味着做完整 runtime，而是约束反馈引擎的执行策略。

负面：

- 现有文档里仍有 AI thought organizer、STEM Learning、cognitive lens memory 等旧口径，需要逐步统一。
- 旧 API 和 MCP 工具仍偏底层对象，需要通过 Surface Gateway 逐步包装。
- GrowthModel / PracticePlan 还没有生产 schema，需要按 milestones 小步推进。

## 相关决策

- ADR-009: Rust-first 后端主线
- ADR-014: Namespace and Feedback Loop Model
- ADR-016: Local-first Trace Learning Runtime
- ADR-017: Sleep-based Memory Consolidation
- ADR-019: Surfaces vs Adapters vs Engine
- ADR-020: Dictation Coach as First Upstream Product
