# FAQ

## Is MemoryNexus an agent memory plugin?

No. Memory belongs to `CognitiveSpace`. Agents can read, write, search, and run
interpretation strategies, but they do not own the memory.

## Can another agent install MemoryNexus by itself?

Yes, if it can run shell commands and edit its own MCP configuration. Give it
[Agent Self-Install](agent-self-install.md). The guide tells the agent how to
start local services, build `memorynexus-mcp`, configure MCP, create a
`CognitiveSpace`, create a Lens, and run smoke tests without exposing tokens.

## What is the minimum agent integration path?

Start the Rust API, provide `MEMORYNEXUS_TOKEN`, configure `memorynexus-mcp`, and
let the agent call MCP tools such as `create_space`, `create_lens`,
`add_memory`, `get_profile`, `search_memories`, and `route_agent_context`.

## Where is the backend?

The Rust crate lives at the repository root. Run Cargo commands from the root.

## Is the old Python API still supported?

No. The historical Python/FastAPI skeleton has been removed. Rust + Axum is the
only backend path.

## How do I test semantic search locally?

Start Qdrant and use the local deterministic embedding provider:

```bash
docker compose up -d postgres qdrant
export QDRANT_URL=http://localhost:6333
export MEMORYNEXUS_EMBEDDING_PROVIDER=local
cargo run --bin memorynexus
```

Then use `memorynexus-cli search --semantic --space <SPACE_ID>`.
