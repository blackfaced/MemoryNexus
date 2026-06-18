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
