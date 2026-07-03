# Lens Evaluation

> Status update: Lens evaluation remains useful for regression coverage, but the
> next evaluation direction is GrowthBench / DictationBench. MemoryNexus should
> evaluate feedback usefulness and growth signals, not only retrieval or summary
> quality.

MemoryNexus includes a small deterministic evaluation harness for Lens quality.
It is intentionally local-first: it does not require PostgreSQL, Qdrant, network
access, or provider API keys.

Run all default deterministic evaluations with:

```bash
cargo run --bin memorynexus-eval
```

The command prints a JSON report with separate sections:

```json
{
  "lens_eval": {},
  "dictation_bench_recurring_errors": {},
  "dictation_bench_next_practice": {},
  "dictation_bench_improvement": {}
}
```

To run only the Lens evaluation:

```bash
cargo run --bin memorynexus-eval -- lens
```

The Lens section keeps the existing report shape:

```json
{
  "total_cases": 3,
  "passed_cases": 3,
  "overall_score": 1.0,
  "provider_backed_cases": 0,
  "deterministic_cases": 3,
  "results": []
}
```

## Current Scenarios

The first fixtures cover three Lens strategies:

- `project_context`: checks Rust-first project direction, Cognitive Space
  ownership, citation correctness, and deterministic fallback behavior.
- `risk_review`: checks that unresolved contradictions are surfaced as signal.
- `learning_review`: checks that deprioritized scratch memory stays out of
  Profile sources while the active learning memory remains available.

## Scoring Dimensions

Each case scores six dimensions from `0.0` to `1.0`:

- `retrieval_relevance`: expected relevant memories were retrieved.
- `citation_correctness`: citations refer to retrieved memories and include the
  expected sources.
- `summary_faithfulness`: summary contains required fixture terms.
- `contradiction_signal`: unresolved contradictions appear when expected.
- `profile_projection_stability`: active memories are included and
  deprioritized memories are excluded from Profile sources.
- `provider_fallback`: deterministic fallback provenance is correct when no
  provider is configured.

The overall score is the average of case scores.

## Provider-Backed Evaluations

The harness leaves room for future provider-backed cases. Those should remain
optional and should not run in default CI unless credentials and network access
are explicitly configured.

Future provider-backed cases can compare:

- AI summary faithfulness against deterministic fixture facts.
- Output format adherence for `brief`, `bullets`, and `detailed` Lens output.
- Provider fallback behavior when a configured model returns empty or invalid
  output.

## Limitations

This is not yet a real benchmark suite:

- Fixtures are handcrafted and small.
- Scores are rule-based, not statistically meaningful.
- Retrieval is evaluated from fixture outputs, not by launching PostgreSQL or
  Qdrant.
- Summary faithfulness checks required terms, not deep semantic grounding.
- Contradiction detection quality is represented by expected structured output,
  not inferred from raw memories yet.

The goal is to make Lens quality regressions visible early while the cognitive
domain model is still changing quickly.

## Next Direction: GrowthBench / DictationBench

The new long-term feedback roadmap needs a benchmark that asks:

```text
Did the system detect repeated patterns, generate a useful next practice, and
observe whether the next attempt improved?
```

Initial DictationBench should evaluate:

- recurring Chinese dictation error detection;
- recurring English spelling / sentence dictation error detection;
- next-practice generation aligned with the detected pattern;
- multi-day improvement signals;
- insufficient-evidence handling;
- latency;
- estimated cost;
- local processing ratio;
- useful feedback rate.

Default evaluation must stay deterministic and local-first. Provider-backed
cases can be optional later, but the baseline should not require external API
credentials.

## DictationBench Fixture Corpus

The first deterministic DictationBench inputs live in
`tests/fixtures/dictation_bench/*.json`. The corpus currently covers Chinese
dictation, English spelling, English sentence dictation, multi-day improvement,
and insufficient-evidence handling. Each fixture records the namespace, locale,
task kind, prompt items and expected text, submitted attempts, expected mistake
patterns, expected next-practice outcome, and local deterministic evaluation
notes.

Follow-up issues #166-#168 should consume these fixtures as plain local JSON.
They should parse the structured fields, run deterministic classification /
growth / planning checks against them, and report whether the expected mistake
patterns, plan expectations, improvement signals, or evidence gaps are met.
They should not treat fixture notes as scoring logic.

The baseline rule is local-first and no-provider: the default DictationBench
path must not require PostgreSQL, Qdrant, network access, OCR, ASR, media
resolution, or provider API keys. Optional provider-backed evaluation can be
added later only outside the default deterministic gate.

### Recurring Error Benchmark

Run the first local DictationBench recurring-error pass with:

```bash
cargo run --bin memorynexus-eval -- dictation-bench-recurring-errors
```

This path loads the local #165 JSON fixture corpus, classifies each submitted
attempt through the deterministic dictation classifier, and reports detected
mistake types against each expected pattern. It does not require PostgreSQL,
Qdrant, network access, OCR, ASR, media resolution, or provider API keys.

The report includes:

- `total_fixture_count`
- `total_expected_pattern_count`
- `passed_pattern_count`
- `failed_pattern_count`
- per-fixture `pattern_results`
- per-pattern expected mistake type, recurrence label, attempt IDs, prompt item
  IDs, detected mistake types, pass/fail status, and notes

Recurrence labels are interpreted narrowly for #166:

- `recurring`: the expected mistake type must appear across repeated relevant
  attempts.
- `single`: the expected mistake type should appear once and not be treated as
  recurring.
- `improving`: repeated pattern evidence must be detected, but improvement
  quality is not scored here.
- `insufficient_evidence`: no recurring plan-worthy pattern is expected; sparse
  or unclassified evidence should not fail the recurring-error pass.

Follow-up #167 should build on the same fixture and detected-pattern report to
score next-practice quality: target mistake type alignment, ten-minute practice
shape, and evidence-gap behavior. Follow-up #168 should score multi-day
improvement quality separately, especially whether later correct attempts
change feedback intensity without erasing earlier repeated evidence.

### Next-Practice Benchmark

Run the first local DictationBench next-practice pass with:

```bash
cargo run --bin memorynexus-eval -- dictation-bench-next-practice
```

This path reuses the #165 fixture corpus and the #166 deterministic dictation
classification path. For each fixture it converts repeated expected/detected
mistake evidence into `GrowthEvidenceRecord` values, calls
`aggregate_growth_model`, then calls
`PracticePlanGeneration::from_growth_model`.

The report includes:

- `total_fixture_count`
- `useful_count`
- `neutral_count`
- `bad_count`
- per-fixture expected outcome
- generated outcome: `plan` or `evidence_gap`
- quality label: `useful`, `neutral`, or `bad`
- expected target mistake types
- generated target/content/effect summary
- evidence IDs and evidence count
- notes explaining outcome, duration, target, or evidence-gap mismatches

The scoring is intentionally semantic and stable rather than prose-snapshot
based. A useful plan must match the expected plan/evidence-gap outcome, keep
the expected ten-minute MVP duration shape when one is present, visibly target
the expected mistake types through the target pattern, content, or expected
effect, and retain evidence linkage. An evidence-gap fixture should return an
`EvidenceGap` instead of inventing a targeted plan.

Bad or irrelevant plans stay visible as per-fixture `bad` results and in the
top-level `bad_count`; the benchmark does not hide them behind an overall pass.

Follow-up #168 should build on this benchmark by scoring multi-day improvement
quality. In particular, it should evaluate whether later correct attempts lower
feedback intensity or change practice wording while preserving earlier repeated
evidence, instead of treating improvement as either a fresh failure or erased
history.

### Multi-Day Improvement Signal Benchmark

Run the first local DictationBench multi-day improvement pass with:

```bash
cargo run --bin memorynexus-eval -- dictation-bench-improvement
```

This benchmark reuses the #165 fixture corpus and the #166 deterministic
dictation classification path. It evaluates each expected mistake pattern over
an ordered attempt timeline, preserving fixture `submitted_at` values when they
are present. The report is a deterministic signal-quality check over local
fixtures only; `improved` does not claim clinical or educational causality.

The report includes:

- `total_fixture_count`
- `improved_count`
- `repeated_count`
- `skipped_count`
- `insufficient_evidence_count`
- per-fixture and per-pattern results
- attempt timelines with attempt IDs, optional submitted dates, prompt item IDs,
  detected mistake types, and correctness
- metric summary for local ratio, evidence/trace count, deterministic simulated
  latency, and zero estimated cost

Improvement labels are interpreted narrowly:

- `improved`: repeated earlier relevant mistakes are followed by a later correct
  relevant attempt for the same prompt item.
- `repeated`: repeated relevant mistakes remain present with no later correct
  relevant attempt in the fixture window.
- `skipped`: expected relevant attempts or prompt items are absent or cannot be
  evaluated, even though the fixture asks for a pattern.
- `insufficient_evidence`: sparse or unclassified evidence is not treated as
  improvement or repeated failure.

Trace-style metrics are simulated from local fixture evidence. The current
deterministic path treats each evaluated timeline item as local Trace-style
evidence, reports a `local_ratio` of `1.0`, and keeps estimated cost at `0 USD`.
Latency is a fixed deterministic per-evidence value for repeatable evaluation,
not wall-clock measurement.

Later evaluation work can add optional provider-backed or longitudinal
benchmarks, but those should stay outside the default local deterministic gate.
Future #195 LoCoMo / LongMemEval work should be added as a separate evaluation
track rather than folded into DictationBench improvement labels.

## Optional Retrieval / Context Baseline

Issue #195 adds a P2 docs-first plan for a LoCoMo / LongMemEval-style retrieval
baseline. This track is secondary evidence only. GrowthBench / DictationBench
remain the primary MemoryNexus evaluation line because they measure repeated
pattern detection, next-practice usefulness, local ratio, improvement signals,
and feedback-loop behavior over time.

The baseline should answer a narrower question:

```text
Can MemoryNexus gather the right compact context from long text histories while
preserving CognitiveSpace ownership, namespace routing, and trace provenance?
```

It must not answer the broader product-quality question of whether the system
helps a learner improve or produces useful next actions. Those claims belong to
GrowthBench / DictationBench.

### Benchmark Subset

The first slice should use a local micro-corpus inspired by the text-only QA
parts of LoCoMo and LongMemEval:

- `locomo_text_qa_micro`: 6 text-only dialogue-history cases based on LoCoMo's
  long-term conversation QA shape. Include 3 single-hop factual recall cases and
  3 multi-session temporal / event-update cases. Exclude image-grounded turns,
  event summarization, and dialogue generation.
- `longmemeval_core_memory_micro`: 6 text-only chat-memory QA cases based on
  LongMemEval's core ability categories. Include 1 information-extraction case,
  1 multi-session reasoning case, 1 temporal-reasoning case, 2 knowledge-update
  cases, and 1 abstention case. Exclude answer generation judged by an LLM.

This exact 12-case micro-corpus is intentionally smaller than the public
benchmarks. It is locally reproducible, deterministic, and credential-free. It
tests context gathering and citation behavior before any optional full public
benchmark adapter is considered.

The fixture content should be hand-authored or checked into the repository
under:

```text
tests/fixtures/retrieval_baseline/locomo_longmemeval_micro.json
```

Do not download a benchmark at runtime, call an external benchmark service, or
require provider credentials in the default path. If maintainers later add a
converter for public LoCoMo / LongMemEval records, that converter should be a
separate optional import step and should emit the same local fixture shape.

### Fixture Shape

Use one JSON file with a top-level version and case list:

```json
{
  "schema_version": 1,
  "suite": "locomo_longmemeval_micro",
  "cases": [
    {
      "id": "locomo_text_qa_micro_001",
      "source_style": "locomo",
      "ability": "single_hop_fact",
      "space": {
        "id": "retrieval-baseline-space-001",
        "name": "Retrieval Baseline Fixture Space"
      },
      "namespace": "benchmark.retrieval.locomo",
      "history": [
        {
          "id": "turn-001",
          "session_id": "session-01",
          "turn_index": 1,
          "speaker": "user",
          "occurred_at": "2026-01-03T09:00:00Z",
          "text": "I moved my piano lesson to Thursday because Tuesday is full."
        }
      ],
      "query": {
        "id": "question-001",
        "text": "Which day did the user move the piano lesson to?",
        "expected_answer": "Thursday",
        "answerable": true
      },
      "expected_context": {
        "must_include_history_ids": ["turn-001"],
        "must_exclude_history_ids": [],
        "required_terms": ["piano lesson", "Thursday"]
      },
      "expected_mapping": {
        "trace_kind": "conversation_turn",
        "surface": "observation",
        "adapter": "eval_fixture",
        "context_output": "retrieved_context"
      }
    }
  ]
}
```

Field rules:

- `source_style` is `locomo` or `longmemeval`; it records inspiration, not a
  runtime dependency.
- `ability` is one of `single_hop_fact`, `multi_session_reasoning`,
  `temporal_reasoning`, `knowledge_update`, or `abstention`.
- `space` represents one `CognitiveSpace`. The baseline may include multiple
  spaces, but records from one case must never retrieve context from another
  space.
- `namespace` is a domain partition inside the space, for example
  `benchmark.retrieval.locomo` or `benchmark.retrieval.longmemeval`. It is not a
  permission boundary.
- `history` entries are local text evidence. They map to Trace-style fixture
  records and, when using existing Lens/search plumbing, may also be converted
  into memory records for retrieval. The conversion must preserve the original
  history IDs as citation/source IDs.
- `query.answerable=false` marks abstention cases. These should reward empty or
  low-confidence context and penalize spurious context.
- `expected_context` scores retrieval/context gathering only. It does not score
  final natural-language answer quality.
- `expected_mapping` documents how the adapter projects the record into
  MemoryNexus concepts; it must not require new Engine schema or Surface
  contract fields.

### MemoryNexus Mapping

The baseline adapter should map benchmark records without changing ownership
boundaries:

- Create or simulate the declared `CognitiveSpace` for the case.
- Route all LoCoMo-style records to `benchmark.retrieval.locomo` and all
  LongMemEval-style records to `benchmark.retrieval.longmemeval`.
- Treat each `history` item as confirmed local text evidence from the
  `eval_fixture` adapter. A future executable version may write Trace records
  through the Surface Gateway, but the first fixture-only evaluator can simulate
  Trace IDs deterministically.
- Feed context gathering through existing space-scoped retrieval / Lens-style
  paths. Retrieval must stay scoped to the case space and namespace filter when
  the fixture provides one.
- Return a `retrieved_context` list containing source IDs, snippets, rank,
  retrieval mode, namespace, and trace/source provenance.
- Do not create new Engine-owned agent memory, global benchmark memory, or
  benchmark-specific ownership model.

This mapping preserves ADR-019: the benchmark runner is an Adapter, retrieval
or Observation is the Surface intent, and MemoryNexus Engine objects remain
owned by `CognitiveSpace`.

### Local Run Command

The planned local command is:

```bash
cargo run --bin memorynexus-eval -- retrieval-baseline \
  --fixture tests/fixtures/retrieval_baseline/locomo_longmemeval_micro.json
```

The command should be optional and outside the default `cargo run --bin
memorynexus-eval` report until an executable slice lands. It must not require
PostgreSQL, Qdrant, network access, OCR, ASR, media resolution, provider API
keys, or a downloaded benchmark service. A first implementation can use the
same deterministic in-memory fixture style as the Lens and DictationBench
evaluators.

The eventual report should be separate from DictationBench:

```json
{
  "retrieval_baseline": {
    "suite": "locomo_longmemeval_micro",
    "total_case_count": 12,
    "hit_at_1": 0.0,
    "hit_at_3": 0.0,
    "citation_recall": 0.0,
    "citation_precision": 0.0,
    "abstention_precision": 0.0,
    "cross_space_leak_count": 0,
    "local_ratio": 1.0,
    "estimated_cost_usd": 0.0,
    "case_results": []
  }
}
```

### Metrics

Report retrieval/context metrics separately from GrowthBench / DictationBench
feedback-loop metrics:

- `hit_at_1` and `hit_at_3`: whether required source IDs appear in the top
  ranked retrieved context.
- `citation_recall`: required source IDs retrieved divided by required source
  IDs.
- `citation_precision`: retrieved cited source IDs that are expected or
  explicitly allowed.
- `required_term_coverage`: required fixture terms present in retrieved
  snippets.
- `abstention_precision`: answerable-false cases that return no required
  context or an explicit insufficient-context result.
- `cross_space_leak_count`: retrieved items whose `space.id` differs from the
  case space. This must be zero.
- `cross_namespace_leak_count`: retrieved items outside the requested namespace
  when a namespace filter is specified.
- `context_token_count`: approximate token count of gathered context.
- `local_ratio`, `estimated_cost_usd`, and deterministic simulated latency,
  following the DictationBench reporting convention.

Do not mix these with DictationBench metrics such as useful feedback rate,
next-practice quality, improvement labels, or repeated mistake detection.

### What This Proves

This baseline can provide secondary evidence that MemoryNexus can:

- keep retrieval scoped to a `CognitiveSpace`;
- preserve namespace and source provenance in compact context;
- retrieve supporting text for simple long-history QA shapes;
- avoid spurious context in abstention cases;
- report local deterministic retrieval behavior without cloud credentials.

It does not prove:

- that MemoryNexus is a better general-purpose agent memory runtime;
- that the roadmap should optimize around pure retrieval accuracy;
- that feedback, growth, or next-practice quality improved;
- that multimodal LoCoMo tasks are supported;
- that public LongMemEval scores are comparable to external leaderboard runs;
- that a cloud LLM can answer the retrieved context correctly.

Any future full LoCoMo / LongMemEval adapter should remain optional P2 evidence
and should not replace the GrowthBench / DictationBench acceptance line.
