# MemoryNexus

MemoryNexus is a local-first long-term feedback engine for personal cognition
and skill acquisition.

See [ADR-022](decisions/ADR-022-memorynexus-brand-semantics.md) for the current
`MemoryNexus` brand semantics: the repository, binaries, MCP server, and release
identity keep the Engine name, while product experiences can use separate names
such as Dictation Coach.

它不是泛化 AI 记忆、个人知识库、agent recall infrastructure，也不和
Supermemory / Mem0 / OpenJarvis 直接竞争。MemoryNexus 的核心问题不是
“AI 如何保存或召回更多内容”，而是：

```text
How can a system use long-term traces to generate better feedback and next
actions over time?
```

中文理解：

```text
本地优先的长期反馈引擎，用 Trace 驱动复盘、成长模型和下一步行动。
```

MemoryNexus sits above raw memory/runtime layers:

- OpenJarvis: local personal AI runtime.
- Supermemory / Mem0: memory runtime, memory cloud, connectors, profile, and
  RAG infrastructure.
- MemoryNexus: memory evolution, feedback loops, growth models, sleep-based
  consolidation, and next-action generation.

Memory belongs to a user-owned `CognitiveSpace`. Agents, apps, CLIs, dashboards,
and voice assistants are only adapters. They access MemoryNexus through Surface
Gateway capabilities such as Capture, Performance, Reflection, Planning, and
Observation; they do not own memory or directly mutate Engine internals.

The first upstream product scenario is Dictation Coach: a daily dictation helper
for Chinese native-language dictation and English spelling / sentence
dictation. It validates the loop:

```text
Capture -> Performance -> Reflection -> Planning -> Observation -> SleepCycle
```

Under the [media evidence architecture contract](docs/media-evidence-contract.md),
Agents and Apps may perform OCR or speech-to-text before calling MemoryNexus.
The Engine works from user-confirmed text, and a future Surface request may
retain provider-neutral media evidence references for later inspection without
requiring media ingestion; reference persistence and resolution are not
implemented today.

Thought Review remains a reflective demo and presentation entry point. Existing
STEM practice work remains useful as a prior learning slice, but the next
product roadmap focuses on dictation because it gives the feedback loop a
clearer daily rhythm and easier mistake taxonomy.

## Try The Current Thought Review Demo

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
- UI: static Thought Review demo and learning practice slice served by the Rust API
- Surface direction: Capture, Performance, Reflection, Planning, Observation
- First upstream product direction: Dictation Coach
- Main crate: repository root

The old Python/FastAPI and empty frontend skeletons have been removed. New
backend work should land in the Rust crate.

## Install Profiles

For agents and non-developer users, the intended path is the binary-first
profiles in [Agent Self-Install](docs/agent-self-install.md):

- Trial Profile: use prebuilt `memorynexus-mcp`, once a release is available,
  with a hosted/demo API through `MEMORYNEXUS_API_URL` and
  `MEMORYNEXUS_TOKEN`. It does not require Rust, Docker, PostgreSQL, or Qdrant
  on the local machine.
- Local One-click Profile: use the release archive containing `memorynexus`,
  `memorynexus-cli`, and `memorynexus-mcp`, verify the checksum, then run local
  PostgreSQL and Qdrant through Docker.
- Production Profile: run release binaries against stable hosted or
  self-hosted PostgreSQL/Qdrant services. It is not Supabase-only.
- Developer Profile: use the source checkout and Cargo for contribution work.

Docker is only needed for the Local One-click Profile or local development when
you want MemoryNexus to run PostgreSQL and Qdrant on the same machine. Trial
Profile avoids Docker by connecting `memorynexus-mcp` to an existing API.
Production Profile avoids per-user Docker by using stable managed or
self-hosted PostgreSQL/Qdrant services.

## Developer Profile Quick Start

The source path below is the Developer Profile for contributors. It is not the
default agent install path and it requires a Rust toolchain. For hosted or
long-term use without local Docker-managed dependencies, see
[Production Profile](docs/production-profile.md).

For Local One-click runtime services without Rust, use
`docker-compose.runtime.yml` with `.env.runtime.example`; the release
`memorynexus` API binary then connects through the documented `DATABASE_URL`
and `QDRANT_URL` values. See
[Agent Self-Install](docs/agent-self-install.md#start-local-one-click-services).

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

## Release Binaries

Tagged GitHub releases are expected to publish prebuilt archives for:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu`

Each archive is named `memorynexus-<tag>-<target>.tar.gz` and contains a Local
One-click bundle layout:

- `bin/memorynexus`
- `bin/memorynexus-cli`
- `bin/memorynexus-mcp`
- `install.sh` for local binary install, Docker service checks, optional MCP
  config output, API health, and MCP `tools/list` smoke
- `README.local-one-click.md` with the one-archive Local One-click flow
- `SHA256SUMS` and `MANIFEST.json` for files inside the archive
- `docker-compose.runtime.yml` and `.env.runtime.example` for Local One-click
  PostgreSQL and Qdrant services

If no release is available yet, use Developer Profile source-build commands or
a maintainer-provided local binary. Download the archive and its matching
`.sha256` file from the
[GitHub Releases](https://github.com/blackfaced/MemoryNexus/releases) page,
verify the checksum, then unpack it:

```bash
sha256sum -c memorynexus-<tag>-<target>.tar.gz.sha256
tar -xzf memorynexus-<tag>-<target>.tar.gz
```

On macOS, use `shasum -a 256 -c memorynexus-<tag>-<target>.tar.gz.sha256`.
Then follow the extracted `README.local-one-click.md` for the one-archive local
flow.

Trial Profile uses only `bin/memorynexus-mcp` plus `MEMORYNEXUS_API_URL` /
`MEMORYNEXUS_TOKEN`; it does not start local PostgreSQL, Qdrant, Docker, or
Rust. Local One-click Profile runs PostgreSQL and Qdrant as external local
services, usually through Docker. Production Profile points the same binaries at
stable hosted or self-hosted services and is better for serious hosted use.
Source-build development remains available with `cargo run` and `cargo build`
only for Developer Profile.

The same release archive is shared by install profiles: Trial Profile can use
`bin/memorynexus-mcp` when an API is already available; Local One-click Profile
uses all three binaries with local PostgreSQL and Qdrant; Production Profile may
use the same binaries as service artifacts. The Developer Profile continues to
use the unchanged source-build workflow.

## Try The Cognitive Lens MVP

Follow [docs/cli.md](docs/cli.md) for the full walkthrough. The shortest local
flow is:

```bash
cargo run --bin memorynexus-cli -- auth register --email you@example.com --name You --password secret123
export MEMORYNEXUS_TOKEN=<token-from-auth-response>

cargo run --bin memorynexus-cli -- space create --name "Project Space"
export MEMORYNEXUS_SPACE_ID=<space-id-from-space-create>

cargo run --bin memorynexus-cli -- memory add --space "$MEMORYNEXUS_SPACE_ID" --content "MemoryNexus is a Rust-first long-term feedback engine."
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

- [Vision](docs/vision.md)
- [Architecture](docs/architecture/README.md)
- [MemoryNexus Engine](docs/architecture/memorynexus-engine.md)
- [Surfaces and Adapters](docs/architecture/surfaces-and-adapters.md)
- [Surface Gateway](docs/architecture/surface-gateway.md)
- [Sleep-driven Feedback Loop](docs/architecture/sleep-driven-feedback-loop.md)
- [Executable Issues](docs/issues.md)
- [Dictation Coach MVP](docs/dictation-coach-mvp.md)
- [API](docs/api.md)
- [CLI](docs/cli.md)
- [MCP Server](docs/mcp.md)
- [Agent Self-Install](docs/agent-self-install.md)
- [Lens Evaluation](docs/evaluation.md)
- [Phase 2 Completion](docs/phase2-completion.md)
- [Development](docs/development.md)
- [Deployment](docs/deployment.md)
- [Production Profile](docs/production-profile.md)
- [Supabase Postgres Compatibility](docs/supabase-postgres.md)
- [Roadmap](docs/TODO.md)
- [GitHub Issues](https://github.com/blackfaced/MemoryNexus/issues)
- [Cognitive Manifesto](docs/cognitive-manifesto.md)
- [Cognitive Concepts](docs/cognitive-concepts.md)
- [Cognitive Lens Roadmap](docs/cognitive-lens-roadmap.md)
- [STEM Learning Feedback MVP PRD](docs/stem-learning-mvp.md)
- [STEM Learning MCP Demo](docs/stem-mcp-demo.md)
- [Trace Contract](docs/trace-contract.md)
- [Sleep Cycle Contract](docs/sleep-cycle-contract.md)
- [Thought Review UI MVP ADR](decisions/ADR-013-thought-review-ui-mvp.md)
- [Namespace and Feedback Loop ADR](decisions/ADR-014-namespace-feedback-loop.md)
- [Local-first Trace Learning Runtime ADR](decisions/ADR-016-local-first-trace-learning-runtime.md)
- [Architecture Decisions](decisions/)

## License

MIT. See [LICENSE](LICENSE).
