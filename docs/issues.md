# MemoryNexus Executable Issues

This file is the planning mirror for the current milestone set. GitHub Issues
are the executable source of truth; keep this file aligned when milestone scope
or acceptance criteria change.

Each issue assumes:

- Rust-first backend.
- `CognitiveSpace` remains the ownership and permission boundary.
- `Namespace` is a domain partition, not a permission model.
- Adapters access Engine capabilities through Surface Gateway.
- No LLM, OCR, complex UI, or broad education platform unless the issue says so.

## Milestone 1: Architecture Refresh

### Issue 1.1: Refresh README Positioning

**Background:** README still presented MemoryNexus partly as an AI thought
organizer and cognitive lens memory app. The new positioning is long-term
feedback engine.

**Scope:**

- Update the first viewport positioning.
- Explain OpenJarvis / Supermemory / MemoryNexus ecosystem boundaries.
- Introduce Dictation Coach as first upstream product direction.
- Link the new architecture docs and issues plan.

**Non-Goals:**

- Do not remove Thought Review docs.
- Do not change business code.

**Acceptance Criteria:**

- README states: "local-first, namespace-based long-term feedback engine".
- README does not lead with second brain, memory app, or agent memory store.
- Documentation links point to `docs/architecture/README.md` and `docs/issues.md`.

**Possible Files:**

- `README.md`

### Issue 1.2: Add Vision Document

**Background:** The repo needs a single product/architecture vision that explains
what MemoryNexus is and is not.

**Scope:**

- Add `docs/vision.md`.
- Record current understanding and current gap.
- Explain MemoryNexus as Memory Evolution / Feedback / Growth Engine.

**Non-Goals:**

- Do not define schema.
- Do not create product UI specs.

**Acceptance Criteria:**

- Vision explains the core question: long-term traces -> better feedback and next action.
- Vision distinguishes MemoryNexus from OpenJarvis and Supermemory/Mem0.
- Vision names Dictation Coach as first upstream product.

**Possible Files:**

- `docs/vision.md`

### Issue 1.3: Add Engine / Surface / Gateway Architecture Docs

**Background:** Current docs mix object model, APIs, and product entry points.
Architecture needs explicit Engine, Surfaces, Adapters, and Surface Gateway.

**Scope:**

- Add Engine architecture doc.
- Add Surfaces and Adapters doc.
- Add Surface Gateway doc.
- Add Sleep-driven feedback loop doc.
- Move existing architecture overview to `docs/architecture/README.md`.

**Non-Goals:**

- Do not implement Surface Gateway.
- Do not change routes.

**Acceptance Criteria:**

- Docs define Adapter, Surface, Engine, and Surface Gateway.
- Docs list Capture, Performance, Reflection, Planning, Observation.
- Docs specify sync Surface Calls vs async Engine Events.

**Possible Files:**

- `docs/architecture/README.md`
- `docs/architecture/memorynexus-engine.md`
- `docs/architecture/surfaces-and-adapters.md`
- `docs/architecture/surface-gateway.md`
- `docs/architecture/sleep-driven-feedback-loop.md`

### Issue 1.4: Add Feedback Engine ADRs

**Background:** Major direction changes must be recorded as ADRs.

**Scope:**

- Add ADR for MemoryNexus as Long-term Feedback Engine.
- Add ADR for Surfaces vs Adapters vs Engine.
- Add ADR for Dictation Coach as First Upstream Product.
- Keep existing ADR-017 as the Sleep-based Consolidation decision.

**Non-Goals:**

- Do not duplicate ADR-017.
- Do not alter implementation code.

**Acceptance Criteria:**

- New ADRs are accepted.
- `decisions/README.md` indexes all new ADRs.
- ADRs clearly preserve `CognitiveSpace` ownership and Rust-first backend.

**Possible Files:**

- `decisions/ADR-018-long-term-feedback-engine.md`
- `decisions/ADR-019-surfaces-adapters-engine.md`
- `decisions/ADR-020-dictation-coach-first-upstream-product.md`
- `decisions/README.md`

### Issue 1.5: Rewrite Roadmap And Issue Plan

**Background:** Roadmap should use milestones for the new Engine / Surface /
Dictation direction.

**Scope:**

- Rewrite `docs/TODO.md` around Milestones 1-7.
- Add `docs/issues.md` as executable issue specs.
- Map old open issues to the new milestone structure.

**Non-Goals:**

- Do not create GitHub issues from this file in the same issue unless requested.
- Do not close existing issues automatically.

**Acceptance Criteria:**

- Roadmap has Milestones 1-7.
- `docs/issues.md` contains executable issue templates.
- Existing open issues are mapped or marked for reconciliation.

**Possible Files:**

- `docs/TODO.md`
- `docs/issues.md`

### Issue 1.6: Align Repository Metadata With MemoryNexus Brand Semantics

**GitHub:** #197

**Background:** ADR-022 keeps the project name `MemoryNexus` but redefines the
brand semantics around long-term feedback, trace-driven learning, growth
models, consolidation, and next actions. Public repository metadata and
first-touch docs should not keep the older family-memory / second-brain
interpretation alive.

**Scope:**

- Update the GitHub repository description to the ADR-022 wording.
- Review first-touch metadata and copy: README first viewport, GitHub About
  fields, release notes template, install-profile docs, and docs that introduce
  the project.
- Ensure product-facing names such as Dictation Coach stay separate from the
  MemoryNexus Engine identity.
- Link ADR-022 from docs that explain the project name or positioning.

**Non-Goals:**

- Do not rename the repository, crate, binaries, MCP server, release artifacts,
  or package paths.
- Do not replace MemoryNexus with Dictation Coach as the project name.
- Do not introduce a new frontend or product surface.

**Acceptance Criteria:**

- GitHub repository description no longer says family photo/video memory
  manager, second brain, or generic AI memory app.
- First-touch copy uses: "Local-first long-term feedback engine for personal
  cognition and skill acquisition."
- Chinese description uses: "本地优先的长期反馈引擎，用 Trace
  驱动复盘、成长模型和下一步行动。"
- Docs preserve the naming split: MemoryNexus is the Engine/repo identity;
  Dictation Coach is the first upstream product scenario.
- ADR-005 points readers to ADR-022 for current brand semantics.

**Possible Files:**

- `decisions/ADR-022-memorynexus-brand-semantics.md`
- `decisions/ADR-005-project-naming.md`
- `decisions/README.md`
- `README.md`
- `docs/vision.md`
- `.github/workflows/release.yml`
- `docs/agent-self-install.md`

## Milestone 2: Core Domain Model

### Issue 2.1: Align Namespace Domain With Engine Vocabulary

**Background:** Namespace exists, but docs and APIs should align with Surface
Gateway and Engine vocabulary.

**Scope:**

- Review current Namespace model.
- Ensure namespace names support Dictation examples.
- Document namespace validation and same-Space rules.

**Non-Goals:**

- Do not add role or permission semantics to Namespace.
- Do not add UI.

**Acceptance Criteria:**

- Namespace supports names like `child.chinese.dictation`.
- Same-Space validation is documented and tested if code changes.
- API copy does not treat Namespace as ownership.

**Possible Files:**

- `src/domain/namespace.rs`
- `src/db/namespaces.rs`
- `docs/architecture/memorynexus-engine.md`
- `docs/api.md`

### Issue 2.2: Define Trace Domain Types And Repository

**Background:** Trace is the evidence layer for feedback effectiveness and
runtime metrics.

**Scope:**

- Define Trace domain enums and structs.
- Add PostgreSQL schema and repository.
- Support source type, task type, mode, runtime, status, summaries, metrics, and generated object links.
- Add same-Space tests.

**Non-Goals:**

- Do not capture traces from all surfaces yet.
- Do not build analytics UI.

**Acceptance Criteria:**

- Trace can be serialized, stored, listed by Space, and filtered by namespace.
- Cross-Space linked objects are rejected.
- Tests pass without external AI credentials.

**Possible Files:**

- `migrations/*trace*.sql`
- `src/domain/trace.rs`
- `src/db/traces.rs`
- `tests/*trace*`
- `docs/trace-contract.md`

### Issue 2.3: Define MemoryAtom Domain Draft

**Background:** MemoryAtom turns raw evidence into small traceable signals.

**Scope:**

- Define MemoryAtom fields and provenance.
- Keep first implementation deterministic or fixture-only.
- Add serialization tests.

**Non-Goals:**

- Do not call LLMs.
- Do not add production extraction pipeline.

**Acceptance Criteria:**

- MemoryAtom includes source Trace / Memory IDs.
- Tests show source provenance is preserved.
- Docs clarify MemoryAtom is not a neural engram.

**Possible Files:**

- `src/domain/memory_atom.rs`
- `docs/cognitive-concepts.md`
- `tests/*memory_atom*`

### Issue 2.4: Define CognitiveScene Domain Draft

**Background:** CognitiveScene groups atoms into a long-running theme, problem
field, or practice scene.

**Scope:**

- Define CognitiveScene fields.
- Include source atom IDs, summary, active patterns, and namespace.
- Add serialization tests.

**Non-Goals:**

- Do not implement automated consolidation.
- Do not expose CognitiveScene as user UI label.

**Acceptance Criteria:**

- CognitiveScene can cite MemoryAtom IDs.
- Scene belongs to a `CognitiveSpace` and optional namespace.
- Tests cover serialization and same-Space validation if persisted.

**Possible Files:**

- `src/domain/cognitive_scene.rs`
- `docs/cognitive-concepts.md`
- `tests/*cognitive_scene*`

### Issue 2.5: Define GrowthModel Domain Draft

**Background:** GrowthModel is the long-term namespace-specific growth picture.

**Scope:**

- Define strengths, weaknesses, recurring patterns, current stage, recommended focus, evidence IDs.
- Keep model namespace-scoped and Space-owned.
- Add serialization tests.

**Non-Goals:**

- Do not build dashboard UI.
- Do not claim causal improvement.

**Acceptance Criteria:**

- GrowthModel has evidence-backed fields.
- GrowthModel is not a generic user profile.
- Tests cover JSON round-trip.

**Possible Files:**

- `src/domain/growth_model.rs`
- `docs/architecture/memorynexus-engine.md`
- `tests/*growth_model*`

### Issue 2.6: Define SleepCycle Domain And Persistence

**Background:** SleepCycle contract exists in docs; implementation needs domain
and schema.

**Scope:**

- Implement SleepCycle domain types.
- Add schema/repository for cycle type, status, evidence window, input/output links.
- Add tests for lifecycle status and same-Space validation.

**Non-Goals:**

- Do not add scheduler.
- Do not run consolidation logic yet.

**Acceptance Criteria:**

- SleepCycle can be created and marked completed/failed.
- Evidence window is stored.
- Linked objects remain same-Space.

**Possible Files:**

- `src/domain/sleep_cycle.rs`
- `src/db/sleep_cycles.rs`
- `migrations/*sleep_cycle*.sql`
- `docs/sleep-cycle-contract.md`

### Issue 2.7: Define PracticePlan / DreamCandidate Domain

**Background:** DreamCandidate is internal; PracticePlan is product-facing next
plan.

**Scope:**

- Define PracticePlan and DreamCandidate relationship.
- Support purpose, content, expected effect, status, selected/executed/evaluated states.
- Add serialization tests.

**Non-Goals:**

- Do not generate plans yet.
- Do not call cloud models.

**Acceptance Criteria:**

- PracticePlan can reference source DreamCandidate or ConsolidationResult.
- Candidate effectiveness can be recorded later.
- Tests cover status transitions.

**Possible Files:**

- `src/domain/practice_plan.rs`
- `src/domain/dream_candidate.rs`
- `docs/sleep-cycle-contract.md`

### Issue 2.8: Define Minimal Lens / Reflection Surface Structures

**Background:** Lens remains an interpretation strategy, not an agent persona.

**Scope:**

- Define minimal structures needed by Reflection Surface.
- Ensure Lens metadata does not imply role-play identity.
- Add tests for serialization or conversion from existing Lens.

**Non-Goals:**

- Do not rewrite all Lens Run behavior.
- Do not introduce new prompt framework.

**Acceptance Criteria:**

- Reflection Surface can reference Lens strategy.
- Lens docs explicitly say it is not an agent persona.
- Existing Lens Run behavior remains compatible.

**Possible Files:**

- `src/domain/lens.rs`
- `src/domain/reflection.rs`
- `docs/cognitive-concepts.md`

## Milestone 3: Surface Gateway MVP

### Issue 3.1: Define SurfaceRequest And SurfaceResponse

**Background:** External adapters need one gateway contract before implementation.

**Scope:**

- Define request/response domain structs.
- Include namespace, surface, action, actor, adapter, payload, and context.
- Include generated trace ID, follow-up suggestions, and visibility in response.

**Non-Goals:**

- Do not implement all surfaces.
- Do not expose raw Engine objects by default.

**Acceptance Criteria:**

- Types serialize/deserialize.
- Tests cover invalid surface/action combinations.
- Docs match `docs/architecture/surface-gateway.md`.

**Possible Files:**

- `src/domain/surface.rs`
- `docs/architecture/surface-gateway.md`
- `tests/*surface*`

### Issue 3.2: Implement Capture Surface Minimum Path

**Background:** Capture answers "what happened?" and should write Trace.

**Scope:**

- Add capture action through Surface Gateway.
- Store captured content as Memory or appropriate existing object.
- Write Trace.
- Publish `ObservationCaptured` if event model exists; otherwise document pending event.

**Non-Goals:**

- Do not do atomization synchronously.
- Do not require AI provider.

**Acceptance Criteria:**

- Capture request returns SurfaceResponse with generated trace ID.
- Captured object remains Space-owned.
- Tests cover auth/namespace validation.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/surface.rs`
- `src/db/traces.rs`
- `tests/*surface_capture*`

### Issue 3.3: Implement Performance Surface Minimum Path

**Background:** Performance answers "how did the attempt go?".

**Scope:**

- Add submitAttempt action.
- Create/update FeedbackLoop or practice attempt.
- Write Trace.
- Return immediate deterministic response.

**Non-Goals:**

- Do not implement full Dictation Coach yet.
- Do not run SleepCycle synchronously.

**Acceptance Criteria:**

- Attempt submission creates Trace and links FeedbackLoop evidence.
- Fast path does not run deep consolidation.
- Tests cover same-Space and namespace validation.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/feedback_loop.rs`
- `src/domain/trace.rs`
- `tests/*surface_performance*`

### Issue 3.4: Implement Reflection Surface Mock

**Background:** Reflection answers "what does this mean?" but first version can be deterministic.

**Status:** Closed on GitHub as #146 on 2026-06-23. This status does not
complete Issue 2.8 / GitHub #142.

**Unlocks:** Issue 3.5 / GitHub #147.

**Dispatcher Ownership:** This issue starts the serialized dispatcher sequence
from the completed Capture/Performance baseline. Reflection-domain workers own
`src/domain/reflection.rs`; only the integration owner edits
`src/api/surfaces.rs`.

**Scope:**

- Add reflect/review/explain mock actions.
- Return structured placeholder insight using existing evidence.
- Write Trace.

**Non-Goals:**

- Do not call LLM.
- Do not run multi-lens deep projection.

**Acceptance Criteria:**

- Reflection Surface returns stable deterministic result.
- Trace task type reflects review/reflection.
- Tests pass without external credentials.
- PostgreSQL integration tests verify Reflection routing, Trace provenance, and
  same-Space behavior through the shared dispatcher.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/reflection.rs`
- `tests/*surface_reflection*`

### Issue 3.5: Implement Planning Surface Mock

**Background:** Planning answers "what should happen next?".

**Status:** Closed on GitHub as #147 on 2026-06-23.
Follow-up #180 added the generic `planning/adjust_plan` Surface action.

**Depends On:** Issue 3.4 / GitHub #146.

**Unlocks:** Issue 3.6 / GitHub #148.

**Dispatcher Ownership:** Take `src/api/surfaces.rs` integration ownership only
after Issue 3.4 / GitHub #146 lands. Planning-domain workers own
`src/domain/practice_plan.rs` and must not concurrently edit the shared
dispatcher.

**Scope:**

- Add `generate_next_task`; #180 later adds `adjust_plan` for adjusting an
  adapter-proposed plan from evidence and constraints.
- Return deterministic next action.
- Write Trace.

**Non-Goals:**

- Do not generate adaptive curriculum.
- Do not call cloud model.

**Acceptance Criteria:**

- Planning Surface can return a next task for a namespace.
- Trace links generated PracticePlan when that model exists, or stores output summary.
- PostgreSQL integration tests verify Planning routing, Trace provenance, and
  same-Space behavior through the shared dispatcher after Issue 3.4 / GitHub
  #146 lands.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/practice_plan.rs`
- `tests/*surface_planning*`

### Issue 3.6: Implement Observation Surface Mock

**Background:** Observation answers "how is long-term state changing?".

**Status:** Closed on GitHub as #148 on 2026-06-23.

**Depends On:** Issue 3.5 / GitHub #147.

**Unlocks:** The typed/pasted path in Issue 5.2 / GitHub #155, the text-capable
generic tools in Issue 6.2 / GitHub #162, and Foundation F1 / GitHub #175 for
media validation.

**Dispatcher Ownership:** Take `src/api/surfaces.rs` integration ownership only
after Issue 3.5 / GitHub #147 lands. Growth-model workers own
`src/domain/growth_model.rs` and must not concurrently edit the shared
dispatcher.

**Scope:**

- Add observeState/getGrowthModel/getTrends/getTimeline mock actions.
- Return deterministic summary from available objects.
- Write Trace when appropriate.

**Non-Goals:**

- Do not build dashboard.
- Do not implement analytics pipeline.

**Acceptance Criteria:**

- Observation Surface can return namespace state.
- Response is adapter-shaped, not raw DB rows.
- PostgreSQL integration tests verify Observation routing, Trace provenance,
  and same-Space behavior through the shared dispatcher after Issue 3.5 /
  GitHub #147 lands.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/growth_model.rs`
- `tests/*surface_observation*`

### Issue 3.CI: Require PostgreSQL Surface Integration CI

**GitHub:** #177

**Status:** Closed on GitHub on 2026-06-30. This is now the completed
PostgreSQL Surface integration CI foundation for the Surface Gateway MVP.

**Background:** Surface Gateway work depends on shared dispatcher behavior and
PostgreSQL-backed integration tests. Unit CI alone was not enough evidence
before additional shared-dispatcher Surface work could stack on the same
contract.

**Scope:**

- Add a stable-name pull-request CI job for PostgreSQL-backed Surface
  integration tests.
- Pin the PostgreSQL service to an exact patch tag or digest and use
  deterministic database isolation.
- Dynamically enumerate `tests/surface_*_postgres_integration.rs`, convert file
  stems to cargo `--test` names, and run every target with `--ignored` and
  serial test threads where required.
- Print both the enumerated and executed integration-test manifests and require
  exact set equality.
- Fail when the enumerated set is empty, any enumerated test is not executed,
  or any target fails.
- Trigger for Rust, tests, migrations, Cargo manifests/lockfile, and CI workflow
  changes. Docs-only PRs may skip DB execution through path-aware logic, but the
  stable check context must remain present and successful.
- Use `cargo --locked` where applicable and cache dependencies without sharing
  mutable database state.
- Document copy-pasteable local equivalent commands in `docs/development.md`.
- Record branch-protection evidence after stable runs make the check safe to
  require.

**Non-Goals:**

- Do not add credentialed provider tests to the required merge gate.
- Do not make Qdrant `latest` part of deterministic CI.
- Do not change Surface/Adapter/Engine behavior or test semantics in the
  CI-only issue.
- Do not modify branch protection from the Worker implementation PR.

**Acceptance Criteria:**

- A PR run dynamically enumerates and executes all existing ignored PostgreSQL
  Surface test targets rather than reporting them ignored.
- Enumerated and executed target manifests are printed and exactly equal; the
  job fails on unexpected empty enumeration, missing execution, or any target
  failure.
- A deliberately failing temporary fixture/assertion blocks a PR run; removing
  it produces a passing rerun URL.
- The stable context can be required after evidence is recorded by the
  Coordinator or administrator.
- `docs/development.md` contains copy-pasteable local commands.
- No `qdrant:latest`, floating PostgreSQL major-only image, external provider
  credential, or network provider call is in the required job.
- Existing Format, Clippy, Build, and Test checks remain.
- Runtime target stays practical; if it exceeds 10 minutes, document timing and
  split deterministic suites without weakening coverage.

**Possible Files:**

- `.github/workflows/*`
- `tests/surface_*_postgres_integration.rs`

## Milestone 4: Event + Sleep Engine MVP

### Issue 4.1: Define Engine Event Model

**Background:** Surface calls need to trigger background evolution without
blocking foreground responses.

**Scope:**

- Define EngineEvent enum.
- Include ObservationCaptured, AttemptSubmitted, FeedbackGenerated,
  SleepCycleRequested, GrowthModelUpdated, PlanGenerated.
- Add serialization tests.

**Non-Goals:**

- Do not add distributed queue.
- Do not add scheduler.

**Acceptance Criteria:**

- Events carry Space, namespace, source trace, and payload references.
- Tests prove event round-trip.

**Possible Files:**

- `src/domain/event.rs`
- `tests/*event*`

### Issue 4.2: Publish ObservationCaptured And AttemptSubmitted

**Background:** Capture and Performance Surface calls should emit events.

**GitHub:** #150

**Status:** Closed on GitHub on 2026-07-01.

**Scheduling:** Issue 4.2 / GitHub #150 proceeded in parallel with the
Dictation critical path. It did not block the initial manual Agent smoke in
Issue 5.7 / GitHub #160.

**Scope:**

- Publish stored or in-process events after successful Surface calls.
- Link events to Trace IDs.
- Add tests.

**Non-Goals:**

- Do not process events asynchronously yet.

**Acceptance Criteria:**

- Capture emits ObservationCaptured.
- submitAttempt emits AttemptSubmitted.
- Failed Surface calls do not emit success events.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/event.rs`
- `src/db/events.rs`
- `tests/*surface_events*`

### Issue 4.3: Add Manual SleepCycle Trigger

**Background:** Manual SleepCycle should exist before any scheduler.

**Scope:**

- Add manual trigger through API/CLI/MCP or Surface Gateway action.
- Create SleepCycle record.
- Select evidence window.
- Write Trace.

**Non-Goals:**

- Do not add cron.
- Do not call cloud AI.

**Acceptance Criteria:**

- Manual trigger creates completed or failed SleepCycle.
- Cross-Space access is rejected.
- Tests cover empty evidence window.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/bin/memorynexus-cli.rs`
- `src/bin/memorynexus-mcp.rs`
- `src/db/sleep_cycles.rs`
- `tests/*sleep_cycle*`

### Issue 4.4: Aggregate Trace Into GrowthModel Update

**Background:** SleepCycle should produce a simple GrowthModel update.

**Depends On:** For the selected Dictation critical path, Issue 5.4 / GitHub
#157 provides the deterministic mistake evidence that this issue
aggregates. Generic deterministic aggregation behavior remains reusable outside
Dictation.

**Unlocks:** Issue 4.5 / GitHub #153.

**Scope:**

- Read recent Trace / FeedbackLoop evidence, including the deterministic
  Dictation classifications produced by Issue 5.4 / GitHub #157.
- Detect repeated patterns and evidence gaps deterministically.
- Update or produce GrowthModel summary.

**Non-Goals:**

- Do not implement advanced ML.
- Do not claim causality.

**Acceptance Criteria:**

- Fixture with repeated mistakes yields a recurring pattern.
- Fixture with insufficient evidence records evidence gap.
- Tests require no external AI.

**Possible Files:**

- `src/domain/growth_model.rs`
- `src/domain/sleep_cycle.rs`
- `tests/fixtures/*`
- `tests/*sleep_growth*`

### Issue 4.5: Generate Simple PracticePlan

**Background:** Planning should use GrowthModel evidence to suggest a next step.

**Depends On:** Issue 4.4 / GitHub #152.

**Unlocks:** Issue 5.5 / GitHub #158.

**Scope:**

- Convert simple GrowthModel update into PracticePlan.
- Link plan to evidence.
- Store Trace of plan generation.

**Non-Goals:**

- Do not generate long curriculum.
- Do not call cloud model.

**Acceptance Criteria:**

- PracticePlan includes target pattern, task text, expected effect, and evidence IDs.
- Later evaluation can reference the plan.

**Possible Files:**

- `src/domain/practice_plan.rs`
- `src/db/practice_plans.rs`
- `tests/*practice_plan*`

## Milestone 5: Dictation Coach Demo

Execution dependency graph:

| Depends On | Issue Unlocked |
| --- | --- |
| Issue 3.4 / GitHub #146 review, PostgreSQL verification, and merge | Issue 3.5 / GitHub #147 |
| Issue 3.5 / GitHub #147 | Issue 3.6 / GitHub #148 |
| Issue 3.6 / GitHub #148 | Issue 5.2 / GitHub #155 typed/pasted path |
| Issue 5.2 / GitHub #155 typed/pasted path | Issue 5.3 / GitHub #156 typed/pasted path |
| Issue 5.3 / GitHub #156 typed/pasted path | Issue 5.4 / GitHub #157 |
| Issue 3.6 / GitHub #148 | Issue 6.2 / GitHub #162 text-capable generic Surface tools |
| Issue 3.6 / GitHub #148 | Foundation F1 / GitHub #175 |
| Foundation F1 / GitHub #175 | Media extensions in Issue 5.2 / GitHub #155, Issue 5.3 / GitHub #156, and Issue 6.2 / GitHub #162 |
| Issue 5.4 / GitHub #157 | Issue 4.4 / GitHub #152 |
| Issue 4.4 / GitHub #152 | Issue 4.5 / GitHub #153 |
| Issue 4.5 / GitHub #153 | Issue 5.5 / GitHub #158 |
| Issues 5.2-5.5 / GitHub #155-#158 + text-capable Issue 6.2 / GitHub #162 | Initial Issue 5.7 / GitHub #160 Agent smoke |
| Issue 4.4 / GitHub #152 + Issue 5.3 / GitHub #156 | Issue 5.6 / GitHub #159 read-only Observation projection |
| Issue 5.6 / GitHub #159 deterministic multi-day fixtures | Extended seven-day Issue 5.7 / GitHub #160 acceptance |
| Initial Issue 5.7 / GitHub #160 acceptance | Issue 6.3 / GitHub #163 |

The graph is acyclic. The serialized shared-dispatcher chain is Issue 3.4 /
GitHub #146 -> Issue 3.5 / GitHub #147 -> Issue 3.6 / GitHub #148. After that,
typed/pasted Dictation work and the text-capable generic MCP Surface tools can
proceed without Foundation F1 / GitHub #175. Foundation F1 / #175 opens only
the media-derived extensions. Every affected issue that edits
`src/api/surfaces.rs` takes dispatcher integration ownership only after its
listed predecessor lands. Adapter workers never own the shared dispatcher, and
domain workers do not concurrently integrate it.

### Issue 5.1: Define Dictation Coach Contract

**Background:** Dictation Coach is the first upstream product, but Engine should not embed roles.

**Scope:**

- Define namespaces, task shapes, attempt shapes, and mistake taxonomy.
- Use confirmed normalized text from manually entered or Agent-prepared sources;
  MemoryNexus still performs no OCR or ASR.
- Map actions to Surfaces.

**Non-Goals:**

- The MemoryNexus contract and Engine perform no OCR or ASR. Agent / App
  Adapters may prepare explicitly user-confirmed normalized text.
- No multi-child management.

**Acceptance Criteria:**

- Contract covers Chinese dictation and English spelling / sentence dictation.
- Contract maps to Capture, Performance, Reflection, Planning, and Observation.

**Possible Files:**

- `docs/dictation-coach-mvp.md`
- `docs/architecture/surfaces-and-adapters.md`

### Foundation F1: Validate External Media Evidence References

**Background:** Dictation Capture may preserve optional provenance to original
media, but ADR-021 and the media evidence contract currently define a docs-only
contract. Descriptor validation must land before Dictation Capture accepts
`EvidenceRefInput`.

**Status:** Closed on GitHub as #175 on 2026-06-23.

**Depends On:** Issue 3.6 / GitHub #148.

**Unlocks:** The media-derived extensions in Issue 5.2 / GitHub #155, Issue 5.3
/ GitHub #156, and Issue 6.2 / GitHub #162. It does not block their typed/pasted
or generic text-capable paths.

**Dispatcher Ownership:** Take `src/api/surfaces.rs` integration ownership only
after Issue 3.6 / GitHub #148 lands. Evidence-domain workers own
`src/domain/evidence.rs` and must not concurrently edit the shared dispatcher.

**Scope:**

- Validate provider-neutral `EvidenceRefInput` descriptors at the Surface
  Gateway boundary.
- Limit the initial action integration to `capture_observation` and
  `submit_attempt`.
- Define the generic, role-neutral request field and validate it for
  `agent_ocr`, `agent_transcribed`, and `mixed` media-derived input:

  ```text
  input_confirmation: {
    status: "confirmed",
    method: "explicit_acceptance" | "explicit_correction"
  }
  ```

- Derive `CognitiveSpace` ownership from the authorized Surface context; callers
  cannot provide or claim evidence ownership.
- Reject an entire `EvidenceRefInput` as one invalid reference when its locator
  or metadata contains any secret, including credentials, tokens, mount secrets,
  and short-lived signed URLs.
- Apply one deterministic, fixture-backed, closed V1 secret policy:
  - Recursively inspect metadata keys at every object and array depth. Percent
    decoding does not apply to metadata. Normalize each key by ASCII lowercasing
    and then removing `-`, `_`, and `.`. Reject exactly this compact deny set:
    `password`, `passwd`, `secret`, `clientsecret`, `token`, `accesstoken`,
    `refreshtoken`, `apikey`, `authorization`, `cookie`, `session`,
    `credential`, `privatekey`, and `mountsecret`.
  - Recursively inspect every string value in metadata at every object and array
    depth, without percent decoding. Apply the same closed secret-value patterns
    used for locator query and fragment values: an ASCII case-insensitive
    `Bearer ` prefix, a PEM private-key header, a JWT-like value of exactly
    three non-empty base64url segments, or fixture prefixes `sk-`, `AKIA`,
    `AIza`, and `ghp_`. Reject the entire reference on any match.
  - Reject URI userinfo in locators.
  - Parse locator query and fragment key/value pairs. Percent-decode names and
    values exactly once as UTF-8 and reject invalid encoding; do not decode a
    second time. Normalize names with the metadata rule. Reject the metadata
    deny set plus `xamzalgorithm`, `xamzcredential`, `xamzsignature`,
    `xamzsecuritytoken`, `xgoogalgorithm`, `xgoogcredential`,
    `xgoogsignature`, `signature`, `sig`, and `expires`.
  - After the one decode, reject query or fragment values with an ASCII
    case-insensitive `Bearer ` prefix, a PEM private-key header, a JWT-like
    value of exactly three non-empty base64url segments, or fixture prefixes
    `sk-`, `AKIA`, `AIza`, and `ghp_`.
  - Reject data URLs and inline bytes as specified by the media evidence
    contract.
  - Accept ordinary metadata and path text containing `token` or `api_key` when
    there is no credential-bearing userinfo, query, or fragment. A plain path
    segment alone is never a secret-key match.
  - This V1 policy is closed. Extensions require a media evidence contract
    change, not ad hoc additions by workers.
- Report only the offending field/path and a stable error code. Never include
  the raw value in diagnostics or logs.
- Redact only diagnostics and log messages. Never write rejected raw payloads or
  secrets to logs, Trace, metadata persistence, or any persistence.
- Allow optional evidence references only on the two initial Surface actions.
- Keep ADR-021 and `docs/media-evidence-contract.md` as the contract sources of
  truth.

**Non-Goals:**

- No persistence, repository, or schema for `EvidenceRef`.
- No resolver execution.
- No upload, download, or media bytes.
- No OCR or ASR implementation in this validation foundation or elsewhere in
  the MemoryNexus path. Agent / App Adapter preprocessing remains allowed.
- No provider SDKs.

**Acceptance Criteria:**

- Valid provider-neutral `EvidenceRefInput` values pass descriptor validation.
- The generic role-neutral `input_confirmation` field is defined by F1 and
  required for `agent_ocr`, `agent_transcribed`, and `mixed` input.
- Space ownership is derived only from authorized Surface context and cannot be
  supplied by the caller.
- Secret-bearing locator or metadata rejects the entire reference, diagnostics
  are redacted, and rejected raw payloads and secrets enter no log, Trace,
  metadata persistence, or other persistence.
- Metadata string values are recursively checked at every depth against the
  same closed V1 secret-value patterns as locator query and fragment values.
- Secret rejection is deterministic against the required fixture corpus and
  returns only field/path plus error code, never a raw value.
- `capture_observation` and `submit_attempt` can carry optional validated
  references.
- Accepted references remain request-local and ephemeral. Descriptor objects,
  raw locators, and metadata are absent from every existing Memory,
  FeedbackLoop, and Trace persistence argument and stored record/summary.
  Surface business writes may still occur, but their fakes/spies prove that
  descriptors are excluded and that no `EvidenceRef` repository or schema
  exists.
- Contract behavior matches ADR-021 and the media evidence contract without
  claiming persistence or resolver support.

**Required Test Matrix:**

- `evidence_refs` is optional; both an absent field and an empty array are
  accepted.
- `input_confirmation` tests for every media-derived source reject a missing
  field, an unconfirmed status, and an invalid method; they accept only the two
  confirmed methods in the F1 shape.
- Required-field tests cover presence of `provider`, `locator`, `media_type`,
  and `metadata`.
- `provider` tests cover `^[a-z][a-z0-9._-]{0,63}$` and accepted 1-byte and
  64-byte values versus a rejected 65-byte value.
- `locator` tests cover non-empty input; accepted 4096 decoded UTF-8 bytes
  versus rejected 4097 bytes; accepted 8192 serialized JSON bytes versus
  rejected 8193 bytes; control characters; URI userinfo; all closed signed/auth
  query and fragment names with case/separator variants; exactly-once percent
  decoding; invalid UTF-8 or percent encoding; all required decoded value
  patterns; data URLs; and inline media bytes.
- `media_type` tests require normalized lowercase `type/subtype` syntax without
  parameters, with both tokens matching `[a-z0-9][a-z0-9!#$&^_.+-]*`, and a
  maximum of 255 ASCII bytes. Tests include an accepted canonical value and
  rejected uppercase, parameterized, invalid-token, and 256-byte values.
- `content_hash` tests cover exactly `sha256:` plus 64 lowercase hexadecimal
  characters and reject another algorithm, uppercase hexadecimal, and wrong
  lengths.
- `original_name` tests cover accepted 255 UTF-8 bytes versus rejected 256
  bytes, control characters, and `/` or `\\` path separators.
- `captured_at` tests accept canonical RFC 3339 UTC `Z` values and reject
  numeric offsets and other noncanonical values.
- `transcript` tests cover valid UTF-8 and accepted 65536 bytes versus rejected
  65537 bytes.
- `transcript_source` tests cover the provider identifier syntax and accepted
  64-byte values versus rejected 65-byte values.
- `metadata` tests require a JSON object; cover accepted 16384 serialized UTF-8
  bytes versus rejected 16385 bytes; count the root object as depth 1, accept
  depth 4, and reject depth 5; recursively exercise every denied key in nested
  objects/arrays with ASCII case and `-`, `_`, `.` normalization variants;
  recursively exercise every closed secret-value pattern in string values at
  all depths, including nested objects and arrays; and prove percent decoding
  does not apply to metadata keys or values.
- Positive false-positive-control fixtures accept ordinary non-secret metadata
  and locators whose plain path text contains words such as `token` or
  `api_key`, for example `s3://study/archive/token-guidelines.pdf`, when there
  is no credential-bearing userinfo or denied query/fragment key.
- Caller-supplied `id` and `space_id` ownership fields are rejected.
- Any unsafe locator or metadata rejects the entire reference. Diagnostics are
  field/path plus error code only, with explicit assertions that rejected raw
  payloads and secrets are absent from diagnostics and captured logs, no Trace
  write occurs, and no persistence or repository call occurs. Because F1 is
  validation-only and no evidence persistence exists, use fakes or spies to
  prove the absence of downstream calls rather than adding persistence.
- Positive-path fakes/spies assert accepted descriptor objects, raw locators,
  and metadata never appear in any existing Memory, FeedbackLoop, or Trace
  persistence argument or stored record/summary. They also prove there is no
  `EvidenceRef` repository/schema or descriptor persistence while allowing
  descriptor-free Surface business writes.

**Required Secret Fixtures:**

| Fixture | Expected result |
| --- | --- |
| `{"outer":[{"Client-Secret":"fixture-value"}]}` | Reject the entire reference with the nested field path and `secret_key_denied`. |
| `{"note":"Bearer secret"}` | Reject the entire reference with the metadata field path and `secret_value_pattern_denied`. |
| `{"value":"sk-test"}` | Reject the entire reference with the metadata field path and `secret_value_pattern_denied`. |
| `{"outer":[{"token_value":"eyJmaXh0dXJlIjoxfQ.cGF5bG9hZA.c2lnbmF0dXJl"}]}` | Reject the entire reference with the nested metadata field path and `secret_value_pattern_denied`. |
| `{"outer":[{"key_material":"-----BEGIN PRIVATE KEY----- fixture"}]}` | Reject the entire reference with the nested metadata field path and `secret_value_pattern_denied`. |
| `https://user:fixture-password@example.test/media/1` | Reject with locator path and `locator_userinfo_denied`. |
| `https://example.test/media/1?X-Amz-Signature=fixture` | Reject with query-field path and `locator_query_denied`. |
| Query/fragment values beginning with `Bearer ` or a private-key PEM header after one decode | Reject with field path and `secret_value_pattern_denied`. |
| Query/fragment value `eyJmaXh0dXJlIjoxfQ.cGF5bG9hZA.c2lnbmF0dXJl` after one decode | Reject with field path and `secret_value_pattern_denied`. |
| Query/fragment values beginning with `sk-`, `AKIA`, `AIza`, and `ghp_` after one decode | Reject each with field path and `secret_value_pattern_denied`. |
| `{"page":2,"label":"weekly review","source":"teacher notes"}` | Accept as ordinary non-secret metadata. |
| `{"note":"authorization awareness lesson","value":"sketch test","nested":{"summary":"safe string"}}` | Accept as ordinary safe metadata strings that do not match a closed V1 value pattern. |
| `s3://study/archive/token-guidelines.pdf` | Accept because `token` appears only in path text. |
| `https://example.test/api_key/access_token_notes.txt?version=3#page=2` | Accept because credential-like text appears only in plain path segments and there is no userinfo or denied query/fragment name. |

For every rejected fixture, assert that diagnostics contain only the stated
field/path and error code, captured diagnostics/logs do not contain the raw
fixture payload or value, and neither Trace nor repository/persistence fakes
are called.

**Required References:**

- `decisions/ADR-021-external-media-evidence-references.md`
- `docs/media-evidence-contract.md`

**Possible Files:**

- `src/domain/evidence.rs`
- `src/domain/surface.rs`
- `src/api/surfaces.rs`
- `tests/*evidence_ref*`

### Issue 5.2: Capture Dictation Word List

**Background:** The daily loop begins by recording today's words, phrases, or sentences.

**Status:** Closed on GitHub as #155 on 2026-06-24.

**Depends On:** Issue 3.6 / GitHub #148 for the typed/pasted path. The media extension for
`agent_ocr`, `agent_transcribed`, `mixed`, `input_confirmation`, and
`evidence_refs` additionally depends on Foundation F1 / GitHub #175.

**Unlocks:** The typed/pasted path unlocks the typed/pasted path in Issue 5.3 /
GitHub #156; the media extension unlocks the corresponding media attempt path.

**Dispatcher Ownership:** Take `src/api/surfaces.rs` integration ownership only
after Issue 3.6 / GitHub #148 lands. Dictation-domain workers own
`src/domain/dictation.rs` and must not concurrently edit the shared dispatcher.

**Scope:**

- Add Capture Surface flow for dictation list.
- Use namespace such as `child.chinese.dictation`.
- First ship `typed` or `pasted` text without a media dependency. Such requests
  must reject `evidence_refs`, `input_confirmation`, and every media-only
  provenance or descriptor field.
- Keep `agent_ocr`, `agent_transcribed`, and `mixed` source values disabled
  until Foundation F1 / GitHub #175 lands.
- After Foundation F1 / GitHub #175 lands, extend the same Surface action with
  `agent_ocr`, `agent_transcribed`, or `mixed` source provenance,
  `input_confirmation`, and optional validated `EvidenceRefInput` descriptors.
- In the media extension, validate the role-neutral `input_confirmation` field
  defined by Foundation F1 / GitHub #175 for `agent_ocr`,
  `agent_transcribed`, and `mixed` media-derived input as defense in depth, even
  when an Adapter already enforced it at transport.
- MemoryNexus cannot infer an undisclosed physical source from normalized text;
  the Adapter or caller must report source provenance truthfully.
- Write Trace.

**Non-Goals:**

- MemoryNexus does not upload images or perform OCR / ASR on worksheets;
  Agent-prepared, explicitly user-confirmed normalized text is accepted.
- No `EvidenceRef` persistence, repository, schema, or resolver.

**Acceptance Criteria:**

- User can capture requests declared as typed or pasted before the media
  extension lands when no media-only fields are present.
- After Foundation F1 / GitHub #175, text prepared through Agent OCR/ASR can use the
  media extension without changing the text-first business path.
- Every media-derived normalized payload requires explicit user acceptance or
  correction before submission.
- Typed/pasted tests reject `evidence_refs`, `input_confirmation`, and every
  media-only provenance or descriptor field.
- Tests reject `agent_ocr`, `agent_transcribed`, and `mixed` source values until
  Foundation F1 / GitHub #175 lands.
- Surface tests reject missing, unconfirmed, or invalid-method
  `input_confirmation` for every media-derived source and accept the two
  confirmed methods.
- Positive-path fakes/spies prove accepted descriptor objects, raw locators,
  and metadata are absent from every existing Memory, FeedbackLoop, and Trace
  persistence argument and stored record/summary. Surface business writes
  remain allowed only when descriptors are excluded; no `EvidenceRef`
  repository/schema or descriptor persistence is introduced.
- Trace links namespace and source.
- Tests cover empty and valid lists.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/dictation.rs`
- `tests/*dictation_capture*`

### Issue 5.3: Submit Dictation Attempt

**Background:** Dictation Coach needs a text-first attempt submission before
automated evaluation.

**Status:** Closed on GitHub as #156 on 2026-06-24.

**Depends On:** Issue 5.2 / GitHub #155 for the typed/pasted path. The media extension for
`agent_ocr`, `agent_transcribed`, `mixed`, `input_confirmation`, and
`evidence_refs` additionally depends on Foundation F1 / GitHub #175.

**Unlocks:** Issue 5.4 / GitHub #157 and the required attempt history for Issue
5.6 / GitHub #159.

**Dispatcher Ownership:** Take `src/api/surfaces.rs` integration ownership only
after Issue 5.2 / GitHub #155 lands. Dictation-domain workers own
`src/domain/dictation.rs` and must not concurrently edit the shared dispatcher.

**Scope:**

- Submit expected items and actual result.
- Typed/pasted requests must reject `evidence_refs`, `input_confirmation`, and
  every media-only provenance or descriptor field.
- Keep `agent_ocr`, `agent_transcribed`, and `mixed` source values disabled
  until Foundation F1 / GitHub #175 lands.
- In the media extension, allow attempts to carry optional validated
  `EvidenceRefInput` values for request-local provenance.
- In the media extension, accept source provenance and validate the Foundation
  F1 / GitHub #175 role-neutral `input_confirmation` field for `agent_ocr`,
  `agent_transcribed`, and `mixed` media-derived input at the Surface boundary.
- MemoryNexus cannot infer an undisclosed physical source from normalized text;
  the Adapter or caller must report source provenance truthfully.
- Record FeedbackLoop attempt.
- Write Trace.

**Non-Goals:**

- MemoryNexus performs neither handwriting recognition nor audio transcription;
  Adapter-prepared, explicitly user-confirmed normalized text is allowed.
- No `EvidenceRef` persistence, repository, schema, or resolver.

**Acceptance Criteria:**

- Attempt submission works for Chinese words, English words, and English sentences.
- Attempt is Space-owned and namespace-scoped.
- Deterministic evaluation uses confirmed text and does not require linked media
  to be available.
- Requests declared as typed/pasted work before Foundation F1 / GitHub #175
  lands when no media-only fields are present.
- Typed/pasted tests reject `evidence_refs`, `input_confirmation`, and every
  media-only provenance or descriptor field.
- Tests reject `agent_ocr`, `agent_transcribed`, and `mixed` source values until
  Foundation F1 / GitHub #175 lands.
- Media-extension Surface tests reject missing, unconfirmed, or invalid-method
  `input_confirmation` for every media-derived source and accept the two
  confirmed methods.
- Positive-path fakes/spies prove accepted descriptor objects, raw locators,
  and metadata are absent from every existing Memory, FeedbackLoop, and Trace
  persistence argument and stored record/summary. Surface business writes
  remain allowed only when descriptors are excluded; no `EvidenceRef`
  repository/schema or descriptor persistence is introduced.

**Possible Files:**

- `src/domain/dictation.rs`
- `src/api/surfaces.rs`
- `tests/*dictation_attempt*`

### Issue 5.4: Deterministic Dictation Mistake Classification

**Background:** First feedback should be deterministic before LLM/OCR.

**Status:** Closed on GitHub as #157 on 2026-06-24.

**Depends On:** Issue 5.3 / GitHub #156.

**Unlocks:** Issue 4.4 / GitHub #152 on the selected Dictation Engine
critical path.

**Scope:**

- Classify Chinese and English mistake types from confirmed normalized text
  supplied through manually entered or Agent-prepared expected/actual sources;
  MemoryNexus still performs no OCR or ASR.
- Return mistake type and short explanation.

**Non-Goals:**

- Do not solve all Chinese NLP or handwriting issues.
- Do not call LLM.

**Acceptance Criteria:**

- Tests cover listed Chinese and English mistake types where deterministic input allows it.
- Unknown cases return `unclassified` or evidence gap, not hallucinated certainty.

**Possible Files:**

- `src/domain/dictation.rs`
- `tests/*dictation_classification*`

### Issue 5.5: Generate Tomorrow 10-Minute Practice

**Background:** Dictation Coach value is next action, not just diagnosis.

**Status:** Closed on GitHub as #158 on 2026-06-29.

**Depends On:** Issue 4.5 / GitHub #153, and therefore transitively Issue 4.4 /
GitHub #152 and Issue 5.4 / GitHub #157.

**Unlocks:** Initial Issue 5.7 / GitHub #160 Agent smoke.

**Scope:**

- Shape or reuse the evidence-linked PracticePlan generated by Issue 4.5 /
  GitHub #153 as tomorrow's focused ten-minute Dictation practice.
- Use the existing GrowthModel -> PracticePlan path; do not create a parallel
  Dictation planning model or generation path.
- Link source traces.

**Non-Goals:**

- No full curriculum.
- No cloud generation.

**Acceptance Criteria:**

- Repeated pattern yields focused next practice.
- Plan is short and actionable.
- The returned exercise retains the PracticePlan evidence links from Issue 4.5
  / GitHub #153.
- Tests cover Chinese and English examples.

**Possible Files:**

- `src/domain/practice_plan.rs`
- `src/domain/dictation.rs`
- `tests/*dictation_plan*`

### Issue 5.6: Dictation 7-Day Observation Summary

**Background:** Observation Surface should show long-term change.

**Status:** Closed on GitHub as #159 on 2026-06-29.

**Depends On:** The canonical GrowthModel output from Issue 4.4 / GitHub #152,
plus the required attempt history from Issue 5.3 / GitHub #156 for the extended
Dictation path.

**Contribution:** Supplies deterministic multi-day fixtures and the extended
seven-day acceptance for Issue 5.7 / GitHub #160. It does not block the initial
Agent smoke.

**Dispatcher Ownership:** This issue does not own GrowthModel aggregation or
writes. Any read-only Observation integration in `src/api/surfaces.rs` starts
only after Issue 3.6 / GitHub #148, Issue 4.4 / GitHub #152, and Issue 5.3 /
GitHub #156 land; fixture workers must not concurrently edit the shared
dispatcher.

**Scope:**

- Build a read-only Observation projection over the canonical Issue 4.4 /
  GitHub #152 GrowthModel output plus evidence and attempt fixtures.
- Summarize recurring errors, stability, and current focus for the last 7 days
  without mutating the canonical model.

**Non-Goals:**

- No dashboard yet.
- No charts unless existing UI makes it trivial.
- Do not create or update a second GrowthModel.
- Do not reclassify mistakes independently of Issue 5.4 / GitHub #157 evidence
  already aggregated by Issue 4.4 / GitHub #152.

**Acceptance Criteria:**

- Summary includes Trace/attempt evidence IDs, never `EvidenceRefInput`
  descriptors or descriptor fields.
- Projection reads the canonical GrowthModel and performs no GrowthModel write.
- Empty history returns useful empty state.

**Possible Files:**

- `src/api/surfaces.rs`
- `tests/fixtures/dictation_agent/*`
- `tests/*dictation_observation*`

### Issue 5.7: Minimal Dictation Agent Demo

**Background:** The first usable product test should run through a Dictation
Agent before a dedicated product App is built, reusing the generic MCP/chat
Surface Adapter from Issue 6.2 / GitHub #162.

**Status:** Closed on GitHub as #160 on 2026-06-30. This is an accepted
text-first Agent smoke, not a standalone app.

**Depends On:** For the initial typed/pasted smoke, Issues 5.2-5.5 / GitHub
#155-#158 and the text-capable portion of Issue 6.2 / GitHub #162. Issue 5.6 /
GitHub #159 contributes only the extended seven-day acceptance. Media prompt
acceptance/correction is a later extension after Foundation F1 / GitHub #175
and the media-capable portion of Issue 6.2 / GitHub #162.

**Unlocks:** Initial acceptance unlocks Issue 6.3 / GitHub #163.

**Scope:**

- Implement Dictation Agent orchestration and product-facing action mapping over
  the generic adapter.
- Run the initial loop entirely through typed/pasted generic Surface tools,
  without a web UI.
- In the media extension, own only the Dictation product prompt/interaction
  that obtains explicit acceptance or correction for Agent-prepared normalized
  text, then map the result to the generic `input_confirmation` field defined
  by Foundation F1 / GitHub #175 through the adapter mapping in Issue 6.2 /
  GitHub #162.
- Exercise one-learner Capture, Performance, Reflection, Planning, and
  Observation through the generic Surface Adapter.

**Non-Goals:**

- No new frontend stack.
- No multi-user product shell.
- No dedicated Dictation Coach App dependency.
- No duplicate MCP/chat protocol transport or tool plumbing.
- No Dictation-specific Engine actions.

**Acceptance Criteria:**

- Initial end-to-end dictation demo works for one learner with requests declared
  as typed or pasted and no media-only fields.
- Dictation orchestration uses Issue 6.2 / GitHub #162 generic tools and does
  not directly access Engine internals.
- Initial acceptance covers all five generic Surfaces and writes Trace
  provenance where required, without a web UI.
- Extended seven-day acceptance uses Issue 5.6 / GitHub #159 deterministic
  multi-day fixtures but does not redefine the initial smoke gate.
- After Foundation F1 / GitHub #175 and media-capable Issue 6.2 / GitHub #162,
  media prompt tests demonstrate explicit acceptance and correction before
  Surface submission.
- Product-facing mappings and prompt/confirmation policy remain outside the
  generic adapter.
- Media-extension prompt-flow tests demonstrate explicit acceptance and
  explicit correction, then assert successful mapping to
  `explicit_acceptance` and `explicit_correction` respectively.
- No parent/child or other product role is added to Engine contracts.

**Possible Files:**

- `docs/dictation-agent-demo.md`
- `tests/fixtures/dictation_agent/*`
- `tests/*dictation_agent_smoke*`

## Milestone 6: Adapters

### Issue 6.1: Define Adapter Capability Policy

**Background:** Adapters should have allowed surfaces rather than direct Engine access.

**Scope:**

- Document adapter types and allowed surfaces.
- Add policy structs if useful.

**Non-Goals:**

- Do not build all adapters.

**Acceptance Criteria:**

- Chat, Practice App, Dashboard, CLI, MCP have allowed surface lists.
- Policy keeps Engine internals private.

**Possible Files:**

- `docs/architecture/surfaces-and-adapters.md`
- `src/domain/adapter.rs`

### Issue 6.2: Generic MCP / Chat Surface Adapter Foundation

**Background:** MCP/chat clients need generic transport and tool plumbing over
Surface Gateway before product-specific Agent orchestration is added.

**Status:** Closed on GitHub as #162 on 2026-06-30.

**Depends On:** Issue 3.6 / GitHub #148 for text-capable generic Surface tools. Media mapping
for `agent_ocr`, `agent_transcribed`, `mixed`, `input_confirmation`, and
`evidence_refs` additionally depends on Foundation F1 / GitHub #175.

**Unlocks:** The text-capable tools unlock the initial Issue 5.7 / GitHub #160
smoke; the media mapping contributes to its later media prompt extension.

**Dispatcher Ownership:** This adapter issue never edits
`src/api/surfaces.rs`. It owns only MCP/chat transport and mapping modules after
the required Surface capabilities land.

**Scope:**

- Implement MCP transport/tool handling and chat adapter integration over
  Surface Gateway.
- Map generic actions and capabilities across Capture, Performance, Reflection,
  Planning, and Observation without product-specific Engine actions.
- Ship the text-capable generic Surface tools after Issue 3.6 / GitHub #148
  without waiting for Foundation F1 / GitHub #175.
- Typed/pasted tool requests must reject `evidence_refs`, `input_confirmation`,
  and every media-only provenance or descriptor field.
- Keep `agent_ocr`, `agent_transcribed`, and `mixed` source values disabled
  until Foundation F1 / GitHub #175 lands.
- In the media mapping, enforce and map the role-neutral adapter request field
  already defined by Foundation F1 / GitHub #175:

  ```text
  input_confirmation: {
    status: "confirmed",
    method: "explicit_acceptance" | "explicit_correction"
  }
  ```

- In the media mapping, enforce `input_confirmation` in the MCP/chat Adapter for `agent_ocr`,
  `agent_transcribed`, and `mixed` media-derived input before making a generic
  Surface call.
- MemoryNexus cannot infer an undisclosed physical source from normalized text;
  the Adapter or caller must report source provenance truthfully.
- Keep OCR, ASR, and media acquisition outside MemoryNexus, and require explicit
  user acceptance or correction before submitting any media-derived normalized
  payload.
- In the media mapping, pass optional validated, opaque `EvidenceRefInput`
  descriptors through generic calls. Return and preserve the generated Trace
  ID/provenance for the Surface call, but do not return or claim persistence of
  media descriptors. Only the generated Surface call Trace ID/provenance is
  returned; descriptors remain ephemeral in this slice. Generic calls do not
  resolve evidence and do not require media, provider, or resolver availability.

**Non-Goals:**

- Do not make agent own memory.
- Do not edit `src/api/surfaces.rs` or access Engine repositories directly.
- Do not add evidence resolution, provider availability detection, or media
  inspection; those behaviors belong to a future resolver issue.
- Do not add `EvidenceRef` persistence or claim that descriptors are persisted
  or resolvable.
- Do not add Dictation orchestration, product-facing mappings, prompt policy, or
  Dictation-specific Engine actions.

**Acceptance Criteria:**

- Generic MCP/chat smoke demonstrates all five Surfaces.
- Text-capable tools work after Issue 3.6 / GitHub #148 without Foundation F1 /
  GitHub #175.
- Typed/pasted tool tests reject `evidence_refs`, `input_confirmation`, and every
  media-only provenance or descriptor field before any Surface call.
- Tests reject `agent_ocr`, `agent_transcribed`, and `mixed` source values until
  Foundation F1 / GitHub #175 lands.
- Agent response includes trace provenance where appropriate.
- Calls use generic Capture, Performance, Reflection, Planning, and Observation
  capabilities and actions.
- Adapter tests reject missing or unconfirmed `input_confirmation` for
  `agent_ocr`, `agent_transcribed`, and `mixed` input before any generic Surface
  call, reject an invalid method, and accept both `explicit_acceptance` and
  `explicit_correction` with `status: "confirmed"`.
- Confirmed-text processing succeeds when an optional opaque
  `EvidenceRefInput` passes Foundation F1 / GitHub #175 validation; the adapter
  makes no availability or persistence claim and performs no resolver call. Its
  response exposes only the generated Surface call Trace ID/provenance, never
  descriptor objects, raw locators, or metadata.

**Possible Files:**

- `src/bin/memorynexus-mcp.rs`
- `docs/mcp.md`
- `docs/agent-integration.md`

### Issue 6.3: Simple Practice App Adapter

**Background:** A practice adapter should use only the surfaces it needs.

**Depends On:** Initial Issue 5.7 / GitHub #160 acceptance.

**Status:** Closed on GitHub as #163 on 2026-07-01. The accepted implementation
is a minimal Rust-served Simple Practice App Adapter over Surface Gateway; it
does not move product roles or memory ownership into the Engine.

**Scope:**

- Provide minimal practice flow through Performance, Planning, Observation.
- Reuse Rust-served static UI if UI is chosen.

**Non-Goals:**

- No React/Vite/Next.
- No dashboard debug details.

**Acceptance Criteria:**

- Practice flow does not expose Trace/GrowthModel raw internals.
- UI copy is dictation-friendly.

**Possible Files:**

- `web/dictation_coach.html`
- `src/api/web.rs`

### Issue 6.4: Developer Dashboard Adapter

**Background:** Developers need debug visibility into Trace, GrowthModel, and SleepCycle.

**Status:** Closed on GitHub as #164 on 2026-07-01. The accepted implementation
defines the Developer Dashboard as a developer/admin/debug adapter contract. It
documents read-only inspected Engine debug visibility while allowing Gateway
audit/provenance Trace writes for dashboard requests.

**Scope:**

- Define read-only dashboard adapter requirements.
- Optionally implement minimal route after docs are accepted.

**Non-Goals:**

- Do not make it a user-facing product.
- Do not bypass permissions.

**Acceptance Criteria:**

- Dashboard access is read-only for Engine debug data.
- Allowed surfaces are documented.

**Possible Files:**

- `docs/architecture/surfaces-and-adapters.md`
- `web/debug_dashboard.html`

## Milestone 7: Evaluation

### Issue 7.1: Define DictationBench Fixtures

**Background:** Evaluation should target feedback quality and growth, not just retrieval.

**Scope:**

- Create deterministic fixtures for Chinese and English dictation.
- Include repeated mistakes and improvement cases.

**Non-Goals:**

- No external AI dependency.

**Acceptance Criteria:**

- Fixtures can run locally.
- Fixtures include expected mistake patterns and next practice expectations.

**Possible Files:**

- `tests/fixtures/dictation_bench/*`
- `docs/evaluation.md`

### Issue 7.2: Evaluate Recurring Error Detection

**Background:** GrowthModel should detect repeated patterns.

**Scope:**

- Run benchmark over fixtures.
- Report detected vs expected patterns.

**Non-Goals:**

- No statistical causal claims.

**Acceptance Criteria:**

- Benchmark output shows pass/fail per pattern.
- Local deterministic run succeeds.

**Possible Files:**

- `src/bin/memorynexus-eval.rs`
- `tests/*dictation_bench*`

### Issue 7.3: Evaluate Next Practice Generation

**Background:** PracticePlan should be judged by relevance to observed mistake patterns.

**Scope:**

- Compare generated practice against expected focus.
- Record useful / neutral / bad labels.

**Non-Goals:**

- Do not require multi-day real users.

**Acceptance Criteria:**

- Benchmark reports next-practice alignment.
- Bad or irrelevant plans are visible.

**Possible Files:**

- `docs/evaluation.md`
- `tests/*practice_plan_eval*`

### Issue 7.4: Evaluate Multi-Day Improvement Signals

**Background:** The loop should track whether errors reduce after practice.

**Scope:**

- Use simulated multi-day traces.
- Detect improved, repeated, skipped, and insufficient evidence cases.

**Non-Goals:**

- Do not claim clinical or educational causality.

**Acceptance Criteria:**

- Evaluation distinguishes improvement from insufficient evidence.
- Metrics include latency/cost/local ratio where Trace exists.

**Possible Files:**

- `tests/fixtures/dictation_bench/multi_day.json`
- `src/domain/growth_model.rs`
- `docs/evaluation.md`
