# ADR-002: Cognitive Lens Memory 产品方向

## 状态
✅ 已接受

## 背景

MemoryNexus 的早期表达容易被理解为普通 agent memory：为某个 Agent 保存长期上下文，让 Agent 在后续对话中继续使用。这种方向会带来几个长期问题：

- memory 所有权绑定到 Agent，导致模型、prompt 或 Agent 实现切换时记忆连续性变弱。
- 多个 Agent 之间容易形成重复、冲突或不可追溯的私有记忆。
- 家庭、个人、项目等真实使用场景需要稳定的记忆空间，而不是围绕单个 Agent 组织。
- 同一批事实和事件在不同场景下需要不同解释方式，例如学习复盘、家庭成长、健康观察或项目总结。

项目已经确定 Rust-first 后端主线，并将 Embedding -> Qdrant -> Rust search API 的语义检索闭环作为 P0。需要进一步明确产品和架构方向：MemoryNexus 的核心不是“Agent 拥有记忆”，而是“Memory Space 被不同 Lens 解释”。

## 决策

MemoryNexus 调整为 **cognitive lens memory**。

核心模型如下：

1. **Memory Space 是核心**
   - Memory Space 是 memory 的所有权边界。
   - 用户、家庭、项目或组织拥有 Memory Space。
   - 原始 memory、元数据、向量索引、关系、派生洞察都必须能追溯到 Memory Space。

2. **Agent 不拥有 memory**
   - Agent 是执行者、解释者或调用方，不是记忆所有者。
   - Agent 可以向 Memory Space 写入 memory，也可以基于权限读取、检索和生成派生内容。
   - Agent 可以被替换；memory 的连续性由 Memory Space 维持。

3. **Lens 是认知视角 / interpretation strategy**
   - Lens 定义如何理解 Memory Space：关注范围、检索策略、排序规则、摘要风格、冲突处理和输出结构。
   - Lens 不复制原始 memory，不形成新的存储孤岛。
   - Lens 输出是可追溯、可重算、可审计的派生解释。

4. **Rust-first 主路径不变**
   - Memory Space、Lens、语义检索和 AI 编排的新功能默认落在 Rust + Axum 服务。
   - 不引入第二套后端主线。

## 架构影响

- 数据模型需要显式引入或强化 `memory_space` 边界。
- memory、embedding、object storage metadata、search payload、Lens Run 都需要携带 `space_id`。
- 搜索和摘要 API 应以 Memory Space 作为上下文，以 Lens 作为可选或必选解释策略。
- Agent 相关字段只能表达调用者、执行者、来源或审计信息，不能表达 memory 所有权。
- 派生洞察需要保留 provenance：原始 memory、Lens 策略版本、模型配置、生成时间和调用 Agent。

## 后果

**正面：**
- 产品定位从单 Agent 记忆插件升级为长期可复用的认知空间。
- Agent、模型和 prompt 可以迭代替换，不破坏 memory 连续性。
- 同一份 memory 可以被多个 Lens 解释，适合家庭、学习、项目、健康等多场景。
- 检索、摘要和洞察更容易审计，因为来源绑定到 Memory Space 和 Lens Run。

**负面：**
- 数据模型会比普通 agent memory 更复杂，需要维护 Space、Lens、Run、provenance 等概念。
- API 需要更明确的上下文参数，不能默认假设“当前 Agent 的 memory”。
- Lens 输出的缓存和持久化需要谨慎设计，避免派生解释污染原始 memory。
- 旧文档中与 Agent 私有记忆相关的描述需要持续清理。

## Phase 1-4 接续任务

详细路线图见 [Cognitive Lens Memory Roadmap](../docs/cognitive-lens-roadmap.md)。

- Phase 1: Memory Space 基础闭环，完成空间归属和语义检索端到端路径。
- Phase 2: Lens 最小模型，定义可配置、可复用、可审计的解释策略。
- Phase 3: Cognitive Lens 工作流，引入 Lens Run、多步解释和派生洞察。
- Phase 4: 多主体协作与开放接口，让外部 Agent 基于权限使用 Memory Space。

## 相关决策

- ADR-001: 使用 Rust 作为后端语言
- ADR-003: Qdrant 向量数据库
- ADR-008: 数据库设计
- ADR-009: Rust-first 后端主线
