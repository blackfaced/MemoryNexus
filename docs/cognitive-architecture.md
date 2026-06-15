# Cognitive Architecture

> The engineering shape for turning the Cognitive Manifesto into Rust code.

> Status update: this remains the theoretical functional-core reference for
> cognitive objects. The current product architecture is now the long-term
> feedback engine described in [Architecture](architecture/README.md),
> [MemoryNexus Engine](architecture/memorynexus-engine.md), and
> [Surfaces and Adapters](architecture/surfaces-and-adapters.md). New work should
> route external apps and agents through Surface Gateway rather than directly
> exposing Engine internals.

## Goal

MemoryNexus should evolve toward a Rust-first cognitive architecture where the core model is not CRUD over notes, but state evolution over a Cognitive Space.

The guiding architecture is:

```text
Functional Core + Imperative Shell
```

The functional core contains cognitive types and pure transformations. The shell contains HTTP routes, database access, vector search, object storage, and LLM calls.

## Layering

```text
src/
  domain/
    memory.rs
    reflection.rs
    concept.rs
    belief.rs
    relation.rs
    contradiction.rs
    lens.rs
    space.rs
    event.rs
    state.rs
    evolution.rs

  api/
    spaces.rs
    observe.rs
    reflect.rs

  db/
    spaces.rs
    cognitive_objects.rs

  ai/
    lens_executor.rs

  vector/
    retrieval.rs
```

The exact file names can change during implementation, but the boundary should remain:

- `domain/`: no Axum, no SQLx, no reqwest, no environment variables.
- `api/`: translates HTTP requests into domain commands.
- `db/`: persists and loads cognitive objects.
- `ai/`: executes Lens reasoning through LLM or other engines.
- `vector/`: retrieves related memory candidates.

## Domain Core

The domain core should define:

```rust
Memory
Reflection
Concept
Belief
Relation
Contradiction
CognitiveSpace
Lens
CognitiveEvent
CognitiveState
```

The most important design rule:

```text
Domain code should express what cognition means, not how PostgreSQL or Axum works.
```

## Functional State Evolution

The core flow should be modeled as transformations:

```text
CognitiveState + CognitiveEvent -> CognitiveState
```

Rust shape:

```rust
pub fn evolve(state: CognitiveState, event: CognitiveEvent) -> CognitiveState
```

This makes the system testable without a database, HTTP server, vector store, or LLM.

`CognitiveProfile` is a projection of that state for external consumers:

```text
CognitiveProfile = project(CognitiveState, target_use)
```

Profile is for LLM, MCP, and UI context. It can contain stable beliefs, active
concepts, current goals, unresolved contradictions, a compact summary, and source
IDs. It does not own memory and does not replace `CognitiveSpace`; it cites
`source_memory_ids` and `source_event_ids` so the projection remains traceable.

Memory salience is part of the functional core. Automatic forgetting means
deprioritizing a Memory for default projections, not deleting it from the
`CognitiveSpace`. A deprioritized Memory remains available for explicit recall,
audit, contradiction review, and later reprioritization.

Contradiction is also part of the functional core. It should carry lifecycle
state, source memory IDs, belief IDs, Lens IDs, confidence, resolution mode, and
the event that last changed it. `CognitiveProfile` should expose unresolved
contradictions for LLM, MCP, and UI consumers while keeping resolved or plural
truth tensions available in `CognitiveState`.

## Minimal Cognitive Loop

The first real loop should be:

```text
capture memory
-> retrieve related memories
-> apply lens
-> generate reflection
-> extract concept candidates
-> detect contradiction
-> update cognitive state
```

This loop should validate two claims:

```text
same Memory + different Lens -> different Reflection
```

and:

```text
repeated Reflections -> Concepts -> Beliefs
```

## Lens Execution Boundary

Lens should not be hard-coded as a prompt string.

Domain-level Lens should be structured:

```rust
pub struct Lens {
    pub id: LensId,
    pub name: String,
    pub attention: AttentionPolicy,
    pub interpretation: InterpretationPolicy,
    pub abstraction: AbstractionPolicy,
    pub contradiction: ContradictionPolicy,
    pub narrative: NarrativePolicy,
}
```

The AI layer may compile this into a prompt, but the domain should remain independent from a specific model provider.

The first built-in Lenses:

- Detective Lens
- Emotional Lens
- Systems Lens
- Narrative Lens
- Philosopher Lens

## API Shape

Avoid low-level APIs like:

```text
GET /memories?lens=systems
```

Prefer cognition-oriented APIs:

```text
POST /api/v1/spaces
POST /api/v1/spaces/:space_id/memories
POST /api/v1/spaces/:space_id/observe
POST /api/v1/spaces/:space_id/reflect
GET  /api/v1/spaces/:space_id/topology
```

The caller asks the system to observe, reflect, or inspect topology. The implementation decides how to retrieve, route, and execute the Lens.

## Persistence Shape

The database should eventually reflect the domain model:

```text
cognitive_spaces
space_members
memories
reflections
concepts
beliefs
relations
contradictions
lenses
cognitive_runs
memory_salience
```

Ownership rule:

```text
space_id owns cognitive objects.
created_by records provenance.
agent_id, if present, records execution source only.
```

Agent must not be the memory ownership boundary.

## Vector Retrieval

Vector retrieval is an adapter, not the cognitive model.

It should answer:

```text
Which memories may be relevant to this observation or reflection?
```

It should not decide:

```text
What does this memory mean?
```

Meaning construction belongs to Lens execution and domain evolution.

## Cognitive Router

The Cognitive Router is a future layer that selects Lens activation.

Inputs:

- user topic
- current CognitiveState
- active Contradictions
- memory topology
- recurring Concepts
- stable Beliefs

Output:

- selected Lens or Lens sequence
- reason for activation
- traceable CognitiveEvent

The router is not a generic tool router. It is perspective activation.

## Rust Style

This project should use Rust to practice precise domain modeling:

- Use `struct` for stable cognitive objects.
- Use `enum` for cognitive events, relation kinds, contradiction status, and lens modes.
- Use newtype IDs where useful, such as `MemoryId`, `LensId`, `SpaceId`.
- Prefer pure functions in `domain/evolution.rs`.
- Keep `Result<T, DomainError>` explicit at domain boundaries.
- Use traits for ports: repositories, lens executors, retrievers.
- Keep Axum extractors and SQLx rows outside the domain core.

Category theory can inform the model, but should not dominate the first implementation. Useful mental mapping:

- CognitiveSpace: stateful substrate
- Lens: transformation over context
- Relation: topology edge
- Evolution: state transition
- Contradiction: tension that triggers further transformation

## Phase 1 Engineering Target

After Phase 0 documentation, the next implementation target is:

```text
src/domain/
```

with unit tests proving:

- same Memory through different Lens creates different Reflection candidates
- repeated Reflections can produce Concept candidates
- Contradictions are preserved as first-class tension
- `CognitiveState + CognitiveEvent -> CognitiveState` works without database access

Only after that should API routes and persistence be reshaped around the domain.
