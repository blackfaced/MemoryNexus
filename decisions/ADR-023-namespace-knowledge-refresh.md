# ADR-023: Namespace Knowledge Refresh

## 状态

✅ 已接受

## 背景

MemoryNexus 已经形成 `Trace -> FeedbackLoop -> GrowthModel -> PracticePlan`
的长期反馈主线。随着 Dictation Coach 和更多 namespace-based 上游产品推进，外部
Skills、Agents、Adapters 可能会发现有用的参考材料：词表来源、课程规则、拼写/默写
rubric、项目文档、领域最佳实践、公开资料或用户明确授权后的私有上下文衍生来源。

这些外部知识可以帮助后续 review、Dreaming、练习设计和缺口识别，但它们不能混入
MemoryNexus 的核心 ownership model：

- Memory 属于 `CognitiveSpace`，不属于 Agent。
- Namespace 是 Space 内的领域分区，不是权限边界。
- 外部知识不是用户 Memory。
- 外部知识不能直接覆盖 `GrowthModel` 或 `PracticePlan`。

因此需要一个 docs-first 的 Namespace Knowledge Refresh 决策：外部系统可以发现和抽取
来源候选，MemoryNexus 只负责校验 scoped input、approval state、provenance、quality、
freshness、privacy opt-in 和 downstream links。

## 决策

引入 Namespace Knowledge Refresh 作为 Engine contract boundary。

V1 合同对象为：

- `KnowledgeSourceCandidate`
- `KnowledgeSourcePolicy`
- `KnowledgeContext`
- `AcquisitionTrace`

规范以 [Knowledge Refresh Contract](../docs/knowledge-refresh-contract.md) 为准。

### 边界

外部 Skills、Agents、Adapters 可以：

- discover source candidates;
- fetch external sources;
- extract candidate claims;
- prepare short evidence snippets;
- submit provenance and quality signals;
- provide explicit opt-in proof when private context influenced discovery or
  extraction.

MemoryNexus 可以：

- validate `CognitiveSpace` and Namespace scope;
- validate candidate approval state;
- validate source policy state;
- validate `AcquisitionTrace`;
- validate provenance, quality, freshness, expiry, privacy opt-in, and
  downstream links;
- store bounded structured claims, provenance, quality/freshness/expiry
  signals, policy links, downstream links, and short evidence snippets.

MemoryNexus must not:

- crawl, fetch, search, subscribe to RSS, or call provider APIs in V1;
- add a Knowledge Surface in V1;
- add a scheduler for refresh in V1;
- persist full external articles, raw provider payloads, or full corpora by
  default;
- treat external knowledge as user Memory;
- directly update `GrowthModel` or `PracticePlan` from external knowledge.

### Scope

Every Knowledge Refresh object must carry:

- `space_id`
- `namespace_id`

`space_id` remains the ownership and permission boundary. `namespace_id` is the
domain partition inside that Space. Cross-Space submissions are invalid.
Cross-Namespace submissions are invalid unless a future contract explicitly
allows them with a recorded reason.

### Approval and Privacy

Candidate source states are:

```text
proposed
approved
rejected
expired
```

Source policy states are:

```text
active
paused
revoked
expired
```

External submissions require `AcquisitionTrace`.

If `private_context_used = true`, the submission must include scoped opt-in
proof. Private Trace-derived discovery without opt-in proof is invalid input.

### Downstream Use

`KnowledgeContext` is candidate context for future work. It may later inform
manual SleepCycle / Dreaming when a separate issue wires that path, but local
Trace, FeedbackLoop, and GrowthModel evidence remain higher-priority evidence
about the user's actual behavior.

Conflicts between external claims and local evidence should become review
questions, hypotheses, experiment candidates, or `DreamCandidate` inputs. They
must not silently overwrite `GrowthModel`, mark `PracticePlan` obsolete, or
mutate user Memory.

## 后果

正面：

- Gives #198 and #199 a durable architecture boundary before implementation.
- Preserves `CognitiveSpace` ownership and Namespace partitioning.
- Allows external tools to do discovery and extraction without turning
  MemoryNexus into a crawler, provider client, or knowledge base.
- Keeps external knowledge bounded, reviewable, approval-gated, and
  provenance-rich.
- Maintains the feedback-engine distinction between user evidence and external
  reference context.

负面：

- Adds a new contract family that future workers must keep aligned across docs,
  Surface contracts, and any later schema.
- Requires explicit policy and acquisition trace handling before external
  knowledge can safely reach Sleep/Dreaming.
- Does not solve external source acquisition; product adapters or external
  Skills still need to own fetching and extraction.

## 非目标

- Do not implement Rust schema, API routes, migrations, repositories, frontend
  UI, provider integrations, crawlers, search adapters, or schedulers in this
  ADR.
- Do not add a Knowledge Surface in V1.
- Do not wire `KnowledgeContext` into `SleepCycle`, Dreaming, `GrowthModel`, or
  `PracticePlan` here.
- Do not store full external corpora by default.
- Do not change Engine ownership: `CognitiveSpace` remains the permission
  boundary and Namespace remains a partition inside it.

## 相关决策

- [ADR-014: Namespace and Feedback Loop](ADR-014-namespace-feedback-loop.md)
- [ADR-016: Local-first Trace Learning Runtime](ADR-016-local-first-trace-learning-runtime.md)
- [ADR-017: Sleep-based Memory Consolidation](ADR-017-sleep-based-memory-consolidation.md)
- [ADR-018: MemoryNexus as Long-term Feedback Engine](ADR-018-long-term-feedback-engine.md)
- [ADR-019: Surfaces vs Adapters vs Engine](ADR-019-surfaces-adapters-engine.md)
- [ADR-021: External Media Evidence References](ADR-021-external-media-evidence-references.md)
