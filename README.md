# MemoryNexus

MemoryNexus is a personal cognitive substrate where one memory universe can be
interpreted by many minds.

一个记忆宇宙，多种心智视角。

It is a Rust-first cognitive lens memory system, not an agent-owned memory
plugin. Memories belong to a Cognitive Space, and agents or users interpret that
space through Lens-style strategies.

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

Run the API with deterministic local embeddings:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

Run the CLI from another terminal:

```bash
cargo run --bin memorynexus-cli -- health
```

The API listens on `http://localhost:8080`.

## Try The Cognitive Lens MVP

Follow [docs/cli.md](docs/cli.md) for the full walkthrough. The shortest local
flow is:

```bash
cargo run --bin memorynexus-cli -- auth register --email you@example.com --name You --password secret123
export MEMORYNEXUS_TOKEN=<token-from-auth-response>

cargo run --bin memorynexus-cli -- space create --name "Project Space"
export MEMORYNEXUS_SPACE_ID=<space-id-from-space-create>

cargo run --bin memorynexus-cli -- memory add --space "$MEMORYNEXUS_SPACE_ID" --content "MemoryNexus is a Rust-first cognitive lens memory system."
cargo run --bin memorynexus-cli -- lens create --space "$MEMORYNEXUS_SPACE_ID" --name "Project Context" --strategy project_context
cargo run --bin memorynexus-cli -- lens run <lens-id-from-lens-create> --query "Summarize the project direction"
```

Lens Run returns a persisted, traceable interpretation result with the query,
Lens metadata, matched memory IDs, and summary provenance. Configure
`OPENAI_API_KEY`, or set `OPENROUTER_API_KEY` to auto-select OpenRouter, to use
AI-generated summaries; without credentials it falls back to a deterministic
local summary. Provider setup and `SPACE_ID` details are covered in
[docs/cli.md](docs/cli.md).

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
- [MCP Server](docs/mcp.md)
- [Lens Evaluation](docs/evaluation.md)
- [Development](docs/development.md)
- [Deployment](docs/deployment.md)
- [Roadmap](docs/TODO.md)
- [GitHub Issues](https://github.com/blackfaced/MemoryNexus/issues)
- [Cognitive Manifesto](docs/cognitive-manifesto.md)
- [Cognitive Concepts](docs/cognitive-concepts.md)
- [Cognitive Lens Roadmap](docs/cognitive-lens-roadmap.md)
- [Architecture Decisions](decisions/)

## License

MIT. See [LICENSE](LICENSE).
