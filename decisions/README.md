# 📋 Architecture Decision Records (ADR)

> 架构决策记录 - MemoryNexus 项目关键决策文档

## 📁 目录结构

```
decisions/
├── ADR-001-backend-language.md    ✅ Rust + Axum 后端选型
├── ADR-002-cognitive-lens-memory.md ✅ Cognitive Lens Memory 产品方向
├── ADR-002-storage-abstraction.md ✅ 存储层抽象设计
├── ADR-003-vector-database.md    ✅ Qdrant 向量数据库
├── ADR-004-whisper-deployment.md ✅ Whisper 语音方案
├── ADR-005-project-naming.md     ✅ 项目命名选择
├── ADR-006-cli-agent-interface.md ✅ CLI + Agent 双模式接口
├── ADR-007-development-methodology.md ✅ 开发方法论
├── ADR-008-database-design.md    ✅ 数据库设计
├── ADR-009-rust-first-backend.md ✅ Rust-first 后端主线
├── ADR-010-memory-salience.md    ✅ Memory salience 与 automatic forgetting
├── ADR-011-contradiction-lifecycle.md ✅ Contradiction 生命周期
├── ADR-012-family-shared-cognitive-space.md ✅ 家庭与共享 Cognitive Space
├── ADR-013-thought-review-ui-mvp.md ✅ Thought Review UI MVP
├── ADR-014-namespace-feedback-loop.md ✅ Namespace 与 Feedback Loop 模型
├── ADR-015-supabase-integration.md ✅ Supabase 集成边界
├── ADR-016-local-first-trace-learning-runtime.md ✅ Local-first Trace Learning Runtime
├── ADR-017-sleep-based-memory-consolidation.md ✅ Sleep-based Memory Consolidation
├── ADR-018-long-term-feedback-engine.md ✅ Long-term Feedback Engine 定位
├── ADR-019-surfaces-adapters-engine.md ✅ Surfaces / Adapters / Engine 分层
├── ADR-020-dictation-coach-first-upstream-product.md ✅ Dictation Coach 首个上游产品
├── ADR-021-external-media-evidence-references.md ✅ 外部媒体证据引用
├── ADR-022-memorynexus-brand-semantics.md ✅ MemoryNexus 品牌语义
├── ADR-023-namespace-knowledge-refresh.md ✅ Namespace Knowledge Refresh
├── ADR-024-private-mac-mini-local-lab.md ✅ Private Mac mini Local Lab
├── ADR-025-personal-feedback-dogfood.md ✅ Personal Feedback Dogfood
├── ADR-026-loose-coupled-source-adapter-sync.md ✅ Loose-Coupled Source Adapter Synchronization
└── ADR-027-sqlite-cli-minimax-feedback-kernel.md ✅ SQLite CLI MiniMax Personal Experiment Feedback Kernel (current default)
```

## 当前默认方向

[ADR-027](ADR-027-sqlite-cli-minimax-feedback-kernel.md) 是当前默认产品方向：本地 SQLite
权威 ledger、用例级 CLI 和 MiniMax Skill，以 Observation、Recommendation、Experiment、
Outcome 四对象支持 personal experiment feedback。它以 #274 的 MiniMax 可行性验证为实施
前置条件；#274 已通过，并确认定时任务结果采用 owner-initiated pull，而非主动推送。

ADR-001 至 ADR-026 保留为历史决策与退役 runtime 的依据；除非 ADR-027 明确保留，不应把它们
解释为新的默认实现路线。

## 📖 ADR 是什么？

ADR（Architecture Decision Record）是记录重要架构决策的文档，包含：

| 字段 | 说明 |
|------|------|
| **状态** | 已接受 / 已废弃 / 待定 |
| **背景** | 决策的背景和动机 |
| **决策** | 最终的选择和理由 |
| **后果** | 正面/负面影响 |

## 🔗 相关资源

- [架构设计文档](../docs/architecture/README.md)
- [Cognitive Lens 路线图](../docs/cognitive-lens-roadmap.md)

## 📝 如何贡献新决策

1. 创建新文件 `ADR-00X-feature-name.md`
2. 使用标准模板
3. 更新本文档目录
4. 提交 PR

## 标准模板

```markdown
# ADR-00X: 决策标题

## 状态
✅ 已接受

## 背景
...

## 决策
...

## 后果
正面：
- ...

负面：
- ...

## 相关决策
- ADR-001: ...
```
