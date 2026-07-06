# Dictation Coach MVP

Dictation Coach is the first upstream product direction for MemoryNexus. It
uses short daily dictation loops to validate MemoryNexus as a long-term
feedback engine rather than a generic memory app.

The MVP proves this loop:

```text
Capture -> Performance -> Reflection -> Planning -> Observation -> SleepCycle
```

This is a docs-only product contract. It is not a database migration, API
schema, Rust implementation, UI specification, or OCR pipeline.

## Goals

- Cover Chinese native-language dictation, English spelling, and English
  sentence dictation with one shared Engine vocabulary.
- Define stable namespaces, task shapes, attempt shapes, and deterministic
  mistake taxonomy for the first implementation issues.
- Map product actions to Capture, Performance, Reflection, Planning, and
  Observation Surfaces.
- Keep the first path text-first, deterministic, local-first, and Trace-backed.
- Preserve `CognitiveSpace` as the ownership and permission boundary.

## Non-Goals

- MemoryNexus does not perform OCR, handwriting recognition, audio
  transcription, or raw-media interpretation in this MVP.
- An Agent/App Adapter may perform OCR or ASR, obtain explicit user acceptance
  or correction of the normalized text, and submit it with optional media
  evidence references.
- No multi-child management.
- No broad education platform.
- No full curriculum engine.
- No cloud-only generation.
- No parent, child, teacher, or coach roles in Engine objects.

Adapter copy may speak to a parent, learner, teacher, or coach. Engine and
Surface contracts must stay role-neutral.

## Positioning

Dictation Coach helps a learner practice:

- Chinese dictation: characters, words, phrases, or short sentences.
- English spelling: individual words or short phrases.
- English sentence dictation: one or more complete sentences.

Dictation Coach is an upstream product and adapter scenario. The Engine remains
generic and works in terms of `Namespace`, `Trace`, `FeedbackLoop`,
`GrowthModel`, `SleepCycle`, and `PracticePlan`.

## Namespaces

Recommended first namespaces:

```text
child.chinese.dictation
child.english.spelling
child.english.sentence-dictation
```

Rules:

- `child.*` is a domain naming convention, not a permission boundary.
- Permissions still come from `CognitiveSpace` membership.
- A namespace partitions evidence, mistake patterns, GrowthModel updates, and
  PracticePlans inside a Space.
- Surface Gateway validates that all referenced tasks, attempts, traces, and
  generated objects belong to the same Space.
- Cross-namespace evidence is allowed only when a later consolidation or review
  records why it is needed.

## Task Shape

A dictation task is the assignment captured before an attempt. It may later map
to a Rust domain type, but the first contract is conceptual:

```text
DictationTask {
  id
  space_id
  namespace
  task_kind
  title?
  prompt_items
  instructions?
  expected_duration_minutes?
  source
  evidence_refs?: EvidenceRefInput[]
  created_at
  metadata
}
```

Fields:

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable task ID. |
| `space_id` | yes | Owning `CognitiveSpace`. |
| `namespace` | yes | One of the dictation namespaces above for MVP. |
| `task_kind` | yes | `chinese_dictation`, `english_spelling`, or `english_sentence_dictation`. |
| `title` | no | Human label such as `Monday words`; not used for permissions. |
| `prompt_items` | yes | Ordered confirmed text items to practice. |
| `instructions` | no | Short adapter-facing guidance. |
| `expected_duration_minutes` | no | MVP default is 10 when omitted. |
| `source` | yes | Acquisition path for canonical confirmed Surface text: `typed`, `pasted`, `imported_text`, `agent_ocr`, `agent_transcribed`, `mixed`, or `test_fixture`. |
| `evidence_refs` | no | Original-media provenance inputs governed by the [Media Evidence Contract](media-evidence-contract.md). |
| `created_at` | yes | Capture timestamp. |
| `metadata` | no | Small structured extension point; not a place for OCR blobs or raw files. |

### Prompt Items

```text
PromptItem {
  id
  item_kind
  expected_text
  display_text?
  hint?
  locale?
  order_index
  metadata
}
```

`item_kind` values:

```text
chinese_character
chinese_word
chinese_phrase
chinese_sentence
english_word
english_phrase
english_sentence
```

Text-first input rules:

- `expected_text` is confirmed normalized text, whether entered directly or
  prepared by an Agent/App Adapter.
- MemoryNexus does not upload or interpret image, handwriting, audio, video, or
  worksheet media in this MVP. An Adapter may preprocess that media and attach
  optional evidence references.
- `display_text` may differ from `expected_text` only for adapter copy, such as
  showing a definition or masked prompt. The Engine compares against
  `expected_text`.
- `locale` is optional but should use stable values such as `zh-Hans`,
  `zh-Hant`, or `en`.

## Attempt Shape

An attempt is the submitted result for one captured task. Its canonical input
is confirmed normalized text.

```text
DictationAttempt {
  id
  space_id
  namespace
  task_id
  task_kind
  submitted_items
  started_at?
  submitted_at
  source
  evidence_refs?: EvidenceRefInput[]
  metadata
}
```

Fields:

| Field | Required | Notes |
| --- | --- | --- |
| `id` | yes | Stable attempt ID. |
| `space_id` | yes | Must match the task Space. |
| `namespace` | yes | Must match the task namespace for MVP. |
| `task_id` | yes | Source task. |
| `task_kind` | yes | Copied from the task for deterministic routing. |
| `submitted_items` | yes | Ordered confirmed text answers aligned to task prompt items. |
| `started_at` | no | Optional practice start time. |
| `submitted_at` | yes | Submission time. |
| `source` | yes | Acquisition path for canonical confirmed Surface text: `typed`, `pasted`, `agent_ocr`, `agent_transcribed`, `mixed`, or `test_fixture`. |
| `evidence_refs` | no | Original-media provenance inputs governed by the [Media Evidence Contract](media-evidence-contract.md). |
| `metadata` | no | Small structured extension point. |

### Submitted Items

```text
SubmittedItem {
  prompt_item_id
  actual_text
  order_index
  self_marked_uncertain?
  metadata
}
```

Rules:

- The MVP compares `actual_text` with the matching `expected_text`.
- Missing submitted items are valid evidence and should classify as
  `missing_item` or `missing_word` depending on task kind.
- Extra submitted items are valid evidence and should classify as `extra_item`
  or a more specific English/Chinese error when deterministic rules allow it.
- The first implementation should not infer handwriting, pronunciation, or
  speech intent from raw media.

### Confirmed Text And Media Evidence

- Feedback and deterministic classification use confirmed normalized text in
  the Surface payload, never inaccessible media or an unconfirmed transcript.
- Optional `evidence_refs` preserve original-media provenance and follow the
  [Media Evidence Contract](media-evidence-contract.md); this document does not
  duplicate its field or validation constraints.
- Every media-derived normalized payload requires explicit user acceptance or
  correction before submission. OCR/ASR confidence may guide how the Adapter
  highlights or reviews text, but it never substitutes for confirmation.
- Media resolution or availability failure affects provenance inspection only;
  it does not invalidate a completed text flow, Trace, or feedback result.

`source` describes how the canonical confirmed Surface text for the complete
task or attempt was acquired. Use the specific source for a single-origin
request and `mixed` when one request combines items acquired through multiple
paths. `transcript_source` belongs to each `EvidenceRefInput` and describes that
media reference's OCR/ASR provenance; its contract remains in the
[Media Evidence Contract](media-evidence-contract.md).

For this first request-level, docs-only slice, `evidence_refs` contains caller
`EvidenceRefInput` values. No persistent `EvidenceRef` model exists yet. A
future persisted task or attempt must link resolved `EvidenceRef` records or
store `evidence_ref_ids`; it must not treat caller input as persisted identity.

Terminology: `evidence_refs` / `EvidenceRefInput` are optional original-media
provenance descriptors. Generic `evidence_ids` in Reflection, Planning, and
Observation are Engine evidence links such as Trace, FeedbackLoop, or Memory
identifiers, not media references.

## Evaluation Shape

Deterministic evaluation turns an attempt into item-level outcomes and summary
evidence.

```text
DictationEvaluation {
  id
  space_id
  namespace
  task_id
  attempt_id
  item_results
  summary
  generated_trace_id?
  created_at
  metadata
}
```

```text
ItemResult {
  prompt_item_id
  expected_text
  actual_text
  status
  mistake_types
  explanation
  evidence
  confidence
}
```

`status` values:

```text
correct
incorrect
partially_correct
missing
extra
unclassified
```

`confidence` is deterministic confidence, not model certainty. Use `high` only
when a rule directly applies. Use `low` or `unclassified` when the input does
not support a listed category.

## Deterministic Mistake Taxonomy

The first classifier should prefer stable, explainable string rules. It should
return `unclassified` or an evidence gap instead of inventing a cause.

### Shared Types

```text
correct
missing_item
extra_item
punctuation_error
spacing_error
unclassified
```

Shared rules:

- Exact normalized match returns `correct`.
- Empty actual text for an expected item returns `missing_item`.
- An answer with no corresponding prompt item returns `extra_item`.
- Punctuation-only differences return `punctuation_error`.
- Whitespace-only differences return `spacing_error`.
- Ambiguous differences return `unclassified`.

### Chinese Dictation Types

```text
wrong_character
visually_similar_character
homophone
missing_stroke
extra_stroke
stroke_order_issue
component_placement_issue
missing_item
extra_item
punctuation_error
unclassified
```

Deterministic baseline:

| Type | Rule |
| --- | --- |
| `wrong_character` | Expected and actual have equal character count, and at least one character differs without a more specific supplied evidence tag. |
| `visually_similar_character` | Difference matches a curated local fixture pair, such as later test fixtures for common visually similar characters. |
| `homophone` | Difference matches a curated local fixture pair with the same pronunciation. |
| `missing_stroke` | Difference matches a curated local fixture pair tagged as missing stroke. |
| `extra_stroke` | Difference matches a curated local fixture pair tagged as extra stroke. |
| `stroke_order_issue` | Only available from manual marker metadata in MVP; never inferred from typed text alone. |
| `component_placement_issue` | Difference matches a curated local fixture pair or manual marker metadata. |
| `missing_item` | Expected item has no submitted text. |
| `extra_item` | Submitted text has no matching prompt item. |
| `punctuation_error` | Chinese punctuation differs while non-punctuation text matches. |
| `unclassified` | Difference is real but no deterministic rule applies. |

Typed text cannot prove stroke order. The MVP may record
`stroke_order_issue` only when an adapter or user explicitly supplies a manual
marker.

### English Spelling Types

```text
missing_letter
extra_letter
letter_order_error
double_letter_error
sound_spelling_mapping_error
capitalization_error
missing_item
extra_item
punctuation_error
spacing_error
unclassified
```

Deterministic baseline:

| Type | Rule |
| --- | --- |
| `missing_letter` | Actual text can be transformed into expected text by inserting one or more letters. |
| `extra_letter` | Actual text can be transformed into expected text by deleting one or more letters. |
| `letter_order_error` | Actual text has the same letters as expected but in a different order, or one adjacent transposition explains the difference. |
| `double_letter_error` | Difference is limited to repeated letters, such as `running` vs `runing` or `runniing`. |
| `sound_spelling_mapping_error` | Difference matches a curated local fixture for common sound-spelling substitutions. |
| `capitalization_error` | Lowercased text matches exactly but casing differs. |
| `missing_item` | Expected item has no submitted text. |
| `extra_item` | Submitted text has no matching prompt item. |
| `punctuation_error` | Punctuation differs while letters match. |
| `spacing_error` | Whitespace differs while non-space text matches. |
| `unclassified` | Difference is real but no deterministic rule applies. |

### English Sentence Dictation Types

```text
missing_word
extra_word
word_order_error
missing_letter
extra_letter
letter_order_error
double_letter_error
sound_spelling_mapping_error
capitalization_error
punctuation_error
spacing_error
unclassified
```

Deterministic baseline:

| Type | Rule |
| --- | --- |
| `missing_word` | Token alignment shows one or more expected words absent from actual text. |
| `extra_word` | Token alignment shows one or more actual words absent from expected text. |
| `word_order_error` | Same normalized words appear in a different order. |
| `missing_letter` | Word-level comparison finds a missing-letter spelling error. |
| `extra_letter` | Word-level comparison finds an extra-letter spelling error. |
| `letter_order_error` | Word-level comparison finds a letter-order error. |
| `double_letter_error` | Word-level comparison finds repeated-letter error. |
| `sound_spelling_mapping_error` | Word-level difference matches a curated local sound-spelling fixture. |
| `capitalization_error` | Sentence matches after lowercasing but not with original casing. |
| `punctuation_error` | Sentence matches after punctuation normalization. |
| `spacing_error` | Sentence matches after whitespace normalization. |
| `unclassified` | Difference is real but no deterministic rule applies. |

Sentence dictation may return multiple mistake types for one submitted item, but
the first implementation should keep explanations short and evidence-backed.

## Surface Action Mapping

Surface Gateway should expose product actions through generic Surface intent.
Dictation-specific verbs belong in the adapter copy or payload shape, not in the
Gateway `SurfaceAction` enum.

| Product / Payload Semantics | Surface | Gateway Action | Trace Task Type | Result |
| --- | --- | --- | --- | --- |
| Record today's dictation list | Capture | `capture_observation` | `practice` | `DictationTask` plus generated Trace. |
| Submit confirmed dictation result | Performance | `submit_attempt` | `practice` | `DictationAttempt`, `DictationEvaluation`, immediate feedback, generated Trace. |
| Explain current mistakes | Reflection | `review_evidence` | `feedback` | Item explanations and recurring pattern hints. |
| Generate tomorrow practice | Planning | `generate_next_task` | `planning` | Response-only next-task draft linked to evidence summaries. |
| Adjust proposed practice | Planning | `adjust_plan` | `planning` | Response-only adjusted draft from evidence and constraints. |
| Show 7-day trend | Observation | `get_state_summary` | `review` | Trend summary with recurring errors, stability, and evidence IDs. |

### Capture Surface

Answers:

```text
What was assigned for practice?
```

Minimum request payload:

```text
{
  namespace,
  task_kind,
  prompt_items,
  expected_duration_minutes?,
  source: "typed" | "pasted" | "imported_text" | "agent_ocr" | "agent_transcribed" | "mixed" | "test_fixture",
  evidence_refs?: EvidenceRefInput[]
}
```

Minimum response:

```text
{
  task,
  generated_trace_id,
  follow_up_suggestions: ["submit attempt"]
}
```

Capture writes Trace evidence with deterministic or local runtime metadata. It
does not evaluate answers.

### Performance Surface

Answers:

```text
How did this attempt go?
```

Minimum request payload:

```text
{
  namespace,
  task_id,
  submitted_items,
  source: "typed" | "pasted" | "agent_ocr" | "agent_transcribed" | "mixed" | "test_fixture",
  evidence_refs?: EvidenceRefInput[]
}
```

Minimum response:

```text
{
  attempt,
  evaluation,
  generated_trace_id,
  follow_up_suggestions: ["review mistakes", "generate tomorrow practice"]
}
```

Performance runs deterministic item comparison and records immediate feedback
evidence. It does not require SleepCycle to return first feedback.

### Reflection Surface

Answers:

```text
What does this mistake pattern mean?
```

Minimum request payload:

```text
{
  namespace,
  attempt_id?,
  evaluation_id?,
  timeframe?: "today" | "7d",
  mode: "fast" | "focused" | "deep"
}
```

Minimum response:

```text
{
  explanation,
  recurring_patterns,
  evidence_ids,
  generated_trace_id
}
```

Reflection may explain one attempt immediately or summarize repeated mistake
patterns across recent traces. MVP Reflection should stay deterministic unless a
later issue explicitly adds AI orchestration.

### Planning Surface

Answers:

```text
What should tomorrow's short practice be?
```

Minimum request payload:

```text
{
  namespace,
  target_date?,
  duration_minutes?: 10,
  evidence_ids?
}
```

Minimum response:

```text
{
  next_task,
  target_patterns,
  evidence_ids,
  generated_trace_id
}
```

The first practice plan should be short and concrete. It should target one or
two recurring mistake patterns rather than generating a broad curriculum.
The generic Surface contract also supports `planning/adjust_plan` for adjusting
an adapter-proposed practice draft from evidence and constraints. That response
is a draft and does not imply a persisted `PracticePlan` ID.

### Observation Surface

Answers:

```text
How is dictation practice changing over time?
```

Minimum request payload:

```text
{
  namespace,
  timeframe: "7d",
  include_evidence?: true
}
```

Minimum response:

```text
{
  timeframe,
  attempts_count,
  recurring_mistake_types,
  improving_patterns,
  unstable_patterns,
  current_focus,
  evidence_ids,
  generated_trace_id?
}
```

Observation is read-oriented. It may write a Trace for provenance, but it should
not mutate GrowthModel directly unless routed through an explicit Engine
consolidation path.

## First End-To-End Flow

1. Capture today's confirmed text list under one dictation namespace, with
   optional original-media evidence references.
2. Submit confirmed text answers for the captured task, with optional evidence
   references when original inspection matters.
3. Evaluate each item with deterministic rules.
4. Return immediate feedback with mistake type, explanation, and evidence.
5. Write Trace and FeedbackLoop evidence for the Capture and Performance calls.
6. Generate a short Planning response for tomorrow's 10-minute practice.
7. Observe the last 7 days of attempts and recurring mistake types.
8. Optionally run manual SleepCycle to consolidate traces, update GrowthModel,
   and produce the next PracticePlan.

## MVP Success Criteria

The MVP succeeds when a local deterministic flow can show:

- what was assigned;
- what was attempted;
- which mistake type appeared;
- whether the same pattern repeats;
- what tomorrow's practice should be;
- how the last 7 days changed;
- which Trace, attempt, evaluation, and plan IDs support each claim.
