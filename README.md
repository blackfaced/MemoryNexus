# MemoryNexus

MemoryNexus is a Rust-first cognitive lens memory system. It is not an
agent-owned memory plugin. Memories belong to a Cognitive Space, and agents or
users interpret that space through Lens-style strategies.

## Current Shape

- Backend: Rust + Axum
- Database: PostgreSQL
- Vector search: Qdrant
- Object storage abstraction: S3/MinIO compatible
- CLI: `memorynexus-cli`
- Main crate: repository root

The old Python/FastAPI and empty frontend skeletons have been removed. New
backend work should land in the Rust crate.

## Quick Start

Start local infrastructure:

```bash
docker compose up -d postgres qdrant
```

Run the API:

```bash
cargo run --bin memorynexus
```

Run the CLI from another terminal:

```bash
cargo run --bin memorynexus-cli -- health
```

The API listens on `http://localhost:8080`.

## Local Semantic Search

Use the deterministic local embedding provider for smoke tests without an
external API key:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

Then follow [docs/cli.md](docs/cli.md) to register, create a space, add memory,
and run `search --semantic --space <SPACE_ID>`.

## Verification

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D clippy::all
```

## Documentation

- [Architecture](docs/architecture.md)
- [API](docs/api.md)
- [CLI](docs/cli.md)
- [Development](docs/development.md)
- [Deployment](docs/deployment.md)
- [Cognitive Manifesto](docs/cognitive-manifesto.md)
- [Cognitive Concepts](docs/cognitive-concepts.md)
- [Cognitive Lens Roadmap](docs/cognitive-lens-roadmap.md)
- [Architecture Decisions](decisions/)

## License

MIT. See [LICENSE](LICENSE).
