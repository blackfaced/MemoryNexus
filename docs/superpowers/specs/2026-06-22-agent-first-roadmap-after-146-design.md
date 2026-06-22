# Agent-first Roadmap After Issue 146

Date: 2026-06-22

## Decision

After Issue #146, optimize the next execution waves for the earliest useful
Dictation Coach test through Claw, Hermes, or another MCP-capable Agent. The
first acceptance target is a text-first vertical loop, not a dedicated web or
mobile App:

```text
capture word list
-> submit attempt
-> classify mistakes
-> update GrowthModel
-> generate next PracticePlan
-> review through the Agent
```

The loop must continue to use the generic Surface Gateway and Trace-backed
Engine. Agent-first does not mean adding Dictation-specific Engine actions or
letting an Adapter access Engine repositories directly.

## Considered Approaches

### Complete architecture first

Finish every Surface, media evidence validation, event publication, sleep
aggregation, Adapter, release, and deployment task before user testing. This
has the cleanest milestone ordering but delays feedback on the product loop.

### Text-first Agent vertical slice

Finish the generic Surfaces, then deliver typed/pasted Dictation Capture and
Performance, generic MCP tools, deterministic feedback, GrowthModel, and
PracticePlan. Develop media evidence validation and event publication in
parallel. This is the selected approach because it validates the product value
without weakening Engine ownership or media safety boundaries.

### Distribution first

Publish and deploy the current binaries before the Dictation loop exists. This
would validate packaging but would not answer whether the Agent experience is
useful, so distribution follows the first Agent loop.

## Acceptance Boundary

The first Agent acceptance run uses one learner and typed or pasted text. It
must exercise Capture, Performance, Reflection, Planning, and Observation via
generic MCP Surface tools, preserve CognitiveSpace and Namespace boundaries,
and return Trace provenance.

The initial run does not require OCR, ASR, an `EvidenceRefInput`, seven days of
real history, a tagged release, or a dedicated App. Agent-prepared media input
becomes eligible only after #175 validates `input_confirmation` and ephemeral
evidence descriptors.

## Execution Waves

### Wave 0: accept Issue 146

1. Review PR #176 rather than treating green unit CI as sufficient evidence.
2. Run `surface_reflection_postgres_integration` against PostgreSQL.
3. Merge PR #176 and close #146 only after its acceptance criteria pass.

### Wave 1: complete the generic Surface Gateway

1. Implement #147 Planning after #146 lands.
2. Implement #148 Observation after #147 lands.
3. Add a P0 CI issue that makes PostgreSQL Surface integration tests a required
   pull-request check. Pin service versions and keep external-provider tests
   scheduled or manual.

The shared `src/api/surfaces.rs` dispatcher remains serialized through this
wave.

### Wave 2: open the text-first Agent path

1. Refine #155 so typed/pasted word-list Capture can land after #148. Media
   sources remain gated by #175.
2. Refine #156 so typed/pasted attempt submission follows #155. Media sources
   remain gated by #175.
3. Promote #162 to P1 and split its delivery acceptance into:
   - generic text Surface tools after #148;
   - media confirmation and evidence mapping after #175.
4. Implement #175 in parallel. It remains the mandatory gate for
   `agent_ocr`, `agent_transcribed`, `mixed`, and all evidence descriptors.

Text-first delivery must not silently accept media-derived content as typed or
pasted input. Adapter tests must preserve that distinction.

### Wave 3: complete the feedback Engine loop

1. Implement #157 deterministic Dictation mistake classification after #156.
2. Implement #152 deterministic Trace/FeedbackLoop aggregation into a
   GrowthModel update.
3. Implement #153 evidence-linked PracticePlan generation from GrowthModel.
4. Refine #158 to shape the generated PracticePlan as tomorrow's focused
   ten-minute Dictation practice rather than building a separate planning path.
5. Develop #150 event publication in parallel; it does not block the first
   manual Agent loop.

This ordering prevents the Dictation slice from bypassing the generic
`Trace -> FeedbackLoop -> GrowthModel -> PracticePlan` path.

### Wave 4: Agent acceptance

1. Promote #160 to P1.
2. Remove #159 as a blocker for the first same-day Agent smoke. #148 provides
   the generic Observation capability for the initial run.
3. Run the loop through a Developer Profile on a Mac mini with Claw or Hermes.
4. Require the Agent smoke to cover word-list capture, attempt submission,
   Reflection, mistake analysis, next practice, Observation, and Trace IDs.
5. Complete #159 with deterministic multi-day fixtures, then add the seven-day
   trend to the extended Agent acceptance suite.

### Wave 5: distribute the validated loop

1. Complete #128 and publish the first release containing the accepted Agent
   loop.
2. Complete #129 so an Agent can connect without local Rust.
3. Promote #130 to P1 and establish a versioned Mac mini or equivalent
   deployment with migration preflight, health smoke, and rollback.
4. Start the separate Dictation Coach App repository only after #160 is
   accepted.

## Issue Changes

| Issue | Planned change |
| --- | --- |
| #146 | Review PostgreSQL integration evidence, merge, and close before #147. |
| #147 | Keep serialized after #146. |
| #148 | Keep serialized after #147; unlock text-first work and #175. |
| #150 | Keep P1 but remove from first Agent-smoke critical path. |
| #152 | Add to the Dictation Engine critical path after #157. |
| #153 | Depend on #152 and feed #158. |
| #155 | Typed/pasted path depends on #148; media extension depends on #175. |
| #156 | Typed/pasted path depends on #155; media extension depends on #175. |
| #158 | Reuse #153 PracticePlan instead of creating a parallel plan model. |
| #159 | Stop blocking initial #160 smoke; retain extended seven-day acceptance. |
| #160 | Promote from P2 to P1; initial smoke depends on #155-#158 and text-capable #162. |
| #162 | Promote from P2 to P1; text tools after #148, media mapping after #175. |
| #175 | Keep P1 and mandatory for every media-derived input path. |
| #128 | Publish after the Agent loop is accepted so the first release is useful. |
| #130 | Promote from P2 to P1 for the post-smoke Mac mini deployment path. |

## CI And Verification

The new required integration job should run PostgreSQL-backed Surface tests on
pull requests that modify Rust code or migrations. Qdrant-backed acceptance
should run when vector behavior changes and in a scheduled full acceptance
workflow. Network/provider acceptance remains optional and must not make the
deterministic loop flaky.

Each shared-dispatcher issue must report:

- unit and contract test results;
- PostgreSQL integration test results;
- changed files and dispatcher ownership;
- Trace creation and authorization evidence;
- any ignored tests and why they remain ignored.

The first Agent acceptance is complete only when it runs through MCP without a
web UI and without direct database or Engine repository access.

## Deferred Work

- Persistent `EvidenceRef` storage and resolver adapters.
- Real OCR or ASR inside MemoryNexus.
- Scheduler-driven Sleep cycles and a distributed event queue.
- Seven days of real-world history before fixture acceptance exists.
- Dedicated Dictation Coach App repository.
- Broad learning-platform or multi-learner features.
