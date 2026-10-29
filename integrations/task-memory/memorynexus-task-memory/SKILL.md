---
name: memorynexus-task-memory
description: Read, remember, correct or hand off an existing local MemoryNexus task across Codex, Traex and MiniMax using an explicit pinned task binding.
---

# Daily task memory

Use one short intent and an absolute **existing task binding**: “接上这个任务，入口
`/absolute/private/binding.json`” or “把这次结果记住，入口同上”。Read the
[shared protocol](../PROTOCOL.md) and the [daily interface](references/daily-interface.md)
before invoking the pinned helper. This is separate from personal-experiment
memory, initial bootstrap and repository publication. Missing binding or failed
history loading stops work that depends on remembered facts; report the gap.
Never guess task identity from cwd or initialize/migrate another ledger.

The task owner supplies the pinned Python, helper/package location and binding.
`verify` checks the existing task and exact binary/package. For **接上**, use
`read`; keep task ID, revision, record IDs, kind/trust/state, sources and omissions
attached to the summary. If truncated, read the relevant explicit IDs; do not
call incomplete history empty or complete. Ordinary context has a separate
candidate list with bodies. The v2 `current read/check/launch` path returns
candidate indexes only and verifies known copy freshness; use that existing
path when current knowledge delivery is needed. Do not silently downgrade a
failed current check to an ordinary context claim.

For **记住 / 收尾**, form one bounded `execution` or `checkpoint` envelope under
its existing capture policy. Preserve independent acceptance as a separately
attributed execution/evidence reference: a worker's passing self-test does not
overwrite a reviewer's rejection. A checkpoint records progress, not truth or
issue closure. Proposed lessons use existing `review` with current source IDs;
retention never endorses them. Do not store raw chat or tool logs.

For **订正 / 撤回**, first read the exact record ID, show the existing text and
proposed replacement/action, and obtain that content-level owner confirmation.
Only then include the existing `confirmation:"confirmed"` field. Also require
exact-content confirmation for new instructions or when execution/review policy
does not authorize retention. Neither a helper nor a generic “use memory” request
may manufacture confirmation. Ambiguous targets stop at a proposal; competing
corrections never auto-follow to a different ID.

Write the complete request once, then `stage` and `apply` using the
[exact interface](references/daily-interface.md). Report the returned actual ID
and readback state. On pending/failure retain the unchanged request and explicitly
retry the same run only when requested; preserve original client/run/time/key
from the first attempt, even in another client. Do not invent a new key to hide
a conflict. Historical/retracted replay does not make evidence current.

Client integrations are prompt-only unless their actual runtime proves a stronger
entry: [Codex](../../codex/task-memory.md), [MiniMax](../../minimax/memorynexus-task-memory/SKILL.md).
Traex uses this same canonical Skill through its locally verified file/command
capabilities, with `client:"traex"` on new envelopes only. No global install,
hook, scheduler, background process or old-chat history merge is supplied.
The original Codex desktop conversation must explicitly re-read on its next turn;
a fresh CLI probe cannot prove that happened.

For initial import use [bootstrap](../memorynexus-task-bootstrap/SKILL.md).
For repository mechanisms/publication use
[repository review](../memorynexus-repository-review/SKILL.md).
Installation compatibility and the source/binary identity handoff are in the
[daily interface](references/daily-interface.md); reuse the
[onboarding guide](../../../docs/task-memory-onboarding.md). This package is
[MIT licensed](../../../LICENSE).
The [installed resource boundary](references/daily-interface.md#installed-resource-boundary)
distinguishes required local execution resources from source-only navigation and
evidence links. Repository sample discovery requires a separate source handoff;
stop that operation if its handoff is unavailable.

Implementation evidence and unverified client/joint gates: [acceptance](references/acceptance.md).
