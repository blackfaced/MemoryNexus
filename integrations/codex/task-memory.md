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
