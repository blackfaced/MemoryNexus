# Agent Self-Install Guide

This guide is written for another local coding agent. Give this file to Claw,
Hermes, or a similar agent when you want it to install and connect
MemoryNexus by itself.

MemoryNexus is a Rust-first cognitive lens memory system. The agent is only a
client. Memory belongs to `CognitiveSpace`, not to the agent.

## Task For The Agent

Install, upgrade, or reconnect MemoryNexus as an MCP server for this local
agent environment. First identify the current state, then choose the smallest
safe path:

- Fresh install: no local MemoryNexus checkout or MCP config exists.
- Source upgrade: a checkout exists and should be updated from git.
- Binary rebuild: the MCP config points at `target/debug/memorynexus-mcp` or
  another built binary, so the binary must be rebuilt after source changes.
- Restart only: source and binary are current, but the API or MCP client is
  still running old code.

Expected result:

- The Rust API is running on `http://localhost:8080`.
- PostgreSQL and Qdrant are running locally.
- The MCP server `memorynexus-mcp` is discoverable by the agent client.
- The MCP tool list includes `create_space`, `create_lens`, `add_memory`,
  `get_profile`, `search_memories`, `run_lens`, `route_agent_context`,
  `get_install_status`, and `upgrade_install`.
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

## Detect Current State

Before installing, check whether MemoryNexus is already present and how the
agent is connected.

1. Find the repository checkout. Prefer an explicit user-provided path. Common
   local paths include:

```bash
test -d /Users/bytedance/code/MemoryNexus && echo /Users/bytedance/code/MemoryNexus
test -d /Users/bytedance/code/worktrees/MemoryNexus && find /Users/bytedance/code/worktrees/MemoryNexus -maxdepth 3 -name AGENTS.md -print
```

2. In the chosen checkout, inspect source state:

```bash
cd /path/to/MemoryNexus
git status --short
git rev-parse --show-toplevel
git log -1 --oneline
```

Do not discard or overwrite dirty files. If `git status --short` shows local
changes, keep them and ask before pulling if the changes could conflict.

3. Check whether local services are already running:

```bash
curl -fsS http://localhost:8080/health
docker compose ps postgres qdrant
```

If the checkout can build the CLI, prefer the built-in status command because it
also reports the local binary version and API version:

```bash
cargo run --quiet --bin memorynexus-cli -- install status --checkout /path/to/MemoryNexus
```

4. Inspect the agent MCP config if the client exposes it. Determine whether the
MemoryNexus server uses development mode:

```json
{
  "command": "cargo",
  "args": ["run", "--quiet", "--bin", "memorynexus-mcp"],
  "cwd": "/path/to/MemoryNexus"
}
```

or built-binary mode:

```json
{
  "command": "/path/to/MemoryNexus/target/debug/memorynexus-mcp"
}
```

If the config already exists, prefer upgrading it in place instead of creating a
second `memorynexus` MCP entry.

## Choose Install Or Upgrade

Use this decision table:

| Current state | Action |
|---------------|--------|
| No checkout exists | Follow fresh install setup below. |
| Checkout exists, no MCP config | Start services, verify API, then add MCP config. |
| Checkout already contains the user's latest local edits | Skip `git pull`; run tests, rebuild if needed, then restart API/MCP. |
| MCP config uses `cargo run` | Pull/update source if requested, run tests, restart/reload the agent MCP client. |
| MCP config uses `target/debug/memorynexus-mcp` | Pull/update source if requested, run tests, rebuild `memorynexus-mcp`, then restart/reload the agent MCP client. |
| API binary or `cargo run --bin memorynexus` is already running | Restart the API after source changes so migrations and new handlers load. |
| Only docs changed | No API or MCP rebuild is required unless the agent needs a refreshed local checkout. |

The API runs SQLx migrations on startup. After migrations are added or changed,
restart the API; do not rely on a running process to pick them up.

## Upgrade Existing Install

Use this path when a checkout already exists.

1. Enter the checkout:

```bash
cd /path/to/MemoryNexus
```

2. Preserve local work:

```bash
git status --short
```

If there are unrelated dirty files, leave them alone. If the user has just made
local edits in this checkout, skip `git pull` and continue to tests/builds. If
the user asked for a repository update and the tree is clean or the changes are
known to be safe, pull the latest source:

```bash
git pull
```

3. Ask MemoryNexus to generate the upgrade plan. This does not execute local
   commands unless `--apply` is present:

```bash
cargo run --quiet --bin memorynexus-cli -- upgrade \
  --checkout /path/to/MemoryNexus \
  --pull \
  --rebuild-mcp
```

Omit `--pull` when the checkout already contains the user's latest local edits.
Omit `--rebuild-mcp` when the MCP config uses `cargo run`.

4. Verify the updated source:

```bash
cargo test
```

5. If the MCP config uses a built binary, rebuild it:

```bash
cargo build --bin memorynexus-mcp
```

If the API is launched from a built binary instead of `cargo run`, rebuild it
too:

```bash
cargo build --bin memorynexus
```

The CLI can execute the test/build part when explicitly requested:

```bash
cargo run --quiet --bin memorynexus-cli -- upgrade \
  --checkout /path/to/MemoryNexus \
  --pull \
  --rebuild-mcp \
  --apply
```

6. Restart the Rust API when backend code or migrations changed:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_agent_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

7. Restart or reload the agent MCP client. This step is required even when the
MCP config uses `cargo run`, because the old stdio server process keeps running
until the client restarts it.

8. Verify the upgraded MCP surface:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" \
    cargo run --quiet --bin memorynexus-mcp
```

If the config uses a built binary, test the same binary that the agent uses:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" \
    ./target/debug/memorynexus-mcp
```

After the MCP client is connected, the agent may call MCP tools instead of the
CLI:

```json
{
  "name": "get_install_status",
  "arguments": {
    "checkout_dir": "/path/to/MemoryNexus"
  }
}
```

```json
{
  "name": "upgrade_install",
  "arguments": {
    "checkout_dir": "/path/to/MemoryNexus",
    "pull": true,
    "rebuild_mcp": true,
    "apply": false
  }
}
```

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

Build the MCP binary when using built-binary mode:

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

Recommended low-latency config. This mode requires `cargo build --bin
memorynexus-mcp` after source changes:

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

Development config if the binary has not been built. This mode recompiles on
MCP server startup, but still requires restarting or reloading the MCP client
after source changes:

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

- Whether this was a fresh install, source upgrade, binary rebuild, or restart
  only.
- Whether the API is running.
- Whether MCP `tools/list` shows MemoryNexus tools.
- Which MCP config entry was added.
- Whether the MCP config uses `cargo run` or a built binary.
- The created `space_id` and lens IDs.
- The smoke result for `add_memory`, `get_profile`, and `search_memories`.
- Any blocker that required user action.

Do not report JWT tokens or API keys.
