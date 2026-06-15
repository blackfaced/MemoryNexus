# Sleep Cycle Contract

`SleepCycle`, `ConsolidationResult`, and `DreamCandidate` define the first
internal contract for Sleep-based Memory Consolidation from
[ADR-017](../decisions/ADR-017-sleep-based-memory-consolidation.md).

This is a docs-only contract. It is not a database migration, API schema, job
scheduler, model runtime, or product UI specification.

## Goals

- Give Sleep and Dreaming workers one shared object contract before schema or
  API work begins.
- Keep all Sleep / Dreaming objects scoped by `CognitiveSpace`.
- Treat `Namespace` as a domain partition inside a Space, not as a permission
  boundary.
- Define the minimum fields needed to connect Sleep outputs to `Trace`,
  `Memory`, `FeedbackLoop`, `CognitiveScene`, `GrowthModel`, `ReviewReport`,
  and later effectiveness evaluation.
- Keep the first implementation path deterministic and local-first.

## Non-Goals

- Do not add database migrations in this contract.
- Do not add Rust schema, repository, API, CLI, or MCP implementation here.
- Do not add a scheduler.
- Do not call cloud AI providers.
- Do not implement model training, fine-tuning, distillation, RL
  self-modification, model catalog, local inference runtime, or local
  accelerator management.
- Do not build `learning.stem` product UI in this contract.
- Do not expose Sleep or Dreaming as ordinary user-facing labels.

## Ownership And Scope

Every object in this contract must carry `space_id`.

```text
CognitiveSpace
  |-- Namespace*
  |-- Trace*
  |-- Memory*
  |-- FeedbackLoop*
  |-- CognitiveScene*
  |-- GrowthModel*
  |-- ReviewReport*
  |-- SleepCycle*
  |-- ConsolidationResult*
  `-- DreamCandidate*
```

Rules:

- `space_id` is the ownership and permission boundary.
- `namespace_id` is optional only for cross-namespace or Space-level
  consolidation. When present, it partitions the work inside the owning Space.
- Linked objects must belong to the same `CognitiveSpace`.
- Cross-namespace evidence is allowed only when the SleepCycle records why it
  needs it.
- Implementations must validate same-Space links before persisting or returning
  these objects.

## SleepCycle

`SleepCycle` represents one offline consolidation run over a bounded evidence
window. It groups inputs, status, runtime traceability, and generated outputs.

SleepCycle answers:

```text
Which evidence window was consolidated, under which domain, and what did it
produce?
```

### Conceptual Shape

```text
SleepCycle {
  id
  space_id
  namespace_id?
  cycle_type
  status
  evidence_window_start
  evidence_window_end
  input_trace_ids
  input_memory_ids
  input_feedback_loop_ids
  input_review_report_ids
  generated_consolidation_result_ids
  generated_dream_candidate_ids
  generated_memory_ids
  generated_cognitive_scene_ids
  updated_cognitive_scene_ids
  updated_growth_model_ids
  triggering_trace_id?
  error?
  started_at?
  completed_at?
  metadata
}
```

### Fields

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable UUID. |
| `space_id` | yes | Owning `CognitiveSpace`; all permissions remain Space-based. |
| `namespace_id` | no | Domain partition such as `learning.stem`; optional for Space-level consolidation. |
| `cycle_type` | yes | `daily`, `weekly`, or `manual`. |
| `status` | yes | `pending`, `running`, `completed`, `failed`, or `cancelled`. |
| `evidence_window_start` | yes | Inclusive start timestamp for evidence considered by the cycle. |
| `evidence_window_end` | yes | Exclusive end timestamp for evidence considered by the cycle. |
| `input_trace_ids` | no | Trace records read as runtime and feedback evidence. |
| `input_memory_ids` | no | Memory records read or cited by the consolidation. |
| `input_feedback_loop_ids` | no | FeedbackLoop records read for practice, attempt, evaluation, feedback, or next task evidence. |
| `input_review_report_ids` | no | Prior review reports used as context or comparison. |
| `generated_consolidation_result_ids` | no | ConsolidationResult records produced by this cycle. |
| `generated_dream_candidate_ids` | no | DreamCandidate records produced directly by this cycle, if Dreaming is part of the run. |
| `generated_memory_ids` | no | Optional Memory records created to preserve human-readable summaries. |
| `generated_cognitive_scene_ids` | no | CognitiveScene records created from consolidated evidence. |
| `updated_cognitive_scene_ids` | no | Existing CognitiveScene records updated by the cycle. |
| `updated_growth_model_ids` | no | GrowthModel or SkillProfile-like records updated by the cycle when that model exists. |
| `triggering_trace_id` | no | Trace for the manual/background/CLI/MCP execution that started this cycle. |
| `error` | no | Redacted failure class/message. |
| `started_at` | no | Timestamp when processing started. |
| `completed_at` | no | Timestamp when processing completed or failed. |
| `metadata` | no | Small structured extension point; not a place for raw provider payloads. |

### Cycle Type

```text
daily
weekly
manual
```

- `daily`: bounded daily evidence window, usually used for incremental
  consolidation.
- `weekly`: broader review window, usually used for longer patterns and review
  preparation.
- `manual`: explicitly triggered by a user, developer, CLI, MCP tool, or test.

This contract does not define a scheduler. `daily` and `weekly` describe the
cycle type and evidence window, not how the cycle is triggered.

### Status

```text
pending
running
completed
failed
cancelled
```

The first implementation may create only completed or failed records if the run
is synchronous. Longer-running workers can later use `pending` and `running`.

## ConsolidationResult

`ConsolidationResult` is the stable output of one SleepCycle over a specific
evidence window. It records patterns, contradictions, improvement signals, and
next-action hints with provenance back to the evidence that supported them.

ConsolidationResult answers:

```text
After consolidating this evidence, what stable pattern or growth signal should
the system carry forward?
```

### Conceptual Shape

```text
ConsolidationResult {
  id
  space_id
  namespace_id?
  sleep_cycle_id
  result_type
  summary
  evidence_trace_ids
  evidence_memory_ids
  evidence_feedback_loop_ids
  evidence_cognitive_scene_ids
  evidence_review_report_ids
  generated_memory_ids
  generated_cognitive_scene_ids
  updated_cognitive_scene_ids
  updated_growth_model_ids
  generated_review_report_ids
  detected_patterns
  detected_contradictions
  improvement_signals
  evidence_gaps
  next_actions
  confidence
  created_at
  metadata
}
```

### Fields

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable UUID. |
| `space_id` | yes | Owning `CognitiveSpace`; must match the source SleepCycle. |
| `namespace_id` | no | Domain partition; should match the SleepCycle unless the result is explicitly cross-namespace. |
| `sleep_cycle_id` | yes | Source SleepCycle. |
| `result_type` | yes | `pattern_summary`, `growth_update`, `scene_update`, `contradiction_update`, `review_input`, or `mixed`. |
| `summary` | yes | Short human-readable consolidation summary. |
| `evidence_trace_ids` | no | Trace records supporting this result. |
| `evidence_memory_ids` | no | Memory records supporting this result. |
| `evidence_feedback_loop_ids` | no | FeedbackLoop records supporting this result. |
| `evidence_cognitive_scene_ids` | no | CognitiveScene records read or updated. |
| `evidence_review_report_ids` | no | ReviewReport records used as prior context. |
| `generated_memory_ids` | no | Memory records created from this result, if any. |
| `generated_cognitive_scene_ids` | no | New CognitiveScene records created from this result. |
| `updated_cognitive_scene_ids` | no | Existing CognitiveScene records updated by this result. |
| `updated_growth_model_ids` | no | GrowthModel records updated by this result when that model exists. |
| `generated_review_report_ids` | no | ReviewReport records generated from this result. |
| `detected_patterns` | no | Structured list of repeated themes, mistake patterns, habits, or skill signals. |
| `detected_contradictions` | no | Structured list of tensions or conflicting evidence. |
| `improvement_signals` | no | Structured list of progress signals, regressions, or trend changes. |
| `evidence_gaps` | no | Missing evidence that limits confidence. |
| `next_actions` | no | Candidate follow-up actions before DreamCandidate generation. |
| `confidence` | no | Best-effort 0.0 to 1.0 confidence score or qualitative equivalent. |
| `created_at` | yes | Timestamp when the result was created. |
| `metadata` | no | Small structured extension point. |

### Result Type

```text
pattern_summary
growth_update
scene_update
contradiction_update
review_input
mixed
```

`ConsolidationResult` may reference future `GrowthModel` records, but this
contract does not require a GrowthModel schema. Until GrowthModel exists,
implementations can keep growth updates as summaries and linked evidence.

## DreamCandidate

`DreamCandidate` is a candidate next step generated from a ConsolidationResult.
It is not proven useful until a later Wake path selects, executes, and evaluates
it through new Trace and FeedbackLoop evidence.

DreamCandidate answers:

```text
What should the next practice, review question, scenario, contradiction
exploration, or planning prompt try?
```

### Conceptual Shape

```text
DreamCandidate {
  id
  space_id
  namespace_id?
  source_sleep_cycle_id
  source_consolidation_result_id
  purpose
  title
  content
  rationale
  expected_effect
  target_feedback_loop_id?
  target_cognitive_scene_id?
  target_growth_model_id?
  status
  selected_at?
  executed_trace_ids
  evaluation_trace_ids
  evaluation_result?
  created_at
  metadata
}
```

### Fields

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable UUID. |
| `space_id` | yes | Owning `CognitiveSpace`; must match source objects. |
| `namespace_id` | no | Domain partition; usually matches the source ConsolidationResult. |
| `source_sleep_cycle_id` | yes | SleepCycle that created or authorized this candidate. |
| `source_consolidation_result_id` | yes | ConsolidationResult that supplied the candidate rationale. |
| `purpose` | yes | One of the allowed DreamCandidate purposes below. |
| `title` | no | Short label for UI, CLI, or report surfaces. |
| `content` | yes | Candidate exercise, prompt, scenario, question, or plan text. |
| `rationale` | no | Why this candidate follows from the source consolidation. |
| `expected_effect` | no | What the candidate is expected to improve, test, or reveal. |
| `target_feedback_loop_id` | no | FeedbackLoop this candidate is intended to continue or adjust. |
| `target_cognitive_scene_id` | no | CognitiveScene this candidate is intended to explore or update. |
| `target_growth_model_id` | no | GrowthModel this candidate is intended to evaluate or improve. |
| `status` | yes | `proposed`, `selected`, `executed`, `evaluated`, `rejected`, or `expired`. |
| `selected_at` | no | Timestamp when the candidate was selected for use. |
| `executed_trace_ids` | no | Traces from Wake interactions that executed this candidate. |
| `evaluation_trace_ids` | no | Later traces used to judge whether it helped. |
| `evaluation_result` | no | Best-effort summary such as useful, neutral, harmful, or inconclusive. |
| `created_at` | yes | Timestamp when the candidate was created. |
| `metadata` | no | Small structured extension point. |

### Purpose

```text
practice_generation
scenario_simulation
contradiction_exploration
review_question
planning_prompt
```

- `practice_generation`: creates a next exercise or practice task.
- `scenario_simulation`: creates a simulated situation to rehearse or inspect.
- `contradiction_exploration`: asks the user or system to examine a tension.
- `review_question`: creates a question for reflection or weekly review.
- `planning_prompt`: creates a prompt for project, learning, or next-step
  planning.

### Status

```text
proposed
selected
executed
evaluated
rejected
expired
```

The Candidate status deliberately separates generation from effectiveness. A
candidate should only become `evaluated` after later Trace or FeedbackLoop
evidence exists.

## Relationships

```text
Trace
  |-- may trigger SleepCycle
  |-- may be input evidence for ConsolidationResult
  |-- may record DreamCandidate generation
  `-- may evaluate DreamCandidate effectiveness

SleepCycle
  |-- reads Trace / Memory / FeedbackLoop / ReviewReport
  |-- produces ConsolidationResult
  |-- may create or update CognitiveScene
  |-- may update GrowthModel
  `-- may authorize DreamCandidate generation

ConsolidationResult
  |-- cites Trace / Memory / FeedbackLoop / CognitiveScene / ReviewReport
  |-- may create Memory summaries
  |-- may update CognitiveScene or GrowthModel
  |-- may feed ReviewReport
  `-- is the direct source for DreamCandidate

DreamCandidate
  |-- comes from SleepCycle + ConsolidationResult
  |-- may target FeedbackLoop / CognitiveScene / GrowthModel
  |-- is selected or rejected during later Wake paths
  `-- is evaluated by later Trace / FeedbackLoop evidence
```

Minimum causal chain:

```text
Wake Trace
-> SleepCycle
-> ConsolidationResult
-> DreamCandidate
-> next Wake Trace
-> effectiveness evaluation
```

## Trace Integration

`docs/trace-contract.md` already reserves:

- `task_type = consolidation`
- `task_type = dreaming`
- `generated_sleep_cycle_ids`
- `generated_consolidation_result_ids`
- `generated_dream_candidate_ids`

Sleep implementations should use `task_type = consolidation` when a Trace
records a SleepCycle execution. Dreaming implementations should use
`task_type = dreaming` when a Trace records DreamCandidate generation.

When cloud generation is ever allowed by a later routing policy, the Trace must
record `runtime`, `model_provider`, `model_name`, token usage, estimated cost,
and redacted summaries. Trial and Local One-click paths should remain
deterministic unless a later issue explicitly changes the profile policy.

## Validation Questions

Before implementing Sleep or Dreaming persistence, answer these in the issue or
PR:

- Which evidence window does the SleepCycle cover?
- Which linked objects are required to be same-Space validated?
- Is the cycle namespace-specific or Space-level?
- Is the run deterministic, local, cloud, or hybrid?
- Which ConsolidationResult fields are structured now, and which stay as
  summaries?
- How will DreamCandidate selection and later effectiveness evaluation be
  traced?
- How does the implementation avoid running deep consolidation on every Wake
  input?
