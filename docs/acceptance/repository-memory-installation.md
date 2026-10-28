# Repository-memory installation / schedule acceptance template

Copy outside the repository into an owner-only evidence directory. Fill observed
values, exact commands, exit codes, artifact paths and final SHA-256 values.
Default every row to NOT RUN; PASS requires evidence. This template does not
authorize installation, bind, migration, client edits or scheduler edits.

| Check | Expected evidence | Status / artifact |
| --- | --- | --- |
| Fixed build | Full source commit, build command/result, actual CLI SHA, toolchain | NOT RUN |
| Version install | New stable directory; source/package/Python/binary hashes; sorted manifest; 0700/0600; task help without storage | NOT RUN |
| Bundle independence | Reduced env/cwd; original checkout/build artifact absent; installed verify still works | NOT RUN |
| Python | Actual sys.executable, SHA, no symlink/group-writable ancestor; external runtime dependencies identified | NOT RUN |
| Machine binding | Owner-only profile/binding/state; exact config/task ID, ledger, format/incarnation, rules, targets; verify and public status | NOT RUN |
| Caller inventory | Every interactive/scheduled/wrapper/backup entry resolves current ledger and compatible binary; no retired-ledger writer | NOT RUN |
| Manual workflow | Same launcher/binding from both entry templates; capture → two Lens decisions → explicit publish approvals; no-op repeat | NOT RUN |
| Installed current entry | Verified package read/check/launch; exact stdin digest/action; cross-install and tamper refusal; permanent delivery claim | NOT RUN |
| Reader/bridge binding | Separate private reader state, matching package/workflow pins; actual bridge/client/config hashes and input inventory | NOT RUN |
| Actual MiniMax | Fresh actual session and executor evidence, not just client label | NOT RUN |
| Actual Codex | Fresh actual session, same task/state and bidirectional read/write; original-byte replay | NOT RUN |
| Schedule definition | Owner-selected existing entry ID; before, exact approved diff, independently read after; preserved unrelated fields/notification policy | NOT RUN |
| Real trigger | Provider run ID/source, expected slot/timezone, actual start, runner runs/artifacts/exit; keep unverified_trigger gap if present | NOT RUN |
| Missed run/catch-up | Expected slot vs provider history; machine availability; no inferred catch-up | NOT RUN |
| Query failure/pending | status with/without immutable plan, retained stdout/errors, no hidden retry/repair or promised push delivery | NOT RUN |
| Backup compatibility | Actual pinned backup caller supports current ledger; consistent snapshot + isolated restore + full logical comparison | NOT RUN |
| Isolated rollback | Previous package CLI supports current format; explicit rebind; no DB downgrade/fallback; restored incarnation invalidates old plans | NOT RUN |

Record per-phase run ID, trigger evidence, task/workflow/profile revision,
incarnation, plan digest and genuine approval provenance, receipts, targets and
before/after hashes, pending IDs/report errors. Capture/review automatic retention
is not truth confirmation; prose publication and enabled fixed neutralization
have distinct authority. Keep original artifacts immutable; use separate addenda.

Recurring backup coverage: **NOT RUN / GAP** until the actual selected entry is
updated under separate authorization and observed. Manual snapshot evidence:
`<path/hash/format/time/restore artifact>`; this alone is temporary coverage.
Deleted review cron restored: **NO**. MCP/maintenance scripts: **NOT CHANGED**.

Completion statement: implementation/isolated tests `<result>`;
production deployment `<result>`; real-client #301 `<result>`;
repository workflow #313 `<result>`; personal experiment #283 `<unchanged>`.
Do not close a real-client or scheduling gate using simulated labels.
