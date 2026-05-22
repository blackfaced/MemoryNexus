# ADR-011: Contradiction Lifecycle

## 状态

✅ 已接受

## 背景

在 cognitive lens memory 中，Contradiction 不是普通错误，也不一定需要立即消除。
同一组 Memory 和 Belief 可能在不同 Lens 下同时成立，例如情绪视角下“保护了关系”，
系统视角下“增加了长期成本”。

如果只把 Contradiction 作为静态 tension 字段保存，系统无法表达它是否仍需处理、
是否已经被接受为多元真相、是否需要用户判断，或者是否已经被新证据替代。

## 决策

`Contradiction` 是带生命周期的 domain object。

状态包括：

- `Detected`
- `Acknowledged`
- `Resolved`
- `AcceptedAsPlural`
- `Ignored`

resolution mode 包括：

- `Resolved`
- `AcceptedAsPlural`
- `StaleConflict`
- `NeedsUserJudgment`
- `DomainSpecific`

Contradiction 必须保留来源：

- `source_memory_ids`
- `belief_ids`
- `lens_ids`
- `confidence`
- `updated_by_event`

`CognitiveProfile` 只投影 unresolved contradictions，也就是 `Detected` 和
`Acknowledged`。`Resolved`、`AcceptedAsPlural` 和 `Ignored` 仍保留在
`CognitiveState` 中，但不进入默认 Profile。

## 后果

正面：

- 可以表达“矛盾已解决”和“矛盾被接受为多元真相”的区别。
- Lens 可以把 contradiction 当作认知信号，而不是简单错误。
- Profile 更适合作为 LLM / MCP / UI 的紧凑上下文，只携带仍需处理的张力。
- 后续 contradiction resolution 可以联动 Memory salience，把 stale conflict 对应的
  Memory 或 Belief 降权，而不是删除。

负面：

- Contradiction 查询和 UI 会多一个生命周期维度。
- 后续持久化需要迁移数据库 schema，不能只依赖当前 domain model。
- Lens Run 要真正展示 unresolved contradictions，还需要把持久化 CognitiveState
  接入运行上下文。

## 相关决策

- ADR-002: Cognitive Lens Memory 产品方向
- ADR-010: Memory Salience and Automatic Forgetting
