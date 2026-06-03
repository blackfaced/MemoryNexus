# STEM Learning MCP Demo (issue #87)

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
