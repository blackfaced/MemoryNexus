# STEM Learning Feedback MVP PRD

> Scope: issue #59. This is a narrow product PRD, not an implementation of the
> Rust APIs or UI.

## Positioning

STEM Learning Feedback is the first product MVP candidate for MemoryNexus.
Thought Review remains the reflective demo and presentation entry point;
`learning.stem` tests whether the same Space-owned memory foundation can become
a concrete long-term feedback engine for skill practice.

The first validation task is parent-assisted feedback for elementary fraction
word problems. The product should help a parent and learner see what was
practiced, what answer was given, what mistake pattern appeared, what feedback
was useful, and what next exercise should come after it. The product direction
is not math-only; later STEM tasks can use the same practice loop when they have
clear acceptance criteria.

## Product Boundary

`CognitiveSpace` remains the ownership and permission boundary. Parents and
learners work inside a Space, and every Namespace, FeedbackLoop, practice
Memory, report, and later projection must be validated through Space membership.

`Namespace` is only a domain partition inside the Space. `learning.stem` means
the practice belongs to the STEM learning domain; it does not introduce separate
membership, authorization, or data ownership.

The MVP should use parent and learner language in the product surface:

- practice
- answer
- mistake pattern
- feedback
- next exercise
- weekly learning review

Backend lifecycle terms such as `MemoryAtom`, `CognitiveScene`,
`CognitiveProjection`, `Lens Run`, and `CognitiveState` may remain in
implementation docs, but they are not primary labels in the learning UI or API
responses aimed at a parent-assisted learning flow.

## Roles

### Parent

The parent starts and guides the practice session. They select the Space, create
or reuse the `learning.stem` namespace, enter the practice goal and task, record
the learner's answer or reasoning, review the evaluation, and use the weekly
learning review to decide what to practice next.

The first MVP assumes the parent is the authenticated user and writer. It does
not require a separate autonomous learner account.

### Learner

The learner solves the fraction word problems and may explain their reasoning.
The system records the learner's answer and reasoning as practice evidence, but
the learner is not expected to manage the product, configure goals, or interact
with a full tutoring chatbot.

## First Scenario

Elementary fraction word problems are the only first validation task.

Example:

```text
Goal: improve fraction word problems
Task: solve five fraction word problems and explain reasoning
Attempt: answers and rough reasoning submitted by the learner
Evaluation: two answers wrong because units were mixed before calculation
Feedback: label the unit for each number before choosing the operation
Adjustment: add a unit-labeling step before solving
Next task: solve three unit-conversion fraction word problems tomorrow
```

The first slice should work with parent-entered tasks and feedback. It does not
need generated exercises, adaptive curriculum selection, or a conversational
tutor.

## MVP User Flow

1. Parent logs in and selects a `CognitiveSpace`.
2. Parent opens the `learning.stem` practice area.
3. The app creates or reuses the `learning.stem` Namespace inside that Space.
4. Parent creates a practice session with a learning goal and fraction word
   problem task.
5. Learner completes the task outside or inside the UI.
6. Parent records the learner's answer and reasoning.
7. Parent or system records correctness, scoring notes, and the mistake pattern.
8. Parent or system adds feedback and a small adjustment strategy.
9. System suggests or parent records the next exercise.
10. The practice event is stored as Memory with FeedbackLoop provenance.
11. Parent opens the weekly learning review to see repeated mistake patterns,
    improvement signals, current focus, and suggested next practice.

## FeedbackLoop Mapping

`FeedbackLoop` remains the generic backend object. The learning product maps its
fields into concrete STEM practice language:

| FeedbackLoop field | STEM practice language | Example |
| --- | --- | --- |
| `goal` | learning goal | Improve fraction word problems |
| `task` | practice assignment | Solve five fraction word problems and explain each answer |
| `attempt` | learner answer / reasoning | Answers, scratch reasoning, or parent-entered summary |
| `evaluation` | correctness and scoring notes | 3/5 correct; wrong answers mixed cups and liters |
| `feedback` | mistake explanation | Label units before calculating |
| `adjustment` | changed strategy or scaffold | Add a unit-labeling step before solving |
| `next_task` | next exercise | Three unit-conversion fraction word problems tomorrow |

The first schema can keep these fields as plain text. More detailed scoring,
rubrics, artifacts, and generated exercise metadata should wait for concrete
acceptance needs.

## FeedbackLoop Event To Memory

A FeedbackLoop does not replace Memory. Each meaningful practice event should
become an immutable Memory record in the same `CognitiveSpace` and
`learning.stem` Namespace.

Minimum event capture:

- Source: FeedbackLoop create, attempt update, feedback update, or weekly review.
- Memory content: a parent-readable practice summary using goal, task, answer /
  reasoning, evaluation, mistake pattern, feedback, adjustment, and next
  exercise when available.
- Provenance: source type, FeedbackLoop ID, Namespace ID, Space ID, event kind,
  and source metadata.
- Permissions: writer and reader checks still use Space membership.
- Validation: FeedbackLoop, Namespace, generated Memory, and later report
  sources must all belong to the same Space.

This lets search, Lens Run, review reports, and later lifecycle work reason over
actual practice events without making FeedbackLoop the raw memory store.

## API And Slice Dependencies

The MVP should land through small slices rather than one broad learning product
change.

| Slice | Purpose | Depends on | Enables |
| --- | --- | --- | --- |
| #67 FeedbackLoop attempt patch | Allow recording the learner's answer and reasoning after session creation | Existing FeedbackLoop API | #69 attempt update |
| #68 FeedbackLoop event as Memory | Preserve practice events as Space-owned Memory with FeedbackLoop provenance | FeedbackLoop foundation; should use #67 when event capture includes attempts | #71 reports, search, future lifecycle work |
| #69 STEM learning practice session API | Product API over Namespace + FeedbackLoop for create, attempt, feedback, list, and get; current first-slice routes may still use `/learning/math` naming | #67; should integrate #68 for Memory capture | #70 UI, #73 MCP tools, #71 report inputs |
| #73 STEM learning MCP tools | Let Claw or another agent drive the practice flow without low-level HTTP calls; current first-slice tool names may still use `learning_math_*` | #69 | Claw end-to-end demo |
| Claw end-to-end demo | Validate the API and MCP flow before locking user-facing surfaces | #73 | #71 report contract and #70 UI shape |
| #71 weekly learning review report | Summarize recurring mistake patterns and next practice focus | #68, #69, and demo learnings | Parent weekly value loop |
| #70 parent-learner static UI slice | Rust-served parent-assisted practice UI | #69 and #71; should incorporate demo learnings | First user-facing learning product slice |

Suggested execution order:

```text
#67 -> #68 -> #69 -> #73
                 -> Claw end-to-end demo
                 -> #71
                 -> #70
```

The UI should wait until the practice API, MCP tools, demo learnings, and weekly
review report contract are clear enough to avoid locking the wrong parent-learner
workflow. The report API should not require a new frontend stack.

## UI Slice

The learning UI must follow the existing Rust-served static UI direction. It may
add a static file under `web/` and routes in `src/api/web.rs`, but it must not
introduce React, Vite, Next.js, a Node dev server, a BFF, or a second backend
line without a later ADR.

Minimum screens or panels:

- Login/session and Space selection using existing patterns.
- `learning.stem` practice entry.
- Create practice goal and fraction word problem task.
- Record answer / reasoning.
- Record or review mistake pattern, feedback, adjustment, and next exercise.
- Recent practice sessions.
- Weekly learning review when #71 is available.

Required UI states:

- loading
- empty
- validation errors
- API errors
- mobile and desktop responsive layout

Primary labels should be parent and learner friendly: practice, answer, mistake
pattern, feedback, next exercise, weekly learning review.

## Weekly Learning Review

The weekly learning review is the recurring value loop for the parent. It should
be generated for a `learning.stem` Namespace and date window inside one
`CognitiveSpace`.

Output fields:

- `practiced_topics`: topics practiced during the review window.
- `recurring_mistake_patterns`: repeated mistakes, misconception patterns, or
  reasoning gaps.
- `improvement_signals`: what improved compared with earlier attempts in the
  window.
- `current_focus`: the one or two highest-value focus areas for the next week.
- `suggested_next_practice`: concrete next exercises or task types.
- `source_practice_session_ids` or `source_feedback_loop_ids`: traceable source
  sessions.
- `provenance`: Space, Namespace, source Memory IDs, FeedbackLoop IDs, date
  window, and summary provider or deterministic fallback.

The report should use deterministic fallback output when no AI provider is
configured.

## Non-Goals

- Do not expand the first MVP into piano, chess, drawing, programming, or other
  non-STEM skill domains.
- Do not build a generic learning platform.
- Do not build a full tutoring chatbot.
- Do not build a curriculum marketplace.
- Do not add generated exercises or adaptive curriculum unless a later issue
  explicitly requires it.
- Do not require autonomous learner accounts in the first MVP.
- Do not expose MemoryAtom, CognitiveScene, CognitiveProjection, Lens Run, or
  CognitiveState as primary product labels.
- Do not move permissions from `CognitiveSpace` membership to Namespace.
- Do not introduce React, Vite, Next.js, a Node dev server, BFF, or second
  backend.

## Acceptance Summary

The MVP PRD is satisfied when the learning product is scoped to STEM Learning
Feedback, validates the loop with a parent-assisted fraction word problem task,
maps FeedbackLoop fields to STEM practice language, keeps practice events as
Memory, defines the API/MCP/UI slice dependencies, and specifies a weekly
learning review that is traceable to Space-owned practice sources.
