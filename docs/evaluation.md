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
