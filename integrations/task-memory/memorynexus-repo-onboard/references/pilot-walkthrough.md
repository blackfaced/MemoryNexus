# Pilot-informed acceptance walkthrough

These are **sanitized synthetic scenarios**, informed by feedback from one real
onboarding pilot. Names, counts, content and identities below are illustrative;
none is a real receipt, consent record or production acceptance result. Walk
through the relevant cases when reviewing an R1 proposal. R2 defines the exact
interfaces and later issues supply executable acceptance; this document does not
assert those mechanisms already exist.

| Scenario supplied to the Skill | Required observable proposal / stopping point |
| --- | --- |
| Repository has an inbox, morning review and three allowed output areas | Map capture/review/knowledge/context and preserve upstream limits; propose how the added flow fits, not a replacement by filename. |
| Root has a task config; a parent has a different task config | Use only the root identity, propose incremental onboarding, no bootstrap/init and no claim of deduplication without ledger evidence. |
| No root config; seed supported by four current text sources | Use existing bootstrap JSON, generate private full preview/digest and stop before apply. |
| One rule allows browser validation; another rule and cron forbid it | Keep the applicable conflict visible. Record an existing owner decision when supplied; show every still-inconsistent entry, not just one line. R1 does not edit them. |
| A source changed after approval; entry texts remain identical | New snapshot/import/digest requires a fresh human confirmation. Do not self-authorize, use the old digest or invoke apply as a trial. |
| Agent already applied a different digest and reports “content matched” | Report applied versus approved identities as different; preserve receipts and add a deviation note, do not rewrite approval history or roll back without scope. |
| Approved automatic capture is mistaken for “confirm every future entry” | Explain helper's continuing execution/review retention policy and unsupported per-entry-only bootstrap policy; stop if the owner rejects it. |
| Seed contains only “see rules.md” and “see incidents.md” | Add supported bounded experience detail that can change a future action; never manufacture an incident to fill an entry quota. |
| Inbox has an untested suggestion labeled `verified: true` | Treat the flag as upstream assertion, not owner confirmation. Free notes remain outside current execution/review inputs pending R2. |
| A proposed lesson cites a base entry about schedules, while its actual evidence is an incident | Create/select the relevant sourced base report; do not rely on a file merely appearing in manifest sources. |
| Inbox rows lack a target; one row already relates to two documents | Propose independent consumer routing. Do not discard the row, backfill upstream or assign one global processed state. |
| Rows lack stable IDs; timestamps are nonmonotonic | Record identity/cursor gaps; do not use ts as high-water. An append call alone cannot prove no other writer/rotation. |
| A JSONL body contains binary image bytes | Exclude payload from seed and report a normalization/acceptance gap. “JSONL” is not proof that all content is permitted text. |
| Two legacy binaries exist; one opens storage for help and one lacks task | Record unsupported/unverified capability; use pinned installation evidence and postpone verified CLI preflight. No speculative invocation, service setup or alternate ledger. |
| Host report claims context needs `{}`; installed version differs from source docs | Use `{}` in examples, record actual binary version and evidence gap. Do not rewrite current contract from one report: v1 also allows empty context stdin and `task --ledger` includes restore. |
| Imported config is untracked; next upstream job runs `git add -A` | Report identity-sharing risk and propose local ignore or deliberate tracking choice; no repository maintenance during R1. |
| Existing backup prompt enumerates other ledgers only | Report missing coverage and separate first-backup/deployment work; no maintenance-script or cron edits under onboarding. |
| Count says “19 = 8 + 6” and excludes a 5-item category | State denominator and exclusions; 8 + 6 = 14. Keep configuration count, planned triggers and actual successful runs separate. |
| Inventory hash was taken before a final edit/chmod | Recompute metadata after final mutations; keep approved artifacts immutable and publish a companion correction instead of changing approval bytes. |
| Generated output sits outside the root but another selected source copies it | Identify the indirect feedback path, exclude derived regions and retain lineage; do not claim the loop impossible based on output location alone. |

## Bounded experience, with evidence status intact

Synthetic source `notes/incident.md`, revision `s1`, says:

> A checker printed the requested target name and the review reported success.
> A later independent readback found the target absent. The printed name came
> from the request, not the observed page. The new readback was tried once.

A useful **reported execution summary** is:

> The incident document reports a false success: the checker echoed a requested
> target without observing it. An independent readback contradicted that report.
> Applicable to success checks that can echo their own inputs; a replacement
> check has only one reported trial. Evidence: incident.md at s1.

A separate **candidate lesson** can propose requiring independent observed output
before reporting this class of action successful. The summary and candidate are
not owner-confirmed facts. Do not label a hypothesis about the root cause as a
proven cause unless the cited evidence actually supports that conclusion. This
fixture is a teaching example, not a seed to import into a real project.

## Shared source, complete dependencies, selective reassessment

Design-only table; “adopt/defer” are hypothetical decisions, not receipts:

| Consumer | Shared input | Decision evidence | Navigation-only destination | Proposed decision |
| --- | --- | --- | --- | --- |
| A: mechanism navigation | incident `s1` | incident `s1`, mechanism index `m1` proving the section exists | `knowledge/checker.md#success-evidence` | Adopt a bounded pointer labeled “document-reported evidence”; no claim that all branches are verified. |
| B: incident lessons | incident `s1` | incident `s1`, correction note `c1` saying replacement remains unverified | `notes/incidents.md#false-success` | Defer the recommendation pending the missing verification. |

When correction `c1 → c2` gains evidence but incident `s1` and index `m1` are
unchanged, **B** must become eligible for reassessment; A must not duplicate its
publication. B's old deferred decision cannot suppress this change. Conversely,
changing the index to remove its section affects A even if the incident is
unchanged. If an output claims a conclusion from a linked document, that document
is evidence, not just navigation, and must be tracked accordingly.

Do not infer that only the originally shared source controls invalidation.
Rule/profile changes also need versioning and renewed approval where relevant.
Source absence/read failure is not confirmed deletion. A file edit can invalidate
a derived claim without confirming its replacement. After publication, ledger
staleness and external file replacement may finish separately; do not report
current output merely because one stage completed.

## Memory lifecycle walk-through to hand to R2

1. A reported observation can be useful while disputed. Store attribution,
   applicability and uncertainty; adoption does not certify truth.
2. New progress makes the prior checkpoint historical. A confirmed correction
   supersedes a record revision; dependent lessons become stale. Those states
   differ from owner withdrawal and from omission by the context budget.
3. Review from current base evidence can create a fresh candidate; no candidate
   may turn itself or another candidate into independent evidence.
4. External-file freshness is unknown unless the selected source was checked.
   Current task v1 stores evidence references without watching their files.
5. Old useful experience need not disappear by age. Automatic TTL deletion does
   not exist; propose periodic applicability review without promising a daemon.

Real-client exchange, actual scheduler runs, data restoration and two published
documents remain separate runtime acceptance. A successful document walkthrough
proves none of them.
