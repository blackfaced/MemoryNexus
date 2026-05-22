# Lens Evaluation

MemoryNexus includes a small deterministic evaluation harness for Lens quality.
It is intentionally local-first: it does not require PostgreSQL, Qdrant, network
access, or provider API keys.

Run it with:

```bash
cargo run --bin memorynexus-eval
```

The command prints a JSON report:

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
