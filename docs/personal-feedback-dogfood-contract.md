# Personal Feedback Dogfood Contract

This is the executable contract for the private M9 dogfood experiment. It is a fourteen-day, owner-run `personal.health.sleep` experiment that tests whether MemoryNexus can turn confirmed local evidence into a useful next adjustment. It is not a healthcare product, diagnosis, treatment plan, clinical record, or permanent product commitment.

ADR-025 records the durable decision. GitHub issues #220 through #227 are the implementation and acceptance sequence.

## Scope and ownership

- Namespace: `personal.health.sleep`.
- `CognitiveSpace` remains the ownership and permission boundary. Namespace is a domain partition within that Space, not a new permission boundary.
- The canonical source of truth is local Engine evidence: confirmed records, Trace, FeedbackLoop, GrowthModel, plans, and outcomes. An Adapter may retain its own operational state but cannot replace this canonical evidence.
- Dictation Coach remains the first upstream learning product and evaluation fixture. This contract tests the generic Engine's owner feedback loop only.

## Daily cadence and fixed gate

The owner may submit one valid evening check-in for each local calendar day. The owner selects the local date at confirmation; an Adapter must not silently infer or revise it from device metadata.

| Point | Required behavior |
| --- | --- |
| Before three valid confirmed days | Observation returns `needs_more_evidence`, the valid-record count, remaining count, and typed evidence identifiers. It returns no personalized trend or suggestion. |
| Three valid confirmed days | A bounded baseline may summarize only allowlisted observations over an explicit window. |
| Day seven | Return a preliminary review of coverage, tried actions, observations, evidence gaps, and hypotheses. It is not a success decision. |
| Day fourteen | Apply the final gate below without changing the threshold after results are observed. |

The final gate passes only when all three conditions are true:

1. At least ten valid confirmed daily records exist.
2. At least five dated suggestion attempts are recorded as `performed`.
3. The owner selects at least one adjustment as worth continuing.

Here, a "tried suggestion" is a dated performed outcome; it does not require five different suggestions. Fewer than ten valid records, or ten valid records with fewer than five performed suggestions, is insufficient usage and an **Adapter failure**: the Engine has not received enough owner exposure to evaluate its feedback. With ten or more valid records, at least five tried suggestions, but no retained useful adjustment, the result is an **Engine feedback failure**. The report may also record deployment or privacy friction, but it must not relabel either fixed classification or move the gate after results are known.

## Confirmed check-in input

The normalized payload has no free-form health history, open-ended metadata, or unbounded notes. `space_id` derives from the authorized Surface request; it is not caller supplied. All values are associated with `personal.health.sleep`.

| Field | Required | Unit / valid range | Correction semantics | Persisted summaries | Trace metadata |
| --- | --- | --- | --- | --- | --- |
| `local_date` | yes | ISO `YYYY-MM-DD`; one record per owner-selected local calendar day | A correction replaces the prior current record for the same day; the superseded record is retained only as linked correction provenance | Window and count only, never a raw daily row by default | May include the date only |
| `sleep_duration_minutes` | yes | integer minutes, 60–960 | Replace only through a confirmed correction | Aggregate min/max/median and coverage allowed | No raw value; presence only |
| `sleep_start_local_time` | no | `HH:MM`, 00:00–23:59; timing of the sleep interval | Replace through the same confirmed correction | Bounded aggregate timing may appear | No raw value |
| `sleep_end_local_time` | no | `HH:MM`, 00:00–23:59; when both times are present, their circular interval must agree with duration within 60 minutes | Replace through the same confirmed correction | Bounded aggregate timing may appear | No raw value |
| `daytime_energy` | yes | owner report, integer scale 1–5; no clinical interpretation | Replace through a confirmed correction | Aggregate distribution and coverage allowed | No raw value; presence only |
| `caffeine_within_six_hours_of_sleep` | no | boolean | Replace through a confirmed correction | Aggregate count may appear only when relevant to the active reversible experiment | No raw value; presence only |
| `screen_minutes_in_final_hour` | no | integer minutes, 0–60 | Replace through a confirmed correction | Aggregate count or bounded average may appear only when relevant to the active reversible experiment | No raw value; presence only |
| `input_source` | yes | enum: `typed` or `agent_ocr` | Cannot be silently changed; a corrected submission records its own source | Count by source allowed | Enum allowed |
| `input_confirmation` | yes | exactly `status: confirmed` and `method: explicit_acceptance` or `explicit_correction` | A correction must use `explicit_correction` and link the earlier accepted record | Confirmation counts and method allowed | Status and method allowed |
| `corrects_record_id` | conditional | stable identifier of an earlier same-Space, same-Namespace record | Required precisely when `method` is `explicit_correction`; must not cross Space or Namespace | No | Linked identifier allowed |

The Engine may persist a valid confirmed normalized record and the narrowly typed provenance required to link it to Surface, Trace, and FeedbackLoop evidence. A summary or Trace metadata must never become an alternate raw record store. Correction preserves the original acceptance and its Trace link, marks the old record superseded, and causes the corrected current record—not both—to count for baseline, coverage, and the final gate. Malformed, unconfirmed, duplicate, cross-Space, cross-Namespace, and out-of-window evidence is invalid and does not influence results.

## Media, OCR, and exclusion boundary

An Adapter may extract a screenshot outside the Engine. For `input_source: agent_ocr`, the owner must inspect the normalized allowlisted fields and submit the role-neutral `input_confirmation` with either explicit acceptance or explicit correction. The Engine persists only those confirmed normalized fields and typed provenance. Typed entry can also use explicit acceptance; it must not claim an OCR confirmation when no media-derived input existed.

The following are rejected or excluded from Engine payloads, persisted summaries, Trace metadata, diagnostics, and contract extensions:

- diagnosis; symptoms intended for diagnosis; medication; treatment; medical reports; clinical notes; medical-device interpretation; provider advice;
- raw screenshots, image bytes, raw OCR output, provider reasoning, hidden conversation state, credentials, host/device identifiers, and unrelated health data;
- open-ended health histories, free-form notes, and unrestricted metadata bags.

`EvidenceRefInput` remains governed by the Media Evidence Contract and is not a way to persist these prohibited values. Source media unavailability must not erase confirmed canonical evidence or prevent a manually corrected normalized submission.

## Observation, planning, and outcome authority

An Observation response separates observed values, deterministic aggregates, correlations, hypotheses, evidence gaps, and owner decisions. It must make no medical, causal, diagnostic, or efficacy claim.

Before the three-record threshold, the response is an explicit evidence gap. At or after it, deterministic policy may choose at most one active experiment from a reviewed, versioned allowlist of low-risk, reversible lifestyle actions. The decision cites its selected evidence identifiers, produces a stable action identifier, and is deterministic for identical evidence and policy version. The policy may choose a bounded screen-free final-hour experiment or a consistent wake-time window; it may not offer clinical, pharmacological, diagnostic, or open-ended advice. There is never more than one active experiment at a time.

Suggestions are advisory only. MemoryNexus does not send reminders, operate calendars, messages, device controls, or any external action. It does not add automatic Knowledge Refresh or health-advice lookup; local Trace, FeedbackLoop, and GrowthModel evidence remains authoritative.

Each offered experiment later records exactly one semantic outcome for the relevant date:

| Outcome | Meaning | Effect on evaluation |
| --- | --- | --- |
| `performed` | Owner says the offered action was done. | Counts as one tried suggestion; may be evaluated with later confirmed evidence. |
| `skipped` | Owner says the offered action was not done. | Does not count as ineffectiveness or a tried suggestion. |
| `not_evaluable` | It was done or attempted but the permitted outcome cannot be evaluated. | Does not prove benefit or lack of benefit. |
| `missing` | No outcome was submitted. | Is absence of evidence, not skipped or failed adherence. |
| `corrected` | A prior record or outcome was replaced through explicit correction. | Superseded evidence is excluded; the corrected current evidence is evaluated. |

Every plan and outcome remains linked to the same Space, Namespace, original plan evidence, FeedbackLoop (or equivalent plan lifecycle), and Trace. An action cannot be reported for an unknown, unoffered, cross-Space, or cross-Namespace experiment.

## Adapter and deployment boundaries

#130 independently validates the private Mac mini Local Lab. It is not a blocker for recording this contract or for #221. #222 may select one controlled private Adapter only after both #221 and #130. An authenticated private LAN web Adapter is preferred first, with explicit identity/session/token and allowed-origin policy; it is an interaction choice, not an Engine contract.

Named channels, browsers, OCR providers, and deployment vendors may be documented as Adapter examples only. They must not create Engine branches. A localhost #130 pass does not close #129's independent Trial Profile acceptance.

## Dependency and acceptance sequence

```text
#220 -> #221
#221 -> #223 -> #224 -> #225 -> #226
#130 + #221 -> #222
#222 + #226 -> #227

#228 -> #229   (adjacent P1 learning-adapter track; independent of M9)
```

#227 is a manual fourteen-calendar-day owner/Coordinator gate using the real proven Adapter path. It cannot close immediately after #226's implementation and deterministic-fixture work.
