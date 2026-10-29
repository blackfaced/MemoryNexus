# #343 implementation and acceptance handoff

Date: 2026-10-29 (Asia/Shanghai). Implementation owner: MemoryNexus.
Source repository: https://github.com/blackfaced/MemoryNexus
Base: `9ceb4608fcf896bcf789e3fb26272965bbe73bbd`.
Branch: `codex/issue343-daily-task-memory`.
Status: uncommitted implementation candidate, not a published fixed revision or
an installed production package. AGENTS.md requires explicit authorization to
commit/push/PR/close; none of those actions were performed. AIWB #155 must pin
the eventual full source commit, not this base alone.

## Resource and identity handoff

Canonical path: `integrations/task-memory/memorynexus-task-memory/SKILL.md`.
License: repository `LICENSE` (MIT), now included by the existing installer.
Direct Skill resources: `references/daily-interface.md`, this evidence record,
`../PROTOCOL.md`, `../memorynexus-task-bootstrap/SKILL.md`,
`../memorynexus-repository-review/SKILL.md`, `../../codex/task-memory.md`,
`../../minimax/memorynexus-task-memory/SKILL.md`,
`../../../docs/task-memory-onboarding.md`, `../../../LICENSE`.
The interface additionally references `docs/current-knowledge-contract.md`.
The installed resources keep those relative paths below `VERSION/package`.

The helper is `integrations/task-memory/scripts/daily.py` (installed as
`VERSION/package/daily.py`). Reused modules: `current_knowledge_task.py`,
`workflow_cli.py`, `workflow_io.py`, `workflow_types.py`; optional reader-binding
repository/bridge behavior retains its existing package dependencies. Install the
complete existing `scripts/` package, not individual files. `install.py` selects
all scripts, canonical resources, shared entry documents and contract documents;
its manifest is the exhaustive installed file/hash/mode inventory. Installation
tests verify canonical/entry/protocol links resolve inside the package, and now
recursively check document links against the exact source-only exceptions in the
[installed resource boundary](daily-interface.md#installed-resource-boundary).
This is bounded execution-resource closure, not a fully packaged source tree.

CLI build source, independently of Python changes:
`9ceb4608fcf896bcf789e3fb26272965bbe73bbd` (no Rust modifications).
Verified binary SHA-256:
`3b1da7c543517a21a6a93a39535c06c20cdbac84497950eb63de39832aaf192c`.
Build used `CARGO_PROFILE_DEV_DEBUG=0` to keep the executable below the existing
128 MiB package limit. Python tests used the real CLT Python 3.9 executable;
the Homebrew Python ancestor is group-writable and the existing path guard
correctly refused it. No permission checks were relaxed.

Final helper SHA-256: `1d7faab811b77c8416fe4a70283d26c0b16c41c2be62ac2295c1c3a3cb444cdb`.
Final source scripts inventory SHA-256: `56e40c0bafb3553598cbd6bbbcd9357e5d22a9e1d36e68e3110f9a41c99ed13e`.
This source-scripts digest is not the larger installed package digest (which also
includes rules/documents); take the latter from the installation manifest.

Use the exact invocations in [daily-interface.md](daily-interface.md). Keep
script source SHA/package SHA and binary build SHA/binary SHA independent.
Installation/version switching remains explicit; uncommitted bytes are not
installable as their base commit. Do not repin a pending write for an upgrade.

## Machine evidence

Local logs are in `/private/tmp/issue343-*`; they contain synthetic data only.
No user ledger, global client configuration or background service was changed.

| Check | Result and limit |
| --- | --- |
| `cargo build --locked --bin memorynexus-feedback-cli` | PASS; separate temporary CARGO_TARGET_DIR, debug info disabled |
| `cargo test --locked --test task_memory_cli` | PASS, 31 tests, zero ignored |
| `python3 scripts/task-memory-smoke.py --binary ABS` | PASS, five groups; explicitly simulated_cli, real-client gate not_run |
| Daily public-process suite | PASS, 10 tests: v1 read/write, candidates/provenance, wrong task/ledger/ID, pin/permission drift, omission and historical ID reads, no invented confirmation, competing correction, lost/truncated/oversize/timeout/nonzero response, exact cross-client retry, failed readback, cleanup recovery, retracted replay |
| Installed package suite | PASS, 13 tests including installed v2 daily roundtrip without repository workflow, complete direct relative resources, immutable files and identity verification |
| Full existing workflow public-process suite | PASS, 154 tests, zero skips (438.004 seconds). First run had one synthetic proxy preflight timeout with a 1-second test window; changed the test window to 5 seconds and reran. Failure log preserved. |
| Skill quick_validate / references / git diff --check | PASS |

Daily final-run file digests were recorded **before execution** and compared
unchanged afterward in `/private/tmp/issue343-final-daily-run.json`; exact command,
CLI SHA and output log are separate from earlier failure logs. Source has no Rust
changes; full legacy Rust fmt/test/clippy was not rerun for this Python/docs slice.

## Actual-client evidence and remaining gate

- Codex Desktop's current implementation conversation executed the pinned daily
  verify/stage/apply against one isolated synthetic task and read back actual
  record `00283140-aa54-4fb6-89d7-83b041265f7b`, task
  `6f307fa7-903b-4d37-b7fa-4771665671b0`, revision 2. The record remained
  execution_reported and retained its evidence reference. This is actual local
  execution, not native Skill discovery or a fresh desktop-conversation test.
- Traex 0.212.1 was invoked once with an explicit canonical Skill, binding and
  precise synthetic correction in its actual `exec --ephemeral` runtime.
  It emitted thread/turn startup plus a skill-budget notice, then **no tool
  actions** before the 180-second harness timeout. The probe was terminated;
  no automatic retry was started. A subsequent Codex read still observed
  revision 2 and the original ID. Traex execution/correction is **NOT VERIFIED**.
- The synthetic correction was an explicitly authorized test case, not owner
  approval of private task content. Client labels in automated fixtures are
  still simulated; they do not replace this real-client result.
- In the original #343 probe, no startup hook or native Skill discovery was installed. Its coverage is
  explicit helper invocation / prompt-only guidance. Fresh-session bridge,
  new worktree and old desktop chat history are separate guarantees.
- AIWB #155 joint gate is **PARTIAL, NOT PASSED** (updated by the follow-up below): owner-selected project discovery in both
  actual clients; Codex write → Traex precise correction → original Codex desktop
  chat's **next turn** explicit read; independent acceptance rejection → repair →
  reacceptance; project-owned business reports and Attempt identities. This
  implementation turn cannot prove a later desktop turn. Do not close #343/#155
  or substitute this sample for #301, #313 or the fourteen-day #283 gate.

Cost observation: Codex used three explicit helper invocations for initial
verify/stage/apply, with readback inside apply; Traex received one test prompt,
then required timeout cleanup. No successful Traex handoff or comparable prior
baseline exists, so coordination-cost reduction is **not evaluated**. The
production short entry remains one intent plus the pinned task binding.

Final-candidate local Codex capability was rechecked after the readback checks
were tightened: task `eb52bed4-5817-4140-9c93-4b50061cecaf`, actual record
`11170be2-2036-45b6-abd7-19d0f558a51b`, revision 2, final scripts digest above.
Four explicit helper calls (read/stage/apply/read) took 0.729 seconds locally;
this excludes context preparation, model time and all cross-client coordination.
The fixture was cleaned. The Traex timeout probe used an earlier candidate
(package `ddadb82a66fce40d5553bd4a4b1867bbae63874cf0cd76e424ba20bd6c7dc386`);
it establishes no successful behavior for either candidate.

All client-probe processes are stopped and both synthetic client ledgers have
been removed. Sanitized evidence/logs remain under `/private/tmp/issue343-*`;
the isolated build remains at `/private/tmp/memorynexus-issue343-target`.

## Resource-boundary repair candidate (local v2, uncommitted)

AIWB #155's frozen `version-1` was assembled from a synthetic test Git snapshot,
not a published upstream source SHA. Its recursive check reached 37 Markdown
documents and found 13 missing edges to eight source targets; #343 independently
reproduced that result. All eight targets exist in the source checkout. The
installed `package/workflow_io.py` exists; its source-evidence link uses the
source layout. Required daily execution resources were present.

The repair documents the exact boundary, including the script path mapping and
the conditional repository sample-discovery handoff. No runtime module is copied
or changed. The install test fixture now includes the selected planning/ADR
documents previously omitted from its synthetic source. Recursive checks reject
unknown missing links, escaped paths, stale exceptions and missing runtime files.
The 13 source-only edges remain explicitly documented; this is not a claim that
all transitive navigation links now resolve locally.

Before-edit copies, the previous delivery manifest and the failed closure report
are retained at `/private/tmp/issue343-resource-boundary-v2/`. The original #155
`version-1` and its evidence are unchanged. Updated installation-suite validation:
**PASS, 14 tests, zero skips, 56.590 seconds**, including the installed daily
roundtrip, recursive resource boundary and negative checks. Command (from repo
root, with `MEMORYNEXUS_TEST_BINARY` set to the isolated binary above):

```text
/Library/Developer/CommandLineTools/Library/Frameworks/Python3.framework/Versions/3.9/bin/python3.9 -B -m unittest discover -s integrations/task-memory/tests -p test_workflow_install.py -v
```

The exact invocation, before/after candidate hashes and log are
`install-run.json` and `install-tests.log` in that evidence directory. Files were
unchanged during execution. After recording this result, the two resource tests
are repeated against the final documentation; their separate evidence is
`final-resource-run.json` and `final-resource-tests.log`. The new candidate's
full changed-file hashes are in `delivery-manifest.json` there; the previous
`/private/tmp/issue343-delivery-manifest.json` is retained as v1 evidence.
Use a separate `version-2` installation for #155 and its own package digest;
never mutate an existing installation or repin a pending write. These test
installations use synthetic Git commits, not upstream release revisions.

Read-only inspection of #155's `evidence/discovery.json` confirms native app-server
`skills/list` returned the canonical repo-scoped Skill, enabled, with zero parse
errors for both the Codex Desktop bundled binary and Traex. That API discovery
does not prove Desktop UI/model invocation or successful task behavior.
The subsequent short-entry Traex read probe (`evidence/traex-read.*`) timed out
after 100 seconds with no tool action (exit -15 from harness termination).
Startup completed in about two seconds; shell snapshot succeeded. State-db
fallback warnings, a Skill-description budget notice and a tenant-config warning
explicitly marked fail-open do not establish the cause. No additional model
request was sent by #343. Traex read/correction and the full next-turn joint
acceptance chain remain unverified.

This repair changes only the canonical Skill, its interface/evidence references
and installation tests. The earlier 154-test workflow run and 31 Rust CLI tests
are prior-candidate evidence; runtime/helper/binary bytes remain unchanged.
