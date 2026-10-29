---
name: memorynexus-task-memory
description: MiniMax compatibility entry for the shared MemoryNexus daily task-memory Skill.
---

# MiniMax task-memory capability entry

Read the [canonical daily Skill](../../task-memory/memorynexus-task-memory/SKILL.md)
for all daily task behavior and the shared protocol. Keep this old path for
compatibility; when installing the canonical Skill for discovery, do not register
both copies under the same name. This is separate from personal-experiment memory.

Verify that the actual MiniMax executor can launch the pinned local helper/CLI,
send stdin, parse JSON stdout and access the explicit task binding and shared
filesystem. A model name or `client` label alone proves none of these. Use
`client:"minimax"` for new envelopes; retries preserve original provenance.

Verify its real Skill-loading entry on the task machine. Prompt-only loading is
best effort. Existing scheduled review may use the canonical protocol with its
explicit binding; preserve its schedule and unrelated steps. No scheduler, hook,
global configuration, remote transport or proactive App/IM delivery is installed.
A manual probe does not accept a real scheduled run or close #301.
