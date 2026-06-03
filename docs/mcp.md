# MemoryNexus MCP Server

`memorynexus-mcp` exposes MemoryNexus as a local MCP stdio server. It is a thin
adapter over the Rust API, not a second backend. Memory still belongs to
`CognitiveSpace`; MCP clients only call tools that operate through the API.

## Configuration

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
| `learning_math_create_practice_session` | Create a parent-assisted `learning.math` practice session |
| `learning_math_record_attempt` | Record a child's answer or reasoning for a practice session |
| `learning_math_record_feedback` | Record mistake pattern, feedback, adjustment, and next exercise |
| `learning_math_list_practice_sessions` | List `learning.math` practice sessions in a Cognitive Space |
| `learning_math_get_practice_session` | Fetch one `learning.math` practice session |
| `get_install_status` | Inspect local version, checkout state, and API health/version before install or upgrade |
| `upgrade_install` | Return or apply a local upgrade plan for source, tests, and built binaries |

## Smoke Test

You can test the protocol without an MCP client:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Expected output includes an `initialize` response and a `tools/list` response
with the tools above.

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

Parent-assisted `learning.math` practice:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"learning_math_create_practice_session","arguments":{"space_id":"<space-id>","practice_goal":"Improve fraction word problems","exercise":"Solve five fraction word problems and explain the reasoning","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"learning_math_record_attempt","arguments":{"practice_session_id":"<practice-session-id>","answer":"I solved 3 out of 5","reasoning":"I changed units in the middle of the problem","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"learning_math_record_feedback","arguments":{"practice_session_id":"<practice-session-id>","mistake_pattern":"Changed units between steps","feedback":"Write the unit next to every number before calculating","practice_adjustment":"Add a unit-labeling step","next_exercise":"Try three unit-conversion fraction problems","status":"completed","capture_memory":true}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"learning_math_list_practice_sessions","arguments":{"space_id":"<space-id>","limit":10}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"learning_math_get_practice_session","arguments":{"practice_session_id":"<practice-session-id>"}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

`learning_math_create_practice_session` accepts `space_id` and optional
`namespace_id`. When `namespace_id` is omitted, the Rust API creates or reuses
the `learning.math` skill Namespace inside the same Cognitive Space. The MCP
tools keep the product-facing fields `practice_goal`, `exercise`, `answer`,
`reasoning`, `mistake_pattern`, `feedback`, `practice_adjustment`, and
`next_exercise`; they do not expose MemoryAtom, CognitiveScene, or
CognitiveProjection as practice-flow inputs.

The tool response returns MemoryNexus API JSON as text content so MCP clients can
read the same traceable payload that the CLI sees.

Install or upgrade inspection:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_install_status","arguments":{"checkout_dir":"/path/to/MemoryNexus"}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

Plan an upgrade without executing local commands:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"upgrade_install","arguments":{"checkout_dir":"/path/to/MemoryNexus","pull":true,"rebuild_mcp":true}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

Apply an upgrade explicitly:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"upgrade_install","arguments":{"checkout_dir":"/path/to/MemoryNexus","apply":true,"pull":true,"rebuild_mcp":true}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

`upgrade_install` defaults to plan-only. It refuses `git pull` when local files
are dirty. It does not restart the API or the current MCP client; the response
reports which restarts are still required.

## MCP vs CLI

Use `memorynexus-cli` when you want explicit shell workflows, scripting, or local
debugging. Use `memorynexus-mcp` when an AI client should call MemoryNexus tools
directly.

Both surfaces use the same Rust API and the same `CognitiveSpace` boundary.
