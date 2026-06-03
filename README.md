# MemoryNexus

MemoryNexus is an AI thought organizer for saving messy ideas, reviewing them
through different perspectives, and noticing what you keep returning to.

一个属于你的 AI 思绪整理器。

Write down the thought currently taking up the most space in your mind.
MemoryNexus can review it through several perspectives, preserve the original
thought and interpretation trace, and later summarize recurring themes across
your private thinking space.

面向普通用户，它是一个本地优先的私人思考空间：记录想法、复盘困惑、
发现长期模式。

面向开发者，它仍然是一个 user-owned cognitive memory layer：one memory
universe can be interpreted by many minds.

It is a Rust-first cognitive lens memory system, not an agent-owned memory
plugin. Memories belong to a Cognitive Space, and agents or users interpret that
space through Lens-style strategies.

Longer term, MemoryNexus is evolving toward a namespace-based long-term feedback
substrate for personal cognition and skill acquisition. Its memory lifecycle is:
raw Memory -> MemoryAtom -> CognitiveScene -> Lens-based CognitiveProjection ->
Reflection / Belief / Next Action. Thought Review is the first reflective demo;
STEM Learning Feedback is the first product MVP candidate, using
`learning.stem` as the product namespace and elementary fraction word problems
as the first validation task. Future skill namespaces can track practice,
feedback, weak patterns, and next tasks for learning or craft domains.

MemoryNexus is also a long-term feedback engine: Thought Review demonstrates
multi-perspective memory, while STEM Learning Feedback turns practice,
feedback, and next exercise into the first product MVP.

That lifecycle is mode-aware: fast interactions should use recent context and
compressed priors, while explicit reviews can run deeper Lens projection and
consolidation.

## Try The Thought Review MVP

Start the API, then open `http://localhost:8080/` or `http://localhost:8080/app`.

The first UI flow is intentionally narrow:

1. Write one thought.
2. Review it through Engineering, Detective, and Narrative perspectives.
3. Save the thought review with traceable Lens Run provenance.
4. Generate a weekly review of recurring themes and inner tensions.

The UI uses user-facing language such as thought, perspective, insight,
recurring theme, and inner tension. The backend still preserves the precise
Memory, Lens, Lens Run, Cognitive Space, and provenance model.

## Current Shape

- Backend: Rust + Axum
- Database: PostgreSQL
- Vector search: Qdrant
- Object storage abstraction: S3/MinIO compatible
- CLI: `memorynexus-cli`
- UI: static Thought Review MVP served by the Rust API at `/`
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
- [Agent Self-Install](docs/agent-self-install.md)
- [Lens Evaluation](docs/evaluation.md)
- [Phase 2 Completion](docs/phase2-completion.md)
- [Development](docs/development.md)
- [Deployment](docs/deployment.md)
- [Roadmap](docs/TODO.md)
- [GitHub Issues](https://github.com/blackfaced/MemoryNexus/issues)
- [Cognitive Manifesto](docs/cognitive-manifesto.md)
- [Cognitive Concepts](docs/cognitive-concepts.md)
- [Cognitive Lens Roadmap](docs/cognitive-lens-roadmap.md)
- [STEM Learning Feedback MVP PRD](docs/stem-learning-mvp.md)
- [STEM Learning MCP Demo](docs/stem-mcp-demo.md)
- [Thought Review UI MVP ADR](decisions/ADR-013-thought-review-ui-mvp.md)
- [Namespace and Feedback Loop ADR](decisions/ADR-014-namespace-feedback-loop.md)
- [Architecture Decisions](decisions/)

## License

MIT. See [LICENSE](LICENSE).
