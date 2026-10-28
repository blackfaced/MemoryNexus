# Codex task-memory entry

Read and follow [the shared protocol](../task-memory/PROTOCOL.md) on every task
session. The operator supplies absolute binary/config paths and the same
`MEMORYNEXUS_TASK_LEDGER` used by the other client and existing review entry.
Verify local execution, stdin/JSON stdout and shared filesystem access from the
actual Codex runtime; a remote/sandboxed runtime without access is blocked.

After verifying that the runtime reads the task's instruction file, append a
reference to this entry and the pinned shared protocol location. Preserve existing
AGENTS.md content. Do not assume a Codex-specific hook configuration or install
global settings. An enforced hook or explicit wrapper requires a separate actual
runtime check; instructions alone provide **best-effort** loading.

Read `context` before work; save a bounded authorized record after meaningful
work. Use `client: "codex"` for new writes only. Preserve the original request
bytes and provenance on retries, even for a MiniMax-created pending request.
Review text remains a sourced candidate; exact-content owner confirmation is
required for instructions, correction and withdrawal. Apply the shared pending,
error and recovery rules. Never silently substitute chat history for a failed
read or claim another machine was accepted from a simulated CLI check.

For first-time initialization from existing repository material, use the shared
[bootstrap Skill](../task-memory/memorynexus-task-bootstrap/SKILL.md). Copy its
complete bundle and retain the common protocol path; show the exact draft and
obtain approval before any initialization/write. Existing tasks require review,
not automatic reseeding.

For repository-source workflows, use the shared
[repository-review Skill](../task-memory/memorynexus-repository-review/SKILL.md)
through the [versioned installation entry](../../docs/repository-memory-installation.md).
Invoke the pinned `PYTHON -I -S -B VERSION/launch.py` with the same explicit private
binding used by MiniMax and the scheduled Agent. Installation does not switch
the ledger, approve a workflow binding, or install Codex hooks.

For current-knowledge delivery, use the same installed launcher with
`current read`, `current check` and `current launch`, and the separate private
reader binding described in the installation guide. Launch requires a reviewed,
pinned bridge for this actual client; no bridge is supplied by this pointer.
Keep fresh-session input/action evidence separate from synthetic acceptance.
