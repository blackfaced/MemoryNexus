---
name: memorynexus-task-memory
description: Continue an owner-authorized directory task using its shared local MemoryNexus task ledger.
---

# MiniMax task-memory entry

Read and follow [the shared protocol](../../task-memory/PROTOCOL.md) on every
invocation. This is separate from the existing personal-experiment feedback Skill.

The operator must supply an absolute binary path, task config path and
`MEMORYNEXUS_TASK_LEDGER`. Verify that the **actual MiniMax executor** can launch
that local binary, send stdin, read JSON stdout and see the same filesystem as
Codex. A model name does not establish these capabilities. If unavailable, report
the access blocker; do not invent remote transport or a substitute ledger.

Start with `context`; finish meaningful work with a bounded authorized record.
Use `client: "minimax"` for new requests only; exact retries keep the original
envelope. Scheduled invocations read the same protocol and explicit environment,
then submit sourced candidates under policy. Save exact pending files and report
failures as specified by the protocol.

The actual executor's Skill location, loading syntax and hooks must be verified
on the task machine. Merely copying this file does not prove it was loaded.
Prompt-only loading is best effort. Do not install a second scheduler, change
global settings, promise proactive delivery or mark #301 accepted from this file.
