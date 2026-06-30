# Development

## Prerequisites

- Rust stable
- Docker and Docker Compose

## Local Services

```bash
docker compose up -d postgres qdrant
```

PostgreSQL uses:

```text
postgresql://postgres:postgres@localhost:5432/memorynexus
```

Ignored acceptance tests should use a separate database:

```text
postgresql://postgres:postgres@localhost:5432/memorynexus_acceptance
```

Create or reset it with:

```bash
dropdb -h localhost -U postgres --if-exists memorynexus_acceptance
createdb -h localhost -U postgres memorynexus_acceptance
```

## Run The API

```bash
cargo run --bin memorynexus
```

The API listens on `http://localhost:8080`.

## Run The CLI

```bash
cargo run --bin memorynexus-cli -- health
cargo run --bin memorynexus-cli -- config
```

`config` is useful after restarting the API with provider keys; it shows the
embedding provider and Lens Run summary provider visible to the API process.

For semantic search and Lens Run smoke tests, start the API with:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local
```

Then follow the full [CLI walkthrough](cli.md#cognitive-lens-mvp-walkthrough)
to register, create a Cognitive Space, add memories, create a Lens, and run a
traceable Lens interpretation.

For provider setup issues, see [Lens Run Troubleshooting](cli.md#lens-run-troubleshooting).

## Run The MCP Server

`memorynexus-mcp` is the local stdio adapter for personal agents. Keep the API
running, set `MEMORYNEXUS_TOKEN`, then inspect the exposed tools:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
  | MEMORYNEXUS_API_URL=http://localhost:8080 \
    MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" \
    cargo run --quiet --bin memorynexus-mcp
```

For agent installation and MCP config snippets, see
[Agent Self-Install](agent-self-install.md).

## Verify

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D clippy::all
```

## PostgreSQL Surface Integration

Pull request CI has a stable `PostgreSQL Surface Integration` check for the
ignored Surface Gateway suites. It starts pinned PostgreSQL, dynamically
enumerates every `tests/surface_*_postgres_integration.rs` target, runs each
target with `--ignored`, and fails if the enumerated and executed manifests do
not match.

Run the same gate locally with a dedicated database:

```bash
docker compose up -d postgres

docker exec memorynexus_db psql -U postgres -d postgres -c \
  "DROP DATABASE IF EXISTS memorynexus_surface_ci"
docker exec memorynexus_db psql -U postgres -d postgres -c \
  "CREATE DATABASE memorynexus_surface_ci"

find tests -maxdepth 1 -type f -name 'surface_*_postgres_integration.rs' -print \
  | sort \
  | tee /tmp/memorynexus-surface-postgres-files.txt

test -s /tmp/memorynexus-surface-postgres-files.txt

sed -E 's#^tests/##; s#\.rs$##' /tmp/memorynexus-surface-postgres-files.txt \
  | tee /tmp/memorynexus-surface-postgres-enumerated-targets.txt

: > /tmp/memorynexus-surface-postgres-executed-targets.txt

while IFS= read -r target; do
  echo "$target" | tee -a /tmp/memorynexus-surface-postgres-executed-targets.txt
  DATABASE_URL=postgresql://postgres:postgres@localhost:5432/memorynexus_surface_ci \
    cargo test --locked --test "$target" -- --ignored --test-threads=1
done < /tmp/memorynexus-surface-postgres-enumerated-targets.txt

diff -u \
  /tmp/memorynexus-surface-postgres-enumerated-targets.txt \
  /tmp/memorynexus-surface-postgres-executed-targets.txt

docker exec memorynexus_db psql -U postgres -d postgres -c \
  "DROP DATABASE IF EXISTS memorynexus_surface_ci"
```

## Acceptance

The local acceptance test drives the real API through the CLI and requires local
PostgreSQL and Qdrant. It is ignored by default, so normal CI and `cargo test`
stay fast.

```bash
docker compose up -d postgres qdrant

MEMORYNEXUS_ACCEPTANCE=1 \
MEMORYNEXUS_ACCEPTANCE_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/memorynexus_acceptance \
QDRANT_URL=http://localhost:6333 \
MEMORYNEXUS_EMBEDDING_PROVIDER=local \
cargo test --test phase1c_acceptance -- --ignored --nocapture
```

The test starts the API, registers a user, creates a Cognitive Space, creates a
memory, verifies keyword search, verifies `search --semantic --space`, creates a
Lens, runs it, and fetches the persisted Lens Run through the CLI.

The acceptance test starts its own API process on a temporary localhost port and
passes `MEMORYNEXUS_API_URL` to the CLI commands it spawns. The API also supports
`MEMORYNEXUS_BIND_ADDR` for manual port selection. Port conflicts are still
possible if another process grabs the selected port between allocation and
server startup; rerun the test if that happens.

## OpenRouter Acceptance

The OpenRouter acceptance test verifies that Lens Run uses a real
OpenAI-compatible summary provider instead of deterministic fallback. It is also
ignored by default.

```bash
docker compose up -d postgres qdrant

MEMORYNEXUS_OPENROUTER_ACCEPTANCE=1 \
MEMORYNEXUS_ACCEPTANCE_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/memorynexus_acceptance \
OPENROUTER_API_KEY="$OPENROUTER_API_KEY" \
QDRANT_URL=http://localhost:6333 \
MEMORYNEXUS_EMBEDDING_PROVIDER=local \
cargo test --test openrouter_acceptance -- --ignored --nocapture
```

Expected provider provenance:

```json
{
  "summary_provider": "openrouter",
  "summary_source": "ai",
  "summary_fallback_reason": null
}
```

## Optional Service Acceptance In CI

Default push and pull request CI stays fast and runs only format, clippy, build,
and normal tests. The service-backed acceptance job is opt-in through GitHub
Actions:

1. Open the `CI` workflow.
2. Choose `Run workflow`.
3. Set `acceptance` to `true`.

That job starts PostgreSQL with the `memorynexus_acceptance` database and Qdrant
as services, then runs:

```bash
cargo test --test phase1c_acceptance -- --ignored --nocapture
```

If the repository has an `OPENROUTER_API_KEY` secret, the same job also runs the
OpenRouter acceptance test. Without that secret, provider-backed acceptance is
skipped explicitly.

## Structure

- `src/`: Rust API, CLI, MCP, domain, repositories, vector and AI modules
- `migrations/`: SQLx migrations
- `tests/`: integration tests
- `docs/`: project and architecture documentation
- `decisions/`: ADRs
