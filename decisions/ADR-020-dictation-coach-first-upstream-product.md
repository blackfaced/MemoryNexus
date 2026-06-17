# ADR-020: Dictation Coach as First Upstream Product

## 状态
✅ 已接受

## 背景

`learning.stem` 和 elementary fraction word problems 已经验证了
FeedbackLoop、practice session、weekly review、MCP flow 和 Rust-served UI 的一条
学习产品路径。但后续产品方向需要一个更强的日常闭环场景。

Dictation Coach 更适合作为第一个上游产品：

- 高频、短周期、天然 daily rhythm。
- Capture / Performance / Reflection / Planning / Observation 五个 Surface 都能用上。
- 错因类型清晰，第一版可 deterministic。
- 不需要 OCR，手动输入即可验证闭环。
- 中文默写和英语 spelling / sentence dictation 都能共享 Engine。

## 决策

第一个上游产品场景定位为：

```text
Dictation Coach / 每日默写助手
```

它用于：

- 中文母语默写；
- 英语 spelling；
- sentence dictation。

底层项目不绑定“家长 / 孩子”角色。角色和交互方式属于 Adapter。Engine 只关心
Namespace、Trace、FeedbackLoop、GrowthModel、SleepCycle 和 PracticePlan。

第一批命名建议：

```text
child.chinese.dictation
child.english.spelling
child.english.sentence-dictation
```

## 典型闭环

```text
Capture:
  record today's words, phrases, or sentences

Performance:
  submit dictation / spelling attempt

Reflection:
  classify mistake causes and recurring patterns

Planning:
  generate tomorrow's 10-minute practice

Observation:
  show 7-day error patterns, mastery, and stability

SleepCycle:
  consolidate traces, update GrowthModel, generate next PracticePlan
```

## 第一版错因分类

中文：

- 错别字。
- 形近字。
- 同音字。
- 漏笔画。
- 多笔画。
- 笔顺问题。
- 部件位置错误。

英语：

- 漏字母。
- 多字母。
- 字母顺序错误。
- 双写错误。
- 发音-拼写映射错误。
- 大小写错误。
- 句子漏词。

## 非目标

- 不做 OCR。
- 不做复杂 UI。
- 不做多孩子管理。
- 不做全科目平台。
- 不做完整英语学习系统。
- 不做自动批改所有手写输入。
- 不把“家长端 / 孩子端”写进 Engine。

## 后果

正面：

- 产品入口更具体，普通用户更容易理解。
- 可用 deterministic 规则先验证反馈闭环。
- Surface Gateway 的五个 Surface 都有实际用例。
- SleepCycle / GrowthModel / PracticePlan 有清晰评估方式。

负面：

- 需要把现有 `learning.stem` 文档定位为 prior learning slice，而不是下一阶段唯一 MVP。
- 部分现有 API/MCP copy 仍然是 STEM/fraction，需要后续 Surface Gateway 或 Dictation
  Adapter 包装。
- Dictation taxonomy 需要防止过早扩展成完整教育平台。

## 相关决策

- ADR-014: Namespace and Feedback Loop Model
- ADR-016: Local-first Trace Learning Runtime
- ADR-017: Sleep-based Memory Consolidation
- ADR-018: MemoryNexus as Long-term Feedback Engine
- ADR-019: Surfaces vs Adapters vs Engine
