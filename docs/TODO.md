# MemoryNexus Roadmap

> Last updated: 2026-06-18
> Source of truth for executable task definitions: GitHub Issues, with
> [docs/issues.md](issues.md) as the planning mirror for the milestone shape.

## Current Direction

MemoryNexus is a local-first, namespace-based long-term feedback engine for
personal cognition and skill acquisition.

It should not be framed as a generic AI memory app, second brain, agent memory
store, connector platform, RAG profile service, or local AI runtime. Its core
question is:

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
- Surface Gateway has landed for Capture, Performance, and manual
  consolidation; Reflection, Planning, Observation, adapter policy, and
  dictation-specific flows remain open issues.
- ADR-021 and the media evidence contract define provider-neutral
  `EvidenceRefInput`, but no runtime validation, persistence, resolver, or media
  handling implementation exists yet.

## Gap Against The New Direction

The project still needs to close these gaps:

- Compatibility paths still expose object-level APIs before all adapters move
  through Surface Gateway.
- MCP remains a compatibility adapter over object-level APIs; Surface Gateway
  MCP/chat tools are still pending.
- Event publishing is partial: Capture returns an `ObservationCaptured` event,
  but `AttemptSubmitted` and stored/in-process event publication remain open.
- GrowthModel and PracticePlan domain drafts exist, but SleepCycle does not yet
  aggregate Trace into GrowthModel updates or generate next PracticePlans.
- `learning.stem` is a useful prior slice, but the next upstream product should
  be Dictation Coach; the dictation-specific capture, attempt, classification,
  next-practice, observation, and adapter issues are still open.
- Dictation Capture still needs a prerequisite `EvidenceRefInput` validation
  foundation before optional original-media provenance can enter Surface
  requests safely.
- No GitHub Release artifact is currently published, so Trial and Local
  One-click binary-first profiles still need release validation.
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

Status: mostly complete; remaining open issues are MemoryAtom draft,
CognitiveScene draft, and minimal Lens / Reflection Surface structures.

Recommended sequence:

1. Stabilize `Namespace` and existing `FeedbackLoop` contracts under the Engine
   vocabulary.
2. Define and persist `Trace`.
3. Define `MemoryAtom` and `CognitiveScene`.
4. Define `GrowthModel`.
5. Define `SleepCycle`.
6. Define `PracticePlan` / `DreamCandidate`.
7. Define minimal Lens / Reflection structures for Surface use.
8. Add serialization, repository, and same-Space validation tests.

Non-goals:

- No OCR.
- No cloud LLM dependency.
- No complex UI.
- No broad education platform.

## Milestone 3: Surface Gateway MVP

Goal: build the unified Engine entry point.

Status: partially complete. `SurfaceRequest` / `SurfaceResponse`, Capture,
Performance, and manual consolidation are implemented; Reflection, Planning,
and Observation mocks remain open.

Recommended sequence:

1. Define `SurfaceRequest` and `SurfaceResponse`.
2. Implement Capture Surface minimum path.
3. Implement Performance Surface minimum path.
4. Add Reflection Surface mock.
5. Add Planning Surface mock.
6. Add Observation Surface mock.
7. Ensure every Surface call writes Trace.
8. Add visibility / permission fields, initially with simple policy.
9. Add tests for routing, validation, response shaping, and Trace creation.

Non-goals:

- Do not replace all existing REST/MCP routes in one migration.
- Do not expose Engine internals as Surface responses.

## Milestone 4: Event + Sleep Engine MVP

Goal: make foreground paths fast and background paths deep.

Status: partially complete. Engine Event types and manual SleepCycle trigger
exist; event publication, GrowthModel aggregation, and PracticePlan generation
remain open.

Recommended sequence:

1. Define basic Engine Event model.
2. Publish `ObservationCaptured` after Capture Surface calls.
3. Publish `AttemptSubmitted` after Performance Surface calls.
4. Implement manual SleepCycle trigger.
5. Aggregate Trace evidence in SleepCycle.
6. Generate a simple GrowthModel update.
7. Generate a simple next PracticePlan.
8. Record SleepCycle status and Trace.
9. Add tests.

Non-goals:

- No scheduler before manual SleepCycle works.
- No distributed queue before in-process / stored events prove the shape.
- No cloud generation in the first path.

## Milestone 5: Dictation Coach Demo

Goal: validate the full loop with a daily dictation product scenario.

Status: not yet usable as an app. The product contract exists, but the
dictation-specific implementation path is still open.

Initial namespaces:

- `child.chinese.dictation`
- `child.english.spelling`
- `child.english.sentence-dictation`

Execution dependency graph:

| Prerequisites | Unlocks |
| --- | --- |
| Foundation F1 | Issues 5.2 and 5.3 |
| Issues 3.4, 3.5, 3.6, and Foundation F1 | Issue 6.2 generic MCP/chat Surface Adapter |
| Issues 5.2 through 5.6 and Issue 6.2 | Issue 5.7 Dictation Agent demo |

These edges are acyclic: Foundation F1 and the three remaining generic Surface
mocks feed Issue 6.2, while Issue 5.7 waits for both the dictation capabilities
and the generic adapter. Live GitHub issue and Foundation F1 synchronization is
Task 5 after these documentation changes merge; do not update GitHub in this
task.

Recommended sequence:

1. Land Foundation F1 to validate provider-neutral `EvidenceRefInput`
   descriptors for only `capture_observation` and `submit_attempt` before
   Dictation Capture can accept optional media references. Derive Space
   ownership from the authorized Surface context rather than caller input.
   Reject an entire reference when its locator or metadata contains any secret;
   redact only diagnostics and log messages, and never write rejected raw
   payloads or secrets to logs, Trace, metadata persistence, or any persistence.
   Keep references optional.
2. Capture today's confirmed word, phrase, or sentence list.
3. Submit confirmed dictation / spelling result.
4. Classify mistakes deterministically from text.
5. Generate tomorrow's 10-minute practice.
6. Show simple 7-day trends.
7. Run manual SleepCycle over dictation traces.
8. Generate weekly review.
9. After the generic MCP/chat Surface Adapter foundation in Issue 6.2 lands,
   expose the first usable Dictation test through its generic Surface Gateway
   tools as Issue 5.7.

The initial smoke uses one learner and manually entered or Agent-confirmed text.
An Agent/App performs OCR or ASR when media is involved and must obtain explicit
user acceptance or correction for every media-derived normalized payload before
submission. The smoke has no dedicated Dictation Coach App dependency.

A future dedicated Dictation Coach App belongs in a separate repository only
after the Agent loop works. It remains a Surface Gateway / MCP client and does
not own memory or access Engine internals.

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

Status: pending. MCP has compatibility tools for memory, lenses, profiles,
reminders, and namespace practice sessions, but it does not yet expose the new
Surface Gateway dictation flow.

Recommended adapter sequence:

1. Define allowed surfaces per adapter.
2. Implement the generic MCP/chat Surface Adapter foundation in Issue 6.2,
   including transport/tool plumbing and generic mappings for Capture,
   Performance, Reflection, Planning, and Observation.
3. Build the product-facing Dictation Agent orchestration in Issue 5.7 on that
   generic adapter; this remains the first user-facing usable flow.
4. Simple Practice App Adapter: can access Performance, Planning, and limited
   Observation.
5. Dashboard Adapter: read-only Trace, GrowthModel, SleepCycle, and debug views.
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
- #128 / #129 / #130 install and deployment issues -> supporting distribution
  track, not core Engine roadmap.

## Supporting Distribution Track

These are important but not on the core Engine critical path:

- #128 Publish first Local One-click offline release bundle.
- #129 Make Trial Profile plug-and-play for agent demos.
- #130 Stand up a real Production Profile deployment and smoke test.

They support external usage once the Engine and Surface Gateway are useful.

## Issue Hygiene

When creating or updating GitHub Issues from [docs/issues.md](issues.md):

1. Use the milestone names from this roadmap.
2. Keep acceptance criteria concrete.
3. Include non-goals.
4. Name likely files.
5. Preserve Rust-first and CognitiveSpace ownership boundaries.
6. Do not mix multiple milestones into one worker task.
