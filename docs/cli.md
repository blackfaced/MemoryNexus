# MemoryNexus CLI

> Current status: CLI MVP v0. The CLI is a thin, stateless client for the Rust REST API.

## Goals

| Principle | Meaning |
|-----------|---------|
| Machine-first | Output is JSON by default for Agent consumption. |
| Stateless | Server owns state; the CLI only sends API requests. |
| Rust-first | The CLI lives in the Rust crate as `memorynexus-cli`. |
| MVP-scoped | Space and Lens commands come after Cognitive Space APIs land. |

## Run Locally

```bash
cd src
cargo run --bin memorynexus-cli -- health
```

The backend must be running separately for API commands.

For a full local smoke test, start PostgreSQL first and run the Rust API in a
second terminal:

```bash
docker compose up -d postgres

cd src
cargo run --bin memorynexus
```

Then run CLI commands from `src/` in another terminal.

## Configuration

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `MEMORYNEXUS_API_URL` | No | `http://localhost:8080` | Rust API base URL |
| `MEMORYNEXUS_TOKEN` | Yes, except `health` and `auth` | - | JWT bearer token returned by login/register |

## Quick Start

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080

cargo run --bin memorynexus-cli -- health

cargo run --bin memorynexus-cli -- auth register \
  --email alice@example.com \
  --name Alice \
  --password secret123

cargo run --bin memorynexus-cli -- auth login \
  --email alice@example.com \
  --password secret123

export MEMORYNEXUS_TOKEN=<token-from-auth-response>

cargo run --bin memorynexus-cli -- memory add \
  --title "Rust practice" \
  --content "Today I practiced Rust cognitive memory." \
  --tags "rust,cognitive-memory"

cargo run --bin memorynexus-cli -- memory list --limit 10

cargo run --bin memorynexus-cli -- search "Rust cognitive memory" --semantic --limit 5
```

## Local Smoke Test

Use this sequence to verify the current CLI MVP against the local Rust API.
The examples avoid semantic search so they only require PostgreSQL.

```bash
docker compose up -d postgres
```

Terminal 1:

```bash
cd src
cargo run --bin memorynexus
```

Terminal 2:

```bash
cd src
export MEMORYNEXUS_API_URL=http://localhost:8080

cargo run --bin memorynexus-cli -- health

cargo run --bin memorynexus-cli -- auth register \
  --email cli-smoke@example.com \
  --name CliSmoke \
  --password secret123

cargo run --bin memorynexus-cli -- auth login \
  --email cli-smoke@example.com \
  --password secret123

export MEMORYNEXUS_TOKEN=<token-from-auth-response>

cargo run --bin memorynexus-cli -- memory add \
  --title "CLI smoke memory" \
  --content "Rust cognitive lens memory CLI smoke test" \
  --tags "cli,smoke"

cargo run --bin memorynexus-cli -- memory list --limit 5

cargo run --bin memorynexus-cli -- search "cognitive lens" --limit 5
```

Expected results:

- `health` returns `{"status":"healthy","version":"0.1.0"}`.
- `auth register` and `auth login` return `ok: true` and `data.token`.
- `memory add` returns the created memory.
- `memory list` includes that memory.
- keyword `search` returns the matching memory.

Semantic search is intentionally a separate check because it requires Qdrant
and an embedding provider:

```bash
docker compose up -d qdrant
export OPENAI_API_KEY=<your-key>

cargo run --bin memorynexus-cli -- search "cognitive lens" --semantic --limit 5
```

Without Qdrant or an embedding key, `--semantic` should return a JSON error
from the API instead of a keyword result.

## Commands

### Health

```bash
memorynexus-cli health
```

Calls `GET /api/v1/health`.

### Auth

```bash
memorynexus-cli auth register \
  --email <EMAIL> \
  --username <USERNAME> \
  --password <PASSWORD>

memorynexus-cli auth register \
  --email <EMAIL> \
  --name <USERNAME> \
  --password <PASSWORD>

memorynexus-cli auth login \
  --email <EMAIL> \
  --password <PASSWORD>
```

Calls:

- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login`

The response includes `data.token`. Export it as `MEMORYNEXUS_TOKEN` before running authenticated commands.

### Memory

```bash
memorynexus-cli memory add \
  --content <TEXT> \
  [--title <TEXT>] \
  [--tags <COMMA_SEPARATED_TAGS>] \
  [--type text|image|audio|video] \
  [--shared]

memorynexus-cli memory list [--limit <N>] [--offset <N>]

memorynexus-cli memory get <MEMORY_ID>

memorynexus-cli memory delete <MEMORY_ID>
```

Calls:

- `POST /api/v1/memories`
- `GET /api/v1/memories`
- `GET /api/v1/memories/:id`
- `DELETE /api/v1/memories/:id`

### Search

```bash
memorynexus-cli search <QUERY> [--semantic] [--limit <N>]
```

Calls `GET /api/v1/search`.

`--semantic` sets `semantic=true`, which uses the backend Embedding -> Qdrant -> search path when configured.

## Output

Successful responses pass through the backend JSON.

```json
{
  "ok": true,
  "data": {}
}
```

CLI-side errors are also JSON:

```json
{
  "ok": false,
  "error": {
    "message": "MEMORYNEXUS_TOKEN is required"
  }
}
```

## Local Cleanup

Some local Rust toolchain builds may leave LLVM profile files named like
`default_*.profraw`. They are temporary profiling artifacts and are safe to
delete. The repository root ignores `*.profraw`, but cleanup keeps `src/`
readable:

```bash
find src -name 'default_*.profraw' -delete
```

If you only want to inspect them first:

```bash
find src -name 'default_*.profraw'
```

## Not In v0

- Local token persistence.
- Interactive config.
- Table or CSV output.
- Shell completions.
- `space` and `lens` commands.
- Direct database access.

## Next CLI Steps

After Cognitive Space APIs land, extend commands as:

```bash
memorynexus-cli space create --name "Personal Space"
memorynexus-cli space list
memorynexus-cli memory add --space <SPACE_ID> --content "..."
memorynexus-cli search --space <SPACE_ID> "..."
```
