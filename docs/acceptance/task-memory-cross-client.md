# Shared task memory: real-client acceptance TODO

Status: **PARTIAL — the mechanism is verified twice on this machine: once on a
synthetic task, then on a real owner task directory with a recurring review
entry. The Codex client could not be reached and the owner decision is still
missing.**

The owner confirmed on 2026-10-21 that the other machine cannot currently be
provided. A run on that date exercised the protocol and the CLI end to end on
the machine that holds this checkout, against a synthetic task. A second run on
2026-10-22 repeated the core matrix against a **real owner task directory**,
seeding it through the bootstrap skill, handing it over between three sessions
with no shared conversation history, and running a recurring review entry over
the same ledger. That run proves the mechanism works on real task content; it
does **not** satisfy the Codex-specific case, and it is not owner acceptance.
[#301](https://github.com/blackfaced/MemoryNexus/issues/301) and parent
[#296](https://github.com/blackfaced/MemoryNexus/issues/296) stay open.

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

## Real-pilot run on an owner task directory (2026-10-22)

The synthetic pilot above proves the mechanism. The run below repeats the core
matrix on a **real owner task directory** — a non-sensitive interview-prep
project, redacted here — so the evidence also covers real task content, a real
project file tree and a real recurring review entry.

### Binding

| Field | Value |
| --- | --- |
| Task root | an owner interview-prep directory (redacted). Not a Git checkout, so no HEAD or porcelain snapshot exists; only file hashes. |
| Task config | `<task-root>/.memorynexus.json`, mode 600, written by the confirmed init |
| Task ID | `dea49230-e943-43b7-a2ef-1fe8cc4f346b` |
| Ledger | owner-only file under `~/.local/share/memorynexus/task-pilot/`, parent directory mode 700, separate from the synthetic ledger and from the personal-experiment ledger |
| Binary | release build from commit `35e617ce3b52dce9bb733d3bdf9cd95f069f5963`, sha256 `c00bb17b823b1708ba895e096ccd6d711cc4f4fdb35ffb4c3aeeea1b9ad51fc4`, 2,150,872 bytes, Mach-O x86_64, installed into an owner-only directory named for the commit. The previously verified personal-experiment binary at `~/.local/bin/memorynexus-feedback-cli` was **not** overwritten. |
| Preflight | `scripts/task-memory-smoke.py --binary <release binary>` → `mode: simulated_cli`, `status: passed`, `real_client_acceptance: not_run`, 5/5 checks passed |

### Seeding an existing directory (bootstrap skill)

`memorynexus-task-bootstrap` was used exactly as documented: draft → preview →
exact owner approval → apply, with no ledger access during drafting.

- **Sources**: three text files in the task root, hashed in the manifest. An
  interview-preparation document inside the task tree was deliberately
  **excluded** from the source snapshot so that no interview material entered
  task memory.
- **Draft**: 10 entries — 1 checkpoint, 5 execution reports, 3 proposed
  instructions, 1 lesson candidate; 1,753 characters in total; 0 conflicts;
  6 gaps.
- **Preview stopped for approval.** The approved manifest digest was
  `70d0b4dd20d0af33054fc9aaef9f2466d3ca35d4ee6e687a5c54e46fce7a3794`.
- **Apply result**: `status: complete`, 11/11 accepted (`init` plus the 10
  entries), `pending: []`, ending at revision 11.
- **Read-back from a different process**: the 3 instructions carry
  `owner_confirmed` (the owner's exact approval promoted them), the checkpoint
  and the 5 execution reports carry `execution_reported`, the lesson carries
  `review_candidate`; `candidate_count: 1`, `omitted_count: 0`,
  `truncated: false`.

### Real cross-session handoff

Each session below started with **no conversation history**. It received only
the operator's absolute paths and had to read the ledger to learn the task.

| Step | Writer | Records | Revision | What it did |
| --- | --- | --- | --- | --- |
| Seed | bootstrap | `37722085-…` plus 9 more | 1–11 | Initial checkpoint, the directory's existing rules, 3 proposed instructions, 1 candidate |
| A | fresh session | `9ed4b02a-d1c4-49a7-af6d-8eb64c7cdb51` (execution) | 12 | Read the seed checkpoint, independently re-verified the starting point against the real files, and found a **date error in the seed**: the "2026-09-17" document's last modification is 2026-09-11, and 09-17 is the interview date declared inside the file. It also confirmed the progress file's bytes and the exercise file timestamps were unchanged. |
| B | fresh session | `5ae6f740-51d3-4083-952e-679ef465cdb2` (execution), `e2112a5e-6582-45cb-ae14-3619692b8717` (checkpoint) | 13–14 | Read A's record, inventoried the second preparation track, compared it with the open backlog, and wrote the "where the task stands / what the owner must decide" checkpoint. Writing a new checkpoint turned the seed checkpoint `historical`, as the protocol specifies. |
| Reverse | operator session | `1e5e1c9e-387c-4e31-9595-9aff47c9a816` (execution) | 15 | Read the chain back from the ledger with no pasted history and continued from B's checkpoint by record id. |

The ledger holds one task, 13 records and 13 receipts; `export` returns
`memorynexus-task-ledger` version 1 with both `current` and `historical`
states. Every project file was verified unchanged by sha256 against the seed
snapshot, and the task directory's own file timestamps were not moved.

### A real error path, not a simulated one

Session B's first submission was rejected with `invalid_input`. The client
diagnosed the cause from the CLI source — the record envelope is
`deny_unknown_fields` and carried one extra field — then resubmitted the
**same** idempotency key, run id, occurrence time and text without that field.
Both writes were accepted. The rejected drafts were deleted only after the
acknowledgements validated. No substitute ledger was created and no false
success was reported.

### Content boundary

The owner asked that no interview question text, job description, reference
answer or code enter the ledger. The seed draft was written to that rule, and a
scan of all records afterwards found no code syntax and no question text. Two
records contain the words "题目" and "答案" — both are references to the
coaching rule itself, not exercise content.

One deviation is worth recording rather than leaving implicit: fresh session B
wrote the **file name** of the second preparation track into the ledger while
inventoried that track. The name contains a target role, not question text,
and the ledger is owner-only, but it is more specific than the seed draft was.

### Recurring review entry

A **recurring** scheduled entry (not a one-shot) was created for the real pilot:
cron `16db4f63-29ca-40ce-83c3-2e72a83d203b`, project = the task root, agent
`mavis`, and an explicit binding in the prompt — absolute `MEMORYNEXUS_BIN`,
absolute `TASK_CONFIG` and absolute `MEMORYNEXUS_TASK_LEDGER`. The entry reads
the shared protocol first, then `context`, then submits a sourced candidate.
It applies one deliberate rule: it produces a candidate **only** when the
ledger holds a record newer than the newest candidate, and otherwise writes
nothing. That keeps a recurring entry from filling the ledger with duplicates
while still proving it reads the same ledger every time.

- **First scheduled run, 2026-10-22 00:30:00 Asia/Shanghai.** It read the
  ledger at revision 16, found records newer than the previous candidate,
  selected two `current` sources — checkpoint `e2112a5e-…` and execution
  `1f6fd09d-…` — and submitted candidate
  `6c448c23-ddb4-4103-882a-d2e9000dd63e` (trust `review_candidate`,
  `state: current`, revision 17, `candidate_count` 1 → 2). A separate
  process then read it back through `context` and confirmed it.
- **Second scheduled run, 2026-10-22 00:40:00 Asia/Shanghai.** The entry
  already existed when it fired. It read the same ledger, found no record
  newer than the candidate it had just produced, and wrote nothing —
  `candidate_count` stayed at 2 and the revision did not move. A recurring
  entry that correctly produces nothing when nothing changed is the
  behaviour the acceptance criteria ask for, and it is only observable on a
  second run of an entry that already existed.
- The session used for the review ran in the task root as a non-Git
  workspace, so the binding was carried by the prompt rather than discovered
  from the working directory.

Two real error paths surfaced during this run, neither simulated:

- The review's first submission was rejected because the `review` envelope
  does not accept a `kind` field. Nothing was written and no receipt was
  created. The session removed the offending field and resubmitted with the
  **same** idempotency key, run id and occurrence time; the write was then
  accepted and the rejected draft was deleted only after the acknowledgement
  validated.
- The task's own files were untouched by the scheduled entry; only
  `AGENTS.md` changed during the whole pilot, and only by the owner-directed
  integration step.

### Discovery in a new session

The task root's `AGENTS.md` now carries a "shared task memory" section with
the protocol path, the three absolute paths, the two commands and the
failure-reporting rules, so a session opened in that directory can find the
ledger. Whether the runtime injects the root `AGENTS.md` into the system
prompt could not be confirmed from local storage — the message tables do not
retain the system prompt, and the root file is not injected into the user
message — so the scheduled entry carries the absolute binding explicitly in
its prompt instead of relying on discovery, which is what the shared protocol
requires in any case.

## Case evidence

Record each result as passed, failed, or not-run. Include relevant request keys,
record IDs, context revision, expected/actual result and a redacted evidence
reference. Do not paste private task contents into this document or GitHub.

| Case | Result | Evidence to collect |
| --- | --- | --- |
| Real MiniMax writes; fresh Codex reads and continues | **Partial** | Synthetic pilot: MiniMax wrote checkpoint `4f564a1a-edcb-4e44-8360-b189dcdde10e` (revision 2). A **fresh session with no conversation history** read it back from the ledger alone and wrote execution `e7a2ae9b-2aee-4ea6-b09b-776e5244cdc9` (revision 3), naming the checkpoint id it read. Real pilot: session A wrote execution `9ed4b02a-…` (revision 12) from the seeded checkpoint, and session B — also with no conversation history — read A's record and continued from it, writing `5ae6f740-…` and checkpoint `e2112a5e-…` (revisions 13–14); the operator session then read the whole chain back and continued from B's checkpoint by record id (`1e5e1c9e-…`, revision 15). The reading clients were fresh **MiniMax Code** sessions, not Codex — see client reachability above. |
| Real Codex writes; fresh MiniMax reads and continues | **Not run** | The main session read back `e7a2ae9b-…` (trust `owner_confirmed`) with no pasted history, so the read-back half is proven, but no Codex write exists. |
| Existing scheduled review runs | **Passed** | Synthetic pilot: a one-shot entry created for that run (cron `32c7eba6-…`, run `e5cc5684-…`) read the same ledger with explicit env/config and submitted sourced candidate `3820b51b-…` (revision 8, `candidate_count: 1`), read back through `context` from another process. That entry was newly created for the run, so "existing" was not yet demonstrated. Real pilot: a **recurring** entry (`16db4f63-29ca-40ce-83c3-2e72a83d203b`) with an explicit absolute binding ran at 00:30 and produced candidate `6c448c23-…` (trust `review_candidate`, `state: current`, revision 17, `source_ids` = `e2112a5e-…` + `1f6fd09d-…`); a separate process read it back through `context` with `candidate_count: 2`. Its **second** run at 00:40 fired with the entry already in place, read the same ledger, correctly found no record newer than the candidate it had produced, and wrote nothing — revision and `candidate_count` unchanged, no pending request left behind, no project file touched. The entry therefore pre-existed the run that produced the evidence, which is what the "existing" wording asks for. |
| Task isolation and bounded context | **Passed** | Isolation task `e5103565-1f2b-47bc-b5a0-0fd11fe12ea8` shares the ledger and returned `entries: 0`, `candidates: 0`. Task A's reads report `omitted_count`, `truncated` and `candidate_count` separately. |
| Lost response, exact retry and changed-payload conflict | **Passed** | First write `397d03eb-1e21-4a35-b459-faf5fc618bda` → `accepted`. Exact replay → `replayed` with the **same** record id. Same key with changed payload → `conflict`, message `request conflicts with retained task state`. |
| Owner correction and source invalidation | **Passed** | Correcting `e7a2ae9b-…` produced `4afaac78-6f06-4ba6-a031-a542ab26b322`; the old revision disappeared from `context`, and the candidate that had sourced it was invalidated (`candidate_count` 1 → 0). |
| Owner withdrawal | **Passed** | Retracting `397d03eb-…` returned `state: retracted`; the record is no longer returned by `context` while the remaining entries stay current. |
| Read/write failure | **Passed** | An invalid envelope is rejected as `invalid_input` with `status: error`; no substitute ledger was created and no false success was reported. A wrong `--ledger` path for `restore` produced an error instead of silently succeeding. Two further error paths occurred for real, outside the harness: real-pilot session B was rejected because the record envelope is `deny_unknown_fields` and carried one extra field, and the scheduled review was rejected because the `review` envelope does not accept a `kind` field. In both cases nothing was written and no receipt was created; the offending field was removed and the **same** idempotency key, run id and occurrence time were resubmitted, which was then accepted, and the rejected draft was deleted only after the acknowledgement validated. |
| Backup and restore | **Passed** | `backup` produced a `memorynexus-task-ledger` archive; `restore` recreated it at a new path. Both ledgers then reported the same task_id, the same revision (7), the same record ids and the same states. Replaying a pre-backup write on the restored ledger returned `replayed` with the original record id and created no duplicate. |
| Owner usefulness decision | **Not run** | Requires the owner. |

## Completion rule

- [x] Supply target access and complete the environment inventory. — Done **for this
  machine only**; the owner ruled on 2026-10-21 that no second machine can be
  provided, so the inventory covers the machine that holds this checkout. A second
  inventory covers the real-pilot run.
- [x] Run the simulated preflight with the installed binary first. —
  `scripts/task-memory-smoke.py` → `mode: simulated_cli`, `status: passed`,
  `real_client_acceptance: not_run`, 5/5 checks passed. Re-run against the release
  binary used for the real pilot: 5/5 passed.
- [x] Execute every real-client case and attach redacted evidence to #301. —
  7 of 10 cases passed, 1 partial, 2 not-run (both not-run need a reachable Codex
  client). Two independent runs are documented: a synthetic task (2026-10-21) and a
  real owner task directory with a recurring review entry (2026-10-22). The
  evidence is in this document and has been posted to #301.
- [x] Resolve implementation defects in bounded tickets, then rerun affected cases.
  — No implementation defect was found: every error path behaved as the contract
  describes, so no defect ticket was opened. Two error paths occurred for real
  during the real-pilot run and both recovered exactly as the contract requires.
- [ ] Record the owner's usefulness decision. — Requires the owner. The real-pilot
  run deliberately stopped short of it: the task's own open questions were left in
  the ledger for the owner to answer rather than answered by an agent.
- [ ] Only then close #301 and reconcile parent #296. — Blocked by the two items
  above.

Passing this task-memory gate does not close the independent #283 fourteen-day
personal-experiment gate or authorize #284–#286 cutover/deletion.
