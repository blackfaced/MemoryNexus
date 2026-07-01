# MemoryNexus Vision

MemoryNexus is a local-first long-term feedback engine for personal cognition
and skill acquisition.

It should not be positioned as a generic recall product, personal knowledge
vault, agent recall store, connector platform, or RAG profile API. Those are
crowded and infrastructure-heavy spaces. MemoryNexus should focus on what
happens after memory is captured:

```text
Trace -> FeedbackLoop -> GrowthModel -> PracticePlan
```

The guiding question is:

```text
How can a system use long-term traces to generate better feedback and next
actions over time?
```

## Ecosystem Boundary

MemoryNexus should be understood as the memory evolution and feedback layer.
See [ADR-022](../decisions/ADR-022-memorynexus-brand-semantics.md) for the
current brand semantics: MemoryNexus is the Engine/repository identity;
Dictation Coach is the first upstream product scenario.

| Layer | Role | MemoryNexus Boundary |
| --- | --- | --- |
| OpenJarvis | Local Personal AI Runtime | Useful reference for local-first runtime and trace learning, but MemoryNexus does not become a model runtime or agent framework. |
| Supermemory / Mem0 | Memory Runtime / Memory Cloud | Useful reference for memory APIs and agent recall, but MemoryNexus does not compete on connectors, generic RAG, or agent profiles. |
| MemoryNexus | Memory Evolution / Feedback / Growth Engine | Owns long-term feedback loops, growth models, sleep consolidation, planning, and skill acquisition. |

## Product Thesis

Most AI memory products answer:

```text
What should the AI remember?
```

MemoryNexus answers:

```text
Given what happened over time, what feedback and next step should the system
generate now?
```

That means the core value is not recall volume. The value is long-term
improvement:

- detect recurring mistakes and patterns;
- explain what those patterns mean;
- update a namespace-specific GrowthModel;
- generate the next useful practice, reflection, or action;
- evaluate whether the next step helped.

## Core Principles

- Local-first by default. Cloud generation is optional, explicit, and traceable.
- Memory belongs to a user-owned `CognitiveSpace`, not to an agent or app.
- `Namespace` partitions a Space into long-running domains, not permission
  boundaries.
- Adapters are interaction channels. Surfaces are intent capabilities. Engine is
  the long-term evolution core.
- Surface Gateway is the only supported external entry into Engine capabilities.
- Deep cognition is not synchronous by default. Wake paths are fast; Sleep paths
  consolidate later.
- Lenses are interpretation strategies, not agent personas.
- First versions should validate feedback loops before adding OCR, complex UI,
  multi-child management, or broad subject coverage.

## First Upstream Product

The first upstream product direction is Dictation Coach: a daily dictation
assistant for Chinese native-language dictation and English spelling / sentence
dictation.

It validates the full MemoryNexus loop:

1. Capture today's words, phrases, or sentences.
2. Submit a dictation or spelling attempt.
3. Reflect on mistake causes and recurring patterns.
4. Plan tomorrow's 10-minute practice.
5. Observe the last 7 days of stability, mastery, and error distribution.
6. Run a SleepCycle to update GrowthModel and PracticePlan.

Dictation Coach is an upstream product, not the Engine itself. The Engine should
remain domain-general enough to later support piano, chess, drawing, programming,
project work, or personal reflection through the same Surface model.

## Current Gap

The current repository already has strong foundations:

- Rust-first backend;
- `CognitiveSpace` ownership;
- Namespace and FeedbackLoop foundation;
- Trace contract and schema direction;
- SleepCycle contract;
- Thought Review demo;
- STEM learning practice slice.

The main gap is architectural expression:

- Some historical docs and compatibility paths still read like an AI thought
  organizer, cognitive lens memory system, or broad STEM learning tool.
- Architecture docs do not yet separate Engine, Surfaces, Adapters, and Surface
  Gateway.
- Roadmap still centers Phase 5 around lifecycle primitives instead of the new
  seven-milestone feedback-engine plan.
- Existing learning docs are STEM/fraction-centered, while the next upstream
  product should be Dictation Coach.
- Event-driven backend boundaries need to be explicit before implementation.

The next milestone is therefore documentation and issue hygiene, not business
code.
