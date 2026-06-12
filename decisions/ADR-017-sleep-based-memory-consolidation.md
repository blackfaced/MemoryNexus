# ADR-017: Sleep-based Memory Consolidation

## 状态
✅ 已接受

## 背景

ADR-014 把 MemoryNexus 扩展为 namespace-based long-term feedback substrate：
系统不仅保存 Memory，还要把长期经历、练习、反馈和复盘演化成可行动的下一步。
ADR-016 又引入 Trace，记录交互、运行时指标、生成对象和反馈效果，为 local-first /
trace-driven learning 提供证据层。

但 MemoryNexus 仍需要回答一个核心工程问题：

```text
哪些认知工作应该在用户交互时同步完成？
哪些工作应该延后到离线阶段完成？
```

如果每次输入都同步运行 atomization、scene consolidation、multi-lens projection、
belief / GrowthModel update、contradiction detection 和 next practice generation，
产品会变慢，也会过度解读普通输入。

外部研究《Language Models Need Sleep: Learning to Self Modify and Consolidate
Memories》提供了一个有价值的系统隐喻：白天快速响应并保留短期痕迹，睡眠时进行
consolidation，并通过 dreaming 生成新的练习或模拟材料。论文的具体实现涉及模型参数
扩展、蒸馏和 self-modification；MemoryNexus 不采用这些训练机制，但吸收
Wake / Sleep / Dreaming 的架构分层。

对 MemoryNexus 来说：

```text
Wake      = 前台低延迟交互，生成 Trace
Sleep     = 离线整合 Trace / Memory / FeedbackLoop
Dreaming  = 生成候选练习、复盘问题、场景模拟或下一步计划
```

这个决策也进一步落实 ADR-014 的双系统原则：

```text
System 2 consolidates.
System 1 retrieves compressed priors.
```

## 决策

MemoryNexus 采用 Wake / Sleep / Dreaming 架构来组织长期反馈演化。

### Wake Phase

Wake Phase 是用户正在交互时的前台路径。目标是低延迟、可恢复、可追踪。

Wake Phase 可以做：

- 记录输入、练习、作答、MCP tool call 或 Lens Run。
- 轻量分类和必要检索。
- 快速反馈或 deterministic fallback。
- 生成 Trace，记录 runtime、latency、cost、mode、generated object IDs 和错误状态。
- 可选 enqueue 后台处理。

Wake Phase 不应该同步做：

- 全量 MemoryAtom extraction。
- CognitiveScene consolidation。
- 多 Lens deep projection。
- Belief / GrowthModel 深度更新。
- DreamCandidate 生成。
- 复杂 contradiction detection。

### Sleep Phase

Sleep Phase 是离线、手动或计划触发的后台整合路径。它把 Trace、Memory、
FeedbackLoop、ReviewReport 等证据转化为更稳定的结构。

第一阶段 Sleep Phase 应支持：

- daily / weekly / manual 三类周期。
- 从 Trace / FeedbackLoop 中识别近期重复主题、错因模式、改善信号和证据不足。
- 输出 `ConsolidationResult`。
- 后续更新 `MemoryAtom`、`CognitiveScene`、`GrowthModel` 或 Review 输入。
- 使用 Trace 记录 SleepCycle 自身的 runtime、status、latency、runtime class 和错误。

Sleep Phase 应优先支持 deterministic / local-first 路径，先用 fixtures 验证价值。

### Dreaming Phase

Dreaming Phase 是基于 consolidation 结果生成下一轮候选行动的路径。

第一阶段 Dreaming 不做模型 self-improvement，而是生成 `DreamCandidate`：

- 下一轮练习题。
- 复盘问题。
- 场景模拟。
- contradiction exploration prompt。
- project / learning planning prompt。

对 `learning.stem`，第一批 DreamCandidate 应从 elementary fraction word problems
开始，基于错因模式生成下一题或复习问题。

Dreaming 的运行策略：

- Trial Profile：只使用 deterministic DreamCandidate generation。
- Local One-click Profile：默认 deterministic，可在单独 issue 中讨论 optional local
  model adapter。
- Production Profile：允许配置 cloud generation，但只应在 explicit deep / manual
  sleep cycle 中使用，并必须写入 Trace。

### 新概念

ADR-017 只接受概念边界；具体字段合同由后续 issue 定义。

#### SleepCycle

表示一次离线整合周期。它连接输入 Trace、生成结果、状态和运行时指标。

概念字段：

```text
SleepCycle {
  id
  space_id
  namespace_id
  cycle_type: daily | weekly | manual
  input_trace_ids
  generated_memory_ids
  generated_scene_ids
  updated_growth_model_ids
  generated_plan_ids
  status: pending | running | completed | failed
  started_at
  completed_at?
}
```

#### ConsolidationResult

表示 Sleep Phase 对一段时间窗口的稳定化输出。

概念字段：

```text
ConsolidationResult {
  id
  space_id
  namespace_id
  sleep_cycle_id
  new_concepts
  updated_growth_model_summary
  detected_patterns
  detected_contradictions
  improvement_signals
  next_actions
}
```

#### DreamCandidate

表示 Dreaming Phase 生成的候选下一步。名称使用 Candidate 是为了强调它需要被选择、
验证和剪枝，而不是直接等同最终计划。

概念字段：

```text
DreamCandidate {
  id
  space_id
  namespace_id
  source_sleep_cycle_id
  source_consolidation_result_id
  purpose: practice_generation | scenario_simulation | contradiction_exploration | review_question | planning_prompt
  content
  expected_effect
  selected
  evaluation_result?
}
```

## 非目标

MemoryNexus 不因为 ADR-017 而实现：

- 模型参数更新。
- automatic fine-tuning。
- RL self-modification。
- 参数扩展或 knowledge distillation。
- 完整本地 inference runtime。
- model catalog。
- local accelerator management。
- 复杂 synthetic training loop。
- 每次用户输入都同步运行完整 cognitive pipeline。

## 后果

正面：

- MemoryNexus 的长期路线从实时 retrieval 进一步明确为 sleep-driven feedback
  engine。
- Trace 有了明确的下游用途：离线 consolidation、DreamCandidate generation 和效果评估。
- `learning.stem` 可以形成更清晰的产品循环：白天练习，晚上整理，第二天给出下一题。
- Trial / Local One-click / Production 三种安装形态可以用不同 runtime 承担 Dreaming。
- 子 agent 可以按 Trace -> SleepCycle -> DreamCandidate -> effectiveness evaluation 的顺序
  拆小任务。

负面：

- 概念层继续增加，必须避免把 Sleep / Dreaming 作为普通用户主入口术语。
- 如果过早引入 scheduler、schema 和云模型，容易把路线做重。
- DreamCandidate 如果没有 Trace-based evaluation，可能退化为普通生成内容。
- Cloud Dreaming 会引入隐私、成本、延迟和可解释性问题，必须通过 ADR-016 的 Trace
  metrics 约束。

## 后续任务

第一批任务按以下顺序推进：

1. 定义 SleepCycle、ConsolidationResult 和 DreamCandidate contract。
2. 基于 Trace / FeedbackLoop fixtures 原型化 deterministic daily sleep consolidation。
3. 为 `learning.stem` 生成 deterministic DreamCandidate。
4. 增加 manual SleepCycle API / CLI / MCP trigger。
5. 定义 deterministic / local / cloud 的 Dreaming runtime routing policy。
6. 用 Trace 和后续 FeedbackLoop 评估 DreamCandidate effectiveness，并剪枝低价值候选。

## 相关决策

- ADR-009: Rust-first 后端主线
- ADR-014: Namespace and Feedback Loop Model
- ADR-015: Supabase Integration Boundary
- ADR-016: Local-first Trace Learning Runtime

## 参考

- Language Models Need Sleep: Learning to Self Modify and Consolidate Memories:
  https://openreview.net/pdf?id=iiZy6xyVVE
