# Cognitive Concepts

> MemoryNexus 认知内核中的核心概念说明。

## 总览

这些概念可以分成七层：

1. **认知原材料**：Memory
2. **认知生命周期结构**：MemoryAtom、CognitiveScene、CognitiveProjection
3. **认知加工结果**：Reflection、Concept、Belief、Relation、Contradiction
4. **长期反馈结构**：Namespace、FeedbackLoop、GrowthModel
5. **运行证据结构**：ObserveMode、Trace
6. **离线演化结构**：SleepCycle、ConsolidationResult、DreamCandidate、PracticePlan
7. **能力入口结构**：Surface、Adapter、SurfaceGateway

`CognitiveProfile` 是 `CognitiveState` 的对外投影视图，用于 LLM、MCP 和 UI
消费。它不是新的所有权边界。

最简单的 Lens 链路是：

```text
Memory
→ Lens
→ Reflection
→ Concept
→ Belief
→ Contradiction
→ CognitiveState
```

长期 memory lifecycle 链路是：

```text
Experience / Thought / Practice
→ Memory
→ MemoryAtom
→ CognitiveScene
→ CognitiveProjection
→ Reflection / Belief / Next Action
```

双系统运行链路是：

```text
System 1 / fast:
recent memories + pinned facts + high-salience scenes + compressed profile
→ low-latency response

System 2 / focused or deep:
MemoryAtom
→ CognitiveScene
→ CognitiveProjection
→ Reflection / Concept / Belief / Contradiction / Next Action
```

长期反馈链路是：

```text
FeedbackLoop
→ Memory
→ MemoryAtom
→ CognitiveScene
→ CognitiveProjection
→ Concept / Pattern / Belief
→ GrowthModel
→ Next FeedbackLoop
```

Wake / Sleep / Dreaming 链路是：

```text
Wake:
Interaction / Practice / MCP Tool Call
→ fast response
→ Trace

Sleep:
Trace / Memory / FeedbackLoop
→ ConsolidationResult
→ CognitiveScene / GrowthModel / Pattern

Dreaming:
ConsolidationResult
→ DreamCandidate
→ PracticePlan
→ next practice / review question / scenario / plan

Next Wake:
new Trace
→ evaluate whether the candidate helped
```

一句话概括：

```text
Memory 是材料，
MemoryAtom 是可追踪的认知原子，
CognitiveScene 是长期主题场，
Lens 是看法，
CognitiveProjection 是某个 Lens 下重构出的当前上下文，
Reflection 是解释，
Concept 是模式，
Belief 是长期倾向，
Relation 是连接，
Contradiction 是张力，
CognitiveSpace 是容器，
Namespace 是领域，
FeedbackLoop 是练习 / 反馈闭环，
GrowthModel 是领域成长画像，
ObserveMode 是读取深度，
Trace 是交互和运行时证据，
SleepCycle 是离线整合周期，
ConsolidationResult 是睡眠后的稳定化输出，
DreamCandidate 是候选下一步，
PracticePlan 是被选择后面向行动的计划，
Surface 是能力意图，
Adapter 是交互方式，
SurfaceGateway 是外部请求进入 Engine 的边界，
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

## MemoryAtom

MemoryAtom 是从一条或多条 Memory 中抽出的最小可追踪认知单元。

它不是神经科学意义上的 engram，也不是直接照搬其他系统的 MemCell。它是
MemoryNexus 自己的中间层：把原始输入拆成可以被聚类、追踪、引用和重新解释的
认知原子。

例子：

```text
Memory:
我觉得 MemoryNexus 有潜力，但不知道怎么吸引普通用户。
孩子学习助手可能是一个落地场景。

MemoryAtom:
- 用户认为 MemoryNexus 有潜力。
- 用户担心普通用户不知道为什么安装。
- 用户把孩子学习助手视为可能落地场景。
- 用户正在从认知系统转向反馈系统思考。
```

MemoryAtom 回答的问题是：

```text
这段经历里有哪些可单独追踪的认知信号？
```

## CognitiveScene

CognitiveScene 是多个 MemoryAtom、Reflection、Concept、Belief 和 Contradiction
围绕同一个长期问题形成的主题场。

它不是更大的笔记，也不是普通 tag 集合。它表达的是一组反复出现、彼此相关、会随时间
被整合和重解释的认知结构。

例子：

```text
Scene: MemoryNexus 产品落地焦虑

包含：
- 普通用户为什么安装？
- 安装后第一动作是什么？
- Thought Review 是否足够窄？
- STEM Learning Feedback / learning.stem 是否是更强的落地入口？
- Namespace / FeedbackLoop 是否会让产品再次过宽？

潜在张力：
- 底层抽象很强
- 用户入口必须足够具体
```

CognitiveScene 回答的问题是：

```text
哪些认知信号正在形成一个长期主题、问题场或练习场？
```

## CognitiveProjection

CognitiveProjection 是某个 Lens 面向当前 query 从 CognitiveSpace 中重构出的上下文。

它必须带有 ObserveMode，因为不是每次交流都应该跑完整 cognition。

普通检索回答的是：

```text
哪些片段最相似？
```

CognitiveProjection 回答的是：

```text
为了用这个 Lens 回答当前问题，哪些 Scene、Atom、Concept、Belief 和 Contradiction
应该被组合成足够用的上下文？
```

同一个 query 在不同 Lens 下可以激活不同上下文。
同一个 query 在不同 mode 下也应该有不同成本和深度。

例子：

```text
query: MemoryNexus 下一步该做什么？

Product Lens projection:
- Thought Review magic moment
- 普通用户第一动作
- STEM Learning Feedback 的家长可理解入口
- 当前 UI 验证缺口

Systems Lens projection:
- CognitiveSpace ownership boundary
- Namespace scoped inside Space
- FeedbackLoop provenance
- MemoryAtom / CognitiveScene lifecycle
```

CognitiveProjection 回答的问题是：

```text
这个 Lens 现在应该如何重构记忆空间，形成可解释、可行动的上下文？
```

## ObserveMode

ObserveMode 描述系统本次读取 / 投影应该有多深。

MemoryNexus 不应该每次输入都执行：

```text
Atomization
→ Scene activation
→ Semantic consolidation
→ Multi-lens projection
→ Reflection generation
→ Belief update
→ Contradiction detection
```

它应该采用双系统策略：

```text
System 1: fast, low-latency, intuitive response
System 2: focused/deep, reflective consolidation and abstraction
```

### fast

用于即时交互。

读取：

- recent memories
- pinned facts
- high-salience scenes
- compressed profile / skill priors

不做：

- 多 Lens projection
- 同步 Belief update
- 同步 Contradiction detection
- 同步 CognitiveScene consolidation

fast 回答的问题是：

```text
现在怎么自然接住用户？
```

### focused

用于普通问题或单次轻量复盘。

读取：

- 一个主 Lens
- 少量相关 atoms / scenes / concepts
- 可追溯但较短的 CognitiveProjection

focused 回答的问题是：

```text
这件事用当前 Lens 怎么理解？
```

### deep

用于用户主动请求的周复盘、学习计划、项目决策和长期整理。

可以触发：

- multi-lens projection
- atomization
- scene update
- concept / belief update
- contradiction detection
- FeedbackLoop adjustment 或 next action

deep 回答的问题是：

```text
这件事说明了什么？最近有什么模式？下一步怎么调整？
```

ObserveMode 回答的问题是：

```text
这次读取应该像直觉一样快，还是像复盘一样深？
```

## Trace

Trace 是一次交互或执行的运行证据。

它记录：

- 输入 / 输出摘要。
- source type 和 task type。
- ObserveMode。
- runtime: deterministic、local、cloud、hybrid 或 unknown。
- latency、token usage、estimated cost 和 local processing ratio。
- 相关 Memory、LensRun、ReviewReport、FeedbackLoop 等对象 ID。
- 后续用户反馈或错误状态。

Trace 不是普通用户笔记，也不是新的 ownership boundary。它的权限仍然由
`CognitiveSpace` 控制。

Trace 回答的问题是：

```text
这次系统执行发生了什么，花了多少成本，生成了哪些对象，后来证明有没有用？
```

## SleepCycle

SleepCycle 是一次离线 consolidation 周期。

字段级 contract 见 [Sleep Cycle Contract](sleep-cycle-contract.md)。后续
Sleep / Dreaming schema、API、CLI 或 MCP 实现必须先对齐该 contract。

它把一段时间内的 Trace、Memory、FeedbackLoop、ReviewReport 等证据拿出来整理，
而不是在用户每次输入时同步运行完整 cognitive pipeline。

常见周期：

- daily
- weekly
- manual

SleepCycle 适合做：

- 去噪和聚类。
- 检测重复错因、主题、趋势和证据不足。
- 更新 CognitiveScene / GrowthModel。
- 生成 ConsolidationResult。
- 为 DreamCandidate generation 提供输入。

SleepCycle 回答的问题是：

```text
过去这段时间的证据应该如何被离线整理成更稳定的结构？
```

## ConsolidationResult

ConsolidationResult 是 SleepCycle 的稳定化输出。

字段级 contract 见 [Sleep Cycle Contract](sleep-cycle-contract.md)。

它不是用户原始输入，而是对一段时间窗口的概括和抽象。它可以包含：

- new concepts
- detected patterns
- detected contradictions
- improvement signals
- updated growth model summary
- next actions

在 `learning.stem` 中，它可能表达：

```text
最近三次分数应用题里，学习者反复把总量和部分量混淆。
正确率有改善，但读题定位仍不稳定。
下一轮应减少新知识点，增加一步式关系识别练习。
```

ConsolidationResult 回答的问题是：

```text
这些 Trace 和练习记录被整理之后，稳定出现了什么模式和下一步重点？
```

## DreamCandidate

DreamCandidate 是基于 ConsolidationResult 生成的候选下一步。

字段级 contract 见 [Sleep Cycle Contract](sleep-cycle-contract.md)。

它可以是：

- 下一轮练习题。
- 复盘问题。
- 场景模拟。
- contradiction exploration prompt。
- planning prompt。

它叫 Candidate，是因为它需要被选择、执行和评估。它不是最终证明有效的计划。

第一版 DreamCandidate 应优先 deterministic / local-first，尤其在 Trial Profile 和
Local One-click Profile 中不要默认调用云模型。Production Profile 可以在 explicit
deep / manual sleep cycle 中使用云模型，但必须写入 Trace。

DreamCandidate 回答的问题是：

```text
基于最近的模式，下一轮最值得尝试的练习、问题或计划是什么？
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
有些 Contradiction 最终会被新证据解决；有些会被接受为 plural truth，也就是在不同
Lens 下同时成立。

生命周期状态包括：

- Detected: 系统发现了张力。
- Acknowledged: 系统或用户确认这条张力值得保留观察。
- Resolved: 新证据或新 Belief 替代了旧冲突。
- AcceptedAsPlural: 双方可以在不同 Lens 或上下文中同时成立。
- Ignored: 明确不再作为认知信号参与默认投影。

resolution mode 包括：

- Resolved: 新证据替代旧 Belief。
- AcceptedAsPlural: 多个视角下都成立。
- StaleConflict: 旧冲突已经过时，可联动 Memory Salience 降权。
- NeedsUserJudgment: 需要用户判断。
- DomainSpecific: 只在特定领域或上下文中冲突。

Contradiction 会记录 source memory IDs、belief IDs、lens IDs、confidence 和
updated event ID，以便追踪它从哪里来、被哪个事件改变。

Contradiction 回答的问题是：

```text
哪里存在认知张力？这条张力现在处于什么生命周期？
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

## Namespace

Namespace 是 CognitiveSpace 内的领域分区。

它不取代 CognitiveSpace，也不引入新的所有权边界。权限仍然由 Space membership
决定。Namespace 只回答这些认知对象属于哪个具体领域、应该被哪些 Lens / Profile /
反馈循环优先消费。

例子：

```text
personal.thoughts
project.memorynexus
learning.stem
music.piano
chess.tactics
art.drawing
programming.rust
```

Namespace 可以分成两类：

- Reflective Namespace: 用于理解自己、项目、决策和长期主题。
- Skill Namespace: 用于追踪学习、练习、表现、错因和下一轮任务。

当前 Thought Review MVP 可以视为：

```text
namespace = personal.thoughts
```

Namespace 回答的问题是：

```text
这批 memory、reflection、feedback loop 属于哪个长期成长领域？
```

## FeedbackLoop

FeedbackLoop 是跨领域的长期反馈闭环对象。

它比 Memory 更面向练习、学习和持续改进。最小结构是：

```text
Goal
Task
Attempt
Evaluation
Feedback
Adjustment
NextTask
```

例子：

```text
learning.stem:
知识点 → 练习题 → 作答结果 → 错因分析 → 个性化强化 → 下一天练习计划

music.piano:
曲目 → 练习片段 → 节奏 / 指法反馈 → 针对性练习 → 每周进展反馈

personal.thoughts:
想法 → 多视角复盘 → 反复主题 → 内在张力 → 下一步观察问题
```

FeedbackLoop 与现有认知对象的关系是：

```text
FeedbackLoop 产生 Memory
Memory 产生 Reflection
Reflection 形成 Concept / Pattern
Concept / Pattern 更新 GrowthModel
GrowthModel 影响下一轮 FeedbackLoop / PracticePlan
```

FeedbackLoop 回答的问题是：

```text
这个领域下一轮应该如何练习、复盘或调整？
```

## GrowthModel

GrowthModel 是某个 Namespace 下的长期成长画像。

它不是通用 user profile，也不是 agent memory profile。它只描述某个领域里的能力、
表现、模式和下一步重点，并且必须能追溯到 Trace、FeedbackLoop、Memory 或
ConsolidationResult 等证据。

它可以包含：

- strengths
- weaknesses
- recurring patterns
- current stage
- recommended focus
- evidence IDs
- confidence / evidence gaps

例子：

```text
namespace: child.chinese.dictation
weakness: visually similar characters
recurring_pattern: component placement errors appear across three days
recommended_focus: practice five similar-shape character pairs tomorrow
```

GrowthModel 回答的问题是：

```text
这个领域的长期状态正在怎样变化，下一步应该关注什么？
```

## PracticePlan

PracticePlan 是面向下一步行动的计划。

它可以来自 Planning Surface，也可以由 SleepCycle / DreamCandidate 生成。与
DreamCandidate 的区别是：

- DreamCandidate 是内部候选，需要被选择、执行和评估。
- PracticePlan 是已经被选中、准备展示或执行的行动计划。

例子：

```text
明天 10 分钟：
1. 复习 5 个形近字。
2. 每个字先说部件位置，再默写。
3. 最后随机听写 3 个昨天错过的词。
```

PracticePlan 回答的问题是：

```text
基于当前 GrowthModel 和最近表现，下一轮具体做什么？
```

## Surface / Adapter / SurfaceGateway

Surface 是能力意图，Adapter 是交互方式，SurfaceGateway 是外部请求进入 Engine 的
边界。

Surfaces：

- Capture: 发生了什么？
- Performance: 做得怎么样？
- Reflection: 这说明了什么？
- Planning: 下一步做什么？
- Observation: 长期状态如何变化？

Adapters：

- Chat Agent
- MCP Tool
- CLI
- Web App
- Mobile App
- Dashboard
- Voice Assistant

SurfaceGateway 负责：

- auth
- namespace routing
- surface routing
- ACL / permissions
- request validation
- response shaping
- Trace writing
- sync / async dispatch
- event publishing

SurfaceGateway 回答的问题是：

```text
这个外部请求应该以什么权限、什么 Surface、什么 Namespace 进入 Engine？
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

`unresolved_contradictions` 默认只包含 Detected 和 Acknowledged 状态的
Contradiction。Resolved、AcceptedAsPlural 和 Ignored 仍在 CognitiveState 中，
但不会自动进入当前 Profile。

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
