# Cognitive Concepts

> MemoryNexus 认知内核中的核心概念说明。

## 总览

这些概念可以分成三层：

1. **认知原材料**：Memory
2. **认知加工结果**：Reflection、Concept、Belief、Relation、Contradiction
3. **系统运行结构**：CognitiveSpace、Lens、CognitiveEvent、CognitiveState

`CognitiveProfile` 是 `CognitiveState` 的对外投影视图，用于 LLM、MCP 和 UI
消费。它不是新的所有权边界。

最简单的链路是：

```text
Memory
→ Lens
→ Reflection
→ Concept
→ Belief
→ Contradiction
→ CognitiveState
```

一句话概括：

```text
Memory 是材料，
Lens 是看法，
Reflection 是解释，
Concept 是模式，
Belief 是长期倾向，
Relation 是连接，
Contradiction 是张力，
CognitiveSpace 是容器，
CognitiveEvent 是变化，
CognitiveState 是变化后的整体状态。
```

再进一步：

```text
CognitiveProfile = project(CognitiveState, target_use)
```

Profile 只引用 Memory / Event 的 ID，不拥有 Memory。

## Memory

Memory 是系统里的认知原材料。

它记录发生过什么、用户输入了什么、看到什么、听到什么、经历了什么。Memory 尽量接近事件本身，不急着解释。

例子：

```text
今天开会时，我先提出了一个想法，没人回应。
后来同事重新说了一遍，大家开始讨论。
```

Memory 回答的问题是：

```text
发生了什么？
```

## Memory Salience / Automatic Forgetting

Memory Salience 表示一条 Memory 在默认认知投影中的当前显著性。

MemoryNexus 里的 automatic forgetting 不是物理删除 Memory，而是降低
salience，让它暂时不进入默认 Profile、搜索排序或 Lens Run 上下文。
Memory 仍然保留在 CognitiveSpace 中，仍然可以被显式召回、审计和恢复。

常见降权原因包括：

- Stale: 信息已经过时。
- Superseded: 被更新、更准确的 Memory 取代。
- LowSignal: 信息噪声较高，默认上下文价值低。
- Contradicted: 被后续证据或 Belief contradiction 挑战。
- UserHidden: 用户主动隐藏，但不要求删除。

Memory Salience 回答的问题是：

```text
这条 memory 当前应该以多高优先级参与默认认知？
```

## Reflection

Reflection 是对 Memory 的一次解释。

同一条 Memory 可以产生多个 Reflection，因为不同 Lens 会看到不同意义。

以上面的会议记忆为例：

- Emotional Lens: 我可能感到被忽视。
- Systems Lens: 会议反馈机制可能有问题。
- Detective Lens: 团队里可能存在话语权差异。
- Philosopher Lens: 观点的价值受说话者身份影响。

Reflection 回答的问题是：

```text
这件事可能意味着什么？
```

## Concept

Concept 是多个 Reflection 反复出现后形成的抽象。

如果多次 Reflection 都指向“被忽视”“表达没有被接住”“需要别人重复才被认可”，系统可能形成一个 Concept：

```text
群体表达中的话语权不对等
```

Concept 回答的问题是：

```text
这里反复出现了什么模式？
```

## Belief

Belief 是更长期、更稳定的解释倾向。

它不是 truth，而是系统逐渐形成的 worldview tendency。Belief 会反过来影响之后的 attention、interpretation、prediction 和 reflection。

例子：

```text
用户在群体中经常担心自己的观点不会被认真对待。
```

Belief 回答的问题是：

```text
系统正在稳定地用什么方式理解世界？
```

## Relation

Relation 表示 Memory、Reflection、Concept、Belief 之间的连接。

例子：

```text
Reflection B derives_from Memory A
Concept X abstracts Reflection B
Belief Y is_influenced_by Concept X
Memory C contradicts Belief Y
```

Relation 让系统不只是存一堆孤立记录，而是形成认知 topology。

Relation 回答的问题是：

```text
这些认知对象之间如何连接？
```

## Contradiction

Contradiction 表示系统里同时存在但互相冲突的解释、信念或记忆。

例子：

```text
Belief A: 我想被别人理解。
Belief B: 我害怕暴露真实想法。
```

Contradiction 不一定要被消灭。它更像一种 tension，会驱动新的 Reflection、Concept 和 Belief evolution。

Contradiction 回答的问题是：

```text
哪里存在尚未解决的认知张力？
```

## CognitiveSpace

CognitiveSpace 是所有认知对象所在的容器。

它包括：

```text
Memories
Reflections
Concepts
Beliefs
Relations
Contradictions
Lenses
```

CognitiveSpace 可以属于个人、家庭、项目或组织。Memory 属于 CognitiveSpace，不属于 Agent。

CognitiveSpace 回答的问题是：

```text
这些认知对象共同存在于哪个长期空间？
```

## Lens

Lens 是认知视角，也是一种 interpretation strategy。

它不是 Agent，也不只是 prompt。Lens 决定系统如何看 Memory，关注什么、忽略什么、如何组织意义、如何处理矛盾。

例子：

- Detective Lens: 关注异常、遗漏、权力关系和隐性动机。
- Emotional Lens: 关注情绪、感受、需求和创伤。
- Systems Lens: 关注结构、反馈、循环和机制。
- Narrative Lens: 关注故事、身份连续性和人生主题。
- Philosopher Lens: 关注意义、价值、存在方式和世界观。

Lens 回答的问题是：

```text
系统正在以什么方式看待这段 memory？
```

## CognitiveEvent

CognitiveEvent 是系统里发生的一次认知动作。

适合用 Rust enum 表达：

```rust
enum CognitiveEvent {
    MemoryCaptured,
    LensApplied,
    ReflectionGenerated,
    ConceptExtracted,
    BeliefUpdated,
    ContradictionDetected,
    MemoryDeprioritized,
    MemoryReprioritized,
}
```

CognitiveEvent 回答的问题是：

```text
系统刚刚发生了什么变化？
```

## CognitiveState

CognitiveState 是某个时刻 CognitiveSpace 的整体状态快照。

它包含当前有哪些 Memory、Reflection、Concept、Belief、Relation、Contradiction 等。
它也包含 Memory Salience，用于表达哪些 Memory 仍在空间中，但默认不进入当前
Profile。

函数式地看：

```text
CognitiveState + CognitiveEvent -> CognitiveState
```

也可以写成：

```rust
fn evolve(state: CognitiveState, event: CognitiveEvent) -> CognitiveState
```

CognitiveState 回答的问题是：

```text
经过这些变化后，整个认知空间现在是什么样？
```

## CognitiveProfile

CognitiveProfile 是从 CognitiveState 导出的紧凑上下文。

它面向外部消费场景：

- LLM context
- MCP tool response
- UI profile panel
- Project / Learning / Family / Risk 等目标视图

它包含：

```text
stable_beliefs
active_concepts
current_goals
unresolved_contradictions
summary
source_memory_ids
source_event_ids
```

`source_memory_ids` 默认只引用 active memories。被 deprioritized 的 Memory
仍属于 CognitiveSpace，但不会自动进入 Profile；这让 Profile 成为当前用途的紧凑
投影，而不是完整存档。

关键边界：

```text
CognitiveState = 内部完整状态
CognitiveProfile = 外部可消费投影
Memory = 仍然只属于 CognitiveSpace
```

CognitiveProfile 回答的问题是：

```text
为了当前使用场景，应该把认知状态压缩成什么上下文？
```

## 最小认知闭环

MemoryNexus 当前最重要的工程目标，是验证最小认知闭环：

```text
capture memory
→ retrieve related memories
→ apply lens
→ generate reflection
→ extract concept
→ update belief
→ detect contradiction
→ evolve cognitive state
```

这个闭环的核心不是“把记忆检索出来塞进 prompt”，而是验证：

```text
认知如何从记忆中涌现。
```
