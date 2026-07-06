# MemoryNexus Roadmap

> Last updated: 2026-07-01
> Source of truth for executable task definitions: GitHub Issues, with
> [docs/issues.md](issues.md) as the planning mirror for the milestone shape.

## Current Direction

MemoryNexus is a local-first long-term feedback engine for personal cognition
and skill acquisition.

See [ADR-022](../decisions/ADR-022-memorynexus-brand-semantics.md) for the
current `MemoryNexus` brand semantics and the Engine/product naming split.

It should not be framed as a generic recall product, personal knowledge vault,
agent recall store, connector platform, RAG profile service, or local AI
runtime. Its core question is:

```text
How can a system use long-term traces to generate better feedback and next
actions over time?
```

The target loop is:

```text
Trace -> FeedbackLoop -> GrowthModel -> PracticePlan -> next Trace
```

## Current Understanding

The repository already has strong foundations:

- Rust + Axum is the main backend.
- Memory belongs to `CognitiveSpace`, not to agents or apps.
- Namespace and FeedbackLoop foundations exist.
- Trace schema/repository and SleepCycle domain/persistence foundations exist.
- Thought Review demonstrates reflective memory and Lens-based interpretation.
- The `learning.stem` slice validates practice sessions, feedback capture,
  weekly review, MCP flow, and a simple Rust-served UI.
- Binary-first install, Local One-click packaging, Production Profile, and
  Supabase Postgres compatibility have documentation and implementation tracks.
- Surface Gateway has landed for Capture, Performance, Reflection, Planning,
  Observation, and manual consolidation. The generic MCP/chat Surface tools
  (#162) and minimal Dictation Agent demo (#160) are merged.
- ADR-021 and the media evidence contract define provider-neutral
  `EvidenceRefInput`; request-time validation and adapter confirmation mapping
  have landed in #175/#162. Evidence descriptor persistence, resolver runtime,
  and media handling remain out of scope.

## Gap Against The New Direction

The project still needs to close these gaps:

- Compatibility paths still expose object-level APIs before all adapters move
  through Surface Gateway.
- MCP now exposes generic Surface Gateway tools for Capture, Performance,
  Reflection, Planning, and Observation. Compatibility object-level APIs still
  exist and should be treated as legacy adapter paths where possible.
- Surface success event publication has landed for `ObservationCaptured` and
  `AttemptSubmitted`; durable event storage and async processors remain future
  work.
- GrowthModel aggregation (#152), simple PracticePlan generation (#153),
  Dictation next-practice (#158), seven-day Observation (#159), the text-first
  Agent smoke (#160), event publication (#150), and the PR-required PostgreSQL
  Surface integration gate (#177) have landed.
- `learning.stem` is a useful prior slice. Dictation Coach is now the first
  upstream product path with accepted Engine + Agent smoke and a minimal
  Rust-served Simple Practice App Adapter (#163). A separate standalone app
  repository remains future work only if product needs outgrow the static
  adapter.
- Typed or pasted Dictation Capture/Attempt and media-derived confirmation
  validation have landed. OCR, ASR, media acquisition, descriptor persistence,
  and descriptor resolution remain Adapter/future-slice work.
- GitHub Release artifact publication and Local One-click release validation
  have landed. Trial Profile remains blocked on a real Trial API endpoint and
  scoped token; Production Profile still needs a real deployment smoke.
- Evaluation should measure growth and feedback usefulness, not just retrieval
  accuracy.

## Architecture Spine

```text
Adapters
  Chat Agent / MCP / CLI / Web / Mobile / Dashboard / Voice
      |
      v
Surface Gateway
  Auth / Namespace routing / Surface routing / ACL / validation
  Response shaping / Trace writing / sync-async dispatch / events
      |
      v
Surfaces
  Capture | Performance | Reflection | Planning | Observation
      |
      v
Engine
  Namespace | Trace | MemoryAtom | CognitiveScene | FeedbackLoop
  GrowthModel | SleepCycle | PracticePlan / DreamCandidate | Lens
```

Principle:

```text
Adapter = how interaction happens
Surface = what intent is requested
Engine  = how memory, feedback, growth, and planning evolve over time
```

## Milestone 1: Architecture Refresh

Goal: update documentation only; no business code changes.

Status: completed on `main`.

Deliverables:

- Update README positioning.
- Add or update [Vision](vision.md).
- Add [MemoryNexus Engine](architecture/memorynexus-engine.md).
- Add [Surfaces and Adapters](architecture/surfaces-and-adapters.md).
- Add [Surface Gateway](architecture/surface-gateway.md).
- Add [Sleep-driven Feedback Loop](architecture/sleep-driven-feedback-loop.md).
- Add ADR-018: MemoryNexus as Long-term Feedback Engine.
- Add ADR-019: Surfaces vs Adapters vs Engine.
- Keep ADR-017 as the Sleep-based Consolidation ADR.
- Add ADR-020: Dictation Coach as First Upstream Product.
- Add [Executable Issues](issues.md).

## Milestone 2: Core Domain Model

Goal: define core domain types and schema without LLM integration or complex UI.

Status: mostly complete; remaining open issues include the MemoryAtom and
CognitiveScene drafts plus #142 Minimal Lens / Reflection Surface Structures.

Recommended sequence:

1. Stabilize `Namespace` and existing `FeedbackLoop` contracts under the Engine
   vocabulary.
2. Define and persist `Trace`.
3. Define `MemoryAtom` and `CognitiveScene`.
4. Define `GrowthModel`.
5. Define `SleepCycle`.
6. Define `PracticePlan` / `DreamCandidate`.
7. Define minimal Lens / Reflection Surface structures in #142.
8. Add serialization, repository, and same-Space validation tests.

Non-goals:

- No OCR or ASR inside the MemoryNexus Engine. Adapter-side preprocessing and
  confirmed normalized text are allowed but are not part of this milestone.
- No cloud LLM dependency.
- No complex UI.
- No broad education platform.

## Milestone 3: Surface Gateway MVP

Goal: build the unified Engine entry point.

Status: functionally complete for the MVP Surface set. `SurfaceRequest` /
`SurfaceResponse`, Capture, Performance, Reflection (#146), Planning (#147),
Observation (#148), manual consolidation, and the PostgreSQL-backed Surface
integration pull-request gate (#177) are implemented.

Completed sequence:

1. Define `SurfaceRequest` and `SurfaceResponse`.
2. Implement Capture Surface minimum path.
3. Implement Performance Surface minimum path.
4. Implement Reflection Surface (#146).
5. Implement Planning Surface (#147).
6. Implement Observation Surface (#148).
7. Ensure Surface calls write Trace/provenance.
8. Add validation, response shaping, and same-Space PostgreSQL integration
   coverage for the MVP Surface paths.

Next:

1. Keep external-provider and Qdrant checks outside the deterministic required
   merge gate unless a future issue explicitly scopes them.
2. Use #177's stable PR gate as the baseline before expanding more shared
   Surface dispatcher behavior.

Non-goals:

- Do not replace all existing REST/MCP routes in one migration.
- Do not expose Engine internals as Surface responses.

## Milestone 4: Event + Sleep Engine MVP

Goal: make foreground paths fast and background paths deep.

Status: MVP complete and GitHub milestone closed. Engine Event types, Surface
success event publication (#150), manual SleepCycle trigger, GrowthModel
aggregation (#152), and PracticePlan generation (#153) have landed. Durable
event storage, async processors, scheduler behavior, and effectiveness
evaluation remain future work outside the M4 MVP.

Recommended sequence:

1. Keep Surface event contracts stable after #150.
2. Add durable event storage only when a follow-up issue needs replay or async
   processors.
3. Add scheduler behavior after manual SleepCycle behavior remains stable.
4. Record effectiveness evidence for generated plans.

Non-goals:

- No scheduler before manual SleepCycle works.
- No distributed queue before in-process / stored events prove the shape.
- No cloud generation in the first path.

## Milestone 5: Dictation Coach Demo

Goal: validate the full loop with a daily dictation product scenario.

Status: MVP complete and GitHub milestone closed. The text-first Dictation
Coach path has landed through contract, typed/pasted Capture and Attempt,
deterministic mistake classification, tomorrow practice, seven-day Observation,
media-evidence validation, and the minimal Agent smoke (#154-#160, #175).
The simple Rust-served Practice App Adapter landed separately in M6 as #163.

Initial namespaces:

- `child.chinese.dictation`
- `child.english.spelling`
- `child.english.sentence-dictation`

Execution dependency graph:

| Depends On | Issue Unlocked |
| --- | --- |
| #146 review, PostgreSQL verification, and merge | #147 Planning Surface |
| #147/#148 shared Surface work | #177 required PostgreSQL Surface integration CI |
| #147 | #148 Observation Surface |
| #148 | #155 typed/pasted word-list Capture |
| #155 typed/pasted path | #156 typed/pasted attempt submission |
| #156 typed/pasted path | #157 deterministic mistake classification |
| #148 | #162 generic text Surface tools |
| #148 | #175 media confirmation and evidence validation |
| #175 | Media extensions in #155, #156, and #162 |
| #157 | #152 Trace/FeedbackLoop aggregation into GrowthModel |
| #152 | #153 PracticePlan generation from GrowthModel |
| #153 | #158 tomorrow's focused ten-minute practice |
| #155 through #158 + text-capable #162 | Initial #160 Agent smoke |
| #159 deterministic multi-day summary | Extended seven-day Agent acceptance |
| Initial #160 acceptance | #163 Simple Practice App Adapter |
| Initial #160 acceptance | #128, #129, and #130 distribution wave |

This graph has now executed through the first app adapter: #146, #147, #148,
#152, #153, #155-#160, #162, #163, #175, and #177 are closed on GitHub. The
remaining adjacent edge is the release/distribution wave after #128's Local
One-click offline bundle; #129 Trial Profile and #130 Production Profile remain
follow-up distribution work.

The accepted #160 smoke uses one learner and genuinely typed or pasted text. It
does not require OCR, ASR, a tagged release, or a dedicated Dictation Coach App.
When media is involved, the Agent/App performs OCR or ASR and must obtain
explicit user acceptance or correction before submission. Media capture and
confirmation stay in the Adapter, while the Engine remains text-first.

#175 owns the generic role-neutral `input_confirmation` request field and V1
validation. #155 and #156 validate it at the Surface boundary for media-derived
input. #162 maps the same field in the MCP/chat adapter without gaining Engine
repository access. #160 owns only product-facing prompt/interaction mapping; no
parent/child role enters the Engine.

#163 delivered the minimal Simple Practice App Adapter as a Rust-served static
Surface Gateway client. A separate Dictation Coach app repository should wait
until product needs exceed the current static adapter; it still must not own
memory or access Engine internals.

Chinese mistake taxonomy:

- wrong character;
- visually similar character;
- homophone;
- missing stroke;
- extra stroke;
- stroke-order issue;
- component placement issue.

English mistake taxonomy:

- missing letter;
- extra letter;
- letter order error;
- double-letter error;
- sound-spelling mapping error;
- capitalization error;
- missing word in sentence dictation.

Non-goals:

- No `EvidenceRef` persistence, repository, or schema in the validation
  foundation.
- No resolver execution, upload/download, or media-byte handling.
- No OCR, ASR, or handwriting recognition inside MemoryNexus.
- No provider SDK.
- No multi-child management.
- No full curriculum.
- No broad multi-subject learning platform.

## Milestone 6: Adapters

Goal: validate one Engine through multiple interaction channels.

Status: complete and GitHub milestone closed. Adapter policy (#161), generic
MCP/chat Surface tools (#162), the Rust-served Simple Practice App Adapter
(#163), and Developer Dashboard Adapter contract (#164) are all closed. #160
documents/proves the first text-first Dictation Agent loop over the generic
Surface tools.

Completed adapter sequence:

1. Define allowed surfaces per adapter (#161).
2. Maintain #162 generic MCP/chat Surface tools for Capture, Performance,
   Reflection, Planning, and Observation.
3. Treat #160 as the accepted product-facing Dictation Agent orchestration over
   generic Surface tools.
4. Implement #163, the minimal Simple Practice App Adapter for Capture,
   Performance, Planning, and limited Observation.
5. Define #164 Dashboard Adapter policy for read-only inspected Engine debug
   objects, with Gateway audit/provenance Trace still allowed.
6. Ensure adapters do not directly access Engine internals.

Non-goals:

- No new frontend stack unless an ADR approves it.
- No adapter-owned memory.
- No adapter-specific database ownership model.

## Milestone 7: Evaluation

Goal: build MemoryNexus' GrowthBench / DictationBench.

Recommended sequence:

1. Define DictationBench fixtures.
2. Evaluate recurring error detection.
3. Evaluate next-practice generation.
4. Evaluate multi-day error reduction.
5. Record latency, cost, local ratio, and useful feedback rate.
6. Report GrowthBench results.
7. Optionally add #195 as a separate P2 LoCoMo / LongMemEval-style
   retrieval/context baseline, reported outside GrowthBench / DictationBench.

Non-goals:

- Do not optimize only for retrieval accuracy.
- Do not claim causality beyond available evidence.
- Do not require external AI credentials for baseline evaluation.

## Existing GitHub Issues To Reconcile

Existing open issues should be mapped to the new milestones rather than blindly
continued under the old Phase 5 wording.

Likely mapping:

- #58 Namespace filters and FeedbackLoop provenance threading -> Milestone 2.
- #99 Trace schema and repository foundation -> Milestone 2.
- #100 Trace for Lens Run and MCP calls -> Milestone 3 / compatibility bridge.
- #97 ObserveMode runtime metrics -> Milestone 2 / 3.
- #95 local/cloud routing policy -> Milestone 3 / 4.
- #119 SleepCycle contract -> completed by `docs/sleep-cycle-contract.md`, verify
  and close if accepted.
- #117 deterministic daily sleep consolidation -> Milestone 4.
- #123 deterministic DreamCandidates for `learning.stem` -> adapt or supersede
  with PracticePlan / Dictation Coach issue.
- #120 manual SleepCycle API / CLI / MCP trigger -> Milestone 4.
- #125 DreamCandidate effectiveness -> Milestone 7.
- #61 / #62 / #63 lifecycle fixtures -> Milestone 2 / 7 prototype work.
- #128 Local One-click offline release bundle -> completed distribution
  foundation after #160.
- #129 / #130 install and deployment issues -> remaining post-#160
  distribution wave, not the initial Agent-smoke critical path.
- #177 required PostgreSQL Surface integration CI -> completed Milestone 3
  hardening foundation before more shared Surface dispatcher work.

## Supporting Distribution Track

These start after the accepted #160 Agent smoke and are not on the completed
#177 CI hardening path:

- #128 Publish first Local One-click offline release bundle. Completed on
  GitHub on 2026-06-30.
- #129 Make Trial Profile plug-and-play for agent demos.
- #130 (P1) stand up a versioned Mac mini or equivalent Production Profile
  deployment with migration preflight, health smoke, and rollback.

They distribute the already validated Agent loop; they do not delay the first
typed/pasted Developer Profile smoke.

## Issue Hygiene

When creating or updating GitHub Issues from [docs/issues.md](issues.md):

1. Use the milestone names from this roadmap.
2. Keep acceptance criteria concrete.
3. Include non-goals.
4. Name likely files.
5. Preserve Rust-first and CognitiveSpace ownership boundaries.
6. Do not mix multiple milestones into one worker task.
