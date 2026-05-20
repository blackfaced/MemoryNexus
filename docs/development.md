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
```

For semantic search smoke tests:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local
```

## Verify

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D clippy::all
```

## Acceptance

The Phase 1C acceptance test drives the real API through the CLI and requires
local PostgreSQL and Qdrant. It is ignored by default, so normal CI and
`cargo test` stay fast.

```bash
docker compose up -d postgres qdrant

MEMORYNEXUS_ACCEPTANCE=1 \
QDRANT_URL=http://localhost:6333 \
MEMORYNEXUS_EMBEDDING_PROVIDER=local \
cargo test --test phase1c_acceptance -- --ignored --nocapture
```

The test starts the API, registers a user, creates a Cognitive Space, creates a
memory, verifies keyword search, and verifies `search --semantic --space`.

## Structure

- `src/`: Rust API, CLI, domain, repositories, vector and AI modules
- `migrations/`: SQLx migrations
- `tests/`: integration tests
- `docs/`: project and architecture documentation
- `decisions/`: ADRs
