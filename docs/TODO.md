# MemoryNexus Roadmap

> Last updated: 2026-06-22
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
  consolidation. Reflection #146 is implemented in PR #176 but remains pending
  acceptance and merge; Planning (#147), Observation (#148), adapter policy,
  and dictation-specific flows remain open.
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
- Typed or pasted Dictation Capture can proceed once the generic Surfaces land.
  Media-derived Capture still needs #175 before `agent_ocr`,
  `agent_transcribed`, `mixed`, or optional original-media provenance can enter
  Surface requests safely.
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

Status: partially complete. `SurfaceRequest` / `SurfaceResponse`, Capture,
Performance, and manual consolidation are implemented. Reflection (#146) is
implemented in PR #176 but remains pending review, PostgreSQL integration
verification, and merge. Planning (#147) must not start until that verification
passes and #146 lands; Observation (#148) follows #147.

Recommended sequence:

1. Define `SurfaceRequest` and `SurfaceResponse`.
2. Implement Capture Surface minimum path.
3. Implement Performance Surface minimum path.
4. Review PR #176 for Reflection (#146), run its PostgreSQL integration test,
   then merge and close #146.
5. Add Planning Surface (#147) only after #146 lands.
6. Add Observation Surface (#148) only after #147 lands.
7. Ensure every Surface call writes Trace.
8. Add visibility / permission fields, initially with simple policy.
9. Add tests for routing, validation, response shaping, and Trace creation.

In parallel with #147 and #148, implement #177 to add a required pull-request
CI job for PostgreSQL-backed Surface integration tests. Pin service versions,
enumerate ignored integration tests explicitly, and fail if any enumerated test
file is not executed. Network or external-provider checks stay manual or
scheduled so they cannot make the deterministic merge gate flaky.

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

These edges are acyclic. The serialized shared-dispatcher chain begins
`#146 -> #147 -> #148`; any issue that edits `src/api/surfaces.rs` takes
dispatcher integration ownership only after its predecessor lands. After #148,
the typed/pasted product path and generic text-capable #162 tools may proceed
without #175. In parallel, #175 opens only the media extensions. Domain-specific
workers remain isolated from shared dispatcher ownership until their dependency
lands.

Recommended sequence:

1. Review and merge #146 only after its PostgreSQL integration verification,
   then implement #147 and #148 in order. Run #177 required PostgreSQL Surface
   integration CI work in parallel with #147/#148; keep external-provider
   checks manual or scheduled.
2. After #148, implement #155 typed/pasted word-list Capture, then #156
   typed/pasted attempt submission. These text-first paths accept genuinely
   typed or pasted text and are not blocked by #175.
3. After #148, implement the text-capable portion of #162 as generic MCP/chat
   tools for Capture, Performance, Reflection, Planning, and Observation.
4. In parallel after #148, implement #175 to define the generic role-neutral
   `input_confirmation: { status: "confirmed", method: "explicit_acceptance" |
   "explicit_correction" }` field and validate it for `agent_ocr`,
   `agent_transcribed`, and `mixed` input. #175 also validates provider-neutral
   `EvidenceRefInput` descriptors for only `capture_observation` and
   `submit_attempt`. Follow #175 and the
   [media evidence contract](media-evidence-contract.md) for the exact closed V1
   validation algorithm. An unsafe reference is rejected in full; diagnostics
   are redacted, and rejected payloads or secrets enter no logs, Trace, metadata
   persistence, or other persistence. Accepted descriptors remain ephemeral and
   are excluded from existing Memory, FeedbackLoop, and Trace persistence.
5. Keep every `agent_ocr`, `agent_transcribed`, `mixed`, and `evidence_refs`
   path in #155, #156, and #162 blocked until #175 lands. Typed/pasted delivery
   must not relabel media-derived content to bypass confirmation.
6. After #156, implement #157 deterministic mistake classification, then #152
   Trace/FeedbackLoop aggregation into GrowthModel, #153 evidence-linked
   PracticePlan generation, and #158 tomorrow's focused ten-minute practice.
   #158 reuses #153 rather than creating a parallel planning model.
7. Once #155 through #158 and the text-capable portion of #162 land, run the
   initial #160 Agent smoke through generic Surface Gateway MCP tools. It covers
   word-list Capture, attempt submission, Reflection, mistake analysis, next
   practice, Observation, and Trace IDs without a web UI or dedicated App.
8. Complete #159 with deterministic multi-day fixtures, then add its seven-day
   trend to the extended Agent acceptance. #159 does not block initial #160.
9. Develop #150 event publication in parallel; it does not block the first
   manual Agent loop.

The initial #160 smoke uses one learner and genuinely typed or pasted text.
It does not require #175, OCR, ASR, evidence references, seven days of real
history, a tagged release, or a dedicated Dictation Coach App. When media is
involved, the Agent/App performs OCR or ASR and must obtain explicit user
acceptance or correction before submission; all such media-derived paths remain
blocked by #175. Media capture and confirmation stay in the Adapter, while the
Engine remains text-first.

#175 owns the generic role-neutral `input_confirmation` request field and
validation, including negative tests for a missing field, unconfirmed status,
and invalid method. #155 and #156 validate it at the Surface boundary only for
media-derived input. #162 maps the already-defined field for its media extension
without gaining Engine repository access. #160 owns only the product
prompt/interaction that obtains acceptance or correction; no parent/child role
enters the Engine.

#163 is the deferred Simple Practice App Adapter. It belongs in a separate
Dictation Coach App repository only after the initial #160 Agent loop is
accepted. It remains a Surface Gateway / MCP client and does not own memory or
access Engine internals.

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
2. Implement #162 generic MCP/chat Surface tools after #148, including
   transport/tool plumbing and text mappings for Capture, Performance,
   Reflection, Planning, and Observation. Add media confirmation and evidence
   mapping only after #175.
3. Build the product-facing #160 Dictation Agent orchestration on the
   text-capable #162 tools; this remains the first user-facing usable flow.
4. Only after the initial #160 Agent loop is accepted, implement #163, the
   deferred Simple Practice App Adapter for Performance, Planning, and limited
   Observation, in its separate repository.
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
- #128 / #129 / #130 install and deployment issues -> post-#160 distribution
  wave, not the initial Agent-smoke critical path.
- #177 required PostgreSQL Surface integration CI -> Milestone 3 enabler in
  parallel with #147/#148.

## Supporting Distribution Track

These start only after the initial #160 Agent acceptance and are not on its
critical path:

- #128 Publish first Local One-click offline release bundle.
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
