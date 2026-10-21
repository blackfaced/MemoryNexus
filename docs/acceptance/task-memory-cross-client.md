# Shared task memory: real-client acceptance TODO

Status: **PARTIAL — mechanism verified on the planning machine; the Codex client
could not be reached and the owner decision is still missing.**

The owner confirmed on 2026-10-21 that the other machine cannot currently be
provided. A second run on the same date exercised the protocol and the CLI end
to end on the machine that holds this checkout. That run proves the mechanism
works; it does **not** satisfy the Codex-specific case, and it is not owner
acceptance. [#301](https://github.com/blackfaced/MemoryNexus/issues/301) and
parent [#296](https://github.com/blackfaced/MemoryNexus/issues/296) stay open.

Follow [the onboarding guide](../task-memory-onboarding.md). Synthetic CLI
processes with `client: minimax` and `client: codex` are preflight only. Keep
private paths, transcripts, credentials, and ledger contents on the task machine.

## Environment inventory

| Field | Value |
| --- | --- |
| Acceptance date and operator | 2026-10-21, planning machine, operator: Codex/MiniMax session |
| Installed commit/version and binary checksum | commit `232060a525b059b4a5e7481bb7e6425e8318808a`; `target/task-memory/debug/memorynexus-feedback-cli`; sha256 `cdf4d4a21077c88cc16ec936a62194659d3b05b8512edad7e09ca40baeb07481`; 12,948,240 bytes; Mach-O x86_64, debug profile |
| OS and CPU architecture | Darwin arm64, macOS 26.5.2 (25F84) |
| Actual MiniMax executor and local command capability | MiniMax Code desktop session; local shell, local file read/write, stdin and JSON stdout confirmed |
| Actual Codex runtime and local command capability | `codex-cli 0.142.5` present on PATH, but **no model call succeeded** — see below |
| Same config/task ID resolved in both clients | task_id `6bc4d04f-43d5-49d7-9bc4-737b17dce5f3`, config `/Users/mac/hc/mn-pilot/.memorynexus.json` |
| Same ledger resolved by both clients and scheduled entry | `MEMORYNEXUS_TASK_LEDGER` = `~/.local/share/memorynexus/task-memory.sqlite`, owner-only, separate from the experiment ledger |
| Pilot task and separate synthetic isolation task | Pilot `6bc4d04f-…` (synthetic) and isolation task `e5103565-…` (synthetic) in one shared ledger |
| Entry mechanism | explicit wrapper / prompt-only best effort; no hook was installed |
| Existing review entry and explicit config/environment mapping | **No pre-existing review entry exists on this machine** — only a ledger backup job. A one-shot scheduled entry (`32c7eba6-b0a1-49bc-b2db-5d78f13c4da2`, project `/Users/mac/hc/mn-pilot`) was created for this run instead; it mapped to the same ledger via `MEMORYNEXUS_TASK_LEDGER` and the same task via `--config /Users/mac/hc/mn-pilot/.memorynexus.json` |

### Build notes (planning machine)

The task-capable binary had to be built here; the release workflow and the
one-click package do not include `memorynexus-feedback-cli`. Three obstacles
were cleared or recorded:

1. **Stale sparse-index cache.** `--offline` failed because the local index
   cache predated `syn 3.0.9` and `async-trait 0.1.92`.
2. **Index refresh stalls.** With a corporate zero-trust HTTP agent, a local
   TUN proxy and the toolchain running under Rosetta, `cargo` held established
   TLS sessions to the crates CDN but transferred no bytes. Direct, proxied and
   offline modes were all tried. **Workaround that succeeded:** fetch the
   missing `.crate` archives over plain HTTPS, unpack them into a cargo
   *directory source*, write `.cargo-checksum.json` with the archive sha256, and
   build with `--offline` plus
   `--config source.crates-io.replace-with=… --config source.<name>.directory=…`.
3. **No native toolchain.** `rustup toolchain install stable-aarch64-apple-darwin`
   is refused because the installed `rustup` binary itself is x86_64; it only
   offers `--force-non-host`.

### Client reachability (planning machine)

The Codex client could not call any model from this machine, so the
Codex-specific case could not be executed:

- Every model in the local model cache returned
  `not supported when using Codex with a ChatGPT account` (`gpt-6.1-sol`,
  `gpt-5.2-codex`, `gpt-5.1-codex`, `gpt-5-codex`, `gpt-5.1`,
  `gpt-5.1-codex-max`, `o3`).
- The websocket endpoint `wss://chatgpt.com/backend-api/codex/responses`
  returned `403 Forbidden`, and the HTML body is an OpenAI edge block page
  that names the client IP. The same block appears with and without the local
  proxy.
- `auth.json` carries ChatGPT tokens only; `OPENAI_API_KEY` is null, and the
  cached model list exposes only the `cyber` access program, not `chatgpt`.

Other clients on the same machine: `claude` CLI 2.1.201 is installed but not
logged in, and `gemini` CLI 0.49.0 answered once and then hit
`429 quota exceeded` on the free tier (`gemini-3.6-flash`, 20 requests per
day). The cross-session handoff case was therefore executed with a **second
fresh MiniMax Code session** instead of Codex. That covers the fresh-session
read-back mechanism and the record-ID continuity; it does **not** satisfy the
"fresh Codex session" wording of the acceptance criteria.

## Case evidence

Record each result as passed, failed, or not-run. Include relevant request keys,
record IDs, context revision, expected/actual result and a redacted evidence
reference. Do not paste private task contents into this document or GitHub.

| Case | Result | Evidence to collect |
| --- | --- | --- |
| Real MiniMax writes; fresh Codex reads and continues | **Partial** | MiniMax wrote checkpoint `4f564a1a-edcb-4e44-8360-b189dcdde10e` (revision 2). A **fresh session with no conversation history** read it back from the ledger alone and wrote execution `e7a2ae9b-2aee-4ea6-b09b-776e5244cdc9` (revision 3), naming the checkpoint id it read. The reading client was a second fresh MiniMax Code session, not Codex — see client reachability above. |
| Real Codex writes; fresh MiniMax reads and continues | **Not run** | The main session read back `e7a2ae9b-…` (trust `owner_confirmed`) with no pasted history, so the read-back half is proven, but no Codex write exists. |
| Existing scheduled review runs | **Partial** | No pre-existing review entry exists on this machine. A one-shot scheduled entry was created for this run (cron `32c7eba6-b0a1-49bc-b2db-5d78f13c4da2`, run `e5cc5684-3415-4287-a920-c445b6da9468`, session `mvs_c66abffebe86498b8796dcd89489c57d`), read the same ledger with explicit env/config, and submitted a sourced candidate. The scheduled run produced candidate `3820b51b-1186-40b6-9c1f-98032b0c8952` (trust `review_candidate`, `state: current`, revision 8, `source_ids` = `4f564a1a-…` + `4afaac78-…`) under receipt key `cron-task-review-20261021T232221-0d12f9b0`. A separate process then read it back through `context` with `candidate_count: 1` at task revision 8. The case is proven for a **newly created** entry; the original "existing" entry is still absent. |
| Task isolation and bounded context | **Passed** | Isolation task `e5103565-1f2b-47bc-b5a0-0fd11fe12ea8` shares the ledger and returned `entries: 0`, `candidates: 0`. Task A's reads report `omitted_count`, `truncated` and `candidate_count` separately. |
| Lost response, exact retry and changed-payload conflict | **Passed** | First write `397d03eb-1e21-4a35-b459-faf5fc618bda` → `accepted`. Exact replay → `replayed` with the **same** record id. Same key with changed payload → `conflict`, message `request conflicts with retained task state`. |
| Owner correction and source invalidation | **Passed** | Correcting `e7a2ae9b-…` produced `4afaac78-6f06-4ba6-a031-a542ab26b322`; the old revision disappeared from `context`, and the candidate that had sourced it was invalidated (`candidate_count` 1 → 0). |
| Owner withdrawal | **Passed** | Retracting `397d03eb-…` returned `state: retracted`; the record is no longer returned by `context` while the remaining entries stay current. |
| Read/write failure | **Passed** | An invalid envelope is rejected as `invalid_input` with `status: error`; no substitute ledger was created and no false success was reported. A wrong `--ledger` path for `restore` produced an error instead of silently succeeding. |
| Backup and restore | **Passed** | `backup` produced a `memorynexus-task-ledger` archive; `restore` recreated it at a new path. Both ledgers then reported the same task_id, the same revision (7), the same record ids and the same states. Replaying a pre-backup write on the restored ledger returned `replayed` with the original record id and created no duplicate. |
| Owner usefulness decision | **Not run** | Requires the owner. |

## Completion rule

- [x] Supply target access and complete the environment inventory. — Done **for this
  machine only**; the owner ruled on 2026-10-21 that no second machine can be
  provided, so the inventory covers the machine that holds this checkout.
- [x] Run the simulated preflight with the installed binary first. —
  `scripts/task-memory-smoke.py` → `mode: simulated_cli`, `status: passed`,
  `real_client_acceptance: not_run`, 5/5 checks passed.
- [ ] Execute every real-client case and attach redacted evidence to #301. — 8 of 9
  cases passed, 1 partial, 2 not-run (both need a reachable Codex client). The
  evidence is in this document; it has **not** been posted to #301 yet.
- [x] Resolve implementation defects in bounded tickets, then rerun affected cases.
  — No implementation defect was found: every error path behaved as the contract
  describes, so no defect ticket was opened.
- [ ] Record the owner's usefulness decision. — Requires the owner.
- [ ] Only then close #301 and reconcile parent #296. — Blocked by the two items
  above.

Passing this task-memory gate does not close the independent #283 fourteen-day
personal-experiment gate or authorize #284–#286 cutover/deletion.
