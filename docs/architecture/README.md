# MemoryNexus Architecture

MemoryNexus is a Rust-first backend for a local-first, namespace-based
long-term feedback engine.

The core ownership boundary is still `CognitiveSpace`. Users own memory through
Spaces. Agents, apps, dashboards, CLIs, and voice assistants are adapters. They
do not own memory and should not directly mutate Engine internals.

## Architecture Shape

```text
Adapters
  Chat Agent / MCP / CLI / Web / Mobile / Dashboard / Voice
      |
      v
Surface Gateway
  auth, namespace routing, surface routing, ACL, validation,
  response shaping, Trace writing, sync/async dispatch, events
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
      +--> S3 / MinIO compatible storage
             media objects and thumbnails
```

## Surface Model

Surfaces express intent, not UI or role.

| Surface | Question | Example Actions |
| --- | --- | --- |
| Capture | What happened? | capture, captureObservation |
| Performance | How did the attempt go? | submitAttempt, evaluateAttempt, getImmediateFeedback |
| Reflection | What does this mean? | reflect, review, explain |
| Planning | What should happen next? | plan, generateNextTask, adjustPlan |
| Observation | How is long-term state changing? | observeState, getGrowthModel, getTrends, getTimeline |

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

### Future Surface Gateway Request

```text
SurfaceRequest {
  namespace: "child.chinese.dictation",
  surface: "performance",
  action: "submitAttempt",
  actor: "...",
  adapter: "mcp",
  payload: { task, attempt },
  context: { mode: "fast", runtime_preference: "deterministic" }
}

-> validate actor and Space access
-> route namespace and surface
-> execute synchronous Performance action
-> write Trace
-> publish AttemptSubmitted event
-> return shaped SurfaceResponse
```

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
