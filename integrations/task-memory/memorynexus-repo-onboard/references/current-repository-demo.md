# Current-repository onboarding demonstration

**R1 design demonstration; not an approval, import, installation or real-client
acceptance.** This uses public MemoryNexus documents inspected at design baseline
`1b2400edd758325ed8cf437eb0a7da129f293ffc`. The example's paths and proposed
consumers are placeholders. No private pilot content, owner UUID, approval token
or machine ledger is included. Regenerate a private preview against the checkout
actually inspected rather than applying a copied historical manifest.

## Inventory and selection

Selected scope is this repository, not nearby projects. Read `AGENTS.md`,
`README.md`, `docs/TODO.md`, ADR-027/028/029, both task/workflow contracts and the
shared protocol. Only the four files named in the seed support its records; the
other reads inform the proposal. No ledger, CLI, upstream execution script, runtime or
scheduler is inspected by this demonstration.

| Mechanism | Source evidence | Interpretation / unknown |
| --- | --- | --- |
| Task context and recording | `integrations/task-memory/PROTOCOL.md`, “Every session” | Existing explicit read/record/review flow; startup enforcement is runtime-specific and unverified here. |
| First seed | `memorynexus-task-bootstrap/SKILL.md` and manifest reference | Existing read-only preview and later approved import; no recurring file collector. |
| Authority and work planning | `AGENTS.md`, `docs/TODO.md`, ADR-027/028/029 | Issues are executable work authority; docs describe direction, not live issue acceptance. |
| Existing native schedule observation | ADR-027, “MiniMax、微信与提醒” | Reported #274 result; no current machine schedule or run evidence inspected. |
| Future file increment/Lens output | ADR-029 and behavior contract | Accepted direction; R2 interfaces and later runtime remain separate work. |

No current-scope conflict was found in these selected documents: ADR-028/029
explicitly bound their extension of ADR-027. This does not certify all repository
files mutually consistent. Root task config presence must be checked in the
actual checkout; the preview route below requires that it is absent.

## Eight-section profile proposal

1. **Identity:** `demo-repository-onboard`, revision 1, design-only. The local
   snapshot belongs in the private artifact inventory. No actual task ID exists
   in this proposal; task identity must come from future confirmed init.
2. **Binding:** local private companion records root/config, an external
   nonexisting `not-created.sqlite`, and bootstrap bundle commit. CLI and future
   workflow script bindings are **unverified/not implemented**. No binary is
   needed for preview. Config ignore choice and backup coverage remain deployment
   gaps; do not manufacture a production ledger location.
3. **Existing mechanisms:** retain the protocol and issue-based workflow shown
   above. Do not replace project docs or install startup hooks. The repository
   does not become a task merely because the Skill generated a proposal.
4. **Sources:** allow the four exact text files in the seed for bootstrap. Use
   their preview hashes as inspected versions. Future source identities and
   extractor version belong to R2, not the bootstrap's `sources` array. Exclude
   runtime directories, ledgers, pending data, private reports and generated Lens
   outputs. Selected public docs are authoritative for design decisions, not
   independent machine observations.
5. **First import:** [the seed template](current-repository-seed.json) contains
   one checkpoint, two attributed execution reports and one candidate lesson.
   It combines task navigation with a concrete reported delivery limitation.
   No instruction is proposed and no free inbox note is disguised as execution.
   The fixed bootstrap capture policy would authorize continuing automatic
   execution/review retention; preview grants no such permission.
6. **Incremental:** propose checking selected document versions before future
   use, producing bounded differences and preserving lineage. Changed external
   files are not confirmed corrections; missing files are not withdrawals. Until
   implementation, keep updates as proposals and use only separately authorized
   current protocol operations. Do not repeatedly bootstrap an initialized task.
7. **Consumers:** design-only A `continuity-notes` asks what a new session can
   rely on; B `verification-notes` asks what remains unverified. Both use the
   `delivery-observation` report. A can propose a bounded owner-query instruction
   *for its document* while B can defer a delivery-success conclusion. Each output
   retains “document reports” attribution; neither promotes the source to truth.
   Proposed external targets are `continuity.md` and `verification.md` in the
   private deployment directory, with managed regions excluded from capture.
   A depends on the report plus current protocol for recommended usage; B on
   the report plus the real-client acceptance document for verification scope.
   These extra future inputs require registration before use and are not secretly
   included in this seed. See the pilot walkthrough for selective invalidation.
8. **Operation:** manual proposal only, no schedule configured. Future manual
   repeat/correction checks precede any owner-selected schedule. Results are
   retrieved from private run artifacts and ledger receipts, not an assumed IM
   notification. No production backup or MCP work is included.

## Reproduce only the preview with the existing helper

First inspect the current checkout and read the four seed sources again. If a
root `.memorynexus.json` exists, take the existing-task route below instead. In a
new external private directory, copy `current-repository-seed.json` to an authored
`draft.json` and replace **only** the following placeholder bindings with the
canonical absolute paths of the inspected checkout and a proposed external path:

- `task_root` and `binding.config` (root `.memorynexus.json`);
- `binding.ledger` (external, explicitly not created by this demonstration).

The existing bootstrap schema is otherwise unchanged: no `source_id`, `trust`,
`consumer`, `policy` or profile fields are appended. Keep complete text, evidence
and uncertainty in the existing supported fields. Run from the bootstrap bundle:

```sh
python3 scripts/bootstrap.py preview \
  --draft /absolute/private-demo/draft.json \
  --state-dir /absolute/private-demo/new-preview
```

Record the returned digest, source hashes, full preview and actual Git snapshot
privately. Independently count entry kinds and Unicode text lengths from
`manifest.json`'s `draft.entries` and `draft.binding.display_name`; do not copy
prose totals. Compute final sizes/hashes/modes after the last write. Confirm no
task config or proposed ledger was created. **Do not invoke apply, even with
this demonstration's digest.** No fixture UUID or approval receipt is published.

## Existing-task route (static walkthrough)

Given a root config, the inventory records its declared identity and binding,
marks ledger verification not run, and delivers this profile's sections 6–8 as
an incremental proposal. It omits bootstrap preview/init and leaves pending
identities intact. Candidate updates are not claimed to be deduplicated until a
later authorized context read can compare the same task. For documented examples
use explicit `{}` as context stdin; the existing task contract also permits empty
stdin. No live context invocation is part of this walkthrough.

## R1 completion evidence and limits

Review A1/A2 against the inventory, eight sections, both task routes and the
[pilot scenarios](pilot-walkthrough.md). The example's supported experience,
candidate grounding, fixed capture policy and exact reapproval boundary must be
visible in the private preview. Any approval request must show the full proposal.
Record actual preview execution evidence separately from this reusable example.
No walkthrough closes #301, #283 or R6, and none verifies live two-Lens behavior.
