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

For first-time initialization from existing task material, use the shared
[bootstrap Skill](../../task-memory/memorynexus-task-bootstrap/SKILL.md). Copy
its complete bundle and retain the common protocol path. Preview first; exact
owner approval precedes initialization and writes. Do not reseed existing tasks.

For repository-source workflows, use the shared
[repository-review Skill](../../task-memory/memorynexus-repository-review/SKILL.md)
through the [versioned installation entry](../../../docs/repository-memory-installation.md).
Interactive and scheduled sessions use the identical absolute Python, installation
and private binding: `PYTHON -I -S -B VERSION/launch.py`. Keep the actual provider's
schedule/delivery settings unchanged until their specific deployment is approved.
Use independent schedule readback and provider run evidence; a manual run or
`client=minimax` label does not prove a real trigger or automatic catch-up.

For current-knowledge delivery, use the same installed launcher with
`current read`, `current check` and `current launch`, and the separate private
reader binding described in the installation guide. Launch requires a reviewed,
pinned bridge for this actual client; no bridge is supplied by this pointer.
Keep fresh-session input/action evidence separate from synthetic acceptance.
