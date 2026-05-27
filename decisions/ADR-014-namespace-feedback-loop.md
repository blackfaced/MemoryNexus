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

如果直接把这些场景都做成产品入口，MemoryNexus 会再次变成“理论上什么都能做，
但普通用户不知道怎么开始”的系统。因此需要区分：

- 底层长期架构可以支持跨领域反馈循环。
- 当前产品入口仍然保持窄场景，先服务 Thought Review。

## 决策

MemoryNexus 的长期技术定位扩展为：

```text
A namespace-based long-term feedback substrate for personal cognition and skill acquisition.
```

引入两个长期概念：

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
Memory 经过 Lens 产生 Reflection
Reflection 聚合成 Concept / Pattern
Concept / Pattern 更新 CognitiveProfile 或 SkillProfile
Profile 影响下一轮 FeedbackLoop
```

因此 MemoryNexus 不是从 Memory 模型迁移到 FeedbackLoop 模型，而是在
Memory + Lens + CognitiveState 之上增加长期反馈循环语义。

## 产品边界

短期产品入口保持：

```text
Thought Review MVP
```

它验证 reflective namespace：

```text
personal.thoughts
```

中期可以新增一个 skill namespace 验证场景，例如：

```text
learning.math
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
- `Lens Run`、`Review Report` 和 `CognitiveProfile` 可以追加
  `namespace_id` / `feedback_loop_id` provenance 和过滤能力，但继续以
  `space_id` 做权限校验。
- `learning.math` 是第一个 Skill Namespace MVP 候选，但必须作为后续独立产品 issue
  验证，不能和底层 schema foundation 混在一个交付里。

## 后果

正面：

- MemoryNexus 的底层方向从 “memory layer” 扩展为长期反馈系统。
- Thought Review 不会被推翻，而是成为第一个 reflective namespace。
- 孩子学习助手、钢琴练习、下棋复盘等未来方向可以复用同一套 Space、Memory、Lens、
  Review 和 Profile 能力。
- Namespace 给跨领域数据隔离、检索过滤和 Lens 选择提供了稳定语义。
- FeedbackLoop 给技能习得、错因分析、练习计划和进度反馈提供了统一模型。

负面：

- 概念层级增加，必须避免过早暴露给普通用户。
- `Namespace` 和 `CognitiveSpace` 的边界需要持续保持清晰，不能让 namespace 变成
  第二套权限系统。
- `FeedbackLoop` 暂不应一次性落成完整复杂 schema；需要从一个具体 skill namespace
  的验收场景反推最小字段。
- 如果过早同时推进多个垂直产品，会稀释 Thought Review MVP 的验证焦点。

## 相关决策

- ADR-002: Cognitive Lens Memory 产品方向
- ADR-009: Rust-first 后端主线
- ADR-012: Family and Shared Cognitive Space
- ADR-013: Thought Review UI MVP
