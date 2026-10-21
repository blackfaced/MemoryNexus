# Shared task memory: real-client acceptance TODO

Status: **NOT RUN — target-machine access unavailable**.

The owner confirmed on 2026-10-21 that the other machine cannot currently be
provided and asked to preserve the remaining work in docs and the issue tracker.
This record does not constitute acceptance. [#301](https://github.com/blackfaced/MemoryNexus/issues/301)
and parent [#296](https://github.com/blackfaced/MemoryNexus/issues/296) remain open
until the real-client evidence and owner decision are recorded.

Follow [the onboarding guide](../task-memory-onboarding.md). Synthetic CLI
processes with `client: minimax` and `client: codex` are preflight only. Keep
private paths, transcripts, credentials, and ledger contents on the task machine.

## Environment inventory

| Field | Value |
| --- | --- |
| Acceptance date and operator | TODO |
| Installed commit/version and binary checksum | TODO |
| OS and CPU architecture | TODO |
| Actual MiniMax executor and local command capability | TODO |
| Actual Codex runtime and local command capability | TODO |
| Same config/task ID resolved in both clients | TODO — use a redacted identifier |
| Same ledger resolved by both clients and scheduled entry | TODO — do not publish a private path |
| Pilot task and separate synthetic isolation task | TODO |
| Entry mechanism | TODO — forced hook / explicit wrapper / prompt-only best effort |
| Existing review entry and explicit config/environment mapping | TODO |

### Planning-machine observations (2026-10-21)

These come from the **planning machine**, not the task machine, and they do not
satisfy any row above. They are recorded because they describe build and access
obstacles worth checking on the task machine before starting the real run.

| Field | Observed on the planning machine |
| --- | --- |
| OS and CPU architecture | Darwin arm64, macOS 26.5.2 (25F84) |
| Rust toolchain | rustc/cargo 1.96.1, but `rustup` reports `stable-x86_64-apple-darwin` and runs under Rosetta emulation |
| Python | 3.13.13 |
| Codex runtime | `codex-cli 0.142.5` on `PATH` |
| MiniMax executor | MiniMax Code desktop session (local shell, local file access, stdin/JSON stdout) |
| Installed CLI | `~/.local/bin/memorynexus-feedback-cli`, built 2026-08-30, Mach-O x86_64, **no `task` command group** — `task` is not in its usage string |

Three obstacles blocked building the task-capable binary here:

1. **A stale sparse-index cache.** `--offline` fails because the local index
   cache predates `syn 3.0.9` and `async-trait 0.1.92`, so the resolver cannot
   select the locked versions.
2. **Index refresh stalls.** With a corporate zero-trust HTTP agent, a local
   TUN proxy and the toolchain running under Rosetta, `cargo` holds established
   TLS sessions to the crates CDN but transfers no bytes.
3. **No native toolchain.** `rustup toolchain install stable-aarch64-apple-darwin`
   is refused because the installed `rustup` binary itself is x86_64; it only
   offers `--force-non-host`.

Consequences for the task machine: install the binary from a checkout that can
fetch its dependencies, or transfer a binary built elsewhere together with its
SHA-256. A prebuilt binary does not currently exist — the release workflow and
the one-click package do not include `memorynexus-feedback-cli`. Also confirm
that the task machine actually runs a **review** entry on a schedule. A backup
entry is not a review entry: a backup job produces no sourced candidate for the
other client to read.

## Case evidence

Record each result as passed, failed, or not-run. Include relevant request keys,
record IDs, context revision, expected/actual result and a redacted evidence
reference. Do not paste private task contents into this document or GitHub.

| Case | Result | Evidence to collect |
| --- | --- | --- |
| Real MiniMax writes; fresh Codex reads and continues | Not run | Receipt, same task/record ID and continuation using ledger context without copied chat |
| Real Codex writes; fresh MiniMax reads and continues | Not run | Reverse-direction receipt and readback |
| Existing scheduled review runs | Not run | Actual scheduled-entry execution, explicit env/config, source IDs, candidate ID and other-client readback; manual invocation is only preflight |
| Task isolation and bounded context | Not run | Task B cannot read task A; candidates distinct from confirmed instructions; omissions reported |
| Lost response, exact retry and changed-payload conflict | Not run | Same IDs on replay; no duplicate; conflict for changed same-key request |
| Owner correction and source invalidation | Not run | Old claim and dependent lesson no longer guide a fresh session |
| Owner withdrawal | Not run | Current reads and new export omit withdrawn content |
| Read/write failure | Not run | Explicit missing-history/pending-write state; no new substitute ledger or false success |
| Backup and restore | Not run | Both clients use restored identities/state; pre-backup request still replays without duplication |
| Owner usefulness decision | Not run | Continue / revise / disable, with one concrete reason |

## Completion rule

- [ ] Supply target access and complete the environment inventory.
- [ ] Run the simulated preflight with the installed binary first.
- [ ] Execute every real-client case and attach redacted evidence to #301.
- [ ] Resolve implementation defects in bounded tickets, then rerun affected cases.
- [ ] Record the owner's usefulness decision.
- [ ] Only then close #301 and reconcile parent #296.

Passing this task-memory gate does not close the independent #283 fourteen-day
personal-experiment gate or authorize #284–#286 cutover/deletion.
