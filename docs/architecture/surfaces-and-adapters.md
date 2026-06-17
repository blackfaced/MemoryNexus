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

Dictation Coach uses manual input only in the MVP:

- Capture records typed or pasted characters, words, phrases, or sentences.
- Performance submits typed or pasted attempts against a captured task.
- Reflection explains deterministic mistake types and recurring patterns.
- Planning generates a short next practice from evidence.
- Observation summarizes 7-day trends, stability, current focus, and evidence.

Conceptual mapping. Dictation-specific verbs belong in adapter copy or payload
semantics; Gateway actions use the generic `SurfaceAction` vocabulary.

| Surface | Gateway Action | Product / Payload Semantics | Trace Task Type | Output Shape |
| --- | --- | --- | --- | --- |
| Capture | `capture_observation` | Record today's dictation list. | `practice` | Manual task list plus Trace provenance. |
| Performance | `submit_attempt` | Submit manual dictation result. | `practice` | Attempt, deterministic evaluation, and immediate feedback. |
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

## Surface Rules

- Surfaces are not UI screens.
- Surfaces are not user roles.
- Surfaces are not database schemas.
- Surfaces shape request/response semantics and trace generation.
- Surfaces must preserve namespace and CognitiveSpace provenance.
