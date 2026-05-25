# Agent Self-Install Guide

This guide is written for another local coding agent. Give this file to Claw,
Hermes, or a similar agent when you want it to install and connect
MemoryNexus by itself.

MemoryNexus is a Rust-first cognitive lens memory system. The agent is only a
client. Memory belongs to `CognitiveSpace`, not to the agent.

## Task For The Agent

Install and connect MemoryNexus as an MCP server for this local agent
environment.

Expected result:

- The Rust API is running on `http://localhost:8080`.
- PostgreSQL and Qdrant are running locally.
- The MCP server `memorynexus-mcp` is discoverable by the agent client.
- The MCP tool list includes `create_space`, `create_lens`, `add_memory`,
  `get_profile`, `search_memories`, `run_lens`, and `route_agent_context`.
- A smoke memory can be written and retrieved through MCP.

## Safety Rules

- Do not commit secrets, JWT tokens, API keys, or local MCP config files.
- Do not paste plaintext tokens into logs or chat output.
- If a token or API key is missing, ask the user to provide it or authorize
  creating a local test account.
- Do not reintroduce the old Python/FastAPI backend.
- Do not make MemoryNexus agent-owned memory; use `CognitiveSpace`.

## Prerequisites

Check these first:

```bash
pwd
cargo --version
docker --version
docker compose version
jq --version
```

Run from the repository root:

```bash
cd /Users/bytedance/code/MemoryNexus
```

If Rust or Docker is missing, ask the user before installing system packages.

## Start Local Services

Start PostgreSQL and Qdrant:

```bash
docker compose up -d postgres qdrant
```

Start the Rust API in a long-running terminal:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_agent_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

In another terminal, verify the API:

```bash
cargo run --quiet --bin memorynexus-cli -- health
```

## Create Or Reuse Auth Token

If the user already has `MEMORYNEXUS_TOKEN`, reuse it.

Otherwise create a local test account:

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080

AUTH_JSON=$(cargo run --quiet --bin memorynexus-cli -- auth register \
  --email "agent-local@example.com" \
  --name AgentLocal \
  --password secret123)

export MEMORYNEXUS_TOKEN=$(printf '%s' "$AUTH_JSON" | jq -r '.data.token')
```

Do not print the token.

## Build MCP Server

Build the MCP binary:

```bash
cargo build --bin memorynexus-mcp
```

Verify the server exposes tools:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" \
    ./target/debug/memorynexus-mcp
```

The output must include:

- `create_space`
- `create_lens`
- `add_memory`
- `get_profile`
- `search_memories`
- `route_agent_context`

## Configure The Agent MCP Client

Add a MemoryNexus MCP server entry to the local agent client's MCP config.
Use the client-specific config path for Claw, Hermes, or the current agent
runtime.

Recommended low-latency config:

```json
{
  "mcpServers": {
    "memorynexus": {
      "command": "/Users/bytedance/code/MemoryNexus/target/debug/memorynexus-mcp",
      "env": {
        "MEMORYNEXUS_API_URL": "http://localhost:8080",
        "MEMORYNEXUS_TOKEN": "<jwt-token>"
      }
    }
  }
}
```

Development config if the binary has not been built:

```json
{
  "mcpServers": {
    "memorynexus": {
      "command": "cargo",
      "args": ["run", "--quiet", "--bin", "memorynexus-mcp"],
      "cwd": "/Users/bytedance/code/MemoryNexus",
      "env": {
        "MEMORYNEXUS_API_URL": "http://localhost:8080",
        "MEMORYNEXUS_TOKEN": "<jwt-token>"
      }
    }
  }
}
```

Replace `<jwt-token>` with the token without printing it in chat.

Restart or reload the agent client after updating its MCP config.

## Bootstrap Through MCP

After the MCP client is connected, use MCP tools directly:

1. Call `create_space`:

```json
{
  "name": "Personal Agent Space",
  "description": "Long-term memory universe for the personal agent",
  "space_type": "personal"
}
```

2. Use the returned `id` as `space_id` and call `create_lens`:

```json
{
  "space_id": "<space-id>",
  "name": "Personal Context",
  "strategy": "personal_context",
  "output_format": "brief",
  "retrieval_mode": "semantic"
}
```

3. Call `add_memory`:

```json
{
  "space_id": "<space-id>",
  "title": "Agent integration smoke",
  "content": "This agent can use MemoryNexus through MCP as a personal cognitive substrate.",
  "tags": ["agent", "mcp", "smoke"]
}
```

4. Call `get_profile`:

```json
{
  "target": "personal_context",
  "limit": 12
}
```

5. Call `search_memories`:

```json
{
  "space_id": "<space-id>",
  "query": "personal cognitive substrate",
  "semantic": true,
  "limit": 5
}
```

6. Call `route_agent_context` before uncertain writes:

```json
{
  "space_id": "<space-id>",
  "message": "Remember this: I prefer Rust-first backend work."
}
```

## Stdio Smoke Without MCP Client

If the MCP client integration is hard to inspect, run this local stdio smoke:

```bash
SPACE_JSON=$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_space","arguments":{"name":"Personal Agent Space","description":"Self-install smoke","space_type":"personal"}}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" \
    ./target/debug/memorynexus-mcp)

printf '%s\n' "$SPACE_JSON"
```

Then extract the `space_id` from the returned API JSON text and continue with
`create_lens`, `add_memory`, `get_profile`, and `search_memories`.

## Completion Report

When done, report:

- Whether the API is running.
- Whether MCP `tools/list` shows MemoryNexus tools.
- Which MCP config entry was added.
- The created `space_id` and lens IDs.
- The smoke result for `add_memory`, `get_profile`, and `search_memories`.
- Any blocker that required user action.

Do not report JWT tokens or API keys.
