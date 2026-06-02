# ADR-014: Namespace and Feedback Loop Model

## 状态
✅ 已接受

## 背景

MemoryNexus 已经从普通 personal memory / second brain 方向演进为
Rust-first cognitive lens memory system。Thought Review MVP 验证了一个
reflective use case：用户写下一段想法，系统用多个 Lens 帮他理解反复主题和内在张力。

后续讨论暴露出更大的长期方向：许多高价值场景并不只是“记住信息”，而是持续追踪
一个长期反馈循环：

```text
输入 → 练习 → 表现 → 反馈 → 调整 → 下一轮练习
```

孩子学习、钢琴、下棋、画画、编程练习、项目复盘和个人思绪复盘都符合这个模式。
它们共享底层需求：

- 记录领域内的经历、练习、结果和反馈。
- 发现反复出现的薄弱点、模式、矛盾或进步趋势。
- 根据长期画像生成下一轮任务或复盘问题。

同时，EverMemOS / EverOS 等长期记忆系统提供了一个有价值的外部参考：长期记忆不应
只是存储孤立记录再做 top-k 检索，而应经历类似 lifecycle 的加工过程：

```text
Episodic Trace Formation
→ MemCells
→ Semantic Consolidation
→ MemScenes
→ Reconstructive Recollection
```

MemoryNexus 可以吸收这里的三个设计启发：

- 经验需要先被拆成可处理、可引用的记忆单元。
- 长期记忆需要被整合成稳定的主题、场景、概念和张力。
- 回忆不是简单检索，而是围绕当前任务和 Lens 重构上下文。

但二者的产品方向不同：

```text
EverMemOS: memory for agent reasoning
MemoryNexus: memory for cognitive perspective and feedback loops
```

因此 EverMemOS 是 memory lifecycle 参考，不是 MemoryNexus 的目标架构或产品定位。
MemoryNexus 不应被带偏成“更强 agent retrieval 系统”。

另一个关键约束是延迟和交互体验。Lens projection、atomization、
semantic consolidation、contradiction detection 和 belief update 都更接近慢速、
反思型的 System 2 过程。如果每次用户输入都完整运行这条 cognitive pipeline，
产品会变慢，也会显得过度解读。

因此长期架构必须区分：

```text
System 1: 快速、直觉、低延迟、轻量记忆
System 2: 慢速、反思、整合、抽象、形成长期结构
```

MemoryNexus 的设计原则是：

```text
System 2 consolidates.
System 1 retrieves compressed priors.
```

慢通道在后台或用户主动复盘时沉淀 Concept、Belief、SkillModel 和 high-salience
Scene 等压缩结构；快通道在即时交互中读取这些结构，而不是重新跑完整深度推理。

如果直接把这些场景都做成产品入口，MemoryNexus 会再次变成“理论上什么都能做，
但普通用户不知道怎么开始”的系统。因此需要区分：

- 底层长期架构可以支持跨领域反馈循环。
- Thought Review 继续作为 reflective demo 和项目演讲入口，帮助人理解多 Lens 认知记忆。
- 第一产品 MVP 候选转向 `learning.math`，用孩子学习的具体反馈闭环验证可衡量价值。

## 决策

MemoryNexus 的长期技术定位扩展为：

```text
A namespace-based long-term feedback substrate for personal cognition and skill acquisition.
```

引入两个长期概念：

### Memory Lifecycle

在 Namespace 和 FeedbackLoop 之下，MemoryNexus 采用自己的长期记忆生命周期：

```text
Experience / Thought / Practice
→ Memory
→ MemoryAtom
→ CognitiveScene
→ Lens-based CognitiveProjection
→ Reflection / Belief / Next Action
→ FeedbackLoop
```

这个 lifecycle 不是替代现有 Memory / Lens / Reflection 模型，而是解释这些对象如何
从原始记录逐步演化为可复盘、可投影、可反馈的长期结构。

这条 lifecycle 不应该在每次输入时同步完整执行。默认执行策略是：

```text
Every input -> fast response + optional async processing
Important input -> focused projection
Scheduled review / explicit request -> deep consolidation and projection
```

### Dual-System Observe Modes

MemoryNexus 的读取和投影接口应支持 mode-aware behavior：

```text
observe(namespace, query, mode)
```

#### `fast`

用于即时对话和低延迟交互。

特点：

- 只取 recent memories、pinned facts、high-salience scenes 和已压缩的 profile /
  skill priors。
- 不触发多 Lens。
- 不同步更新 Belief / Contradiction / CognitiveScene。
- 可以异步 enqueue atomization 或 deeper processing。

目标问题：

```text
现在怎么接住用户？
```

#### `focused`

用于普通问题和轻量复盘。

特点：

- 激活一个主 Lens。
- 选择少量相关 scene / atom / concept。
- 生成可追溯但较短的 CognitiveProjection。
- 可以产生 Reflection，但不默认触发完整 consolidation。

目标问题：

```text
这件事用当前 Lens 怎么理解？
```

#### `deep`

用于周复盘、学习计划、项目决策和用户明确要求的深度整理。

特点：

- 可运行多 Lens。
- 可触发 atomization、scene update、concept update、belief revision 和
  contradiction detection。
- 可生成 Next Action 或 FeedbackLoop adjustment。
- 高延迟可接受，但必须有清晰 provenance。

目标问题：

```text
这件事说明了什么？最近有什么模式？下一步怎么调整？
```

工程策略：

```text
Foreground:
capture raw input
light classification
fast retrieval
response

Background:
atomization
scene update
concept / belief update
contradiction detection

Explicit deep review:
multi-lens projection
deep synthesis
next action / practice plan
```

#### MemoryAtom

`MemoryAtom` 是从 `Memory` 中抽出的最小可追踪认知单元。它用于把一段混乱输入拆成
可聚类、可引用、可重新解释的信号。

示例：

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

`MemoryAtom` 不是神经科学意义上的 engram，也不是直接照搬其他系统的 MemCell。

#### CognitiveScene

`CognitiveScene` 是多个 `MemoryAtom`、`Reflection`、`Concept`、`Belief` 和
`Contradiction` 围绕同一长期主题形成的认知场景。

它不是普通 tag，也不是更大的笔记。它回答：

```text
哪些认知信号正在形成一个长期主题、问题场或练习场？
```

示例：

```text
Scene: MemoryNexus 产品落地焦虑

包含：
- 普通用户为什么安装？
- Thought Review 的第一动作和 magic moment
- learning.math 是否是更强落地入口
- Namespace / FeedbackLoop 是否会让产品再次过宽

潜在张力：
- 底层抽象很强
- 用户入口必须足够具体
```

#### Lens-based CognitiveProjection

`CognitiveProjection` 是某个 `Lens` 面向当前 query 从 `CognitiveSpace` 中重构出的
上下文。

普通 retrieval 关注：

```text
哪些片段最相似？
```

Lens-based projection 关注：

```text
为了用这个 Lens 回答当前问题，哪些 Scene、Atom、Concept、Belief 和 Contradiction
应该被组合成足够用的上下文？
```

这使 MemoryNexus 的 Recollection 更接近 perspective-specific meaning composition，
而不是 agent-specific context stuffing。

`CognitiveProjection` 必须携带 mode。`fast` projection 可以只是轻量上下文包；
`focused` projection 是单 Lens 的短合成；`deep` projection 才是完整多 Lens /
consolidation 入口。

### Namespace

`Namespace` 是 `CognitiveSpace` 内的领域分区，不取代 `CognitiveSpace` 的所有权边界。

示例：

```text
personal.thoughts
project.memorynexus
learning.math
music.piano
chess.tactics
art.drawing
programming.rust
```

`CognitiveSpace` 仍然回答：

```text
这些认知对象属于哪个长期空间，以及谁有权限访问？
```

`Namespace` 回答：

```text
这些记忆、反馈和练习属于哪个具体成长领域？
```

第一版实现可以先把 namespace 作为 Memory / Lens / FeedbackLoop 的逻辑字段或标签，
不急于引入独立复杂权限模型。权限仍以 `CognitiveSpace` membership 为准。

### FeedbackLoop

`FeedbackLoop` 是跨领域的长期反馈闭环对象。它比 `Memory` 更面向落地应用。

最小结构：

```text
Goal
Task
Attempt
Evaluation
Feedback
Adjustment
NextTask
```

它适用于两类 namespace：

1. Reflective Namespace
   - 示例：`personal.thoughts`、`life.decisions`、`project.memorynexus`
   - 核心：meaning、belief、contradiction、identity、direction
   - 当前 Thought Review MVP 属于这一类。

2. Skill Namespace
   - 示例：`learning.math`、`music.piano`、`chess.tactics`、`art.drawing`
   - 核心：goal、attempt、error pattern、feedback、next practice
   - 后续 Learning Assistant 可以作为第一个 skill namespace 产品验证。

FeedbackLoop 与现有 cognitive memory 的关系：

```text
FeedbackLoop 产生 Memory
Memory 被拆成 MemoryAtom
MemoryAtom 被整合进 CognitiveScene
CognitiveScene 经过 Lens 产生 CognitiveProjection
CognitiveProjection 产生 Reflection / Concept / Belief / Next Action
Reflection / Concept / Belief 更新 CognitiveProfile 或 SkillProfile
Profile 影响下一轮 FeedbackLoop
```

Practice Layer 会同时使用快慢两层：

```text
练习中：fast feedback
练习后：focused review
每日/每周：deep consolidation and practice adjustment
```

因此 MemoryNexus 不是从 Memory 模型迁移到 FeedbackLoop 模型，而是在
Memory + Lens + CognitiveState 之上增加长期反馈循环语义。

## 产品边界

Thought Review 继续保留为 reflective demo 和项目演讲入口：

```text
Thought Review MVP
```

它验证 reflective namespace：

```text
personal.thoughts
```

但它不再作为第一商业/产品 MVP 候选。第一产品 MVP 候选调整为：

```text
learning.math
```

`learning.math` 的第一场景是 parent-assisted elementary math mistake feedback
loop，用家长可理解的语言验证：

```text
知识点 → 练习题 → 作答 → 错因 → 反馈 → 下一题 → 每周学习报告
```

但不要在同一版 UI 中同时展开孩子学习、钢琴、下棋、画画、编程和个人复盘。每个具体
产品入口必须有自己的 issue、验收标准和用户语言。

## 第一阶段实现解释

#52 的第一阶段不直接落成完整 schema 或学习产品 UI，而是先定义最小模型和 API
拆分，见 [Namespace and Feedback Loop Minimal Design](../docs/namespace-feedback-loop-design.md)。

第一阶段采用以下实现边界：

- `Namespace` 必须 scoped inside `CognitiveSpace`，只表达领域分区，不表达权限。
- `FeedbackLoop` 必须属于同一个 `CognitiveSpace` 和 `Namespace`。
- `Memory` 仍然是原始认知材料；`FeedbackLoop` 通过 provenance 产生或关联
  Memory，而不是取代 Memory。
- `MemoryAtom`、`CognitiveScene` 和 `CognitiveProjection` 先作为 Phase 5 设计与
  prototype 对象推进，不要求和 Namespace / FeedbackLoop schema foundation 在同一个
  issue 中落库。
- `CognitiveProjection` 必须先定义 `fast` / `focused` / `deep` mode，不允许默认把
  每次 observe 都实现成 deep multi-lens pipeline。
- `Lens Run`、`Review Report` 和 `CognitiveProfile` 可以追加
  `namespace_id` / `feedback_loop_id` provenance 和过滤能力，但继续以
  `space_id` 做权限校验。
- `learning.math` 是第一产品 MVP 候选，但必须作为独立产品 issue 验证，不能和底层
  schema foundation 混在一个交付里。

## 后果

正面：

- MemoryNexus 的底层方向从 “memory layer” 扩展为长期反馈系统。
- Thought Review 不会被推翻，而是成为 reflective namespace demo 和项目演讲入口。
- `learning.math` 给 MemoryNexus 提供更具体、可衡量、普通用户更容易理解的产品 MVP。
- 孩子学习助手、钢琴练习、下棋复盘等未来方向可以复用同一套 Space、Memory、Lens、
  Review 和 Profile 能力。
- Namespace 给跨领域数据隔离、检索过滤和 Lens 选择提供了稳定语义。
- FeedbackLoop 给技能习得、错因分析、练习计划和进度反馈提供了统一模型。
- MemoryAtom / CognitiveScene / CognitiveProjection 给系统补上从原始经验到长期结构、
  再到 Lens 重构上下文的 lifecycle。
- 双系统模式让 MemoryNexus 同时支持低延迟即时交互和高质量长期反思。
- EverMemOS 类系统的价值被吸收到 lifecycle 设计中，但 MemoryNexus 保持
  user-owned cognitive perspective / feedback loop 的定位。

负面：

- 概念层级增加，必须避免过早暴露给普通用户。
- `Namespace` 和 `CognitiveSpace` 的边界需要持续保持清晰，不能让 namespace 变成
  第二套权限系统。
- `FeedbackLoop` 暂不应一次性落成完整复杂 schema；需要从一个具体 skill namespace
  的验收场景反推最小字段。
- `MemoryAtom` 和 `CognitiveScene` 如果过早落库，可能造成 schema 先行、产品验证不足；
  应先用 fixtures / prototype 验证 atomization、consolidation 和 projection 是否真的有用。
- `fast` 模式如果过度简化，可能显得“不够聪明”；`deep` 模式如果默认启用，则会造成
  延迟和过度解读。需要在产品入口上明确触发时机。
- 如果过早同时推进多个垂直产品，会稀释 `learning.math` MVP 的验证焦点。

## 相关决策

- ADR-002: Cognitive Lens Memory 产品方向
- ADR-009: Rust-first 后端主线
- ADR-012: Family and Shared Cognitive Space
- ADR-013: Thought Review UI MVP
