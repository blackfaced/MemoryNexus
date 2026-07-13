# MemoryNexus MCP Server

`memorynexus-mcp` exposes MemoryNexus as a local MCP stdio server. It is a thin
adapter over the Rust API, not a second backend. Memory still belongs to
`CognitiveSpace`; MCP clients only call tools that operate through the API.

MCP includes both compatibility tools over existing object-level APIs and
generic Surface Gateway tools for Capture, Performance, Reflection, Planning,
and Observation. The Surface tools treat MCP as an Adapter: they call
`/api/v1/surfaces` with `adapter: "mcp"` and do not give agents direct Engine
ownership or repository access.

## Configuration

For ordinary agent installs, prefer a release `memorynexus-mcp` binary once a
release is published. Trial Profile points that binary at an existing
hosted/demo API with `MEMORYNEXUS_API_URL` and `MEMORYNEXUS_TOKEN` and does not
need Rust or Docker. Local One-click Profile uses the same release archive plus
a local API binary and local PostgreSQL/Qdrant services, usually started with
Docker. Production Profile points the MCP binary at a stable hosted or
self-hosted API. Until the first release artifact is published, the `cargo run`
examples below are the working Developer Profile source-build path.

Start the MemoryNexus API first:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

In the MCP client environment, configure:

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `MEMORYNEXUS_API_URL` | No | `http://localhost:8080` | Rust API base URL |
| `MEMORYNEXUS_TOKEN` | Yes for tools | - | JWT bearer token returned by auth login/register |

Run the MCP server:

```bash
MEMORYNEXUS_TOKEN='<jwt-token>' \
cargo run --bin memorynexus-mcp
```

For Claw, Hermes, or another personal agent, see
[Personal Agent Integration](agent-integration.md). That guide includes a
recommended personal `CognitiveSpace`, personal Lens templates, write policy,
and MCP client JSON snippets.

If another agent should perform the setup itself, give it
[Agent Self-Install Guide](agent-self-install.md).

## Upgrade Behavior

An MCP client keeps using the MCP server process it started. Updating the
MemoryNexus source checkout is not enough; restart or reload the MCP client so
it starts a fresh `memorynexus-mcp` process.

There are two common configurations:

- `cargo run --quiet --bin memorynexus-mcp`: source changes are picked up on the
  next MCP server start, but the agent client still needs to restart the stdio
  server.
- `target/debug/memorynexus-mcp`: rebuild the binary with `cargo build --bin
  memorynexus-mcp`, then restart or reload the agent client.

When backend API code or migrations change, restart `memorynexus` as well. The
API runs SQLx migrations on startup.

Minimal upgrade sequence:

```bash
cd /path/to/MemoryNexus
git pull
cargo test
cargo build --bin memorynexus-mcp
```

Skip `git pull` when the latest changes are already local edits in this
checkout. Skip the build step only when the MCP config uses `cargo run`. Rebuild
`memorynexus` too if the API is launched from a built binary.

## Tools

| Tool | Purpose |
|------|---------|
| `list_spaces` | List Cognitive Spaces visible to the authenticated user |
| `create_space` | Create a Cognitive Space for agent bootstrap |
| `add_memory` | Add a text memory to a Cognitive Space |
| `search_memories` | Search memories by `space_id` or `lens_id` |
| `list_lenses` | List Lenses in a Cognitive Space |
| `create_lens` | Create a Lens interpretation strategy in a Cognitive Space |
| `run_lens` | Run a Lens query and return a traceable Lens Run |
| `get_lens_run` | Fetch a persisted Lens Run by ID |
| `get_profile` | Project and persist a compact Cognitive Profile for a personal agent |
| `add_reminder` | Create a scheduled recall reminder in a Cognitive Space |
| `list_reminders` | List pending or due scheduled recall reminders |
| `complete_reminder` | Mark a pending reminder as completed |
| `mark_reminder_delivery` | Record in-app reminder delivery as delivered or failed |
| `route_agent_context` | Recommend write/search/lens/profile/ignore for agent context |
| `create_practice_session` | Canonical: create a practice session in a Skill Namespace such as `learning.stem` |
| `record_practice_attempt` | Canonical: record a learner's answer or reasoning for a namespace practice session |
| `record_practice_feedback` | Canonical: record mistake pattern, feedback, adjustment, and next exercise for a namespace practice session |
| `list_practice_sessions` | Canonical: list practice sessions in a Skill Namespace |
| `get_practice_session` | Canonical: fetch one practice session from a Skill Namespace |
| `surface_capture_observation` | Generic Surface Gateway Capture action `capture_observation` |
| `surface_submit_attempt` | Generic Surface Gateway Performance action `submit_attempt` |
| `surface_review_evidence` | Generic Surface Gateway Reflection action `review_evidence` |
| `surface_generate_next_task` | Generic Surface Gateway Planning action `generate_next_task` |
| `surface_adjust_plan` | Generic Surface Gateway Planning action `adjust_plan` |
| `surface_get_state_summary` | Generic Surface Gateway Observation action `get_state_summary` |
| `learning_math_create_practice_session` | Compatibility: create a parent-assisted `learning.math` practice session |
| `learning_math_record_attempt` | Compatibility: record a learner's answer or reasoning for a `learning.math` practice session |
| `learning_math_record_feedback` | Compatibility: record mistake pattern, feedback, adjustment, and next exercise for `learning.math` |
| `learning_math_list_practice_sessions` | Compatibility: list `learning.math` practice sessions in a Cognitive Space |
| `learning_math_get_practice_session` | Compatibility: fetch one `learning.math` practice session |
| `get_install_status` | Inspect profile-aware install state: OS/arch, release target, binary path, API health, MCP smoke, and fallback |
| `upgrade_install` | Return profile-aware binary-first or Developer source-build install/upgrade plans |

## Smoke Test

You can test the protocol without an MCP client:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Expected output includes an `initialize` response and a `tools/list` response.
The `tools/list` response must include MemoryNexus tools such as
`create_space`, `add_memory`, `search_memories`, `run_lens`, the generic
Surface tools, `get_install_status`, `upgrade_install`, and the canonical
practice-session tools.

To call a tool, keep the API running and send a `tools/call` request:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_memories","arguments":{"query":"cognitive lens","lens_id":"<lens-id>","limit":5}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Personal agent profile projection:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_profile","arguments":{"target":"personal_context","limit":12}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Agent bootstrap from MCP:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_space","arguments":{"name":"Personal Agent Space","description":"Long-term memory universe for a personal agent","space_type":"personal"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_lens","arguments":{"space_id":"<space-id-from-create-space>","name":"Personal Context","strategy":"personal_context","output_format":"brief","retrieval_mode":"semantic"}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Agent routing recommendation:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"route_agent_context","arguments":{"message":"Remember this: I prefer Rust-first backend work.","space_id":"<space-id>"}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Scheduled recall:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"add_reminder","arguments":{"space_id":"<space-id>","title":"Review project direction","content":"Run a project_context Lens and decide the next task.","remind_at":"2026-05-26T09:00:00Z","repeat_rule":"weekly:2","delivery_channel":"in_app"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_reminders","arguments":{"space_id":"<space-id>","due_only":true,"limit":20}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"mark_reminder_delivery","arguments":{"reminder_id":"<reminder-id>","status":"delivered"}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Reminder `repeat_rule` accepts `daily`, `weekly`, `monthly`, or interval forms
such as `daily:3`, `weekly:2`, and `monthly:6`.

Parent-assisted STEM learning practice should use the canonical namespace-driven
tools. Create or select a Skill Namespace such as `learning.stem` through the
HTTP Namespace API, then pass its `namespace_id` to MCP:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_practice_session","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_goal":"Improve fraction word problems","exercise":"Solve five fraction word problems and explain the reasoning","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"record_practice_attempt","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_session_id":"<practice-session-id>","answer":"I solved 3 out of 5","reasoning":"I changed units in the middle of the problem","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"record_practice_feedback","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_session_id":"<practice-session-id>","mistake_pattern":"Changed units between steps","feedback":"Write the unit next to every number before calculating","practice_adjustment":"Add a unit-labeling step","next_exercise":"Try three unit-conversion fraction problems","status":"completed","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_practice_sessions","arguments":{"namespace_id":"<learning-stem-namespace-id>","limit":10}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_practice_session","arguments":{"namespace_id":"<learning-stem-namespace-id>","practice_session_id":"<practice-session-id>"}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

The older `learning_math_*` tools remain as compatibility aliases for clients
already using the first implementation slice:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"learning_math_create_practice_session","arguments":{"space_id":"<space-id>","practice_goal":"Improve fraction word problems","exercise":"Solve five fraction word problems and explain the reasoning","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"learning_math_record_attempt","arguments":{"practice_session_id":"<practice-session-id>","answer":"I solved 3 out of 5","reasoning":"I changed units in the middle of the problem","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"learning_math_record_feedback","arguments":{"practice_session_id":"<practice-session-id>","mistake_pattern":"Changed units between steps","feedback":"Write the unit next to every number before calculating","practice_adjustment":"Add a unit-labeling step","next_exercise":"Try three unit-conversion fraction problems","status":"completed","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"learning_math_list_practice_sessions","arguments":{"space_id":"<space-id>","limit":10}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"learning_math_get_practice_session","arguments":{"practice_session_id":"<practice-session-id>"}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

`create_practice_session` and the other canonical tools require `namespace_id`;
optional `space_id` is only a guard and must match the Namespace Space when
supplied. `learning_math_create_practice_session` accepts `space_id` and
optional `namespace_id`. When `namespace_id` is omitted, the Rust API creates or
reuses the current `learning.math` skill Namespace inside the same Cognitive
Space. All practice tools keep the product-facing fields `practice_goal`,
`exercise`, `answer`, `reasoning`, `mistake_pattern`, `feedback`,
`practice_adjustment`, and `next_exercise`; they do not expose MemoryAtom,
CognitiveScene, or CognitiveProjection as practice-flow inputs.

The tool response returns MemoryNexus API JSON as text content so MCP clients can
read the same traceable payload that the CLI sees.

Generic Surface Gateway tools use this shared argument shape:

### Idempotent normalized Performance outcome

An upstream Adapter can retry one completed learning session with a stable,
provider-neutral `source_event_id`. Its scope is `(CognitiveSpace, Namespace,
source_event_id)`; it is not a provider conversation or session ID. The same
normalized payload returns the original `feedback_loop_id` and
`generated_trace_id` with status `attempt_replayed`; changed content returns a
conflict and performs no writes. Do not include raw chat, media bytes, provider
payloads, credentials, or provider session identifiers.

```json
{
  "namespace": "child.english.spelling",
  "actor": "<user-id>",
  "payload": {
    "space_id": "<space-id>",
    "source_event_id": "adapter.learning-session:2026-07-13.1",
    "task": "Daily spelling",
    "input_source": "typed",
    "normalized_outcome": {
      "summary": "Completed five spelling words",
      "mistake": {
        "expected_text": "because",
        "actual_text": "becuase",
        "mistake_type": "letter_order"
      }
    }
  }
}
```

```json
{
  "namespace": "child.english.spelling",
  "actor": "<authenticated-user-id>",
  "payload": {
    "space_id": "<space-id>"
  },
  "context": {
    "mode": "fast",
    "locale": "en-US",
    "device": "desktop",
    "runtime_preference": "deterministic"
  }
}
```

The MCP adapter fills in the generic Surface fields:

| Tool | Surface | Action |
| --- | --- | --- |
| `surface_capture_observation` | `capture` | `capture_observation` |
| `surface_submit_attempt` | `performance` | `submit_attempt` |
| `surface_review_evidence` | `reflection` | `review_evidence` |
| `surface_generate_next_task` | `planning` | `generate_next_task` |
| `surface_adjust_plan` | `planning` | `adjust_plan` |
| `surface_get_state_summary` | `observation` | `get_state_summary` |

Capture can derive the target Space from the namespace resolved by the API.
Performance, Reflection, Planning, and Observation follow the current HTTP
Surface contract and require `payload.space_id`.
For `surface_adjust_plan`, the payload contains `proposed_plan`, optional
generic `evidence`, optional `constraints`, and optional `objective`; the
response is not a persisted PracticePlan.

Typed or pasted requests are text-first. When `payload.source` or
`payload.input_source` is `typed` or `pasted`, the adapter rejects
`evidence_refs`, `input_confirmation`, and media descriptor fields before
calling the API. Media-derived sources `agent_ocr`, `agent_transcribed`, and
`mixed` require:

```json
{
  "input_confirmation": {
    "status": "confirmed",
    "method": "explicit_acceptance"
  }
}
```

`method` may also be `explicit_correction`. Optional `EvidenceRefInput`
descriptors may pass through only on confirmed media-derived Capture or
Performance calls. MemoryNexus does not perform OCR, ASR, media inspection,
resolver execution, descriptor persistence, or provider availability checks in
this path.

Surface tool responses preserve generated Surface provenance such as
`generated_trace_id` when returned by the API. The MCP adapter does not return
evidence descriptor objects, raw locators, provider metadata, or claim that
descriptors were stored or are resolvable.

Generic Surface smoke:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"surface_capture_observation","arguments":{"namespace":"child.english.spelling","actor":"<user-id>","payload":{"source":"typed","content":"because\nfriend"},"context":{"mode":"fast","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"surface_submit_attempt","arguments":{"namespace":"child.english.spelling","actor":"<user-id>","payload":{"space_id":"<space-id>","source":"typed","attempt":{"target":"because","submitted":"becuase"}},"context":{"mode":"fast","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"surface_review_evidence","arguments":{"namespace":"child.english.spelling","actor":"<user-id>","payload":{"space_id":"<space-id>","question":"What pattern appears?","evidence":[]},"context":{"mode":"focused","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"surface_generate_next_task","arguments":{"namespace":"child.english.spelling","actor":"<user-id>","payload":{"space_id":"<space-id>","objective":"Review the because spelling pattern"},"context":{"mode":"focused","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"surface_get_state_summary","arguments":{"namespace":"child.english.spelling","actor":"<user-id>","payload":{"space_id":"<space-id>","timeframe":"7d"},"context":{"mode":"focused","runtime_preference":"deterministic"}}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Install or upgrade inspection:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_install_status","arguments":{"profile":"trial"}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

The status output distinguishes Trial, Local One-click, Production, and
Developer profiles. It reports detected OS/arch, release target, binary path,
API URL/health, MCP initialize/tools-list smoke commands, and whether a
source-build fallback is required.

Use the same tool to inspect other profiles:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_install_status","arguments":{"profile":"local-one-click"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_install_status","arguments":{"profile":"production"}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_install_status","arguments":{"profile":"developer","checkout_dir":"/path/to/MemoryNexus"}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

Plan a binary-first Local One-click install without executing downloads or
install commands:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"upgrade_install","arguments":{"profile":"local-one-click"}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

Trial and Production plans use the same profile field. Trial is
MCP-binary-only against an existing API; Production targets stable hosted or
self-hosted API/database/vector services and is not Supabase-only:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"upgrade_install","arguments":{"profile":"trial"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"upgrade_install","arguments":{"profile":"production"}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

Plan a Developer Profile source-build upgrade without executing local commands:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"upgrade_install","arguments":{"profile":"developer","checkout_dir":"/path/to/MemoryNexus","pull":true,"rebuild_mcp":true}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

Apply a Developer Profile source-build upgrade explicitly:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"upgrade_install","arguments":{"profile":"developer","checkout_dir":"/path/to/MemoryNexus","apply":true,"pull":true,"rebuild_mcp":true}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

`upgrade_install` defaults to plan-only. Trial and Local One-click plans are
binary-first and do not compile Rust. `apply=true` currently executes only
Developer Profile source-build commands, refuses `git pull` when local files are
dirty, and does not restart the API or current MCP client; the response reports
which restarts are still required.

## MCP vs CLI

Use `memorynexus-cli` when you want explicit shell workflows, scripting, or local
debugging. Use `memorynexus-mcp` when an AI client should call MemoryNexus tools
directly.

Both surfaces use the same Rust API and the same `CognitiveSpace` boundary.

For the first Dictation Coach agent loop over these generic tools, see
[Minimal Dictation Agent Demo](dictation-agent-demo.md).
