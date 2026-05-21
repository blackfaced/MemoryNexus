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

## Verify

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D clippy::all
```

## Acceptance

The local acceptance test drives the real API through the CLI and requires local
PostgreSQL and Qdrant. It is ignored by default, so normal CI and `cargo test`
stay fast.

```bash
docker compose up -d postgres qdrant

MEMORYNEXUS_ACCEPTANCE=1 \
QDRANT_URL=http://localhost:6333 \
MEMORYNEXUS_EMBEDDING_PROVIDER=local \
cargo test --test phase1c_acceptance -- --ignored --nocapture
```

The test starts the API, registers a user, creates a Cognitive Space, creates a
memory, verifies keyword search, verifies `search --semantic --space`, creates a
Lens, runs it, and fetches the persisted Lens Run through the CLI.

## OpenRouter Acceptance

The OpenRouter acceptance test verifies that Lens Run uses a real
OpenAI-compatible summary provider instead of deterministic fallback. It is also
ignored by default.

```bash
docker compose up -d postgres qdrant

MEMORYNEXUS_OPENROUTER_ACCEPTANCE=1 \
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

## Structure

- `src/`: Rust API, CLI, domain, repositories, vector and AI modules
- `migrations/`: SQLx migrations
- `tests/`: integration tests
- `docs/`: project and architecture documentation
- `decisions/`: ADRs
