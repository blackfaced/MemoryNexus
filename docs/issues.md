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

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/reflection.rs`
- `tests/*surface_reflection*`

### Issue 3.5: Implement Planning Surface Mock

**Background:** Planning answers "what should happen next?".

**Scope:**

- Add plan/generateNextTask/adjustPlan mock actions.
- Return deterministic next action.
- Write Trace.

**Non-Goals:**

- Do not generate adaptive curriculum.
- Do not call cloud model.

**Acceptance Criteria:**

- Planning Surface can return a next task for a namespace.
- Trace links generated PracticePlan when that model exists, or stores output summary.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/practice_plan.rs`
- `tests/*surface_planning*`

### Issue 3.6: Implement Observation Surface Mock

**Background:** Observation answers "how is long-term state changing?".

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

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/growth_model.rs`
- `tests/*surface_observation*`

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

**Scope:**

- Read recent Trace / FeedbackLoop evidence.
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

### Issue 5.1: Define Dictation Coach Contract

**Background:** Dictation Coach is the first upstream product, but Engine should not embed roles.

**Scope:**

- Define namespaces, task shapes, attempt shapes, and mistake taxonomy.
- Keep manual input only.
- Map actions to Surfaces.

**Non-Goals:**

- No OCR.
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

**Scope:**

- Validate provider-neutral `EvidenceRefInput` descriptors at the Surface
  Gateway boundary.
- Limit the initial action integration to `capture_observation` and
  `submit_attempt`.
- Derive `CognitiveSpace` ownership from the authorized Surface context; callers
  cannot provide or claim evidence ownership.
- Reject an entire `EvidenceRefInput` as one invalid reference when its locator
  or metadata contains any secret, including credentials, tokens, mount secrets,
  and short-lived signed URLs.
- Redact only diagnostics and log messages. Never write rejected raw payloads or
  secrets to logs, Trace, metadata persistence, or any persistence.
- Allow optional evidence references only on the two initial Surface actions.
- Keep ADR-021 and `docs/media-evidence-contract.md` as the contract sources of
  truth.

**Non-Goals:**

- No persistence, repository, or schema for `EvidenceRef`.
- No resolver execution.
- No upload, download, or media bytes.
- No OCR or ASR.
- No provider SDKs.

**Acceptance Criteria:**

- Valid provider-neutral `EvidenceRefInput` values pass descriptor validation.
- Space ownership is derived only from authorized Surface context and cannot be
  supplied by the caller.
- Secret-bearing locator or metadata rejects the entire reference, diagnostics
  are redacted, and rejected raw payloads and secrets enter no log, Trace,
  metadata persistence, or other persistence.
- `capture_observation` and `submit_attempt` can carry optional validated
  references.
- Tests cover omitted and empty refs; valid canonical provider, locator,
  `media_type`, hash, timestamp, and bounds; invalid size, nesting, and encoding;
  rejected caller ownership; and secret-bearing locator or metadata rejected
  with redacted diagnostics.
- Contract behavior matches ADR-021 and the media evidence contract without
  claiming persistence or resolver support.

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

**Dependencies:** Foundation F1.

**Scope:**

- Add Capture Surface flow for dictation list.
- Use namespace such as `child.chinese.dictation`.
- Accept confirmed text with `typed`, `pasted`, `agent_ocr`,
  `agent_transcribed`, or `mixed` source provenance and optional validated
  `EvidenceRefInput` descriptors.
- Write Trace.

**Non-Goals:**

- Do not upload images.
- Do not OCR worksheets.
- MemoryNexus does not perform OCR or ASR.

**Acceptance Criteria:**

- User can capture typed or pasted text, or text prepared through Agent OCR/ASR.
- Every media-derived normalized payload requires explicit user acceptance or
  correction before submission.
- Trace links namespace and source.
- Tests cover empty and valid lists.

**Possible Files:**

- `src/api/surfaces.rs`
- `src/domain/dictation.rs`
- `tests/*dictation_capture*`

### Issue 5.3: Submit Dictation Attempt

**Background:** Dictation Coach needs a text-first attempt submission before
automated evaluation.

**Dependencies:** Foundation F1.

**Scope:**

- Submit expected items and actual result.
- Allow attempts to link optional validated `EvidenceRefInput` values for
  provenance.
- Record FeedbackLoop attempt.
- Write Trace.

**Non-Goals:**

- No handwriting recognition.
- No audio transcription.

**Acceptance Criteria:**

- Attempt submission works for Chinese words, English words, and English sentences.
- Attempt is Space-owned and namespace-scoped.
- Deterministic evaluation uses confirmed text and does not require linked media
  to be available.

**Possible Files:**

- `src/domain/dictation.rs`
- `src/api/surfaces.rs`
- `tests/*dictation_attempt*`

### Issue 5.4: Deterministic Dictation Mistake Classification

**Background:** First feedback should be deterministic before LLM/OCR.

**Scope:**

- Classify Chinese and English mistake types from manual expected/actual input.
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

**Scope:**

- Generate simple next practice from mistake patterns.
- Produce PracticePlan.
- Link source traces.

**Non-Goals:**

- No full curriculum.
- No cloud generation.

**Acceptance Criteria:**

- Repeated pattern yields focused next practice.
- Plan is short and actionable.
- Tests cover Chinese and English examples.

**Possible Files:**

- `src/domain/practice_plan.rs`
- `src/domain/dictation.rs`
- `tests/*dictation_plan*`

### Issue 5.6: Dictation 7-Day Observation Summary

**Background:** Observation Surface should show long-term change.

**Scope:**

- Summarize last 7 days of attempts.
- Include recurring errors, stability, and current focus.

**Non-Goals:**

- No dashboard yet.
- No charts unless existing UI makes it trivial.

**Acceptance Criteria:**

- Summary includes evidence IDs.
- Empty history returns useful empty state.

**Possible Files:**

- `src/domain/growth_model.rs`
- `src/api/surfaces.rs`
- `tests/*dictation_observation*`

### Issue 5.7: Minimal Dictation Agent Demo

**Background:** The first usable product test should run through a Dictation
Agent before a dedicated product App is built, reusing the generic MCP/chat
Surface Adapter from Issue 6.2.

**Dependencies:** Issue 6.2.

**Scope:**

- Implement Dictation Agent orchestration and product-facing action mapping over
  the generic adapter.
- Define Dictation-specific prompt and confirmation policy for manually entered
  or Agent-prepared normalized text.
- Exercise one-learner Capture, Performance, Reflection, Planning, and
  Observation through the generic Surface Adapter.

**Non-Goals:**

- No new frontend stack.
- No multi-user product shell.
- No dedicated Dictation Coach App dependency.
- No duplicate MCP/chat protocol transport or tool plumbing.
- No Dictation-specific Engine actions.

**Acceptance Criteria:**

- End-to-end dictation demo works for one learner with manually entered or
  Agent-confirmed text.
- Dictation orchestration uses Issue 6.2 generic tools and does not directly
  access Engine internals.
- The flow covers all five Surfaces and writes Trace provenance where required.
- Product-facing mappings and prompt/confirmation policy remain outside the
  generic adapter.

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

**Scope:**

- Implement MCP transport/tool handling and chat adapter integration over
  Surface Gateway.
- Map generic actions and capabilities across Capture, Performance, Reflection,
  Planning, and Observation without product-specific Engine actions.
- Define generic confirmation and Trace provenance policy for adapter requests
  and responses.
- Keep OCR, ASR, and media acquisition outside MemoryNexus, and require explicit
  user acceptance or correction before submitting any media-derived normalized
  payload.
- Attach optional validated evidence references and preserve generated Trace
  provenance without requiring media availability.

**Non-Goals:**

- Do not make agent own memory.
- Do not let unavailable evidence block confirmed-text feedback or planning.
- Do not add Dictation orchestration, product-facing mappings, prompt policy, or
  Dictation-specific Engine actions.

**Acceptance Criteria:**

- Generic MCP/chat smoke demonstrates all five Surfaces.
- Agent response includes trace provenance where appropriate.
- Calls use generic Capture, Performance, Reflection, Planning, and Observation
  capabilities and actions.
- Agent-mediated OCR/ASR text is explicitly confirmed, and unavailable evidence
  degrades only provenance inspection.

**Possible Files:**

- `src/bin/memorynexus-mcp.rs`
- `docs/mcp.md`
- `docs/agent-integration.md`

### Issue 6.3: Simple Practice App Adapter

**Background:** A practice adapter should use only the surfaces it needs.

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
