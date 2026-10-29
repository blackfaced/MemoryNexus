# Codex task-memory entry

Use the [canonical daily Skill](../task-memory/memorynexus-task-memory/SKILL.md)
for an existing task's read, record, correction and handoff. It owns the shared
workflow and references PROTOCOL.md; this file only describes Codex capabilities.

The operator supplies the exact pinned Python/helper and private task binding.
Verify local process execution, stdin, JSON stdout and shared filesystem access
in the actual Codex runtime. A remote or restricted runtime without access is
blocked; changing a `client` label is not client acceptance.

Use `client:"codex"` on new write envelopes only. An explicit retry retains the
original bytes and provenance. Register the canonical Skill or a pointer in the
actual runtime-supported project instruction entry only after its location and
loading behavior are verified. Preserve existing AGENTS.md and global settings.
Prompt-only loading is best effort; no startup hook is installed by this entry.

The existing [Codex bridge](../../docs/acceptance/codex-bridge.md) supports pinned,
read-only fresh-session delivery through `current launch`. This is distinct from
daily writes and an already-open desktop chat. After another client changes the
ledger, the original desktop chat must explicitly run the daily read again on
its next turn. Neither old conversation merging nor automatic removal of old
model context is promised. Real desktop-turn evidence and fresh CLI evidence
must be recorded separately with source IDs and revisions.
