# MemoryNexus API

The Rust API is the main backend path. Memory ownership is scoped by
`CognitiveSpace`; users and agents operate inside spaces instead of owning
memory directly.

## Conventions

- Base URL: `http://localhost:8080`
- API prefix: `/api/v1`
- Authenticated endpoints require `Authorization: Bearer <JWT>`.
- Responses are JSON and generally follow:

```json
{
  "ok": true,
  "data": {}
}
```

## Auth

### Register

`POST /api/v1/auth/register`

```json
{
  "email": "alice@example.com",
  "username": "Alice",
  "password": "secret123"
}
```

Registration also creates a default personal Cognitive Space for the user.

### Login

`POST /api/v1/auth/login`

```json
{
  "email": "alice@example.com",
  "password": "secret123"
}
```

The auth response includes `data.token`.

## Cognitive Spaces

### Create Space

`POST /api/v1/spaces`

```json
{
  "name": "Learning Space",
  "description": "Rust and cognitive memory practice"
}
```

### List Spaces

`GET /api/v1/spaces`

Returns spaces where the current user is a member.

### Get Space

`GET /api/v1/spaces/:id`

Returns a space only if the current user is a member.

## Lenses

Lens is a reusable interpretation strategy scoped to a Cognitive Space. It does
not own or copy memory; it describes how later retrieval and interpretation
should read the space.

### Create Lens

`POST /api/v1/lenses`

```json
{
  "space_id": "space-uuid",
  "name": "Project Context",
  "description": "Interpret project memory for planning",
  "strategy": "project_context",
  "output_format": "brief",
  "retrieval_mode": "semantic"
}
```

`strategy`, `output_format`, and `retrieval_mode` are persisted as explicit
configuration strings. Lens Run currently uses `retrieval_mode` to choose the
space-scoped retrieval path and records the rest as provenance.

### List Lenses

`GET /api/v1/lenses?space_id=<SPACE_ID>`

Returns lenses in the requested space if the current user is a member.

### Get Lens

`GET /api/v1/lenses/:id`

Returns a lens only if the current user can access its Cognitive Space.

## Lens Runs

Lens Run is one synchronous interpretation pass over a Lens and a query. The
server retrieves memories inside the Lens's Cognitive Space, records the matched
memory IDs, and stores a traceable JSON output.

### Run Lens

`POST /api/v1/lens-runs`

```json
{
  "lens_id": "lens-uuid",
  "query": "Summarize the current project direction",
  "limit": 5
}
```

The response is a completed `lens_runs` record. `output` contains the Lens
metadata, query, retrieval mode, matched memory summaries, summary provider
provenance, and the generated interpretation. When a summary API key is
configured, Lens Run uses the configured OpenAI-compatible chat model to
summarize the retrieved memories. Without a summary provider, or if provider
generation fails, it stores a deterministic fallback summary and records
`summary_fallback_reason`. If semantic dependencies are not configured, Lens Run
falls back to keyword retrieval so local CLI usage still works.

Summary provider configuration:

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEMORYNEXUS_SUMMARY_PROVIDER` | inferred from keys, else `openai` | `openai`, `openrouter`, or `none` |
| `MEMORYNEXUS_SUMMARY_API_KEY` | `OPENAI_API_KEY` / `OPENROUTER_API_KEY` | Summary-only key override |
| `MEMORYNEXUS_SUMMARY_MODEL` | `OPENAI_MODEL` or provider default | Chat model used by Lens Run |
| `MEMORYNEXUS_AI_BASE_URL` | `OPENAI_BASE_URL` or provider default | OpenAI-compatible API base URL |
| `LENS_RUN_SUMMARY_MAX_WORDS` | output-format based | Summary length override |

If `MEMORYNEXUS_SUMMARY_PROVIDER` is not set and only `OPENROUTER_API_KEY` is
present, the provider is inferred as `openrouter`.

Example response shape:

```json
{
  "ok": true,
  "data": {
    "id": "run-uuid",
    "lens_id": "lens-uuid",
    "space_id": "space-uuid",
    "query": "Summarize the current project direction",
    "input_memory_ids": ["memory-uuid"],
    "output": {
      "lens": {
        "id": "lens-uuid",
        "name": "Project Context",
        "strategy": "project_context",
        "output_format": "brief",
        "retrieval_mode": "semantic"
      },
      "query": "Summarize the current project direction",
      "search_mode": "semantic",
      "memory_count": 1,
      "memories": [],
      "summary": "Lens 'Project Context' interpreted 1 memories for query 'Summarize the current project direction' using strategy 'project_context'.",
      "summary_provider": "deterministic",
      "summary_model": null,
      "summary_fallback_reason": "summary provider not configured"
    },
    "status": "completed",
    "created_by": "user-uuid",
    "created_at": "2026-05-21T00:00:00Z",
    "completed_at": "2026-05-21T00:00:00Z"
  }
}
```

### Get Lens Run

`GET /api/v1/lens-runs/:id`

Returns a run only if the current user can access its Cognitive Space.

## Memories

### Create Memory

`POST /api/v1/memories`

```json
{
  "space_id": "optional-space-uuid",
  "title": "Rust practice",
  "content": "Today I practiced Rust cognitive memory.",
  "memory_type": "text",
  "tags": ["rust", "learning"],
  "is_shared": false
}
```

If `space_id` is omitted, the server uses the user's default Cognitive Space.
Created memories always persist with a concrete `space_id`.

### List Memories

`GET /api/v1/memories?space_id=<SPACE_ID>&limit=20&offset=0`

If `space_id` is omitted, the default Cognitive Space is used.

### Get, Update, Delete Memory

- `GET /api/v1/memories/:id`
- `PATCH /api/v1/memories/:id`
- `DELETE /api/v1/memories/:id`

Memory access is checked against ownership and Cognitive Space membership.

## Search

### Keyword Search

`GET /api/v1/search?q=<QUERY>&space_id=<SPACE_ID>&limit=20`

If `space_id` is omitted, the default Cognitive Space is used.

### Semantic Search

`GET /api/v1/search?q=<QUERY>&space_id=<SPACE_ID>&semantic=true&limit=20`

Semantic search uses the Embedding -> Qdrant -> PostgreSQL recall path when
Qdrant and an embedding provider are configured. Vector payloads include
`space_id`, `memory_id`, `source_type`, `created_at`, and `visibility` so
semantic retrieval can stay inside the Cognitive Space boundary and preserve
basic provenance.

When `QDRANT_URL` is set, the Rust API ensures the configured Qdrant collection
exists during startup. Local development can set
`MEMORYNEXUS_EMBEDDING_PROVIDER=local` to use a deterministic embedding provider
for semantic smoke tests without external API credentials. Production-like
deployments should keep the default OpenAI embedding provider and configure
`OPENAI_API_KEY`.
