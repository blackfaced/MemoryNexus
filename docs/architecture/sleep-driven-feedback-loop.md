# Sleep-driven Feedback Loop

MemoryNexus should not perform every cognitive operation during the foreground
interaction. It uses a Wake / Sleep architecture.

```text
Wake  = foreground capture, performance, immediate feedback, and Trace
Sleep = offline consolidation, GrowthModel update, and next plan generation
```

## Wake Path

Wake paths are synchronous Surface calls.

They should:

- validate the request;
- route to the correct namespace and surface;
- record a Trace;
- return quick feedback or the requested current state;
- publish an Engine event if deeper work is needed.

Wake paths should not:

- run full CognitiveScene consolidation;
- update every GrowthModel synchronously;
- fan out through many Lenses;
- generate broad plans unless the request explicitly asks for planning;
- block daily practice on cloud model availability.

## Sleep Path

Sleep paths are manual, daily, weekly, or idle-time consolidation cycles.

They should:

- aggregate Trace, Memory, and FeedbackLoop evidence;
- detect recurring patterns and evidence gaps;
- update GrowthModel;
- produce ConsolidationResult;
- generate PracticePlan / DreamCandidate records;
- record their own Trace and status.

The first implementation should support manual SleepCycle before adding any
scheduler.

## Dictation Coach Example

```text
Morning / school day:
  Capture today's Chinese characters, English words, or sentences.

Practice time:
  Submit dictation attempt.
  Return immediate feedback using deterministic rules.
  Write Trace.

Night:
  SleepCycle reads today's traces.
  GrowthModel updates recurring error patterns.
  PracticePlan proposes tomorrow's 10-minute practice.

Next day:
  Learner practices from the plan.
  New Trace evaluates whether the plan helped.
```

## GrowthModel Inputs

GrowthModel should be evidence-backed, not a vague profile.

Inputs:

- attempts;
- evaluations;
- feedback;
- repeated error types;
- stability across days;
- skipped or incomplete practice;
- user feedback on usefulness.

Outputs:

- strengths;
- weaknesses;
- recurring patterns;
- current stage;
- recommended focus;
- evidence IDs;
- confidence or evidence gaps.

## PracticePlan / DreamCandidate

`PracticePlan` is the user-facing plan. `DreamCandidate` is the internal
candidate that may become a plan.

First Dictation Coach plans should be simple:

- 5-10 items;
- one focus pattern;
- one short review prompt;
- no OCR or ASR inside MemoryNexus; plans use manually entered or Agent-prepared,
  user-confirmed normalized text;
- no broad curriculum generation;

## Runtime Policy

Trial and Local One-click should use deterministic/local-first behavior by
default.

Cloud generation may be allowed in Production only when:

- the call is explicit or scheduled;
- Trace records runtime, provider, cost, and latency;
- summaries are redacted;
- deterministic fallback exists.

## Evaluation

The loop is successful only if later traces can evaluate whether a plan helped.

Metrics:

- repeated error reduced;
- same error repeated;
- plan completed;
- insufficient evidence;
- latency;
- cost;
- local processing ratio;
- useful feedback rate.
