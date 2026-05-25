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
| `route_agent_context` | Recommend write/search/lens/profile/ignore for agent context |

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
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"add_reminder","arguments":{"space_id":"<space-id>","title":"Review project direction","content":"Run a project_context Lens and decide the next task.","remind_at":"2026-05-26T09:00:00Z","repeat_rule":"weekly"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_reminders","arguments":{"space_id":"<space-id>","due_only":true,"limit":20}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

The tool response returns MemoryNexus API JSON as text content so MCP clients can
read the same traceable payload that the CLI sees.

## MCP vs CLI

Use `memorynexus-cli` when you want explicit shell workflows, scripting, or local
debugging. Use `memorynexus-mcp` when an AI client should call MemoryNexus tools
directly.

Both surfaces use the same Rust API and the same `CognitiveSpace` boundary.
