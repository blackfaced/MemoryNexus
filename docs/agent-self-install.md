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

## Execution Strategy

Work in phases and stop cleanly at blockers.

1. **Repository ready**: use the existing local repository if present.
2. **Rust/MCP ready**: build `memorynexus-mcp` before starting Docker services.
3. **Services ready**: start PostgreSQL and Qdrant.
4. **API ready**: run the Rust API and verify `/health`.
5. **Token ready**: reuse or create `MEMORYNEXUS_TOKEN`.
6. **Agent connected**: write the MCP config and reload the client.
7. **End-to-end smoke**: write, profile, route, and search through MCP.

If a phase fails twice for the same reason, do not loop. Report the blocker,
what was tried, and which later phases can still be completed.

## Safety Rules

- Do not commit secrets, JWT tokens, API keys, or local MCP config files.
- Do not paste plaintext tokens into logs or chat output.
- If a token or API key is missing, ask the user to provide it or authorize
  creating a local test account.
- Do not reintroduce the old Python/FastAPI backend.
- Do not make MemoryNexus agent-owned memory; use `CognitiveSpace`.

## Prerequisites

First check whether the repository already exists:

```bash
test -d /Users/bytedance/code/MemoryNexus && echo "repo exists"
```

If it exists, use it. Do not clone a second copy. If it does not exist, ask the
user before cloning.

Then check tools:

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

## Build MCP Server

Build the MCP binary:

```bash
cargo build --bin memorynexus-mcp
```

This phase does not require PostgreSQL, Qdrant, or a running API.

Verify the server exposes tools. The token can be a placeholder because
`tools/list` does not call the API:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="placeholder-token" \
    ./target/debug/memorynexus-mcp
```

The output must include:

- `create_space`
- `create_lens`
- `add_memory`
- `get_profile`
- `search_memories`
- `route_agent_context`

If this passes, the MCP binary is ready even if Docker is blocked.

## Start Local Services

Start PostgreSQL and Qdrant:

```bash
docker compose up -d postgres qdrant
```

If Docker image pulling fails, do not keep retrying blindly. Go to
[Docker Pull Or Proxy Issues](#docker-pull-or-proxy-issues).

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

Otherwise create a local test account after the API is healthy:

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080

AUTH_JSON=$(cargo run --quiet --bin memorynexus-cli -- auth register \
  --email "agent-local@example.com" \
  --name AgentLocal \
  --password secret123)

export MEMORYNEXUS_TOKEN=$(printf '%s' "$AUTH_JSON" | jq -r '.data.token')
```

Do not print the token.

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

## Docker Pull Or Proxy Issues

Docker image pulls are performed by the Docker daemon, not by the current shell.
Shell variables such as `HTTP_PROXY` may not affect `docker compose up`.

If Docker pull fails:

1. Check whether Docker works at all:

```bash
docker version
docker info
```

2. Check daemon proxy visibility:

```bash
docker info | grep -i proxy
```

3. If the proxy is missing, ask the user to configure Docker Desktop or the
   Docker daemon proxy and restart Docker. Do not assume shell proxy variables
   are enough.

4. If local images already exist, continue with them:

```bash
docker images | grep -E 'postgres|qdrant|minio'
```

5. If Docker remains blocked, complete the non-Docker phases:

- repository check
- Rust build
- `memorynexus-mcp` build
- MCP `tools/list` smoke
- MCP client config draft

Then report Docker as the blocker for API and end-to-end smoke.

## Partial Success Criteria

If full installation is blocked, report the highest completed level:

- **Level 1: Repo Ready**: repository exists and prerequisites checked.
- **Level 2: MCP Binary Ready**: `cargo build --bin memorynexus-mcp` succeeds.
- **Level 3: MCP Discoverable**: stdio `tools/list` shows MemoryNexus tools.
- **Level 4: API Ready**: API health check succeeds.
- **Level 5: Agent Connected**: MCP config is installed and visible in the
  agent client.
- **Level 6: End-to-End Ready**: MCP can create a space, create a Lens, write a
  memory, project a profile, and search.

Do not redo earlier successful levels unless files changed.

## Completion Report

When done, report:

- Highest completed level from the partial success list.
- Whether the API is running.
- Whether MCP `tools/list` shows MemoryNexus tools.
- Which MCP config entry was added.
- The created `space_id` and lens IDs.
- The smoke result for `add_memory`, `get_profile`, and `search_memories`.
- Any blocker that required user action.

Do not report JWT tokens or API keys.
