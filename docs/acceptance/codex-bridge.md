# Installed Codex bridge acceptance

Status: **implementation plus bounded local real-client probe; no production
activation or complete #301/#313 acceptance.** Ticket #337 follows #312 and the
initial disposable bridge evidence in #336. See the [bridge guide](../repository-memory-codex-bridge.md).

## Automated scope

`test_workflow_codex_bridge.py` covers strict spec/source verification, owner-only
new destination, no overwrite, exact stdin/ack/environment, config/client/dependency
and symlink-selection drift, version mismatch, invalid task/cwd/history/material,
malformed/unknown/out-of-order/missing/failed client events, nonzero exit, stdout/
stderr bounds, timeout, post-execution drift and adapter-owned process groups.
A real installed MemoryNexus adapter/CLI proves receipt and duplicate-claim behavior.
The deterministic Codex event emitter is synthetic, not model acceptance.

The final focused suite has 8 tests (including multi-case boundary loops).
Full workflow/bootstrap and exact-head required CI results belong to the PR.
Rust/schema/ledger semantics are unchanged; no extra Rust unit tests were added.

## Real local Codex — 2026-10-28

Fixed installed source: `ff0bb042252b0d8345ea6836157ccba6ade7b811`.
Installed using the official installer into a fresh private temporary version,
then generated the executable using that installation's bridge generator. This
proves isolated installation, not stable production deployment. The unchanged
MemoryNexus test CLI source is `46434b06a7304ce421aa8d020657675066f8249e`, SHA-256
`3ff8ad8c4ae9d398ccde82e82bdff10f86010d408376e6ec9a639d883a275b23`.

The existing local client is Codex 0.162.0 with ChatGPT login. A transparent private
test observer forwarded the actual client events unchanged to the generated bridge,
retaining only stdin digest, session identity and command/output-digest metadata.
It neither fabricated events nor chose the marker. Its executable was the selected
client pin; 15 additional pins covered the actual AIWB wrapper, registry, native
Codex selection/bytes, wrapper release manifest/payload, Python and user config.

Initial preflight correctly refused the default Homebrew Python dependency path's
permissions, and also exposed the old 128 MiB hash-read limit for the 246,508,160-byte
Codex executable. Hashing now streams bounded chunks up to 512 MiB per file. Path
trust checks were retained. The successful isolated observer executed the **same
AIWB policy wrapper code with the installation's pinned Python**, instead of the
wrapper's Homebrew shebang. The production wrapper, registry, interpreter paths
and permissions were not changed. This is a reviewed test invocation difference,
not evidence that the default production wrapper's dependency closure is accepted.

The test harness created a new two-task v2 ledger, recorded instruction A, corrected
that exact ID to B, and used installed `current read` followed by `current launch`.
The generator's fixed instruction contains neither A nor B. Fresh Codex reported
`/bin/zsh -lc 'cat workspace-b/marker.txt'` with exit0; its output digest matches the
B marker. No command event read A. The process stdin/read/delivery hashes matched;
material was purged and a repeat launch returned `delivery_already_attempted`
without a second client invocation.

| Evidence | Identity / digest |
| --- | --- |
| read_id | `40438065-12ae-4876-98ac-4030ecdd33fb` |
| fresh session | `01a180ef-49ca-7070-a043-184ebb790db3` |
| 646-byte material | `7b03e02b788f8f38c3c56d52b428c2c0d4e9afbebf4ce5969ed17ff63114f33b` |
| B action output | `df9dcce272cb6dc6e5e249fb5753710b4b18121305026e9d8141f918f9a9eb01` |
| generated bridge | `a1012fb2058d80532590e8e5a84c89893cb310cc615fb0d115fb498f8cba1102` |
| sanitized client proof | `367f9f766b34f3f58be3aa03d5cf296676f667c596703ede28a10db1248f65a5` |

Private artifacts retain the spec, installed manifest, generated bridge, synthetic
ledger, read/result/evidence, delivery claim/receipt and sanitized proof/verdict.
The observer held stdout in memory only while forwarding it; it did not retain raw
model output, full model input or stderr. Existing authentication was not copied.

**PASS in this scope:** generated bridge pre/post declared-pin checks, real fresh
Codex process delivery/action and adapter repeat refusal. This is CLI-reported
command evidence, not independent server-side model-input capture. Complete
AGENTS/skills/native-memory/cache inventory remains UNKNOWN. Layered configuration,
undeclared dependencies, detached client children and finite-observation races are
not proven covered. The wrapper must be reviewed with a suitable interpreter and
complete declared inventory before any stable production activation.

MiniMax↔Codex bidirectional handoff, real repository consumer input, scheduler,
recurring backup and owner continuation decision remain separate. No production
binding, existing private ledger, client settings or schedule was modified. #301,
#312, #313, parent #317 and #283 retain their own gates.
