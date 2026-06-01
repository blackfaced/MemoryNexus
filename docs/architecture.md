# MemoryNexus Architecture

MemoryNexus is a Rust-first backend for cognitive lens memory. The core
ownership boundary is `CognitiveSpace`; users and agents operate inside spaces,
but they do not own memory directly.

## Runtime Components

```text
CLI / MCP / HTTP clients
      |
      v
Rust Axum API
      |
      +--> PostgreSQL
      |      users, cognitive spaces, memories, tags, lenses, lens runs
      |
      +--> Qdrant
      |      memory embeddings scoped by space_id
      |
      +--> S3 / MinIO compatible storage
             media objects and thumbnails
```

## Rust Layout

```text
src/
  api/       Axum handlers and route composition
  ai/        embedding, summary, and AI provider abstractions
  auth/      JWT and password handling
  db/        PostgreSQL repositories
  domain/    functional cognitive model
  search/    keyword and semantic search orchestration
  state/     application state and repository wiring
  storage/   S3 and thumbnail storage helpers
  vector/    Qdrant vector store and vector repository
  bin/       memorynexus-cli, memorynexus-mcp, memorynexus-eval

migrations/ PostgreSQL schema migrations
tests/      integration test entry points
```

## Request Flow

### Memory Create

```text
POST /api/v1/memories
  -> resolve Cognitive Space
  -> persist memory in PostgreSQL
  -> embed content if an embedder is configured
  -> upsert vector to Qdrant with space provenance
```

Vector payloads include:

- `space_id`
- `memory_id`
- `user_id`
- `source_type`
- `created_at`
- `visibility`
- title/type metadata

### Search

```text
GET /api/v1/search?q=...&space_id=...&semantic=true
  -> resolve Cognitive Space
  -> embed query
  -> search Qdrant with space_id filter
  -> hydrate matching memories from PostgreSQL
```

Keyword search uses PostgreSQL full-text/ILIKE matching and the same
`CognitiveSpace` boundary.

## Cognitive Model

- `Memory` is raw or user-authored material.
- `MemoryAtom` is a minimal traceable cognitive signal extracted from Memory.
- `CognitiveScene` consolidates related atoms, reflections, concepts, beliefs,
  and contradictions into a long-running theme or practice field.
- `CognitiveSpace` is the durable ownership and permission boundary.
- `Lens` is an interpretation strategy over a space.
- `CognitiveProjection` is the Lens-specific reconstructed context for a current
  query; it is not just top-k retrieval.
- `ObserveMode` selects whether a query uses `fast`, `focused`, or `deep`
  projection behavior.
- `Reflection`, `Concept`, `Belief`, `Relation`, and `Contradiction` are domain
  primitives used by the functional core.
- `Namespace` partitions a Space into long-running domains, and `FeedbackLoop`
  tracks goal, task, attempt, evaluation, feedback, adjustment, and next task.

The Phase 5 lifecycle direction is:

```text
Experience / Thought / Practice
-> Memory
-> MemoryAtom
-> CognitiveScene
-> Lens-based CognitiveProjection
-> Reflection / Belief / Next Action
-> FeedbackLoop
```

The lifecycle is mode-aware:

```text
fast:
recent memories + pinned facts + high-salience scenes + compressed profile
-> low-latency response

focused:
one primary Lens + limited scenes / atoms
-> short CognitiveProjection

deep:
multi-lens projection + atomization + consolidation + belief / contradiction update
-> review, next action, or practice adjustment
```

See [cognitive-concepts.md](cognitive-concepts.md) for definitions and
[cognitive-architecture.md](cognitive-architecture.md) for the theoretical
architecture.

## Current Constraints

- The CLI is the primary manual MVP surface. MCP is the primary local agent
  surface for Claw/Hermes-style clients.
- Semantic search is available when `QDRANT_URL` and an embedding provider are
  configured.
- `MEMORYNEXUS_EMBEDDING_PROVIDER=local` is intended for deterministic local
  smoke tests.
- Lens persistence is available through REST and CLI create/list/get commands.
- MCP exposes agent bootstrap tools for creating spaces and lenses, plus
  memory write/search, Lens Run, profile projection, reminders, and routing.
- Lens Run execution is synchronous in the MVP: it retrieves memories inside the
  Lens's Cognitive Space, persists matched memory IDs, and stores traceable
  output with summary provider provenance.
- Lens Run uses an OpenAI-compatible chat summarizer when summary credentials
  are configured; `MEMORYNEXUS_AI_BASE_URL` / `OPENAI_BASE_URL` can point at
  providers such as OpenRouter. Without credentials, it records a deterministic
  fallback summary.
- Strategy compilation, richer multi-step interpretation, and async Lens
  workflows are the next runtime layer.
