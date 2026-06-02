# Namespace and Feedback Loop Minimal Design

> Scope: Phase 5 issue #52 first stage.
> Status: design only. This document defines the smallest model and API plan; it
> does not implement migrations, repositories, or product UI.

## Goals

- Add a minimal `Namespace` model scoped inside a `CognitiveSpace`.
- Add a minimal `FeedbackLoop` model for long-running reflective and skill
  feedback cycles.
- Define how the Phase 5 memory lifecycle fits around Namespace and
  FeedbackLoop: `MemoryAtom`, `CognitiveScene`, and Lens-based
  `CognitiveProjection`.
- Preserve `CognitiveSpace` as the only ownership and permission boundary.
- Define how Memory, Lens Run, Review Report, and Profile keep namespace and
  feedback-loop provenance.
- Define `fast`, `focused`, and `deep` observe modes so lifecycle work does not
  force every interaction through the slow path.
- Split the follow-up implementation into small migration, API, and test issues.

## Non-Goals

- Do not move permissions from `CognitiveSpace` to `Namespace`.
- Do not turn `Namespace` into a second Space selector or membership model.
- Do not build the `learning.math` product UI in this issue.
- Do not implement multiple vertical products such as piano, chess, drawing, and
  math at the same time.
- Do not copy EverMemOS object names or product direction into MemoryNexus.
  EverMemOS is a lifecycle reference for agent memory; MemoryNexus remains a
  user-owned cognitive perspective and feedback-loop system.
- Do not require `MemoryAtom`, `CognitiveScene`, or `CognitiveProjection` to land
  in the same schema migration as Namespace and FeedbackLoop.
- Do not make deep multi-lens projection the default for every user input.
- Do not introduce a second backend. All future implementation belongs in the
  Rust + Axum service.

## Model Boundary

`CognitiveSpace` answers:

```text
Who owns this long-term cognitive space, and who may access it?
```

`Namespace` answers:

```text
Which long-running domain inside the Space does this object belong to?
```

`FeedbackLoop` answers:

```text
What goal, task, attempt, feedback, adjustment, and next task are being tracked
inside that domain?
```

`MemoryAtom` answers:

```text
Which minimal cognitive signals were extracted from a raw Memory?
```

`CognitiveScene` answers:

```text
Which atoms, reflections, concepts, beliefs, and contradictions are consolidating
into a long-running theme or practice field?
```

`CognitiveProjection` answers:

```text
How should a Lens reconstruct enough context for the current query from active
scenes, atoms, concepts, beliefs, and contradictions?
```

`ObserveMode` answers:

```text
Should this interaction use fast intuitive recall, focused single-lens
projection, or deep reflective consolidation?
```

The containment hierarchy is:

```text
CognitiveSpace
  -> Namespace
       -> FeedbackLoop
            -> Memory
            -> MemoryAtom
            -> CognitiveScene
            -> CognitiveProjection
            -> Lens Run
            -> Review Report
            -> Profile Snapshot
```

The permission check always starts from `space_id` and
`cognitive_space_members`. A user who cannot access the Space cannot access any
Namespace or FeedbackLoop inside it.

The lifecycle flow is:

```text
Experience / Thought / Practice
-> Memory
-> MemoryAtom
-> CognitiveScene
-> Lens-based CognitiveProjection
-> Reflection / Belief / Next Action
-> FeedbackLoop
```

This lifecycle has two runtime channels:

```text
System 1 / fast:
recent memories + pinned facts + high-salience scenes + compressed profile
-> low-latency response
-> optional async processing

System 2 / focused or deep:
atomization
-> scene update
-> cognitive projection
-> reflection / concept / belief / contradiction / next action
```

The first Phase 5 implementation should still start with Namespace and
FeedbackLoop schema foundation. MemoryAtom, CognitiveScene, and
CognitiveProjection should begin as design/prototype issues so their usefulness
can be validated before committing to permanent tables.

## Observe Modes

Future observe / projection APIs should support three modes:

### `fast`

Use for immediate conversation, lightweight capture, and practice-in-progress
feedback.

Allowed work:

- recent-memory retrieval
- pinned facts
- high-salience scenes
- compressed `CognitiveProfile` / `SkillProfile` priors
- optional async enqueue for later atomization or consolidation

Avoid:

- multi-lens projection
- synchronous belief update
- synchronous contradiction detection
- synchronous scene consolidation

### `focused`

Use for ordinary questions and single-step review.

Allowed work:

- one primary Lens
- a small number of active scenes / atoms / concepts
- short `CognitiveProjection` with provenance
- Reflection generation when useful

Avoid:

- full weekly-style consolidation
- unrelated Lens fan-out

### `deep`

Use for explicit weekly review, learning plan generation, project decisions, and
other user-triggered deep work.

Allowed work:

- multi-lens projection
- atomization
- scene update
- concept / belief update
- contradiction detection
- FeedbackLoop adjustment or next action generation

Deep mode may be slower, but it must return clear provenance so the user can see
which memories, atoms, scenes, and lenses shaped the result.

## Minimal Namespace Model

Table candidate: `namespaces`.

Required fields:

- `id uuid primary key`
- `space_id uuid not null references cognitive_spaces(id) on delete cascade`
- `name varchar not null`
- `kind varchar not null`
- `description text null`
- `created_by uuid not null references users(id) on delete cascade`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`

Constraints:

- Unique `(space_id, name)`.
- `kind` should start as one of `reflective` or `skill`.
- `name` is a stable dotted identifier, for example `personal.thoughts`,
  `project.memorynexus`, or `learning.math`.

Recommended validation:

- Trim whitespace.
- Require non-empty names.
- Allow lowercase letters, numbers, dots, underscores, and hyphens.
- Reject names that look like Space names but do not express a domain.

Default namespaces:

- New personal Spaces may eventually get `personal.thoughts`.
- This can be added lazily when the Thought Review flow first needs it, or by a
  follow-up migration/backfill.

## Minimal FeedbackLoop Model

Table candidate: `feedback_loops`.

Required fields:

- `id uuid primary key`
- `space_id uuid not null references cognitive_spaces(id) on delete cascade`
- `namespace_id uuid not null references namespaces(id) on delete cascade`
- `goal text not null`
- `task text not null`
- `attempt text null`
- `evaluation text null`
- `feedback text null`
- `adjustment text null`
- `next_task text null`
- `status varchar not null default 'active'`
- `created_by uuid not null references users(id) on delete cascade`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`

Initial status values:

- `active`
- `completed`
- `paused`

Validation:

- `namespace_id` must belong to the same `space_id`.
- `goal` and `task` must be non-empty.
- `status` must be one of the initial values.

The fields are intentionally plain text in the first schema. More structured
skill-specific scoring, rubric, due date, and attempt artifacts should wait for a
concrete `learning.math` acceptance flow.

## How FeedbackLoop Produces Memory

A FeedbackLoop should not replace Memory. It produces Memory records that remain
the raw cognitive material.

Minimal flow:

```text
FeedbackLoop goal/task
-> user or agent records an attempt/evaluation/feedback
-> Rust API creates one or more Memory rows in the same space and namespace
-> Memory stores the observable event or reflection text
-> FeedbackLoop keeps provenance to generated Memory IDs
```

First implementation options:

- Add `namespace_id` and nullable `feedback_loop_id` to `memories`.
- When creating or updating a FeedbackLoop, optionally create a Memory snapshot
  that captures the loop event in user-readable text.
- Store generated Memory IDs in a join table only if a single loop event needs to
  create multiple memories. Otherwise `memories.feedback_loop_id` is enough for
  the first phase.

Issue #68 first step:

- FeedbackLoop create and patch accept explicit opt-in memory capture through
  `capture_memory` (`create_memory_snapshot` is accepted as an alias).
- The generated Memory stays in the same `space_id`, uses the authenticated user
  as `user_id`, stores `memory_type = text`, keeps `is_shared = false`, and uses
  `source_type = feedback_loop_event`.
- `source_metadata` carries `feedback_loop_id`, `namespace_id`, `space_id`,
  `event_kind` (`create` or `patch`), and `included_fields`.
- Snapshot content uses parent-friendly practice language: practice goal,
  practice task, answer / reasoning, mistake pattern / evaluation, feedback,
  practice adjustment, and next exercise.
- Patch capture only writes a Memory when the patch includes at least one
  non-empty practice field. Status-only or whitespace-only patches do not create
  empty snapshots. Patch snapshots are event snapshots, so they include only the
  practice fields supplied by the current patch and do not repeat older loop
  fields.
- Space writer permission and same-Space Namespace validation happen before any
  Memory is created.
- FeedbackLoop create/update and generated Memory insert happen in one database
  transaction. Vector indexing, when configured, runs only after that transaction
  succeeds.

Longer-term recommended step:

- Add `namespace_id` and `feedback_loop_id` columns to `memories`.
- Keep the existing `space_id` on Memory as the permission boundary.
- Enforce `memories.namespace_id` belongs to `memories.space_id`.
- Enforce `memories.feedback_loop_id`, when present, belongs to the same Space
  and Namespace.

## Memory Lifecycle Prototype Boundary

The lifecycle objects should be validated with a small MemoryNexus project-note
fixture before they become database tables.

Prototype flow:

```text
10-20 project memories
-> 20-40 MemoryAtom candidates
-> 3-5 CognitiveScene candidates
-> several Lens-based CognitiveProjection examples for the same query
```

Useful fixture query:

```text
MemoryNexus 下一步该做什么？
```

Expected Lens examples:

- Product Lens should emphasize first action, ordinary-user entry, Thought
  Review magic moment, and `learning.math` as a possible concrete entry.
- Systems Lens should emphasize CognitiveSpace boundary, Namespace,
  FeedbackLoop provenance, MemoryAtom, and CognitiveScene.
- Learning Coach Lens should emphasize skill namespace loops, practice design,
  error patterns, and next practice.

Acceptance signal:

```text
Different Lens projections over the same query select meaningfully different
context while citing the scenes and atoms they used.
```

## Provenance for Derived Objects

### Lens Run

`lens_runs` should keep the namespace and feedback-loop scope that shaped the
run:

- Add nullable `namespace_id`.
- Add nullable `feedback_loop_id`.
- Keep existing `space_id`, `lens_id`, `input_memory_ids`, and output JSON.
- When a request specifies `namespace_id`, retrieval only considers memories in
  that namespace.
- When a request specifies `feedback_loop_id`, retrieval may narrow to memories
  produced by or associated with that loop.
- The output JSON should include a provenance block:

```json
{
  "provenance": {
    "space_id": "...",
    "namespace_id": "...",
    "feedback_loop_id": "...",
    "source_memory_ids": ["..."]
  }
}
```

### Review Report

`cognitive_review_reports` should support namespace-scoped review windows:

- Add nullable `namespace_id`.
- Add nullable `feedback_loop_id`.
- Keep `space_id`, `lens_id`, `source_memory_ids`, and `source_lens_run_ids`.
- List and create APIs should accept optional namespace and feedback-loop
  filters.
- Report JSON should expose the same provenance block as Lens Run.

### Profile Snapshot

`cognitive_profile_snapshots` should support domain-specific projections:

- Add nullable `namespace_id`.
- Add nullable `feedback_loop_id`.
- Keep `space_id`, `lens_id`, `target`, `source_memory_ids`, and
  `source_lens_run_ids`.
- A `target` such as `personal_context` can still work without namespace.
- A future `skill_context` or `learning_context` target should require a
  namespace filter.

## API Surface

### Namespace API

Minimum endpoints:

- `POST /api/v1/namespaces`
- `GET /api/v1/namespaces?space_id=<SPACE_ID>`
- `GET /api/v1/namespaces/:id`

Create request:

```json
{
  "space_id": "...",
  "name": "learning.math",
  "kind": "skill",
  "description": "Math practice feedback loop"
}
```

All endpoints must verify Space membership through `space_id`.

### FeedbackLoop API

Minimum endpoints:

- `POST /api/v1/feedback-loops`
- `GET /api/v1/feedback-loops?space_id=<SPACE_ID>&namespace_id=<NAMESPACE_ID>`
- `GET /api/v1/feedback-loops/:id`
- `PATCH /api/v1/feedback-loops/:id`

Create request:

```json
{
  "space_id": "...",
  "namespace_id": "...",
  "goal": "Improve fraction word problems",
  "task": "Complete five fraction word problems and explain each mistake",
  "capture_memory": true
}
```

Patch request can update `attempt`, `evaluation`, `feedback`, `adjustment`,
`next_task`, and `status`. `capture_memory` is an explicit opt-in flag; it
defaults to false. When enabled on patch, a Memory snapshot is created only if
the patch includes meaningful practice content.

## APIs That Need Namespace Filters

Add optional `namespace_id` filters to:

- `POST /api/v1/memories`
- `GET /api/v1/memories`
- `GET /api/v1/search`
- `GET /api/v1/search/suggest`
- `POST /api/v1/search/semantic`
- `GET /api/v1/search/semantic`
- `POST /api/v1/lenses`
- `GET /api/v1/lenses`
- `POST /api/v1/lens-runs`
- `GET /api/v1/lens-runs`
- `POST /api/v1/review-reports`
- `GET /api/v1/review-reports`
- `POST /api/v1/profiles`

Add optional `feedback_loop_id` filters only where they make immediate sense:

- `POST /api/v1/memories`
- `GET /api/v1/memories`
- `GET /api/v1/search`
- `POST /api/v1/lens-runs`
- `GET /api/v1/lens-runs`
- `POST /api/v1/review-reports`
- `POST /api/v1/profiles`

Every filter must be validated against the same Space. A request with
`space_id=A` and `namespace_id` from Space B should return `400 Bad Request`, not
silently broaden scope.

## Learning Math Candidate

`learning.math` is the first recommended Skill Namespace MVP candidate because it
has a clear loop:

```text
goal -> task -> attempt -> evaluation -> feedback -> adjustment -> next_task
```

Example:

```text
Goal: improve fraction word problems
Task: solve five problems and explain reasoning
Attempt: submitted answers and written reasoning
Evaluation: two wrong due to unit conversion
Feedback: check units before calculation
Adjustment: add a unit-labeling step
NextTask: solve three unit-conversion fraction problems tomorrow
```

This design does not implement a learning product UI. The first product surface
should be a narrow issue with acceptance criteria for one math practice flow.

## Follow-Up Issue Split

Recommended implementation sequence:

1. **Migration: Namespace foundation**
   Add `namespaces`, indexes, validation constraints, and a backfill or lazy
   creation plan for `personal.thoughts`.

2. **Rust API: Namespace CRUD**
   Add repository, request/response structs, Axum routes, permission checks, and
   unit/API tests for Space-scoped access.

3. **Migration: FeedbackLoop foundation**
   Add `feedback_loops`, indexes, same-Space constraints, and initial status
   validation.

4. **Rust API: FeedbackLoop CRUD**
   Add create/list/get/patch endpoints and tests for namespace ownership,
   status validation, and same-Space enforcement.

5. **Migration: Namespace provenance columns**
   Add nullable `namespace_id` and `feedback_loop_id` columns to memories,
   lenses, lens_runs, cognitive_review_reports, and cognitive_profile_snapshots
   where needed.

6. **API filters and retrieval**
   Thread `namespace_id` through memory list/create, keyword search, semantic
   search, lens list/create, Lens Run, Review Report, and Profile creation.

7. **FeedbackLoop-to-Memory capture**
   Decide and implement the first event-to-Memory behavior, with tests proving
   generated Memory rows remain Space-owned and namespace-scoped.

8. **Acceptance tests**
   Add end-to-end tests for creating a Space, creating `learning.math`, creating
   a FeedbackLoop, capturing Memory from the loop, running a Lens over that
   namespace, and generating a namespace-scoped Review Report/Profile.

9. **Learning Math MVP design**
   Create a separate issue for the first Skill Namespace product flow. Do not
   combine it with schema foundation work.

## Test Strategy

Each implementation issue should include tests before production code:

- Model serde and validation tests for namespace names, kind, status, and
  same-Space rules.
- Repository tests for list filters and inaccessible Space behavior.
- API tests for unauthorized Space access and cross-Space namespace mismatch.
- Search/Lens/Review/Profile tests proving namespace filters narrow results.
- Migration smoke tests through `cargo test` and, where service dependencies are
  required, the existing service-backed acceptance pattern.

## Compatibility

Existing Thought Review behavior should continue without requiring callers to
send `namespace_id`. When omitted, APIs should use current Space-scoped behavior.
Only new namespace-aware surfaces should require explicit namespace selection.
