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

## Structure

- `src/`: Rust API, CLI, domain, repositories, vector and AI modules
- `migrations/`: SQLx migrations
- `tests/`: integration tests
- `docs/`: project and architecture documentation
- `decisions/`: ADRs
