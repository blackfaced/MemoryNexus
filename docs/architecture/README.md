# MemoryNexus Architecture

MemoryNexus is a Rust-first backend for a local-first long-term feedback engine
for personal cognition and skill acquisition.

See [ADR-022](../../decisions/ADR-022-memorynexus-brand-semantics.md) for the
current `MemoryNexus` brand semantics. Architecture docs use `MemoryNexus` for
the Engine/repository identity, not for a product-specific adapter experience.

The core ownership boundary is still `CognitiveSpace`. Users own memory through
Spaces. Agents, apps, dashboards, CLIs, and voice assistants are adapters. They
do not own memory and should not directly mutate Engine internals.

## Architecture Shape

```text
Adapters
  Chat Agent / MCP / CLI / Web / Mobile / Dashboard / Voice
  Agent / App performs OCR or ASR and confirms normalized text
      |
      +--> EvidenceResolver (conceptual; pending)
      |      optional authorized integration infrastructure
      |      -> External media providers
      |         local disk, removable drive, WebDAV, object storage
      |
      v
Surface Gateway
  auth, namespace routing, surface routing, ACL, validation,
  response shaping, Trace writing, dispatch, event contracts
  media contract: confirmed text + EvidenceRef provenance only
      |
      v
Surfaces
  Capture | Performance | Reflection | Planning | Observation
      |
      v
MemoryNexus Engine
  Namespace, Trace, MemoryAtom, CognitiveScene, FeedbackLoop,
  GrowthModel, SleepCycle, PracticePlan / DreamCandidate, Lens
      |
      +--> PostgreSQL
      |      users, cognitive spaces, namespaces, memories, traces,
      |      feedback loops, lenses, reports, future growth objects
      |
      +--> Qdrant
      |      memory embeddings scoped by space_id
      |
      +--> Optional managed object storage
             S3 / MinIO compatible provider when MemoryNexus owns bytes
```

The external-provider and `EvidenceResolver` path is conceptual and pending.
An Agent, App, or optional authorized integration would inspect external media;
Surface Gateway and the Engine retain only `EvidenceRef` provenance. This does
not imply current reference persistence or resolver capability. The existing
`src/storage/` S3 and thumbnail helper modules support the separate optional
managed-storage boundary when MemoryNexus owns bytes, but they are not currently
exposed as an operational runtime media capability.

## Surface Model

Surfaces express intent, not UI or role.

| Surface | Question | Example Actions |
| --- | --- | --- |
| Capture | What happened? | `capture_observation` |
| Performance | How did the attempt go? | `submit_attempt` |
| Reflection | What does this mean? | `review_evidence` (pending) |
| Planning | What should happen next? | `generate_next_task` (pending) |
| Observation | How is long-term state changing? | `request_consolidation`; `get_state_summary` (pending) |

Adapters decide how people or agents interact. Surface Gateway decides how a
request becomes an Engine operation.

## Engine Loop

```text
Wake:
  SurfaceRequest
  -> quick response / immediate feedback
  -> Trace
  -> optional Engine Event

Sleep:
  Trace / Memory / FeedbackLoop evidence
  -> ConsolidationResult
  -> GrowthModel update
  -> PracticePlan / DreamCandidate

Next Wake:
  user performs next task
  -> Trace
  -> effectiveness evaluation
```

## Rust Layout

```text
src/
  api/       Axum handlers and route composition
  ai/        embedding, summary, and AI provider abstractions
  auth/      JWT and password handling
  db/        PostgreSQL repositories
  domain/    functional cognitive and feedback model
  search/    keyword and semantic search orchestration
  state/     application state and repository wiring
  storage/   S3 and thumbnail storage helpers
  vector/    Qdrant vector store and vector repository
  bin/       memorynexus-cli, memorynexus-mcp, memorynexus-eval

migrations/ PostgreSQL schema migrations
tests/      integration test entry points
```

Domain code should express Engine meaning, not Axum or SQLx details.

## Current Request Flow Examples

### Memory Create

```text
POST /api/v1/memories
  -> resolve Cognitive Space
  -> persist memory in PostgreSQL
  -> embed content if an embedder is configured
  -> upsert vector to Qdrant with space provenance
```

### Search

```text
GET /api/v1/search?q=...&space_id=...&semantic=true
  -> resolve Cognitive Space
  -> embed query
  -> search Qdrant with space_id filter
  -> hydrate matching memories from PostgreSQL
```

### Current Surface Gateway Subset

```text
Capture / capture_observation
  -> persist Memory
  -> persist completed Trace

Performance / submit_attempt
  -> create or update FeedbackLoop attempt
  -> persist completed Trace

Observation / request_consolidation
  -> persist completed triggering Trace
  -> create manual SleepCycle linked by triggering_trace_id and input_trace_ids
```

Reflection, Planning, `get_state_summary`, scheduled Sleep, and Dreaming actions
remain pending. The current subset does not imply general event publishing or a
mutable Trace lifecycle.

## Cognitive And Feedback Model

- `Namespace` partitions a Space into long-running domains.
- `Trace` records interactions and runtime/effectiveness evidence.
- `MemoryAtom` extracts minimal cognitive signals.
- `CognitiveScene` consolidates long-running themes or practice fields.
- `FeedbackLoop` tracks goal, task, attempt, evaluation, feedback, and next task.
- `GrowthModel` records namespace-specific strengths, weaknesses, patterns, and
  recommended focus.
- `SleepCycle` performs offline consolidation.
- `PracticePlan` / `DreamCandidate` represents candidate next practice, review
  questions, scenarios, or plans.
- `Lens` is an interpretation strategy, not an agent identity.

## Event-driven Backend

Foreground operations should be fast. Deep work should happen in background or
manual Sleep paths.

Example events:

- `ObservationCaptured`
- `AttemptSubmitted`
- `FeedbackGenerated`
- `SleepCycleRequested`
- `GrowthModelUpdated`
- `PlanGenerated`

The first event implementation can be in-process or persisted. A distributed
queue is not required until the MVP proves the loop.

## Current Constraints

- Rust + Axum is the only main backend.
- No second frontend/backend stack without ADR.
- `CognitiveSpace` remains the permission boundary.
- `Namespace` is not a permission model.
- Trial and Local One-click paths should remain deterministic/local-first by
  default.
- Dictation Coach is the first upstream product direction, but the Engine should
  stay domain-general.

## Related Docs

- [Vision](../vision.md)
- [MemoryNexus Engine](memorynexus-engine.md)
- [Surfaces and Adapters](surfaces-and-adapters.md)
- [Surface Gateway](surface-gateway.md)
- [Sleep-driven Feedback Loop](sleep-driven-feedback-loop.md)
- [Cognitive Concepts](../cognitive-concepts.md)
- [Trace Contract](../trace-contract.md)
- [Sleep Cycle Contract](../sleep-cycle-contract.md)
