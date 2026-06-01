# MemoryNexus Roadmap

> Last updated: 2026-06-01
> Source of truth for executable tasks: GitHub Issues.

This file is now a roadmap summary. Do not maintain detailed task status in
Markdown. Create or update GitHub Issues instead, and keep long-term architecture
decisions in `decisions/` as ADRs.

## Tracking

- [All open issues](https://github.com/blackfaced/MemoryNexus/issues)
- [Phase 2 Cognitive Lens MVP](https://github.com/blackfaced/MemoryNexus/milestone/3)
- [Phase 3 Personal Cognitive Features](https://github.com/blackfaced/MemoryNexus/milestone/1)
- [Phase 4 User Interface](https://github.com/blackfaced/MemoryNexus/milestone/2)
- [Phase 5 Namespace Memory Lifecycle and Feedback Loops](https://github.com/blackfaced/MemoryNexus/milestone/4)

Recommended labels:

- `phase:2`, `phase:3`, `phase:4`, `phase:5`
- `area:lens`, `area:ai`, `area:cli`, `area:frontend`, `area:cognition`
- `priority:p0`, `priority:p1`, `priority:p2`
- `type:feature`, `type:docs`

## Completed Baseline

The current baseline is the Rust-first Cognitive Lens MVP foundation:

- Rust + Axum backend is the main backend path.
- Memory belongs to `CognitiveSpace`, not Agent.
- Space-scoped memory CRUD, keyword search, semantic search, and Qdrant indexing
  are implemented.
- `memorynexus-cli` supports auth, spaces, memories, search, lenses, Lens Run,
  Lens Run history, Lens templates, reminders, and runtime config inspection.
- Lens Run stores traceable output with matched memory IDs, summary provenance,
  structured key points, open questions, next actions, and citations.
- OpenAI-compatible summary providers are supported, including OpenRouter via
  `OPENROUTER_API_KEY`.
- Phase 2 Cognitive Lens MVP is complete and the GitHub milestone is closed.
- Phase 4 has a Rust-served Thought Review MVP at `/` and `/app`.
- ADR-014 extends the long-term direction toward Namespace, MemoryAtom,
  CognitiveScene, Lens-based CognitiveProjection, and FeedbackLoop as a feedback
  substrate, while keeping CognitiveSpace as the ownership boundary.
- ADR-015 records Supabase as a deployment compatibility path: first as managed
  PostgreSQL, with Auth / Storage / Realtime only as later adapters.

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

- [#35 Reminder notification delivery channels](https://github.com/blackfaced/MemoryNexus/issues/35)
- [#36 Advanced reminder recurrence and rule engine](https://github.com/blackfaced/MemoryNexus/issues/36)
- [#8 Voice capture with Whisper transcription](https://github.com/blackfaced/MemoryNexus/issues/8)

Recently completed:

- [#6 Family member and shared Cognitive Space model](https://github.com/blackfaced/MemoryNexus/issues/6):
  shared `CognitiveSpace` roles, invite codes, member listing, role updates, and
  role-gated memory writes.
- Personal agent integration guide and templates for Claw/Hermes-style MCP
  clients.
- [#41 Agent-ready MCP bootstrap tools](https://github.com/blackfaced/MemoryNexus/issues/41):
  MCP `create_space` / `create_lens`, default-space `get_profile`, and an
  agent self-install guide so another agent can connect itself.
- [#33 Persist personal agent profile and cognitive state projection](https://github.com/blackfaced/MemoryNexus/issues/33):
  profile snapshot API, persisted provenance, and MCP `get_profile`.
- [#34 Add personal agent write policy and cognitive router](https://github.com/blackfaced/MemoryNexus/issues/34):
  deterministic agent router API and MCP `route_agent_context`.
- [#7 Reminder and scheduled recall system](https://github.com/blackfaced/MemoryNexus/issues/7):
  Space-scoped reminder storage, API, CLI, and MCP tools for poll-based
  scheduled recall.
- [#20 Periodic cognitive review reports](https://github.com/blackfaced/MemoryNexus/issues/20):
  manual API/CLI review report generation over a Lens and time window with
  source memory citations and summary provenance.
- [#16 CLI commands for family spaces and reminders](https://github.com/blackfaced/MemoryNexus/issues/16):
  `family` CLI for shared Cognitive Space create/list/members/invite/accept/role
  plus `remind` alias for reminder commands.

## Phase 4: User Interface

Goal: build the first user-facing UI around Thought Review: capture one messy
thought, interpret it through multiple perspectives, save the review with
provenance, and summarize recurring weekly themes.

Current open work:

- [#54 Auth UI polish and session states](https://github.com/blackfaced/MemoryNexus/issues/54)
- [#55 Memory list filter and sort controls](https://github.com/blackfaced/MemoryNexus/issues/55)

Recently completed:

- [#24 Choose UI technology stack and scope](https://github.com/blackfaced/MemoryNexus/issues/24):
  Phase 4 starts with a Rust-served static Thought Review UI, documented in
  [ADR-013](../decisions/ADR-013-thought-review-ui-mvp.md).
- [#42 Thought Review MVP: first user-facing experience](https://github.com/blackfaced/MemoryNexus/issues/42):
  static first UI served by the Rust API at `/` and `/app`.
- [#45 First action: capture the thought occupying the user mind](https://github.com/blackfaced/MemoryNexus/issues/45):
  first screen asks for the thought currently taking up the most mental space.
- [#48 Instant multi-lens interpretation for a single thought](https://github.com/blackfaced/MemoryNexus/issues/48):
  Engineering, Detective, and Narrative perspectives run over one thought.
- [#43 Save a thought review with user-facing language and provenance](https://github.com/blackfaced/MemoryNexus/issues/43):
  thought review saves Memory and Lens Run provenance while using product language.
- [#47 Weekly cognitive review: recurring themes and inner tensions](https://github.com/blackfaced/MemoryNexus/issues/47):
  weekly reports expose recurring themes, inner tensions, forming direction, and
  next step.
- [#49 Reframe public positioning around AI thought review](https://github.com/blackfaced/MemoryNexus/issues/49):
  README now leads with AI thought organizer positioning.
- [#46 User-facing terminology map for Thought Review UI](https://github.com/blackfaced/MemoryNexus/issues/46):
  UI and docs separate user-facing language from backend model terms.
- [#25 Cognitive Space list and switch UI](https://github.com/blackfaced/MemoryNexus/issues/25):
  Thought Review lists accessible spaces, persists the active space, routes
  Memory/Lens/Lens Run/Search/Review work to it, and shows visible space errors.
- [#22 Lens Run result UI](https://github.com/blackfaced/MemoryNexus/issues/22):
  Thought Review can run a selected Lens and inspect traceable Lens Run output.
- [#23 Semantic search UI](https://github.com/blackfaced/MemoryNexus/issues/23):
  Thought Review exposes space-scoped keyword and semantic search with provider
  error handling.
- [#12 Memory detail and delete flow](https://github.com/blackfaced/MemoryNexus/issues/12):
  saved thoughts can be opened, edited with tags, and deleted from the static UI.
- [#10 Login and registration UI](https://github.com/blackfaced/MemoryNexus/issues/10):
  original MVP auth form and JWT persistence scope is covered; remaining polish
  continues in #54.
- [#11 Memory create, list, and detail UI](https://github.com/blackfaced/MemoryNexus/issues/11):
  broad MVP memory UI scope is covered; remaining filter/sort work continues in
  #55.

## Phase 5: Namespace Memory Lifecycle and Feedback Loops

Goal: extend the cognitive memory foundation into a namespace-based long-term
feedback substrate with an explicit memory lifecycle, while keeping Thought
Review as the first narrow product entry point.

Direction:

- `CognitiveSpace` remains the ownership and permission boundary.
- `Namespace` partitions a Space into long-running domains such as
  `personal.thoughts`, `learning.math`, `music.piano`, or `chess.tactics`.
- `FeedbackLoop` captures goal, task, attempt, evaluation, feedback, adjustment,
  and next task.
- `MemoryAtom` captures the smallest traceable cognitive signal extracted from a
  Memory.
- `CognitiveScene` consolidates related atoms, reflections, concepts, beliefs,
  and contradictions into a long-running theme or practice field.
- `CognitiveProjection` is the Lens-specific reconstructed context for the
  current query; it is not plain top-k retrieval.
- Observe modes split projection cost into `fast`, `focused`, and `deep`, so
  ordinary interaction stays low-latency while explicit review can go deep.
- Reflective namespaces focus on meaning, belief, contradiction, identity, and
  direction.
- Skill namespaces focus on practice, error pattern, progress, feedback, and
  next practice.
- EverMemOS is a useful reference for memory lifecycle ideas, but MemoryNexus
  keeps a different product boundary: user-owned cognitive perspective and
  feedback loops, not agent memory for reasoning.

Lifecycle:

```text
Experience / Thought / Practice
→ Memory
→ MemoryAtom
→ CognitiveScene
→ Lens-based CognitiveProjection
→ Reflection / Belief / Next Action
→ FeedbackLoop
```

Runtime policy:

```text
Every input -> fast response + optional async processing
Important input -> focused projection
Scheduled review / explicit request -> deep consolidation and projection
```

Recently completed:

- [#52 Define Namespace and FeedbackLoop foundation](https://github.com/blackfaced/MemoryNexus/issues/52):
  minimal Namespace and FeedbackLoop design documented in
  [Namespace and Feedback Loop Minimal Design](namespace-feedback-loop-design.md),
  with ADR-014 supplemented and implementation work split below.

Current open work:

- [#56 Minimal Namespace database model and API](https://github.com/blackfaced/MemoryNexus/issues/56)
- [#57 Minimal FeedbackLoop database model and API](https://github.com/blackfaced/MemoryNexus/issues/57)
- [#58 Namespace filters and FeedbackLoop provenance threading](https://github.com/blackfaced/MemoryNexus/issues/58)
- [#59 learning.math Skill Namespace MVP design](https://github.com/blackfaced/MemoryNexus/issues/59)
- [#60 Define MemoryAtom and CognitiveScene lifecycle](https://github.com/blackfaced/MemoryNexus/issues/60)
- [#61 Build MemoryNexus self-dataset atomization fixture](https://github.com/blackfaced/MemoryNexus/issues/61)
- [#62 Define Lens-based CognitiveProjection contract](https://github.com/blackfaced/MemoryNexus/issues/62)
- [#63 Prototype CognitiveScene consolidation](https://github.com/blackfaced/MemoryNexus/issues/63)
- [#65 Define dual-system observe modes](https://github.com/blackfaced/MemoryNexus/issues/65)

Candidate follow-up work after #56-#59:

- Add end-to-end acceptance tests for Space -> Namespace -> FeedbackLoop ->
  Memory -> Lens Run -> Review Report/Profile.
- Add prototype tests for Memory -> MemoryAtom -> CognitiveScene ->
  CognitiveProjection over a small MemoryNexus project-note fixture.
- Add latency and behavior acceptance criteria proving `fast` mode does not run
  the full deep cognitive pipeline.
- Keep product entry points narrow; do not expose every possible namespace in
  the first UI.

## Parking Lot

- [#17 Human-friendly output formats and shell completion](https://github.com/blackfaced/MemoryNexus/issues/17)

## Deployment Compatibility

Supabase is a supported future deployment target, but not a replacement backend
line. Follow [ADR-015](../decisions/ADR-015-supabase-integration.md):

1. Validate Supabase Postgres compatibility first: [#66](https://github.com/blackfaced/MemoryNexus/issues/66).
2. Keep Rust + Axum as the only main backend.
3. Keep `CognitiveSpace` membership and permissions in MemoryNexus.
4. Treat Supabase Auth, Storage, and Realtime as optional later adapters.

## Issue Hygiene

When adding future work:

1. Create a GitHub Issue with concrete acceptance criteria.
2. Add a milestone when the work belongs to a phase.
3. Add `phase:*`, `area:*`, `priority:*`, and `type:*` labels.
4. Add or update an ADR in `decisions/` for long-term architecture decisions.
5. Update this roadmap only when phase-level direction changes.
