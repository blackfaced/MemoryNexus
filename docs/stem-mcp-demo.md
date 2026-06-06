# STEM Learning MCP Demo

This file keeps two validation records:

- Issue #91 canonical namespace-driven MCP demo using `learning.stem`.
- Issue #87 compatibility demo using the older `learning_math_*` tools.

## Canonical Namespace-Driven Demo (issue #91)

> Scope: issue #91 after merge to `main`. This transcript validates that an
> agent can create or reuse a `learning.stem` skill Namespace through the Rust
> Namespace API, then drive the practice loop end to end through the canonical
> namespace-driven MCP tools.

### Result

The canonical loop passes end to end through the MCP stdio surface:

`create_practice_session -> record_practice_attempt -> record_practice_feedback -> list_practice_sessions -> get_practice_session`.

`capture_memory=true` produced three traceable `Memory` snapshots in the same
`CognitiveSpace`, all linked to the same `namespace_id` and practice session.

`tools/list` still includes both the canonical tools and the older
`learning_math_*` compatibility tools.

No external AI credentials were required (`MEMORYNEXUS_EMBEDDING_PROVIDER=local`).

### Environment

- Source: `26704ca Add namespace-driven practice sessions (#91)` on branch
  `canonical-stem-mcp-demo`, matching `origin/main`.
- API: fresh `target/debug/memorynexus` on `MEMORYNEXUS_BIND_ADDR=0.0.0.0:8090`,
  using local Postgres + Qdrant and
  `QDRANT_COLLECTION=memorynexus_canonical_stem_mcp_demo`.
- MCP: `target/debug/memorynexus-mcp` over stdio, `MEMORYNEXUS_API_URL` pointing
  at the fresh API.
- Token: a local test account JWT in `MEMORYNEXUS_TOKEN` (never printed).

The demo used a separate port so it did not depend on, or disturb, any existing
API process on `:8080`.

### Rust API Setup

The setup used HTTP API calls before the MCP practice flow:

1. `POST /api/v1/auth/register` for a throwaway local account.
2. `POST /api/v1/spaces` to create a family `CognitiveSpace`.
3. `GET /api/v1/namespaces?space_id=<space-id>` to check for an existing
   `learning.stem` skill Namespace.
4. `POST /api/v1/namespaces` to create `learning.stem` because the new demo
   space did not have one yet.

Observed IDs:

- Space: `eb073f87-29ed-4903-b592-7ee17a4919c5`
- Namespace: `87500f13-e330-40ab-b519-e6484639b41b`
- Namespace action: `created learning.stem`

### Tool List Verification

`tools/list` returned the canonical practice tools:

- `create_practice_session`
- `record_practice_attempt`
- `record_practice_feedback`
- `list_practice_sessions`
- `get_practice_session`

It also still returned the compatibility aliases:

- `learning_math_create_practice_session`
- `learning_math_record_attempt`
- `learning_math_record_feedback`
- `learning_math_list_practice_sessions`
- `learning_math_get_practice_session`

### Canonical MCP Tool Calls

All calls below are MCP `tools/call` requests. Each tool returns the
MemoryNexus API JSON as text content.

#### 1. create_practice_session

```json
{"name":"create_practice_session","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_goal":"Improve elementary fraction word problems","exercise":"Maya has 3/4 cup of trail mix and shares half with her brother. How much trail mix does her brother get?","capture_memory":true}}
```

Returned session:

- `data.id`: `70fab100-6548-4d4a-b881-843efc24be0f`
- `data.namespace_id`: matched the `learning.stem` Namespace ID.
- `data.status`: `active`

#### 2. record_practice_attempt

```json
{"name":"record_practice_attempt","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_session_id":"<practice-session-id>","answer":"3/8 cup","reasoning":"Half of 3/4 means multiply 3/4 by 1/2, so 3/8.","capture_memory":true}}
```

The API stored the answer as:

```text
3/8 cup

Reasoning: Half of 3/4 means multiply 3/4 by 1/2, so 3/8.
```

#### 3. record_practice_feedback

```json
{"name":"record_practice_feedback","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_session_id":"<practice-session-id>","mistake_pattern":"No mistake this round; learner kept the fraction operation clear.","feedback":"Correct. Keep writing the operation before simplifying so the unit stays attached to the answer.","practice_adjustment":"Add one sentence explaining why half means multiply by 1/2.","next_exercise":"A pitcher has 2/3 liter of juice. If Sam drinks half of it, how much juice does Sam drink?","status":"completed","capture_memory":true}}
```

`status` moved to `completed`; feedback fields persisted on the session.

#### 4. list_practice_sessions

```json
{"name":"list_practice_sessions","arguments":{"namespace_id":"<learning-stem-namespace-id>","limit":10,"offset":0}}
```

The paginated `data.items[]` response included
`70fab100-6548-4d4a-b881-843efc24be0f`.

#### 5. get_practice_session

```json
{"name":"get_practice_session","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_session_id":"<practice-session-id>"}}
```

Returned the full session with `status: "completed"` and the same
`namespace_id` as the canonical path.

### Memory Capture Verification

`GET /api/v1/memories?space_id=<space-id>&limit=20&offset=0` returned three
traceable `feedback_loop_event` Memory snapshots for the practice session:

| Event | Memory ID | Included fields |
|-------|-----------|-----------------|
| `create` | `09694e4e-4707-423f-a1be-f8a6439da66e` | `goal`, `task` |
| `patch` | `9724ddf1-2dcf-4237-b8e0-2a5543eeb95a` | `attempt` |
| `patch` | `5c78d8c2-5b70-4e81-a879-a2d06594bf18` | `evaluation`, `feedback`, `adjustment`, `next_task` |

All three snapshots were titled `Practice snapshot` and carried:

- `source_type = "feedback_loop_event"`
- `source_metadata.feedback_loop_id = "70fab100-6548-4d4a-b881-843efc24be0f"`
- `source_metadata.namespace_id = "87500f13-e330-40ab-b519-e6484639b41b"`

This confirms `capture_memory=true` preserves create, attempt, and feedback
events as traceable Space-owned Memory records while keeping the MCP practice
flow in parent/learner language.

### Friction Notes

1. **Multi-step local scripts may need sandbox escalation.** Single `curl`
   health checks worked in the normal sandbox, but the multi-step HTTP/MCP
   transcript could not reliably reach the API until run outside the sandbox.
2. **Canonical practice tools require `namespace_id`.** The `learning.stem`
   Namespace must be selected or created through the Rust Namespace API first.

## Compatibility Demo (issue #87)

> Scope: issue #87. This is a validation transcript proving an agent can drive
> the parent-assisted STEM practice loop end to end through the current
> `learning_math_*` MCP tools. It does not build UI and does not rename tools.

## Result

The full loop passes end to end through the MCP stdio surface:

`create practice session -> record attempt -> record feedback -> list -> get`.

`capture_memory=true` produces traceable `Memory` snapshots in the same
`CognitiveSpace` for all three write steps (create, attempt, feedback).

No external AI credentials were required (`MEMORYNEXUS_EMBEDDING_PROVIDER=local`).

## Environment

- API: fresh `target/debug/memorynexus` on `MEMORYNEXUS_BIND_ADDR=0.0.0.0:8090`,
  sharing the local Postgres + Qdrant from `docker compose`.
- MCP: `target/debug/memorynexus-mcp` over stdio, `MEMORYNEXUS_API_URL` pointing
  at the fresh API.
- Token: a local test account JWT in `MEMORYNEXUS_TOKEN` (never printed).

A separate port was used so the demo did not disrupt an already-running API on
`:8080`. See the friction note below for why a fresh API was necessary.

## Tool Calls And Payloads

All calls are MCP `tools/call` requests. Each tool returns the MemoryNexus API
JSON as text content.

### 1. create_space

```json
{"name":"create_space","arguments":{"name":"STEM Demo Space #87","description":"issue 87 stem mcp demo","space_type":"family"}}
```

Returns `data.id` used as `space_id`.

### 2. learning_math_create_practice_session

```json
{"name":"learning_math_create_practice_session","arguments":{"space_id":"<space-id>","practice_goal":"Improve fraction word problems","exercise":"A recipe uses 3/4 cup of flour. If we make half the recipe, how much flour is needed?","capture_memory":true}}
```

Required fields are `space_id`, `practice_goal`, `exercise`. The API created or
reused the `learning.math` Namespace and returned `data.id`, `namespace_id`, and
`status: "active"`. `data.id` is the `practice_session_id`.

### 3. learning_math_record_attempt

```json
{"name":"learning_math_record_attempt","arguments":{"practice_session_id":"<session-id>","answer":"3/8 cup","reasoning":"Half of 3/4 is 3/8","capture_memory":true}}
```

`answer` and `reasoning` are merged into a single stored `answer`
(`"3/8 cup\n\nReasoning: Half of 3/4 is 3/8"`).

### 4. learning_math_record_feedback

```json
{"name":"learning_math_record_feedback","arguments":{"practice_session_id":"<session-id>","mistake_pattern":"None this time","feedback":"Correct: half means multiply by 1/2.","practice_adjustment":"Try one with a different numerator","next_exercise":"A garden uses 2/3 bag of soil. How much for half the garden?","status":"completed","capture_memory":true}}
```

`status` moved to `completed`; feedback fields persisted on the session.

### 5. learning_math_list_practice_sessions

```json
{"name":"learning_math_list_practice_sessions","arguments":{"space_id":"<space-id>","limit":10}}
```

Returns a paginated envelope: `data.items[]`, `data.total`, `data.limit`,
`data.offset`. Not a bare array.

### 6. learning_math_get_practice_session

```json
{"name":"learning_math_get_practice_session","arguments":{"practice_session_id":"<session-id>"}}
```

Returns the full session with the recorded answer, feedback, adjustment, next
exercise, and `status: "completed"`.

## Memory Capture Verification

`search_memories` (query `"Practice"`, scoped to the space) returned
`data.total: 3` snapshots, one per write step, all titled `Practice snapshot`:

- Create: `Practice goal: ... / Practice task: ...`
- Attempt: `Answer / reasoning: 3/8 cup ...`
- Feedback: `Mistake pattern / evaluation: ... / Feedback: ... / Practice adjustment: ... / Next exercise: ...`

This confirms `capture_memory=true` produces traceable, Space-owned `Memory`
records for each step of the loop. The transcript stays in parent/learner
language (practice goal, exercise, answer, mistake pattern, feedback, next
exercise) and does not expose `MemoryAtom`, `CognitiveScene`, or
`CognitiveProjection`.

## Friction Notes

For #70 static UI and #71 weekly learning review:

1. **Stale running API hides new routes.** A `memorynexus` process started
   before the `learning.math` routes landed returns `HTTP 404` for every
   practice endpoint, surfaced through MCP as
   `MemoryNexus API returned HTTP 404: {}`. The route exists in source; the live
   process was just old. Restarting (or rebuilding) the API after pulling
   `learning.math` changes is required. The 404 body is empty, so the MCP error
   is not self-explanatory — agents should check API freshness when a known tool
   404s.
2. **List vs get response shapes differ.** `get` returns the session object
   directly under `data`, while `list` wraps results in `data.items` with a
   `data.total` count. UI/clients must branch on shape, not assume a uniform
   `data`.
3. **`answer` + `reasoning` are merged server-side** into one `answer` string.
   A UI that wants them shown separately cannot recover the split from the API
   response.
4. **Doc field-name drift (fixed here).** `docs/agent-self-install.md` used
   `goal`/`task`/`attempt`/`evaluation`/`adjustment`/`next_task` and extra
   `student_label`/`topic` fields that the MCP tools do not accept, and omitted
   the required `practice_goal`/`exercise`. Following it verbatim would fail.
   The examples now match the real tool schema in `docs/mcp.md` and
   `src/bin/memorynexus-mcp.rs`.
