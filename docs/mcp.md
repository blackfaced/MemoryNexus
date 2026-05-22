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

## Tools

| Tool | Purpose |
|------|---------|
| `list_spaces` | List Cognitive Spaces visible to the authenticated user |
| `add_memory` | Add a text memory to a Cognitive Space |
| `search_memories` | Search memories by `space_id` or `lens_id` |
| `list_lenses` | List Lenses in a Cognitive Space |
| `run_lens` | Run a Lens query and return a traceable Lens Run |
| `get_lens_run` | Fetch a persisted Lens Run by ID |

## Smoke Test

You can test the protocol without an MCP client:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

Expected output includes an `initialize` response and a `tools/list` response
with the six tools above.

To call a tool, keep the API running and send a `tools/call` request:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_memories","arguments":{"query":"cognitive lens","lens_id":"<lens-id>","limit":5}}}' \
  | MEMORYNEXUS_TOKEN='<jwt-token>' cargo run --quiet --bin memorynexus-mcp
```

The tool response returns MemoryNexus API JSON as text content so MCP clients can
read the same traceable payload that the CLI sees.

## MCP vs CLI

Use `memorynexus-cli` when you want explicit shell workflows, scripting, or local
debugging. Use `memorynexus-mcp` when an AI client should call MemoryNexus tools
directly.

Both surfaces use the same Rust API and the same `CognitiveSpace` boundary.
