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

## Runtime Config

### Get AI Config

`GET /api/v1/ai/config`

Returns the AI-related configuration visible to the running API process. This is
useful when the CLI shell and API shell have different environment variables.

```json
{
  "ok": true,
  "data": {
    "model": "gpt-3.5-turbo",
    "embedding_model": "text-embedding-ada-002",
    "embedding_provider": "local",
    "enabled": false,
    "summary_enabled": true,
    "summary_provider": "openrouter",
    "summary_model": "openrouter/free",
    "summary_max_words": 120
  }
}
```

`enabled` only reflects the legacy `OPENAI_API_KEY` chat configuration.
Lens Run summary availability is represented by `summary_enabled` and the
summary provider fields.

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
  "description": "Rust and cognitive memory practice",
  "space_type": "project"
}
```

`space_type` is optional and defaults to `personal`. Supported values are
`personal`, `family`, `project`, and `organization`.

### List Spaces

`GET /api/v1/spaces`

Optional query parameters:

- `space_type`: filter visible spaces by `personal`, `family`, `project`, or
  `organization`.

Returns spaces where the current user is a member.

### Get Space

`GET /api/v1/spaces/:id`

Returns a space only if the current user is a member.

### List Space Members

`GET /api/v1/spaces/:id/members`

Returns members only when the current user is already a member of the space.
Member roles are:

- `owner`: manages members and writes content.
- `editor`: writes content and can update/delete their own memories.
- `viewer`: reads visible space content.

### Create Space Invite

`POST /api/v1/spaces/:id/invites`

Only `owner` members can create invite codes. Invite codes can grant `editor` or
`viewer`; `owner` cannot be granted by invite.

```json
{
  "role": "viewer",
  "expires_in_days": 7
}
```

### Accept Space Invite

`POST /api/v1/spaces/invites/accept`

```json
{
  "code": "invite-code"
}
```

Accepting an invite creates or updates the current user's membership in the
target Cognitive Space.

### Update Member Role

`PATCH /api/v1/spaces/:id/members/:user_id`

Only `owner` members can update roles. This endpoint supports `editor` and
`viewer`; owner transfer is intentionally out of scope for the first shared
space model.

```json
{
  "role": "editor"
}
```

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
      "key_points": [],
      "open_questions": [],
      "suggested_next_actions": [],
      "citations": [],
      "unresolved_contradictions": [],
      "summary": "Lens 'Project Context' interpreted 1 memories for query 'Summarize the current project direction' using strategy 'project_context'.",
      "summary_provider": "deterministic",
      "summary_source": "deterministic",
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

### List Lens Runs

`GET /api/v1/lens-runs?lens_id=<LENS_ID>&limit=20`

`GET /api/v1/lens-runs?space_id=<SPACE_ID>&limit=20`

Returns visible Lens Runs ordered by newest first. At least one of `lens_id` or
`space_id` is required.

## Feedback Loops

FeedbackLoop captures one long-running feedback cycle inside a Namespace and
Cognitive Space. The first API stores the loop state itself; creating Memory
snapshots and threading `feedback_loop_id` provenance through Memory, Lens Run,
Review Report, and Profile remains a follow-up boundary.

### Create Feedback Loop

`POST /api/v1/feedback-loops`

```json
{
  "space_id": "space-uuid",
  "namespace_id": "namespace-uuid",
  "goal": "Improve fraction word problems",
  "task": "Complete five fraction word problems and explain each mistake",
  "attempt": "optional attempt notes",
  "evaluation": "optional evaluation",
  "feedback": "optional feedback",
  "adjustment": "optional adjustment",
  "next_task": "optional next task",
  "status": "active"
}
```

`namespace_id` must belong to the same `space_id`. `status` supports `active`,
`completed`, and `paused`; omitted status defaults to `active`.

### List Feedback Loops

`GET /api/v1/feedback-loops?space_id=<SPACE_ID>&namespace_id=<OPTIONAL_NAMESPACE_ID>`

Returns visible loops ordered by newest first. `namespace_id` narrows the list to
one namespace and must belong to the requested space.

### Get Feedback Loop

`GET /api/v1/feedback-loops/:id`

Returns a loop only if the current user can access its Cognitive Space.

### Patch Feedback Loop

`PATCH /api/v1/feedback-loops/:id`

```json
{
  "evaluation": "What changed after the attempt",
  "feedback": "Observed error pattern",
  "adjustment": "What to change next round",
  "next_task": "The next concrete task",
  "status": "paused"
}
```

Patch supports `evaluation`, `feedback`, `adjustment`, `next_task`, and
`status`. Writers are Space `owner` or `editor` members.

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

## Voice Capture

### Transcribe Audio And Create Memory

`POST /api/v1/voice-captures?space_id=<SPACE_ID>&filename=thought.webm&language=zh`

The request body is the uploaded audio bytes. The endpoint requires
authentication and Space write permission, transcribes the audio through the
configured transcription provider, then creates an `audio` Memory in the same
Cognitive Space.

Configuration:

- `MEMORYNEXUS_TRANSCRIPTION_PROVIDER=openai`
- `OPENAI_API_KEY` or `MEMORYNEXUS_TRANSCRIPTION_API_KEY`
- Optional `MEMORYNEXUS_TRANSCRIPTION_MODEL`, defaulting to `whisper-1`

If no transcription provider is configured, the endpoint returns a visible
client error instead of creating a Memory. Created memories include
`source_type = "voice_transcription"` and `source_metadata` with provider,
model, language, filename, content type, audio size, and provider metadata.

## Reminders

Reminders are scheduled recall items scoped to a `CognitiveSpace`. They can
optionally reference a memory, but they do not make an agent own memory.

### Create Reminder

`POST /api/v1/reminders`

```json
{
  "space_id": "space-uuid",
  "memory_id": "optional-memory-uuid",
  "title": "Review Rust notes",
  "content": "Review this week's Rust practice and extract next actions.",
  "remind_at": "2026-05-26T09:00:00Z",
  "repeat_rule": "weekly"
}
```

`remind_at` must be an RFC3339 timestamp. `repeat_rule` is optional and
currently supports `daily`, `weekly`, and `monthly`. Reminders are surfaced by
listing due items.

### List Reminders

`GET /api/v1/reminders?space_id=<SPACE_ID>&due_only=false&include_completed=false&limit=20`

Set `due_only=true` to fetch pending reminders whose `remind_at` is not in the
future. Completed reminders are hidden unless `include_completed=true`.

### Complete Reminder

`POST /api/v1/reminders/:id/complete`

Marks a pending reminder as completed if the current user is a member of the
reminder's Cognitive Space. For a reminder with `repeat_rule`, this acknowledges
the current occurrence and advances `remind_at` to the next interval while
keeping the reminder pending.

## Search

### Keyword Search

`GET /api/v1/search?q=<QUERY>&space_id=<SPACE_ID>&limit=20`

If `space_id` is omitted, the default Cognitive Space is used.

### Lens-Scoped Search

`GET /api/v1/search?q=<QUERY>&lens_id=<LENS_ID>&limit=20`

When `lens_id` is supplied, the server resolves the Lens, enforces membership in
the Lens's Cognitive Space, and searches inside that space. If `space_id` is
also supplied, it must match the Lens's space. A Lens with
`retrieval_mode=semantic` enables semantic search for the request.

The response includes Lens provenance:

```json
{
  "ok": true,
  "data": {
    "query": "cognitive lens",
    "search_mode": "semantic",
    "lens": {
      "id": "lens-uuid",
      "space_id": "space-uuid",
      "name": "Project Context",
      "strategy": "project_context",
      "output_format": "brief",
      "retrieval_mode": "semantic"
    },
    "items": []
  }
}
```

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

## Cognitive Profiles

Profiles are compact projections of a Cognitive Space for LLM, MCP, and UI
clients. A profile is not agent-owned memory. It is persisted as a snapshot with
source memory IDs and Lens Run IDs.

### Create Profile Snapshot

`POST /api/v1/profiles`

```json
{
  "space_id": "optional-space-uuid",
  "lens_id": "optional-lens-uuid",
  "target": "personal_context",
  "limit": 12
}
```

If `space_id` is omitted, the server uses the user's default Cognitive Space. If
`lens_id` is provided without `space_id`, the server uses the Lens's Cognitive
Space.

Supported `target` values:

- `llm_context`
- `personal_context`
- `preference_review`
- `decision_history`
- `risk_review`
- `project_context`

The response includes the persisted snapshot:

```json
{
  "ok": true,
  "data": {
    "snapshot": {
      "id": "profile-snapshot-uuid",
      "space_id": "space-uuid",
      "lens_id": null,
      "target": "personal_context",
      "profile": {
        "summary": "Cognitive profile for 'Personal Agent Space' using 3 recent memories and 1 Lens Runs.",
        "stable_preferences": [],
        "active_projects": [],
        "decision_history": [],
        "recent_context": [],
        "unresolved_contradictions": [],
        "source_memory_ids": [],
        "source_lens_run_ids": []
      },
      "source_memory_ids": [],
      "source_lens_run_ids": [],
      "created_by": "user-uuid",
      "created_at": "2026-05-25T00:00:00Z"
    }
  }
}
```

Use profile snapshots when an agent needs compact working context. Use search
when it needs raw recall. Use Lens Run when it needs interpretation with
citations.

### Get Profile Snapshot

`GET /api/v1/profiles/:id`

Returns a profile snapshot only if the current user is a member of the snapshot's
Cognitive Space.

## Cognitive Review Reports

Review reports are persisted derived interpretations over a time window. They
are generated through a Lens, cite source memory IDs, and record summary
provider provenance.

### Generate Review Report

`POST /api/v1/review-reports`

```json
{
  "space_id": "space-uuid",
  "lens_id": "lens-uuid",
  "window_start": "2026-05-18T00:00:00Z",
  "window_end": "2026-05-25T00:00:00Z",
  "report_type": "weekly_review",
  "limit": 30
}
```

Supported `report_type` values:

- `periodic_review`
- `daily_review`
- `weekly_review`
- `monthly_review`

The response stores `report.summary`, `source_memory_ids`,
`summary_provider`, `summary_source`, `summary_model`, and
`summary_fallback_reason`. Reports are derived interpretations; they do not copy
or own memories.

### Get Review Report

`GET /api/v1/review-reports/:id`

Returns a report only if the current user can access its Cognitive Space.

### List Review Reports

`GET /api/v1/review-reports?space_id=<SPACE_ID>&lens_id=<OPTIONAL_LENS_ID>&limit=20`

Returns visible reports ordered by newest first.

## Agent Router

The agent router is a deterministic, conservative policy layer for personal
agents. It recommends a MemoryNexus action but does not execute the action. This
keeps writes explicit and prevents the router from silently storing secrets or
scratchpad noise.

### Route Agent Context

`POST /api/v1/agent/route`

```json
{
  "message": "Remember this: I prefer Rust-first backend work.",
  "space_id": "optional-space-uuid",
  "lens_id": "optional-lens-uuid",
  "target": "personal_context"
}
```

Possible `action` values:

- `write_memory`
- `search_memory`
- `run_lens`
- `get_profile`
- `ignore`

Example response:

```json
{
  "ok": true,
  "data": {
    "action": "write_memory",
    "confidence": 0.92,
    "reason_codes": ["explicit_memory_intent"],
    "safety_flags": [],
    "suggested_tool": "add_memory",
    "suggested_arguments": {
      "space_id": "space-uuid",
      "title": "I prefer Rust-first backend work.",
      "content": "I prefer Rust-first backend work.",
      "tags": ["agent", "explicit-memory"]
    }
  }
}
```

Secret-like input returns `ignore` with `do_not_persist_secret`. Long command
output and transient build/test logs return `ignore` with
`transient_or_low_signal`.

## AI

### Summarize Content

`POST /api/v1/ai/summarize`

```json
{
  "content": "Text to summarize",
  "lens_id": "optional-lens-uuid"
}
```

When `lens_id` is supplied, the server verifies the user can access the Lens and
returns Lens provenance in the response. The Lens does not own the content; it
records the interpretation strategy used for the summary request. If request
`options` are omitted, the Lens `output_format` influences the default summary
style: `brief` maps to concise output, and `bullets` maps to bullet points.

Summary requests use the configured summary provider when available. Without a
provider, or when the provider returns empty output or an error, the API returns
a deterministic local summary with provenance:

```json
{
  "ok": true,
  "data": {
    "summary": "Text to summarize",
    "keywords": ["text", "summarize"],
    "language": "en",
    "original_length": 17,
    "summary_length": 17,
    "processing_time_ms": 0,
    "summary_source": "deterministic",
    "summary_provider": "deterministic",
    "fallback_reason": "summary provider not configured"
  }
}
```

Long content is supported by the deterministic fallback path, which extracts a
bounded summary and keywords locally.

### Summarize Memory

`POST /api/v1/memories/:id/summarize`

```json
{
  "content": "",
  "lens_id": "optional-lens-uuid"
}
```

For memory summaries, `lens_id` must belong to the same Cognitive Space as the
memory.

### Smart Tags

`POST /api/v1/ai/autotag`

```json
{
  "content": "Rust project roadmap for Cognitive Space and Lens Run"
}
```

Smart tags are suggestions only; clients can edit or discard them before saving.
The local deterministic tagger returns tags, categories, and structured
suggestions without requiring provider credentials:

```json
{
  "ok": true,
  "data": {
    "suggested_tags": ["rust", "cognitive-lens", "memory-space"],
    "categories": ["technology", "cognition", "architecture"],
    "suggestions": [
      { "tag": "rust", "category": "technology" }
    ],
    "confidence": 0.8,
    "source": "deterministic",
    "editable": true
  }
}
```
