# MemoryNexus Roadmap

> Last updated: 2026-05-25
> Source of truth for executable tasks: GitHub Issues.

This file is now a roadmap summary. Do not maintain detailed task status in
Markdown. Create or update GitHub Issues instead, and keep long-term architecture
decisions in `decisions/` as ADRs.

## Tracking

- [All open issues](https://github.com/blackfaced/MemoryNexus/issues)
- [Phase 2 Cognitive Lens MVP](https://github.com/blackfaced/MemoryNexus/milestone/3)
- [Phase 3 Personal Cognitive Features](https://github.com/blackfaced/MemoryNexus/milestone/1)
- [Phase 4 User Interface](https://github.com/blackfaced/MemoryNexus/milestone/2)

Recommended labels:

- `phase:2`, `phase:3`, `phase:4`
- `area:lens`, `area:ai`, `area:cli`, `area:frontend`
- `priority:p0`, `priority:p1`, `priority:p2`
- `type:feature`, `type:docs`

## Completed Baseline

The current baseline is the Rust-first Cognitive Lens MVP foundation:

- Rust + Axum backend is the main backend path.
- Memory belongs to `CognitiveSpace`, not Agent.
- Space-scoped memory CRUD, keyword search, semantic search, and Qdrant indexing
  are implemented.
- `memorynexus-cli` supports auth, spaces, memories, search, lenses, Lens Run,
  Lens Run history, Lens templates, and runtime config inspection.
- Lens Run stores traceable output with matched memory IDs, summary provenance,
  structured key points, open questions, next actions, and citations.
- OpenAI-compatible summary providers are supported, including OpenRouter via
  `OPENROUTER_API_KEY`.
- Phase 2 Cognitive Lens MVP is complete and the GitHub milestone is closed.

## Phase 2: Cognitive Lens MVP

Goal: make Lens a reliable runnable interpretation strategy over a Cognitive
Space.

Status:

Completed. The GitHub milestone has 0 open issues. The implemented MVP includes
Lens CRUD, Lens templates, Lens Run execution/history, lens-scoped search and
summaries, deterministic/provider-backed summaries, smart tag suggestions, local
evaluation fixtures, MCP access, and optional service-backed acceptance CI.

## Phase 3: Personal Cognitive Features

Goal: extend MemoryNexus from project memory to personal and family cognitive
workflows while keeping Cognitive Space as the ownership boundary.

Current open work:

- [#6 Family member and shared Cognitive Space model](https://github.com/blackfaced/MemoryNexus/issues/6)
- [#7 Reminder and scheduled recall system](https://github.com/blackfaced/MemoryNexus/issues/7)
- [#8 Voice capture with Whisper transcription](https://github.com/blackfaced/MemoryNexus/issues/8)
- [#20 Periodic cognitive review reports](https://github.com/blackfaced/MemoryNexus/issues/20)
- [#16 CLI commands for family spaces and reminders](https://github.com/blackfaced/MemoryNexus/issues/16)

## Phase 4: User Interface

Goal: choose and build the first UI around Cognitive Space, Memory, Search, and
Lens Run workflows.

Current open work:

- [#24 Choose UI technology stack and scope](https://github.com/blackfaced/MemoryNexus/issues/24)
- [#10 Login and registration UI](https://github.com/blackfaced/MemoryNexus/issues/10)
- [#25 Cognitive Space list and switch UI](https://github.com/blackfaced/MemoryNexus/issues/25)
- [#11 Memory create, list, and detail UI](https://github.com/blackfaced/MemoryNexus/issues/11)
- [#12 Memory detail and delete flow](https://github.com/blackfaced/MemoryNexus/issues/12)
- [#23 Semantic search UI](https://github.com/blackfaced/MemoryNexus/issues/23)
- [#22 Lens Run result UI](https://github.com/blackfaced/MemoryNexus/issues/22)

## Parking Lot

- [#17 Human-friendly output formats and shell completion](https://github.com/blackfaced/MemoryNexus/issues/17)

## Issue Hygiene

When adding future work:

1. Create a GitHub Issue with concrete acceptance criteria.
2. Add a milestone when the work belongs to a phase.
3. Add `phase:*`, `area:*`, `priority:*`, and `type:*` labels.
4. Add or update an ADR in `decisions/` for long-term architecture decisions.
5. Update this roadmap only when phase-level direction changes.
