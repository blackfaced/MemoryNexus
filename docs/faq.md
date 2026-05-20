# FAQ

## Is MemoryNexus an agent memory plugin?

No. Memory belongs to `CognitiveSpace`. Agents can read, write, search, and run
interpretation strategies, but they do not own the memory.

## Where is the backend?

The Rust crate lives at the repository root. Run Cargo commands from the root.

## Is the old Python API still supported?

No. The historical Python/FastAPI skeleton has been removed. Rust + Axum is the
only backend path.

## How do I test semantic search locally?

Start Qdrant and use the local deterministic embedding provider:

```bash
docker compose up -d postgres qdrant
export QDRANT_URL=http://localhost:6333
export MEMORYNEXUS_EMBEDDING_PROVIDER=local
cargo run --bin memorynexus
```

Then use `memorynexus-cli search --semantic --space <SPACE_ID>`.
