# Understanding and questions

Use this Lens when the owner asks what their own explanations or practice show
about **one named topic in one bound task**. This is a model-derived observation
in the current conversation. Ordinary “接上” only follows the daily read flow;
it does not start this review.

## Bound and read the evidence

Use the task binding and pinned public calls in the
[daily interface](daily-interface.md). Ask for the missing topic or task entry
when either is ambiguous. Read only the owner-authorized bounded records and
their relevant evidence references. Retain the loaded task ID, response revision,
record ID/revision, kind, trust, state, source IDs, client/run, occurrence time and
read time, associated with each supported observation. Preserve these exact
fields for evidence inspection; compact presentation does not discard them or
require writing a separate file. These are existing fields, not a new truth
schema; a revision alone is not an immutable content identity.

`context` selects by state, category and ledger commit order, **not topic semantic
search**. Filter the supplied bounded evidence for the named topic. Preserve
`omitted_count`, `truncated`, `candidate_count`, unresolved references and missing
coverage; say which records were actually read. Inspect relevant known same-task
IDs explicitly when necessary within the authorized bound. Insufficient evidence
remains a gap, not permission to scan other tasks, the whole ledger or raw chat,
or to start deep review.

Keep the selected reading path's guarantees. The v2 current adapter supplies
candidate indexes without bodies, including `purpose:"history"`; do not claim
those bodies were read or switch paths to evade a failed current check. If the
owner's question requires candidate review, use the protocol's explicit ordinary
daily `read --request ABS` with the relevant `record_ids`, label this as candidate
review, and keep the original candidate identity. Candidate text is not a current
fact. Failed history loading stops conclusions that depend on it.

## Attribute before interpreting

| Material | What it can support |
| --- | --- |
| Owner's source-linked explanation or practice | The specific explanation/attempt shown, its result and its limits; distinguish an observed artifact from the owner's report of doing it |
| Owner self-report or owner-confirmed summary | What the owner says or accepts as a description; confirmation of wording does not demonstrate mastery |
| Agent answer, code, reading or passing test | Agent/tool work with its actual attribution; correctness does not establish the owner's understanding |
| `execution_reported` with a path | A reported action; a reference alone is not independent execution verification |
| Review candidate or earlier Lens output | A proposed interpretation with original sources, never an additional independent source |

Read references only within the authorized scope and retain whether each material
reference was read and whether a declared SHA-256 was checked. Show material gaps
in plain language in the compact answer; provide the per-reference read/hash
details on evidence inspection. Missing, unread, unhashed or mismatched references
keep the corresponding limits visible. A hash match establishes bytes, not
authorship, independent verification or mastery.
Trust labels are local provenance assertions, not human authentication. Keep
conflicting records attributed; time order alone does not settle their truth.

## Show four parts

Match the owner's language. For a Chinese request, write the result in Chinese,
including the topic and explanatory labels; keep technical field names in the
underlying evidence. Aim for **150–250 Chinese prose characters** on the first
display, excluding citation markers and URL targets. Use a brief scope line with
the task's display name, topic and plain-language coverage; full task UUIDs and
technical reading-path details belong to evidence inspection. Then give **one
short sentence with one main point per part**, avoiding long chains of clauses:

1. **Your explanations and practice:** the bounded, traceable owner evidence and
   exactly what it demonstrates. If none is available, say so.
2. **Self-reports and unverified material:** claims of knowing/doing, Agent work,
   candidates and verification gaps, with their actual origins.
3. **Questions, changes and conflicts:** unresolved questions and differences
   between sources, with currentness and missing coverage visible.
4. **One testable next step:** one question or small attempt with an observable
   response that could resolve a stated gap. Do not score ability or infer
   psychology, personality or hidden motives.

Use familiar display labels such as **解释与实践 / 还未验证 / 疑问与变化 /
下一步**. Describe provenance in the owner's language: for example, “你的自述”、
“助手写的答案”、“只有执行报告” or “待核实的推测”, rather than presenting native
trust/state fields as the explanation. Use everyday wording such as “原请求” or
“原样重发” instead of literal technical translations. Select the evidence and
material gap that matter to this review, rather than listing every input item;
retain the rest for evidence inspection without suppressing a material conflict.

Attach a compact evidence marker and a plain-language limitation to each material
judgment. Use an unambiguous record-ID abbreviation plus revision within this
one task; expand the abbreviation if it collides. Preserve the original
`evidence_refs.path` in the loaded evidence. Resolve an existing readable local
reference against its explicitly authorized evidence base to an **absolute
clickable file path** in the displayed link, so moving the answer does not break
it. Do not guess the base from cwd. Identify missing or unresolved references as
gaps, without fabricating working links. Full IDs, native fields, times, original
and resolved source paths, and read/hash checks remain available on request, tied
to those markers. Avoid full evidence tables and long technical inventories in
the first display; if requested, expand the specific evidence in the conversation.
Do not automatically save a detailed report as a substitute for showing it.

Use existing source identities directly; do not turn “the Agent wrote it
correctly” or “the owner accepted this summary” into an unsupported mastery
conclusion. A more polished Agent answer must not strengthen judgments about
the person's evidence.

## Re-read changes; retain only on request

On a reported correction or withdrawal, re-read the relevant current records and
explicit history markers through the bound path before rerunning. Superseded
content is historical; stale/retracted markers are not usable current evidence.
Do not reconstruct withdrawn text from an earlier Lens response. If the changed
source is unavailable, withdraw the dependent observation and show the gap. A
fresh review uses current source identities, not the old conclusion as evidence.

Display is read-only: do not write an execution, checkpoint or lesson merely
because this Lens ran. If the owner separately asks to retain it, use the shared
protocol's existing `review`/`correct` flow, exact confirmation rules and current
same-task non-lesson source IDs. A retained interpretation remains model-derived;
owner confirmation accepts its wording, not a proof of ability. Reuse existing
source-dependency invalidation; do not self-cite Lens outputs, auto-promote lessons
or update authoritative GrowthModel/PracticePlan state.
