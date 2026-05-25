# Personal Agent Integration

This guide targets local personal agents such as Claw or Hermes. The goal is to
let an agent use MemoryNexus as a personal cognitive substrate without making
the agent own memory.

## Mental Model

- `CognitiveSpace` is the memory universe.
- `Memory` is durable material inside a space.
- `Lens` is an interpretation strategy over that space.
- The agent is a client. It reads, writes, searches, and runs lenses through API
  or MCP tools, but it does not own memory.

Use one personal space per human by default. Create additional spaces only when
the memory universe should be intentionally separate, for example family,
project, or organization context.

## Recommended Setup

Start the API:

```bash
docker compose up -d postgres qdrant

export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_personal_agent
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

Create or log in with the CLI:

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080

AUTH_JSON=$(cargo run --quiet --bin memorynexus-cli -- auth register \
  --email "agent-local@example.com" \
  --name AgentLocal \
  --password secret123)

export MEMORYNEXUS_TOKEN=$(printf '%s' "$AUTH_JSON" | jq -r '.data.token')
```

Create a personal agent space:

```bash
SPACE_JSON=$(cargo run --quiet --bin memorynexus-cli -- space create \
  --name "Personal Agent Space" \
  --description "Long-term memory universe for Claw or Hermes")

export MEMORYNEXUS_SPACE_ID=$(printf '%s' "$SPACE_JSON" | jq -r '.data.id')
```

Create the recommended lenses:

```bash
PERSONAL_LENS_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --template personal_context)

PREFERENCE_LENS_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --template preference_review)

DECISION_LENS_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --template decision_history)

export MEMORYNEXUS_PERSONAL_LENS_ID=$(printf '%s' "$PERSONAL_LENS_JSON" | jq -r '.data.id')
export MEMORYNEXUS_PREFERENCE_LENS_ID=$(printf '%s' "$PREFERENCE_LENS_JSON" | jq -r '.data.id')
export MEMORYNEXUS_DECISION_LENS_ID=$(printf '%s' "$DECISION_LENS_JSON" | jq -r '.data.id')
```

## MCP Configuration

Point Claw, Hermes, or another MCP client at the stdio server:

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

For a lower-latency setup, build once and use the binary directly:

```bash
cargo build --bin memorynexus-mcp
```

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

## Agent Tool Policy

Use `add_memory` when the user explicitly says to remember something or when a
fact is clearly durable:

- stable preferences and dislikes
- long-running project direction
- important decisions and rationale
- recurring constraints
- personal working style
- meaningful relationships between projects, people, and goals

Do not write routine scratchpad content:

- transient command output
- one-off errors already resolved
- raw chat noise
- secrets, credentials, tokens, or private keys
- large pasted files without user intent to preserve them

Use `route_agent_context` before choosing a MemoryNexus tool when the agent is
uncertain. The router is deterministic and conservative: it recommends
`write_memory`, `search_memory`, `run_lens`, `get_profile`, or `ignore`, but it
does not execute the action. The agent should inspect `safety_flags` before
following the suggestion.

Use `search_memories` before answering when the question depends on durable user
context. Prefer semantic search for natural language queries.

Use `get_profile` at the start of a personal-agent session or before a task that
needs compact user context. It persists a profile snapshot with source memory
IDs and Lens Run IDs, so later answers can explain which Cognitive Space
materials shaped the context.

Use `list_reminders` with `due_only=true` at the start of a personal-agent
session when the agent should surface scheduled recall. Complete a reminder only
after the user or agent has handled it.

Use `run_lens` when the agent needs interpretation, not just retrieval:

- `personal_context`: "What should I know about this user before helping?"
- `preference_review`: "What stable preferences affect this task?"
- `decision_history`: "What related decisions have already been made?"
- `risk_review`: "What contradictions or unresolved concerns should I surface?"

Recommended order for Claw/Hermes:

1. `route_agent_context` when the correct MemoryNexus action is not obvious.
2. `get_profile` for compact working context.
3. `list_reminders` with `due_only=true` to surface scheduled recall.
4. `search_memories` for raw recall when the task mentions a specific topic.
5. `run_lens` when the agent needs an interpretation, tradeoff review, or
   contradiction check.
6. `add_memory` only when the information is durable and safe to persist.

## Memory Shape

Recommended `add_memory` arguments:

```json
{
  "space_id": "<personal-space-id>",
  "title": "User prefers Rust-first backend work",
  "content": "The user prefers Rust practice, functional programming ideas, category theory, and pragmatic Rust-first backend development.",
  "tags": ["preference", "rust", "working-style"]
}
```

Keep content compact and factual. If the agent is interpreting a memory, record
that interpretation through Lens Run rather than rewriting the original memory.

## Smoke Test

With the API running and `MEMORYNEXUS_TOKEN` set:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add_memory","arguments":{"space_id":"'"$MEMORYNEXUS_SPACE_ID"'","title":"Agent integration smoke","content":"Claw or Hermes can use MemoryNexus through MCP as a personal cognitive substrate.","tags":["agent","mcp","smoke"]}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_profile","arguments":{"space_id":"'"$MEMORYNEXUS_SPACE_ID"'","target":"personal_context","limit":12}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"route_agent_context","arguments":{"space_id":"'"$MEMORYNEXUS_SPACE_ID"'","message":"What do you remember about my personal cognitive substrate?"}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"search_memories","arguments":{"space_id":"'"$MEMORYNEXUS_SPACE_ID"'","query":"personal cognitive substrate","semantic":true,"limit":5}}}' \
  '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"list_reminders","arguments":{"space_id":"'"$MEMORYNEXUS_SPACE_ID"'","due_only":true,"limit":5}}}' \
  | cargo run --quiet --bin memorynexus-mcp
```

The response should include the tool list, a successful memory creation, a
persisted profile snapshot, a routing recommendation, at least one search result
from the same `CognitiveSpace`, and any due reminders.

## Current Gaps

- Router policy is deterministic and conservative; it recommends actions but
  does not execute them automatically.
- MCP does not create spaces or lenses yet; use CLI for setup.
- Reminder delivery is poll-based. Background dispatch, external notification
  channels, and richer recurrence rules are still Phase 3 follow-up work.
