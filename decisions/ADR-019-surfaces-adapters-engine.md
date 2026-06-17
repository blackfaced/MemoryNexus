# ADR-019: Surfaces vs Adapters vs Engine

## 状态
✅ 已接受

## 背景

MemoryNexus 需要支持 Chat Agent、MCP、CLI、Web App、Mobile App、Dashboard、Voice
Assistant 等不同交互方式。如果直接把“家长端”“孩子端”“对话端”“可视化端”写进核心
架构，Engine 会被上游产品角色污染。

同时，外部 App / Agent 不应该直接操作 Engine 内部对象。它们应该通过能力面访问
MemoryNexus。

## 决策

MemoryNexus 采用三层边界：

```text
Adapter = 怎么交互
Surface = 想做什么
Engine  = 长期如何记忆、反馈与演化
```

### Engine

Engine 是长期反馈内核。它包含：

- Namespace
- Trace
- MemoryAtom
- CognitiveScene
- FeedbackLoop
- GrowthModel
- SleepCycle
- PracticePlan / DreamCandidate
- Lens

### Surfaces

Surface 是能力意图，不是 UI，不是角色。

第一批 Surfaces：

- Capture Surface: 发生了什么？
- Performance Surface: 做得怎么样？
- Reflection Surface: 这说明了什么？
- Planning Surface: 下一步做什么？
- Observation Surface: 长期状态如何变化？

### Adapters

Adapter 是具体交互方式：

- Chat Agent
- MCP Tool
- CLI
- Web App
- Mobile App
- Dashboard
- IDE Plugin
- Voice Assistant

一个 Adapter 可以访问一个或多个 Surface。一个 Surface 可以被多个 Adapter 使用。

### Surface Gateway

所有外部 Adapter 必须通过 Surface Gateway 访问 Surface 能力。

Surface Gateway 负责：

- Auth。
- Namespace routing。
- Surface routing。
- ACL / permissions。
- Request validation。
- Response shaping。
- Trace writing。
- Sync / async dispatch。
- Event publishing。

概念请求：

```text
SurfaceRequest {
  namespace
  surface
  action
  actor
  adapter
  payload
  context: { mode, locale, device, runtime_preference }
}
```

概念响应：

```text
SurfaceResponse {
  surface
  action
  result
  generated_trace_id
  follow_up_suggestions
  visibility
}
```

## 非目标

- 不把 Surface 当成 UI screen。
- 不把 Adapter 当成所有权边界。
- 不允许 Agent 拥有 memory 或 identity。
- 不直接暴露 Engine 内部对象给上游 App。
- 不为了 Surface Gateway 引入第二后端。

## 后果

正面：

- Dictation Coach、Thought Review、MCP、Dashboard 可以共享 Engine。
- 子 agent 可以按 Surface 切任务，不会混淆角色和能力。
- Trace 写入和 async dispatch 有统一入口。

负面：

- 现有 REST/MCP API 会有一段过渡期同时存在底层对象接口和 Surface Gateway。
- 需要额外文档约束，避免 worker 直接把家长/孩子角色写进 Engine。

## 相关决策

- ADR-009: Rust-first 后端主线
- ADR-014: Namespace and Feedback Loop Model
- ADR-016: Local-first Trace Learning Runtime
- ADR-018: MemoryNexus as Long-term Feedback Engine
