# ADR-016: Local-first Trace Learning Runtime

## 状态
✅ 已接受

## 背景

MemoryNexus 已经通过 ADR-014 从 cognitive lens memory system 扩展为
namespace-based long-term feedback substrate。它的核心不再只是保存 Memory，而是把
长期经历、练习、反馈和复盘转化为可持续演化的认知结构。

OpenJarvis 提供了另一个重要参考：personal AI 不应默认把私密上下文发送给云端模型，
而应优先运行在用户设备上，并把效率、成本、延迟和本地 trace learning 作为一等约束。
OpenJarvis 的定位是 local-first personal AI runtime；MemoryNexus 不应复制它的完整
agent runtime / inference engine，但应吸收三个架构信号：

```text
local-first
trace-driven learning
efficiency-aware evaluation
```

这也回应了 MemoryNexus 自身的双系统设计问题：

```text
fast / System 1    -> 低延迟、低成本、尽量本地
focused / balanced -> 单 Lens、有限上下文、可追溯
deep / System 2    -> 高质量整合、可接受更高延迟和成本
```

如果 MemoryNexus 不记录 Trace，它只能知道“记住了什么”。如果记录 Trace，它才能知道：

- 哪个 runtime / model / fallback 被使用。
- 一次 Lens Run、Review、MCP tool call 或 FeedbackLoop 花了多久。
- 哪些 Memory、Reflection、FeedbackLoop 或 Review 是由这次交互产生的。
- 用户是否认为这次反馈有用。
- 某个 namespace 的反馈策略是否随着时间变好。

因此 Trace 是连接认知演化和系统优化的关键对象。

## 决策

MemoryNexus 将引入 `Trace` 作为一等架构概念，用于记录交互、运行时和反馈效果。

### Trace 的定位

`Trace` 不是普通日志，也不是新的用户可见笔记。它是：

```text
one interaction / execution
with inputs, outputs, runtime metrics, generated object links, and user feedback
```

Trace 用于：

- local-first execution metrics。
- ObserveMode 的成本和延迟边界。
- Lens / Review / FeedbackLoop 的效果评估。
- STEM Learning Feedback 中“反馈是否带来改善”的长期观察。
- 未来 local/cloud routing policy。

Trace 不取代：

- `Memory`：Memory 仍然是用户可回忆、可检索的认知材料。
- `FeedbackLoop`：FeedbackLoop 仍然表达 goal -> task -> attempt -> feedback -> next task。
- `LensRun`：LensRun 仍然是某个 Lens 的可追溯解释结果。
- `ReviewReport`：ReviewReport 仍然是一个时间窗口上的复盘输出。

Trace 连接这些对象，但不拥有它们。

### Local-first runtime boundary

MemoryNexus 不成为 OpenJarvis 式完整 local personal AI runtime。它不负责：

- model catalog。
- inference engine。
- local accelerator management。
- agent presets。
- automatic fine-tuning / prompt optimization system。

MemoryNexus 负责在自己的 Rust-first backend 内记录和使用 runtime signals：

```text
runtime = local | cloud | hybrid | deterministic
mode = fast | focused | deep
latency_ms
token_usage
estimated_cost
local_processing_ratio
generated object IDs
user feedback
```

### Trace-first learning loop

长期方向：

```text
Interaction / Practice / Review
→ Trace
→ Memory / FeedbackLoop / LensRun / ReviewReport links
→ metrics + user feedback
→ namespace-specific improvement signal
→ better next action / practice plan / routing policy
```

第一阶段不要求自动学习模型权重，也不要求复杂异步 pipeline。先记录足够稳定的 Trace
contract，让后续 issue 可以逐步落地。

### Mode-aware metrics

每个 Trace 必须能表达 ObserveMode：

- `fast`：低延迟优先，默认本地 / deterministic / compressed context。
- `focused`：单 Lens 或有限上下文，平衡质量和成本。
- `deep`：用户主动复盘、周报、学习计划或项目决策，高延迟可接受。

Trace metrics 应帮助验证：

```text
fast mode 不运行 deep pipeline
deep mode 的成本和延迟是显式触发的
local-first fallback 可用且可度量
```

## 后果

正面：

- MemoryNexus 的定位从 memory evolution / feedback substrate 进一步明确为
  local-first trace-driven feedback engine。
- Trace 给 Lens、Review、FeedbackLoop 和 Namespace 提供共同的运行时和效果证据。
- 后续可以用同一套对象衡量 latency、cost、local/cloud ratio 和 feedback effectiveness。
- STEM Learning Feedback 可以回答“这次反馈是否帮助下一次练习变好”。
- MemoryNexus 可以和 OpenJarvis / Supermemory 区分边界：它不做完整 runtime，也不做
  通用 memory cloud，而做长期反馈和成长演化。

负面：

- 概念层增加，必须避免把 Trace 暴露成普通用户主入口。
- 过早落库可能导致 schema 先行；第一阶段应先定义 contract，再选择最小落地面。
- Trace 涉及输入、输出和运行时元数据，必须明确隐私、保留策略和 redaction 边界。
- 如果把 Trace 当成万能 analytics，可能稀释 MemoryNexus 的核心反馈循环。

## 后续任务

第一批任务应按以下顺序推进：

1. 定义 Trace domain contract。
2. 设计 Trace schema / repository 的最小落地方案。
3. 为 Lens Run / MCP tool call / FeedbackLoop / ReviewReport 捕获 lightweight Trace。
4. 为 ObserveMode 增加 runtime metrics 和 local/cloud/deterministic runtime 字段。
5. 在 STEM Learning Feedback 中使用 Trace 评估 feedback effectiveness。

## 相关决策

- ADR-009: Rust-first 后端主线
- ADR-013: Thought Review UI MVP
- ADR-014: Namespace and Feedback Loop Model
- ADR-015: Supabase Integration Boundary

## 参考

- OpenJarvis documentation: https://open-jarvis.github.io/OpenJarvis/
- Stanford Scaling Intelligence Lab OpenJarvis blog: https://scalingintelligence.stanford.edu/blogs/openjarvis/
- OpenJarvis GitHub repository: https://github.com/open-jarvis/OpenJarvis
