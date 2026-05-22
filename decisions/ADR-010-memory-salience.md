# ADR-010: Memory Salience and Automatic Forgetting

## 状态

✅ 已接受

## 背景

MemoryNexus 需要支持 automatic forgetting，但项目主线要求 Memory 属于
`CognitiveSpace`，不属于 Agent，也不应该被隐式删除。

如果把 forgetting 实现为物理删除，会破坏可追溯性、历史解释、Contradiction
分析和用户审计。更合适的语义是：Memory 仍在 CognitiveSpace 中，但在默认认知
投影和后续 Lens 上下文中被降低优先级。

## 决策

引入 `MemorySalience` 作为 domain-level 状态：

- `Active`: 默认参与 Profile、搜索排序和 Lens Run 上下文。
- `Deprioritized`: 保留在 CognitiveSpace 中，但默认不进入 Profile。

引入可追踪的降权原因：

- `Stale`
- `Superseded`
- `LowSignal`
- `Contradicted`
- `UserHidden`

引入认知事件：

- `MemoryDeprioritized`
- `MemoryReprioritized`

这些事件通过 `CognitiveState + CognitiveEvent -> CognitiveState` 演化 salience。
降权必须保留 reason 和 event id，恢复也必须保留 event id。

## 后果

正面：

- automatic forgetting 不会破坏 Memory 的所有权和审计链路。
- Profile 可以保持紧凑，只引用当前 active memories。
- 后续搜索、Lens Run 和 UI 可以统一基于 salience 做默认降权。
- Contradiction resolution 可以把被反驳的 Memory 标记为 `Contradicted`，而不是删除。

负面：

- 查询语义会多一层状态：完整召回和默认召回必须区分。
- 数据库和 API 后续需要显式持久化 salience，否则 domain 行为无法跨进程保留。
- UI 需要向用户解释“隐藏 / 降权 / 删除”的区别。

## 相关决策

- ADR-002: Cognitive Lens Memory 产品方向
- ADR-008: 数据库设计
- ADR-009: Rust-first 后端主线
