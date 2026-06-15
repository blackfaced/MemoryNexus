# Dictation Coach MVP

Dictation Coach is the first upstream product direction for MemoryNexus.

It validates MemoryNexus as a long-term feedback engine rather than a generic
memory app. The product uses daily dictation and spelling practice to prove the
loop:

```text
Capture -> Performance -> Reflection -> Planning -> Observation -> SleepCycle
```

## Positioning

Dictation Coach helps a learner practice Chinese native-language dictation and
English spelling / sentence dictation through short daily loops.

The Engine remains generic. Dictation Coach is an upstream product and adapter
scenario, not a hard-coded Engine role model.

## Namespaces

Recommended first namespaces:

```text
child.chinese.dictation
child.english.spelling
child.english.sentence-dictation
```

`child.*` is a domain naming convention, not a permission boundary. Permissions
still come from `CognitiveSpace`.

## Surface Mapping

| Surface | Dictation Coach Use |
| --- | --- |
| Capture | Record today's characters, words, phrases, or sentences. |
| Performance | Submit manual dictation / spelling result. |
| Reflection | Explain mistake type and recurring pattern. |
| Planning | Generate tomorrow's 10-minute practice. |
| Observation | Show 7-day trends, mastery, stability, and error distribution. |

## First Flow

1. Capture today's word list.
2. Submit expected items and actual result.
3. Classify mistakes deterministically.
4. Return immediate feedback.
5. Write Trace and FeedbackLoop evidence.
6. Run manual SleepCycle.
7. Update GrowthModel.
8. Generate PracticePlan for tomorrow.
9. Observe 7-day trend.

## Mistake Taxonomy

Chinese:

- wrong character;
- visually similar character;
- homophone;
- missing stroke;
- extra stroke;
- stroke-order issue;
- component placement issue.

English:

- missing letter;
- extra letter;
- letter order error;
- double-letter error;
- sound-spelling mapping error;
- capitalization error;
- missing word in sentence dictation.

## MVP Boundaries

Do:

- manual input only;
- deterministic baseline classification;
- short next practice generation;
- Trace-backed evidence;
- GrowthModel and PracticePlan as the value loop.

Do not:

- OCR;
- handwriting recognition;
- full tutoring chatbot;
- broad curriculum;
- multi-child management;
- complex UI;
- cloud-only generation.

## Success Criteria

The MVP succeeds when a local deterministic flow can show:

- what was assigned;
- what was attempted;
- which mistake type appeared;
- whether the same pattern repeats;
- what tomorrow's practice should be;
- how the last 7 days changed.
