# Surface Gateway

Surface Gateway is the boundary between external adapters and the MemoryNexus
Engine.

External apps and agents should call Surface Gateway capabilities instead of
directly manipulating Engine objects such as Trace, MemoryAtom, CognitiveScene,
GrowthModel, SleepCycle, or PracticePlan.

Adapter-specific allowances are defined in
[Surfaces And Adapters](surfaces-and-adapters.md#adapter-capability-policy).
Gateway policy should authorize adapter access as allowed surfaces and actions;
it must not turn product roles or interaction channels into Engine ownership
boundaries.

The Developer Dashboard is a special developer/admin/debug adapter, not a
direct Engine browser. Its default path is Observation that is read-only with
respect to inspected Engine debug objects; the Gateway may still write
audit/provenance Trace for the dashboard request. Reflection and Planning
require explicit review/planning workflows, and Capture or Performance are
limited to controlled fixtures or smoke paths. See the
[Developer Dashboard Adapter Contract](surfaces-and-adapters.md#developer-dashboard-adapter-contract)
for the read-only debug visibility and visibility-label requirements.

## Responsibilities

Surface Gateway owns:

- authentication;
- namespace routing;
- surface routing;
- actor and adapter policy;
- ACL and permission checks;
- request validation;
- response shaping;
- Trace writing;
- sync / async dispatch decisions;
- Engine event publishing.

## SurfaceRequest

Conceptual shape:

```text
SurfaceRequest {
  namespace
  surface: capture | performance | reflection | planning | observation
  action
  actor
  adapter
  payload
  context {
    mode
    locale
    device
    runtime_preference
  }
}
```

### Fields

| Field | Notes |
| --- | --- |
| `namespace` | Domain partition such as `child.chinese.dictation`; must resolve inside a `CognitiveSpace`. |
| `surface` | Intent boundary: capture, performance, reflection, planning, or observation. |
| `action` | Surface-specific action such as submitAttempt or generateNextTask. |
| `actor` | User or service actor making the request; not memory owner by itself. |
| `adapter` | Interaction channel such as mcp, cli, web, mobile, chat, dashboard, or voice. |
| `payload` | Surface-specific input. |
| `context.mode` | `fast`, `focused`, `deep`, or `none`. |
| `context.locale` | Product language and locale hints. |
| `context.device` | Optional device/channel hint. |
| `context.runtime_preference` | deterministic, local, cloud, hybrid, or auto preference. |

### MVP Action Vocabulary

The first contract keeps actions narrow so adapters can share one request shape
without exposing Engine internals:

| Surface | Initial action |
| --- | --- |
| `capture` | `capture_observation` |
| `performance` | `submit_attempt` |
| `reflection` | `review_evidence` |
| `planning` | `generate_next_task`; `adjust_plan` |
| `observation` | `get_state_summary` |

Requests must use an action that belongs to the selected surface. For example,
`submit_attempt` is valid for `performance` and invalid for `capture`.
`planning/adjust_plan` adjusts an adapter-proposed plan from generic evidence
and constraints, returns a response-only draft, and does not persist or expose a
`PracticePlan` ID.

## SurfaceResponse

Conceptual shape:

```text
SurfaceResponse {
  surface
  action
  result
  generated_trace_id
  follow_up_suggestions
  visibility
}
```

### Fields

| Field | Notes |
| --- | --- |
| `surface` | Echoes the surface that handled the request. |
| `action` | Echoes the action that ran. |
| `result` | Adapter-shaped result, not raw Engine internals by default. |
| `generated_trace_id` | Trace ID for provenance and later feedback effectiveness. |
| `follow_up_suggestions` | Optional next actions; may come from Planning Surface or deterministic defaults. |
| `visibility` | Intended wire-contract visibility: user, coach, developer, debug, internal, or adapter-specific. Product copy may render `user` as learner-facing content. |

The response `result` is intentionally shaped for the adapter. Raw Engine
objects such as `MemoryAtom`, `CognitiveScene`, `GrowthModel`, or
`PracticePlan` should not be returned by default.

Reflection results may carry a stable Lens strategy reference when the request
needs to preserve which interpretation strategy was used. That reference is
adapter-neutral metadata such as `{ "name": "learning_review" }`; it is not an
agent persona, speaking role, prompt template, or role-play identity.

## Sync vs Async

Surface Gateway decides whether a request is synchronous or asynchronous.

Sync Surface Calls:

- capture a short observation;
- submit an attempt;
- return immediate deterministic feedback;
- get today's practice;
- fetch a current GrowthModel summary.

Async Engine Events:

- SleepCycle consolidation;
- GrowthModel update;
- CognitiveScene consolidation;
- PracticePlan generation;
- DreamCandidate effectiveness evaluation.

Principle:

```text
foreground fast, background deep
```

## Gateway Events

The Gateway may publish Engine events after validating and tracing a request.

Examples:

- `ObservationCaptured`
- `AttemptSubmitted`
- `FeedbackGenerated`
- `SleepCycleRequested`
- `GrowthModelUpdated`
- `PlanGenerated`

The first implementation can use an in-process event model or stored events. A
distributed queue is not required for the MVP.

## Non-Goals

- Do not build a second backend.
- Do not expose every Engine table as a Surface endpoint.
- Do not make adapters responsible for namespace or permission correctness.
- Do not require cloud LLMs for Gateway behavior.
- Do not implement a full event bus before the sync MVP proves the shape.
