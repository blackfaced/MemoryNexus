# Surfaces And Adapters

MemoryNexus separates Engine, Surfaces, and Adapters.

```text
Adapter = how a human or agent interacts
Surface = what intent/capability is requested
Engine  = how long-term memory, feedback, and growth evolve
```

This distinction prevents product roles from leaking into the core architecture.
For example, "parent app" and "child app" are adapters, not surfaces. A chat
agent is an adapter, not the owner of memory.

## Surfaces

Surfaces are capability and intent boundaries exposed by Surface Gateway.

### Capture Surface

Answers:

```text
What happened?
```

Used for observations, thoughts, source materials, and external events.

Example actions:

- `capture(namespace, content, source)`
- `captureObservation(namespace, content)`

### Performance Surface

Answers:

```text
How did the attempt go?
```

Used for evaluable performance: dictation result, spelling answer, piano audio,
chess game, drawing, programming answer, or project deliverable.

Example actions:

- `submitAttempt(namespace, task, attempt)`
- `evaluateAttempt(namespace, attempt)`
- `getImmediateFeedback(namespace, attempt)`

### Reflection Surface

Answers:

```text
What does this mean?
```

Used for mistake explanation, pattern analysis, trend review, tension discovery,
and reflective insight.

Example actions:

- `reflect(namespace, question, mode)`
- `review(namespace, timeframe)`
- `explain(namespace, target)`

### Planning Surface

Answers:

```text
What should happen next?
```

Used for tomorrow's practice, next task generation, revision plans, and project
next steps.

Example actions:

- `plan(namespace, goal, constraints)`
- `generateNextTask(namespace)`
- `adjustPlan(namespace, feedback)`

### Observation Surface

Answers:

```text
How is long-term state changing?
```

Used to inspect GrowthModel, trends, mastery, error distribution, SleepCycle
outputs, and timelines.

Example actions:

- `observeState(namespace)`
- `getGrowthModel(namespace)`
- `getTrends(namespace)`
- `getTimeline(namespace)`

## Adapters

Adapters are concrete interaction channels.

Examples:

- Chat Agent
- Mobile App
- Web App
- MCP Tool
- CLI
- Dashboard
- IDE Plugin
- Voice Assistant

One adapter may access multiple surfaces. One surface may be used by many
adapters.

Examples:

- Dictation practice app: Performance + Planning + limited Observation.
- Parent / coach / user agent: Capture + Performance + Reflection + Planning +
  Observation.
- Developer dashboard: read-only Observation plus Engine debug views.
- CLI: any surface needed for smoke tests and developer workflows.
- MCP: agent access through Gateway, not direct Engine mutation.

## Adapter Capability Policy

Adapter capability is expressed as allowed Surface Gateway surfaces and actions,
not as direct repository access, Engine object mutation, or product roles.

All adapters must preserve these invariants:

- `CognitiveSpace` is the ownership and permission boundary.
- `Namespace` is only a domain partition inside a Space.
- Adapter roles such as parent, learner, coach, developer, or agent stay in
  adapter copy and caller context; they do not become Engine roles.
- Adapters call Surface Gateway. They do not directly access Engine
  repositories or mutate `Trace`, `FeedbackLoop`, `GrowthModel`, `SleepCycle`,
  `PracticePlan`, `MemoryAtom`, `CognitiveScene`, or Lens internals.
- Product-facing responses should expose shaped summaries, feedback, next
  actions, and provenance handles. Raw Engine records and internal IDs are
  developer/debug-only unless a future contract explicitly says otherwise.
- OCR, ASR, media capture, transcript preparation, and user confirmation are
  Adapter responsibilities. MemoryNexus accepts confirmed normalized text plus
  optional provider-neutral evidence descriptors where the Surface contract
  allows them.

Visibility levels:

| Level | Audience | Safe Content | Not Safe By Default |
| --- | --- | --- | --- |
| `learner` | Product learner / end user | Current task, submitted text, immediate feedback, mistake explanation, next practice, short trend summaries. | Raw Trace payloads, GrowthModel records, SleepCycle details, repository IDs, debug diagnostics. |
| `coach` | Product coach / guardian / teacher copy | Learner-safe content plus higher-level patterns, focus areas, practice plan summaries, and review prompts. | Engine role claims, cross-Space data, raw consolidation artifacts. |
| `developer` | CLI, local developer, integration operator | Request/response envelopes, generated Trace IDs, validation diagnostics, adapter policy decisions, selected debug summaries. | Secrets, media locators with credentials, raw private records unrelated to the current Space. |
| `debug` | Explicit diagnostics / dashboard debug mode | Narrow Engine provenance for troubleshooting, behind authorization and explicit debug affordances. | Ordinary product UI contract; any debug field that implies adapter ownership of memory. |
| `internal` | Engine implementation only | Repository records, mutable domain objects, consolidation state. | Adapter responses unless deliberately shaped by Surface Gateway. |

### Chat Agent

| Policy Area | Capability |
| --- | --- |
| Allowed Surfaces | Capture, Performance, Reflection, Planning, Observation. |
| Representative Actions | `capture_observation`, `submit_attempt`, `review_evidence`, `generate_next_task`, `get_state_summary`. |
| Interaction Mode | Conversational orchestration. Usually `fast` for capture/submission, `focused` for review and planning, and `deep` only when the user explicitly asks for a deeper review. |
| Response Visibility | `learner` or `coach` by default, depending on adapter copy. May include concise provenance such as `generated_trace_id` only when useful for follow-up or troubleshooting. |
| Disallowed Internals | No direct Trace editing, GrowthModel mutation, SleepCycle triggering beyond Gateway-supported actions, repository reads, or raw Memory/Lens object exposure. |
| Media Boundary | If the chat agent receives image/audio/video, it performs OCR/ASR or delegates it outside MemoryNexus, shows normalized text to the user, and submits only explicitly accepted or corrected text. Optional evidence descriptors are request provenance only. |
| Dictation Notes | Can guide one learner through daily list capture, attempt submission, mistake explanation, tomorrow practice, and 7-day trend using generic Surface actions. Product copy may say learner or coach; Engine payloads remain role-neutral. |

### MCP

| Policy Area | Capability |
| --- | --- |
| Allowed Surfaces | Capture, Performance, Reflection, Planning, Observation. |
| Representative Actions | Generic Surface MCP tools map one-to-one to `capture_observation`, `submit_attempt`, `review_evidence`, `generate_next_task`, and `get_state_summary`. |
| Interaction Mode | Tool-call adapter for agents and automation. Defaults should be deterministic/local where possible and should keep product semantics in tool arguments, not in Engine entry points. |
| Response Visibility | `developer` for tool diagnostics and integration traces; `learner`/`coach` only when the calling agent is relaying shaped product content to a user. |
| Disallowed Internals | No product-specific Engine tools, direct database/repository access, arbitrary Trace insertion, GrowthModel patching, SleepCycle object mutation, or raw internal object dumps as normal MCP responses. |
| Media Boundary | MCP callers may pass confirmed media-derived text with allowed `input_confirmation` and optional evidence descriptors where supported. MCP does not imply OCR, ASR, resolver execution, or evidence storage. |
| Dictation Notes | The Dictation Agent demo uses MCP as a generic Surface Gateway adapter. Dictation-specific wording belongs in the agent prompt and payload semantics, not in separate Engine APIs. |

### CLI

| Policy Area | Capability |
| --- | --- |
| Allowed Surfaces | Capture, Performance, Reflection, Planning, Observation; additionally may expose explicit developer smoke or migration aids that are documented as non-product workflows. |
| Representative Actions | `capture_observation`, `submit_attempt`, `review_evidence`, `generate_next_task`, `get_state_summary`, and explicit health/smoke commands. |
| Interaction Mode | Developer and local operator workflow. CLI may be scriptable and verbose, but product-level examples should still demonstrate Surface Gateway access. |
| Response Visibility | `developer` by default. CLI may show generated Trace IDs, request validation errors, and debug summaries, but should label debug-only fields clearly. |
| Disallowed Internals | No CLI command should become the ordinary way for product adapters to bypass Surface Gateway, mutate Engine repositories, or expose raw private records across Spaces. |
| Media Boundary | CLI may submit typed, pasted, imported, or preconfirmed text. It should not claim media ingestion, OCR, ASR, resolver execution, or evidence descriptor durability unless a future issue implements that path. |
| Dictation Notes | Useful for deterministic Dictation Coach smoke tests and fixture replay. It should not encode parent/child permissions; Space membership and namespace routing remain the boundary. |

### Practice App

| Policy Area | Capability |
| --- | --- |
| Allowed Surfaces | Capture, Performance, Reflection, Planning, limited Observation. |
| Representative Actions | `capture_observation` for word/sentence lists, `submit_attempt` for dictation answers, `review_evidence` for immediate explanation, `generate_next_task` for tomorrow practice, `get_state_summary` for learner-safe trend summaries. |
| Interaction Mode | Product UI for repeated practice. Foreground interactions should be low-latency: capture list, submit attempt, show feedback, fetch next practice or recent trend. |
| Response Visibility | `learner` by default. `coach` summaries may exist in adapter copy, but the Engine response remains role-neutral. Show friendly task, mistake, focus, and trend language instead of Engine terms. |
| Disallowed Internals | No raw Trace timeline, GrowthModel document, SleepCycle state, PracticePlan internals, repository IDs, or Engine debug fields as ordinary product UI. No adapter-side role should become an Engine permission model. |
| Media Boundary | The app may capture photos/audio or run OCR/ASR, but it must obtain explicit acceptance or correction of normalized text before calling Surface Gateway. Media unavailable errors must not block a confirmed text flow. |
| Dictation Notes | This is the future #163 adapter path. It may support daily dictation, spelling attempts, mistake type explanations, tomorrow's 10-minute practice, and 7-day trends without building multi-child management or a broad education platform. |

### Dashboard

| Policy Area | Capability |
| --- | --- |
| Allowed Surfaces | Observation by default; Reflection and Planning only for explicit review/planning workflows; Capture and Performance only for controlled test fixtures or admin smoke paths. |
| Representative Actions | `get_state_summary`, `review_evidence`, `generate_next_task`, and explicitly marked smoke-only `capture_observation` / `submit_attempt` calls when needed. |
| Interaction Mode | Developer/admin/debug visibility for inspecting adapter behavior, Surface health, trends, and deterministic policy decisions. Not the primary learner product UI. |
| Response Visibility | `developer` or `debug` by default. A dashboard may show richer provenance than product adapters, but it must label debug-only fields and keep learner-safe summaries separate from diagnostic views. |
| Disallowed Internals | No arbitrary repository browser, cross-Space inspection, raw secret-bearing payloads, direct SleepCycle mutation, direct GrowthModel edits, or treating internal IDs as ordinary product contracts. |
| Media Boundary | May display evidence descriptor validation status when authorized. It must not imply MemoryNexus can resolve, persist, OCR, transcribe, or ingest referenced media unless that capability exists in a separate contract. |
| Dictation Notes | Useful for checking 7-day trend, mistake taxonomy output, and adapter request shaping. It should not become the Practice App, encode parent/child roles in Engine, or introduce a new frontend/backend stack without an ADR. |

## Dictation Coach Surface Contract

Dictation Coach is an upstream product scenario, not a new Engine role model.
Its first namespaces are:

```text
child.chinese.dictation
child.english.spelling
child.english.sentence-dictation
```

These names partition domain evidence inside a `CognitiveSpace`; they do not
create a new permission boundary. Adapter copy may describe a parent, learner,
teacher, or coach, but Surface and Engine payloads stay role-neutral.

Dictation Coach is text-first in the MVP. MemoryNexus feedback operates on
confirmed normalized text; an Agent/App Adapter may prepare that text from
media outside MemoryNexus:

```text
media -> Agent/App OCR or ASR -> user-confirmed text
  -> Surface Gateway(text + optional EvidenceRefInput)
  -> Engine feedback objects and Trace
```

- Capture records typed, pasted, imported, or Adapter-confirmed characters,
  words, phrases, or sentences, with optional `evidence_refs`.
- Performance submits typed, pasted, or Adapter-confirmed attempts against a
  captured task, with optional `evidence_refs`.
- Reflection explains deterministic mistake types and recurring patterns.
- Planning generates a short next practice from evidence.
- Observation summarizes 7-day trends, stability, current focus, and evidence.

Confirmed Surface text is canonical for feedback and deterministic
classification. Every media-derived normalized payload requires explicit user
acceptance or correction before submission. OCR/ASR confidence may guide how
the Adapter highlights or reviews text, but it never substitutes for
confirmation. Optional evidence references preserve provenance under the
[Media Evidence Contract](../media-evidence-contract.md); media availability
must not block or invalidate the completed text flow.

`EvidenceResolver` belongs to future optional integration/Adapter
infrastructure; this contract does not claim current resolver execution. A
future resolver may check availability or resolve authorized provider media,
but it performs no OCR, ASR, classification, reflection, planning, or other
cognitive analysis.

Conceptual mapping. Dictation-specific verbs belong in adapter copy or payload
semantics; Gateway actions use the generic `SurfaceAction` vocabulary.

| Surface | Gateway Action | Product / Payload Semantics | Trace Task Type | Output Shape |
| --- | --- | --- | --- | --- |
| Capture | `capture_observation` | Record today's dictation list. | `practice` | Confirmed text task plus optional media provenance and Trace. |
| Performance | `submit_attempt` | Submit confirmed dictation result. | `practice` | Confirmed text attempt, deterministic evaluation, optional media provenance, and immediate feedback. |
| Reflection | `review_evidence` | Explain current mistakes. | `feedback` | Mistake explanation, recurring patterns, and evidence IDs. |
| Planning | `generate_next_task` | Generate tomorrow practice. | `planning` | 10-minute `PracticePlan` targeting one or two mistake patterns. |
| Observation | `get_state_summary` | Show 7-day trend. | `review` | 7-day trend summary with recurring errors and evidence IDs. |

The full product contract is in
[Dictation Coach MVP](../dictation-coach-mvp.md).

## Adapter Rules

- Adapters do not own memory.
- Adapters should not bypass Surface Gateway.
- Adapters should not directly mutate Engine internals.
- Adapter permissions should be expressed as allowed surfaces and actions.
- Adapter copy can be role-specific, but Engine and Surface names remain generic.
- Agent-facing product-specific tools still map to generic Surface actions;
  they do not create product-specific Engine entry points.

## Surface Rules

- Surfaces are not UI screens.
- Surfaces are not user roles.
- Surfaces are not database schemas.
- Surfaces shape request/response semantics and trace generation.
- Surfaces must preserve namespace and CognitiveSpace provenance.
