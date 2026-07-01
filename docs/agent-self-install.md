# Agent Self-Install Guide

This guide is written for another local coding agent. Give this file to Claw,
Hermes, or a similar agent when you want it to install and connect
MemoryNexus by itself.

MemoryNexus is a local-first long-term feedback engine. The agent is only a
client/adapter. Memory belongs to `CognitiveSpace`, not to the agent.

## Task For The Agent

Install, upgrade, or reconnect MemoryNexus as an MCP server for this local
agent environment. First identify the current state, then choose the smallest
safe binary-first path:

- Trial Profile: no local checkout is required once a release binary is
  available. Use a prebuilt `memorynexus-mcp` binary with an existing
  hosted/demo API through `MEMORYNEXUS_API_URL` and `MEMORYNEXUS_TOKEN`.
- Local One-click Profile: use the release archive containing `memorynexus`,
  `memorynexus-cli`, and `memorynexus-mcp`; verify the checksum; install the
  binaries; start or verify local Docker PostgreSQL and Qdrant.
- Production Profile: use release binaries against stable hosted or
  self-hosted services. This is not Supabase-only.
- Developer Profile: use the source checkout and Cargo only for contributors or
  when no compatible release binary exists.
- Restart only: binaries and config are current, but the API or MCP client is
  still running old code.

Expected Local One-click result:

- The Rust API is running on `http://localhost:8080`.
- PostgreSQL and Qdrant are running locally.
- The MCP server `memorynexus-mcp` is discoverable by the agent client.
- The MCP tool list includes `create_space`, `create_lens`, `add_memory`,
  `get_profile`, `search_memories`, `run_lens`, `route_agent_context`,
  `create_practice_session`, `record_practice_attempt`,
  `record_practice_feedback`, `list_practice_sessions`,
  `get_practice_session`, `get_install_status`, `upgrade_install`, and the
  compatibility `learning_math_*` practice tools.
- A smoke memory can be written and retrieved through MCP.
- A STEM learning practice session can be created, patched with an attempt and
  feedback, then listed and retrieved through MCP using the canonical
  namespace-driven practice tool names.

## Execution Strategy

Work in phases and stop cleanly at blockers.

1. **Profile selected**: choose Trial, Local One-click, Production, or
   Developer.
2. **Target detected**: map OS/CPU to a supported release target.
3. **Binary ready**: download or locate the required release binary or archive
   and verify checksum when an archive is used.
4. **Services ready**: for Local One-click only, start PostgreSQL and Qdrant.
5. **API ready**: use the hosted/demo API for Trial, or start/verify the local
   Rust API for Local One-click.
6. **Token ready**: reuse or create `MEMORYNEXUS_TOKEN`.
7. **Agent connected**: write the MCP config and reload the client.
8. **MCP smoke**: run `initialize` and `tools/list`; the `tools/list` response
   must include MemoryNexus tools such as `create_space`, `add_memory`,
   `get_profile`, `get_install_status`, and `upgrade_install`.
9. **End-to-end smoke**: write, profile, route, and search through MCP.
10. **STEM learning smoke**: create a practice session, record an attempt,
   record feedback, list sessions, and retrieve the session.

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

Release archives are the target binary-first distribution path for
`aarch64-apple-darwin`, `x86_64-apple-darwin`, and
`x86_64-unknown-linux-gnu`.

As of 2026-06-17, the first GitHub Release artifact is still pending. If no
release is available, use Developer Profile source-build commands or a
maintainer-provided local binary. Do not pretend Trial or Local One-click is
plug-and-play until the release archive and checksum exist.

When published, each archive is named `memorynexus-<tag>-<target>.tar.gz` and
contains a Local One-click bundle layout:

- `bin/memorynexus`
- `bin/memorynexus-cli`
- `bin/memorynexus-mcp`
- `install.sh` for local binary install, Docker checks, optional MCP config
  output, API health, and MCP `tools/list` smoke
- `README.local-one-click.md` for the one-archive Local One-click flow
- `SHA256SUMS` and `MANIFEST.json` for files inside the archive
- `docker-compose.runtime.yml` for Local One-click PostgreSQL and Qdrant
  services
- `.env.runtime.example` with matching Docker service settings and host API
  binary environment values

This guide uses those binaries by default for Trial and Local One-click
profiles after a release exists. Source build fallback is explicit and should
happen when no compatible binary exists or the user chooses Developer Profile.

Profile mapping for the current release artifacts:

- Trial Profile: use `bin/memorynexus-mcp` when the Rust API is already
  available or managed separately.
- Local One-click Profile: use `bin/memorynexus`, `bin/memorynexus-cli`, and
  `bin/memorynexus-mcp` together with external PostgreSQL and Qdrant.
- Production Profile: use the same binaries as service artifacts; hosted
  service provider choices are outside this guide.
- Developer Profile: keep using `cargo run` and `cargo build` from source.

For Developer Profile, first check whether the repository already exists:

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

If Rust is missing, do not install it for Trial or Local One-click. Ask the user
whether they want Developer Profile before installing a Rust toolchain. If
Docker is missing, Trial can still proceed; Local One-click needs Docker or a
separate user-approved service setup.

## Binary-First Profiles

Use this path by default for Trial and Local One-click. If a release tag is not
provided, generate a plan with `<release-tag>` and ask the user which release to
use before downloading.

Choose the target triple:

```bash
uname -s
uname -m
```

Map common local machines to release targets:

| Local machine | Release target |
|---------------|----------------|
| macOS arm64 | `aarch64-apple-darwin` |
| macOS x86_64 | `x86_64-apple-darwin` |
| Linux x86_64 | `x86_64-unknown-linux-gnu` |

Download both files from the GitHub Release page:

```text
memorynexus-<tag>-<target>.tar.gz
memorynexus-<tag>-<target>.tar.gz.sha256
```

Verify and unpack:

```bash
sha256sum -c memorynexus-<tag>-<target>.tar.gz.sha256
tar -xzf memorynexus-<tag>-<target>.tar.gz
```

On macOS, use:

```bash
shasum -a 256 -c memorynexus-<tag>-<target>.tar.gz.sha256
```

Then verify the binaries directly:

```bash
./memorynexus-<tag>-<target>/bin/memorynexus-cli version
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="placeholder-token" \
    ./memorynexus-<tag>-<target>/bin/memorynexus-mcp
```

If this succeeds, use `./memorynexus-<tag>-<target>/bin/memorynexus-mcp` in the
MCP client config and `./memorynexus-<tag>-<target>/bin/memorynexus` for the API
server. For Trial Profile, only `bin/memorynexus-mcp` and the hosted API/token
are needed.
For Local One-click, PostgreSQL and Qdrant still need to be running separately,
usually through Docker.

## Developer Profile Source Build

Use this path only when no compatible release binary exists or the user
explicitly chooses Developer Profile. Build the MCP binary early. This phase
does not require PostgreSQL, Qdrant, or a running API:

```bash
cargo build --bin memorynexus-mcp
```

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
- `run_lens`
- `route_agent_context`
- `create_practice_session`
- `record_practice_attempt`
- `record_practice_feedback`
- `list_practice_sessions`
- `get_practice_session`
- `get_install_status`
- `upgrade_install`
- `learning_math_create_practice_session`
- `learning_math_record_attempt`
- `learning_math_record_feedback`
- `learning_math_list_practice_sessions`
- `learning_math_get_practice_session`

The `create_practice_session`, `record_practice_attempt`,
`record_practice_feedback`, `list_practice_sessions`, and
`get_practice_session` tools are canonical. They operate on a supplied
`namespace_id`, so `learning.stem` and `learning.math` are Namespace data inside
one Cognitive Space. The `learning_math_*` tools remain compatibility aliases
for the first implementation slice.

If this passes, the MCP binary is ready even if Docker is blocked.

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

Prefer the built-in status command when `memorynexus-cli` is available. It
reports the selected profile, detected OS/arch, release target, binary path,
API URL, API health, MCP smoke command, and source-build fallback reason:

```bash
memorynexus-cli install status --profile trial
memorynexus-cli install status --profile local-one-click
memorynexus-cli install status --profile production
memorynexus-cli install status --profile developer --checkout /path/to/MemoryNexus
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
| User wants hosted/demo trial | Use Trial Profile with release `memorynexus-mcp`; do not require local Docker or Rust. |
| User wants local-first without Rust | Use Local One-click Profile with release archive, checksum, local bin install, Docker services, API health, and MCP smoke. |
| User wants stable hosted deployment | Use Production Profile with release binaries and hosted/self-hosted PostgreSQL/Qdrant services. |
| No compatible release target exists | Report the unsupported OS/arch and ask before falling back to Developer Profile. |
| Checkout exists, no MCP config | Prefer release-binary config unless the user chose Developer Profile. |
| Checkout already contains the user's latest local edits | For Developer Profile, skip `git pull`; run tests, rebuild if needed, then restart API/MCP. |
| MCP config uses `cargo run` | Developer Profile only: pull/update source if requested, run tests, restart/reload the agent MCP client. |
| MCP config uses `target/debug/memorynexus-mcp` | Developer Profile only: pull/update source if requested, run tests, rebuild `memorynexus-mcp`, then restart/reload the agent MCP client. |
| MCP config uses release `memorynexus-mcp` | Replace the release directory only when the user requests a new release tag, then restart/reload the agent MCP client. |
| API binary or `cargo run --bin memorynexus` is already running | Restart the API after source changes so migrations and new handlers load. |
| Only docs changed | No API or MCP rebuild is required unless the agent needs a refreshed local checkout. |

The API runs SQLx migrations on startup. After migrations are added or changed,
restart the API; do not rely on a running process to pick them up.

## Upgrade Existing Install

Use this path when a checkout already exists and the selected profile is
Developer.

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
  --profile developer \
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
  --profile developer \
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
    "profile": "trial"
  }
}
```

```json
{
  "name": "get_install_status",
  "arguments": {
    "profile": "developer",
    "checkout_dir": "/path/to/MemoryNexus"
  }
}
```

```json
{
  "name": "upgrade_install",
  "arguments": {
    "profile": "developer",
    "checkout_dir": "/path/to/MemoryNexus",
    "pull": true,
    "rebuild_mcp": true,
    "apply": false
  }
}
```

## Start Local One-click Services

This section applies to Local One-click Profile. Trial Profile skips it because
the API, PostgreSQL, and Qdrant already exist elsewhere. Production Profile uses
stable hosted or self-hosted services instead of per-user Docker-managed
dependencies. Developer Profile may use the same local Docker services for
contributor testing.

Start PostgreSQL and Qdrant:

```bash
./install.sh --start-services
```

The install script checks Docker availability before starting services and does
not call `cargo`, `rustc`, or `rustup`. The equivalent manual command is
`docker compose -f docker-compose.runtime.yml --env-file .env.runtime.example up
-d postgres qdrant`.

If Docker image pulling fails, do not keep retrying blindly. Go to
[Docker Pull Or Proxy Issues](#docker-pull-or-proxy-issues).

The runtime compose file starts only PostgreSQL and Qdrant. It does not build or
run the Rust API and does not require Rust or Cargo. The Local One-click primary
path is still the release API binary running on the host.

Load the same runtime values into the host shell, then start the API from the
release binary in a long-running terminal:

```bash
set -a
. ./.env.runtime.example
set +a

./memorynexus-<tag>-<target>/bin/memorynexus
```

For Developer Profile only, the equivalent source-build command is
`cargo run --bin memorynexus`.

In another terminal, verify the API:

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080
./memorynexus-<tag>-<target>/bin/memorynexus-cli health
```

For Developer Profile only, use
`cargo run --quiet --bin memorynexus-cli -- health`.

To stop only the Local One-click runtime services:

```bash
docker compose \
  -f docker-compose.runtime.yml \
  --env-file .env.runtime.example \
  down
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

For Local One-click Profile, use the release CLI instead of Cargo:

```bash
AUTH_JSON=$(./memorynexus-<tag>-<target>/bin/memorynexus-cli auth register \
  --email "agent-local@example.com" \
  --name AgentLocal \
  --password secret123)

export MEMORYNEXUS_TOKEN=$(printf '%s' "$AUTH_JSON" | jq -r '.data.token')
```

The `cargo run` form is Developer Profile only.

Do not print the token.

## Configure The Agent MCP Client

Add a MemoryNexus MCP server entry to the local agent client's MCP config.
Use the client-specific config path for Claw, Hermes, or the current agent
runtime.

Recommended release-binary config when using a downloaded archive:

```json
{
  "mcpServers": {
    "memorynexus": {
      "command": "/path/to/memorynexus-<tag>-<target>/bin/memorynexus-mcp",
      "env": {
        "MEMORYNEXUS_API_URL": "http://localhost:8080",
        "MEMORYNEXUS_TOKEN": "<jwt-token>"
      }
    }
  }
}
```

Recommended source-build low-latency config. This mode requires `cargo build
--bin memorynexus-mcp` after source changes:

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

7. Create or select the `learning.stem` Skill Namespace through the Rust API.
   MCP practice tools are namespace-driven, so this is a one-time HTTP setup
   step unless the namespace already exists:

```bash
curl -fsS -X POST http://localhost:8080/api/v1/namespaces \
  -H "Authorization: Bearer $MEMORYNEXUS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "space_id": "<space-id>",
    "name": "learning.stem",
    "kind": "skill",
    "description": "Parent-assisted STEM practice feedback"
  }'
```

8. Call `create_practice_session` for the local STEM learning smoke. Use the
   returned `namespace_id`; optional `space_id` is only a guard and must match
   the Namespace Space.

```json
{
  "namespace_id": "<learning-stem-namespace-id>",
  "capture_memory": true,
  "practice_goal": "Practice fraction word problems",
  "exercise": "A recipe uses 3/4 cup of flour. If we make half the recipe, how much flour is needed?"
}
```

9. Read the returned `data.id` as `practice_session_id`, then call
   `record_practice_attempt`:

```json
{
  "namespace_id": "<learning-stem-namespace-id>",
  "practice_session_id": "<practice-session-id>",
  "answer": "3/8 cup",
  "reasoning": "Half of 3/4 is 3/8",
  "capture_memory": true
}
```

10. Call `record_practice_feedback`:

```json
{
  "namespace_id": "<learning-stem-namespace-id>",
  "practice_session_id": "<practice-session-id>",
  "mistake_pattern": "None this time",
  "feedback": "Good fraction multiplication: half means multiply by 1/2.",
  "practice_adjustment": "Try one similar word problem with a different numerator.",
  "next_exercise": "A garden uses 2/3 bag of soil. How much is needed for half the garden?",
  "status": "completed",
  "capture_memory": true
}
```

11. Verify the session is discoverable:

```json
{
  "namespace_id": "<learning-stem-namespace-id>",
  "limit": 10
}
```

Use those arguments with `list_practice_sessions`, then call
`get_practice_session`:

```json
{
  "namespace_id": "<learning-stem-namespace-id>",
  "practice_session_id": "<practice-session-id>"
}
```

For STEM learning smoke, do not expose backend terms like `MemoryAtom` or
`CognitiveProjection` to the learner-facing transcript. Use product language
such as practice session, attempt, feedback, and next task.

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

To smoke STEM learning over stdio, create or select `learning.stem` through the
HTTP Namespace API first, then create the session with the canonical MCP tool:

```bash
SESSION_JSON=$(printf '%s\n' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_practice_session","arguments":{"namespace_id":"<learning-stem-namespace-id>","capture_memory":true,"practice_goal":"Practice fraction word problems","exercise":"A recipe uses 3/4 cup of flour. If we make half the recipe, how much flour is needed?"}}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" \
    ./target/debug/memorynexus-mcp)

printf '%s\n' "$SESSION_JSON"
```

Then extract `data.id` from the returned API JSON text as
`practice_session_id` and continue with `record_practice_attempt`,
`record_practice_feedback`, `list_practice_sessions`, and
`get_practice_session`. Do not reuse the placeholder `<practice-session-id>` in
real calls. The older `learning_math_*` tools remain compatibility aliases when
an existing client still uses the first-slice `learning.math` surface.

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

- release binary check
- `memorynexus-mcp` binary verification
- MCP `tools/list` smoke
- MCP client config draft

Then report Docker as the blocker for API and end-to-end smoke.

## Partial Success Criteria

If full installation is blocked, report the highest completed level:

- **Level 1: Profile Ready**: profile, release target, and prerequisites checked.
- **Level 2: MCP Binary Ready**: release `memorynexus-mcp` is verified or
  Developer Profile `cargo build --bin memorynexus-mcp` succeeds.
- **Level 3: MCP Discoverable**: stdio `tools/list` shows MemoryNexus tools.
- **Level 4: API Ready**: API health check succeeds.
- **Level 5: Agent Connected**: MCP config is installed and visible in the
  agent client.
- **Level 6: End-to-End Ready**: MCP can create a space, create a Lens, write a
  memory, project a profile, and search.
- **Level 7: STEM Learning Ready**: MCP can create a STEM learning practice
  session, record an attempt, record feedback, list sessions, and retrieve the
  session.

Do not redo earlier successful levels unless files changed.

## Completion Report

When done, report:

- Highest completed level from the partial success list.
- Whether this was a fresh install, source upgrade, binary rebuild, or restart
  only.
- Whether the API is running.
- Whether MCP `tools/list` shows MemoryNexus tools.
- Which MCP config entry was added.
- Whether the MCP config uses `cargo run` or a built binary.
- The created `space_id` and lens IDs.
- The smoke result for `add_memory`, `get_profile`, and `search_memories`.
- The canonical practice tool availability and, if run, the created
  `namespace_id` and `practice_session_id`.
- Whether STEM learning create/attempt/feedback calls captured memory snapshots.
- Any blocker that required user action.

Do not report JWT tokens or API keys.
