# Repository memory: versioned installation and external scheduling (#312)

This guide packages the existing CLI and runner. Installation creates files in a
new directory only. Deployment, workflow binding, ledger migration, client edits
and scheduler edits are separate owner-authorized operations. Keep MCP and
maintenance-script packaging deferred. The exact behavior remains in the
[v1 contract](repository-memory-v1.md); record machine evidence in the
[installation acceptance template](acceptance/repository-memory-installation.md).

## 1. Install one immutable version

Use a clean, fixed source commit containing #312, a CLI built from that commit,
and one or two reviewed Lens rule files. Build the existing binary, not a legacy
alias (`cargo build --release --locked --bin memorynexus-feedback-cli`). Preserve the build command/result and actual
binary SHA-256. The installer verifies supplied bytes and storage-free task
capabilities; a hash alone does **not** prove which source produced a binary.

Use absolute real paths without symlinks. Choose a persistent, owner-only parent,
for example `/Users/OWNER/.local/share/memorynexus/versions` (0700), then a **new**
child named for the full source revision plus a rule-bundle revision. Do not use
`/tmp`, a worktree's `target/`, or a mutable `current` symlink for deployment.
Never change an existing version to keep its old package digest.

Select an absolute Python interpreter that passes the same owner/ancestor trust
checks as the runner. `/usr/bin/python3` can be a CLT shim returning a different
`sys.executable`; a Homebrew installation may have group-writable ancestors.
These are rejected. One option is an owner-only `venv --copies` created with a
trusted Python under a persistent private path. Probe its actual `sys.executable`
and permissions before installation. Do not chmod system/Homebrew ancestors or
weaken path checks. The interpreter executable is pinned by path **and SHA-256**;
its standard library/system dependencies remain external. This is not a portable
Python distribution. Updating that environment requires a fresh validation/install.

After substituting reviewed absolute values (none are discovered from PATH):

```sh
"$PYTHON" -B "$CHECKOUT/integrations/task-memory/install.py" \
  --source "$CHECKOUT" --revision "$FULL_COMMIT_SHA" \
  --binary "$BUILT_BINARY" --binary-sha256 "$BINARY_SHA256" \
  --python "$PYTHON" --destination "$NEW_VERSION" \
  --rule-file "$MECHANISMS_RULES" --rule-file "$LESSONS_RULES"
"$PYTHON" -I -S -B "$NEW_VERSION/launch.py" verify
```

The installer reads tracked bytes from that exact Git commit, ignoring dirty
checkout changes. It requires Git only during installation, copies the supplied
CLI and rules, calls `task --help` (no ledger), and creates:

```text
VERSION/                         0700; never replace in place
  manifest.json                  0600; source commit, Python/CLI hashes, inventory
  launch.py                      0600; shared restricted entry
  bin/memorynexus-feedback-cli    0700
  package/                       0700
    workflow.py, workflow_*.py   0600; runner modules at PACKAGE ROOT
    rules/lens-1.txt, lens-2.txt  0600; second file optional if rules are shared
    integrations/...             full Skills, bootstrap helper, protocol/pointers
    docs/..., decisions/...      relevant contracts and handoff documents
```

`package_sha256` is SHA-256 of canonical sorted `{path,sha256}` entries relative
to `package/`, exactly as the existing runner defines it. `manifest.json` is
outside the package to avoid a recursive digest; its installation inventory also
covers launcher and binary, with size/mode. Runtime imports only occur after this
inventory matches. Owner control is the trust boundary, not a signed distribution.
No ledger/config/profile is generated or opened. All new directories/files are
0700/0600 except the executable. An interrupted installation may leave a partial
directory: preserve it for diagnosis and select a new destination; there is no
overwrite, deletion, automatic retry, or active-version switch.

## 2. Bind the same entry in every client

Keep machine `profile.json`, `binding.json`, state and ledger outside Git in
owner-only directories. Use the [exact binding/profile fields](repository-memory/artifacts-v1.md),
not the eight-section R1 proposal format. `binding.binary`, `binary_sha256`,
`python`, `package`, `package_sha256` come from `verify`; each consumer's
`rules_sha256` identifies its installed rule file. Pin absolute config, task root,
ledger, profile, state root and both targets. The config task ID must match; do
not initialize a new task to cure a binding error. Task root/config trust checks
are explicit deployment prerequisites; installation does not chmod a repository.

```sh
"$PYTHON" -I -S -B "$VERSION/launch.py" verify --binding "$BINDING"
"$PYTHON" -I -S -B "$VERSION/launch.py" status --binding "$BINDING"
```

`verify --binding` checks files, paths, hashes, profile/config identity and CLI
capabilities **without querying the ledger**. `status` uses the public CLI and
checks the actual workflow/ledger state; an unbound workflow or incompatible
schema is a separate failure. Missing/changed data never selects a fallback DB.
Activation uses the existing `task --config CONFIG workflow-bind` JSON request
and fresh exact owner approval; see the [walkthrough](repository-memory/walkthrough-v1.md).
Changing a package, rules or binding requires that explicit transition. It can
stale existing decisions/publications; it does not rewrite Markdown by itself.
Do not run a neutralization/publication loop merely to finish an installation.

MiniMax, Codex and the scheduled Agent all use this same absolute `VERSION`,
`PYTHON` and `BINDING`. The shared repository-review Skill remains responsible for
interpreting prepared evidence and creating submissions; the runner has no model
SDK or model invocation. The launcher accepts only `verify`, `inspect`, `prepare`,
`preview`, `apply`, `status`, using the existing workflow arguments. It does not
dispatch arbitrary shell commands, bootstrap, bind, migrate or backup requests.
Always use `-I -S -B`; `-I` is enforced. Inherited PATH/PYTHONPATH and ledger env
cannot override the binding. Optional `MEMORYNEXUS_TRIGGER_FILE` is preserved as
unverified runner evidence; CLI child processes retain their existing allowlist.

| Operation | Entry and authority |
| --- | --- |
| First import | Packaged bootstrap Skill/helper; complete preview + fresh exact digest approval; new tasks only. |
| Increment | `launch.py prepare --workflow capture`; Agent submits bounded drafts; preview/apply under saved capture policy. |
| Lens review | `prepare --workflow review`; each consumer decides independently with tracked dependencies. |
| Document publish | `prepare --workflow publish`; complete diff + precise plan approval for prose. Pure neutralization may use an explicitly enabled profile policy. |
| Backup/restore | Pinned **compatible** binary's `task --ledger ... backup/restore`; separate operation and evidence. No launcher scheduling or maintenance-script replacement. |

Ordinary task `context/record/review` still uses the packaged CLI and the shared
protocol with explicit config/ledger. Repository workflow `review` is a distinct
operation. Client Skill loading is best effort unless the actual runtime proves
otherwise. Copying a pointer or changing a client label does not pass #301/#313.

## Current knowledge from the installed version

The same verified installation exposes `launch.py current read`, `current check`
and `current launch`. These dispatch the frozen [current-knowledge adapter](current-knowledge-contract.md);
the existing workflow commands above retain their meanings. Installation includes
its modules, contract/examples, acceptance matrix and ADR-030. It does not create
or select a real client bridge.

Prepare a separate owner-only reader binding using the frozen
[current binding fields](current-knowledge-contract.md#binding). Copy the five
installation values (`binary`, `binary_sha256`, `package`, `package_sha256`,
`python`) from installed `verify`. Pin the existing task/config/ledger and a
separate private reader state root; do not reuse workflow runs. Repository mode
also references the matching workflow binding, whose package upgrade/reconciliation
requires its existing explicit authorization. Never silently rebind to make a read
pass. Use a fresh request and a new direct child of the reader state for each call:

```sh
"$PYTHON" -I -S -B "$VERSION/launch.py" current read \
  --binding "$READER_BINDING" --request "$READ_REQUEST" --run-dir "$READ_RUN"
"$PYTHON" -I -S -B "$VERSION/launch.py" current check \
  --binding "$READER_BINDING" --evidence "$READ_RUN/evidence.json" --run-dir "$CHECK_RUN"
"$PYTHON" -I -S -B "$VERSION/launch.py" current launch \
  --binding "$READER_BINDING" --evidence "$READ_RUN/evidence.json" --run-dir "$LAUNCH_RUN"
```

Every call verifies the immutable installation before importing its adapter and
rejects a binding for another installation before creating reader artifacts.
Launcher preflight errors use the existing failed/exit2 envelope; after dispatch,
the adapter retains its exact result/artifact schema and exit0/2/3 semantics.
Unlike workflow dispatch, current dispatch preserves the authentication environment
needed by the pinned bridge; the adapter still overrides the CLI/ledger environment.
Authentication values must never be copied into evidence.

`check` does not start a client. `launch` requires an explicitly selected executable
and SHA in the reader binding; it sends exact JSON bytes on stdin after rechecking.
Retain the original read evidence. An unknown delivery cannot be retried by deleting
the claim or changing run directories. Installation success and synthetic bridge
acknowledgement do not establish an actual client session or model action.

Before activating an actual bridge, retain this private handoff:

| Item | Required evidence |
| --- | --- |
| Selected runtime | Client/executor version, exact executable/config hashes, supported fresh-session invocation; no guessed hook flags |
| Bridge | Reviewed fixed executable/hash, client configuration pins, stdin delivery trace and bounded acknowledgement only after receipt |
| Input inventory | AGENTS/skills/native memory/cache and managed targets; every declared external copy remains unknown in v1 |
| Fresh action | New real session ID, matching input digest, harmless B-marker action after A→B; no B answer preloaded in prompt/env/argv |
| Unknown outcome | Retained delivery claim and client evidence reconciliation; no automatic resend |

If the actual executor cannot provide a fresh-session delivery trace, keep bridge
activation pending. Do not invent an ack, erase copy inventory, or count a synthetic
bridge as MiniMax/Codex acceptance. Track actual client evidence under #301 and
repository/scheduler evidence under #313, preserving their earlier partial results.
This installation slice does not change client settings, bind a production workflow
or modify a schedule.

## 3. External schedule handoff (a template, not a scheduler)

Prepare a **diff against the owner's selected existing entry**, using its supported
management API. If no entry was selected, leave this handoff undeployed; do not
create a job. Keep this deployment record private:

```text
entry_id / provider: <owner-selected exact identity>
before_definition: <private file + SHA-256 from supported read interface>
requested_delta: prompt only (or separately approved exact fields)
preserve: entry_id, name, schedule, timezone, project/cwd, executor/model,
          enabled state, session mode, notification/delivery settings
prompt_before: <exact old prompt>
prompt_after: <substituted text below + retained unrelated instructions>
after_readback: NOT RUN
real_trigger: NOT RUN
missed_run / catch_up: NOT VERIFIED
```

Prompt-only configuration diff template (review material, not a provider-specific
API payload; apply through the verified management interface only):

```diff
--- selected-entry.before
+++ selected-entry.after
@@
 entry_id: <same exact owner-selected ID>
 schedule: <unchanged>
 timezone: <unchanged>
 project: <unchanged>
 model: <unchanged>
 enabled: <unchanged>
 session_mode: <unchanged>
 notification_settings: <unchanged>
-prompt: <full existing prompt>
+prompt: <full existing prompt with the approved insertion below>
```

Prompt insertion template (substitute actual paths and retain the entry's existing
time/work budget and notification preference):

```text
Use the installed repository-review Skill at <VERSION>/package/integrations/
task-memory/memorynexus-repository-review/SKILL.md and the approved <BINDING>.
PYTHON=<absolute pinned real interpreter>; VERSION=<absolute immutable version>;
BINDING=<absolute private binding>; STATE_ROOT=<binding's existing state root>.
Run PYTHON -I -S -B VERSION/launch.py verify --binding BINDING, then status.
Stop on a binding, version, recovery or read error; never choose another ledger.

For each authorized phase, choose a NEW directory below STATE_ROOT:
  PYTHON -I -S -B VERSION/launch.py prepare --binding BINDING
    --workflow capture --client <actual executor label> --run-dir <NEW_PREPARE>
Read prepared.json and its evidence. When no work and no pending gap remain, do
not call a model just to rewrite an unchanged result. Otherwise create a bounded
submission using only prepared current units, then:
  PYTHON -I -S -B VERSION/launch.py preview --binding BINDING
    --prepared <PREPARED_JSON> --submission <SUBMISSION_JSON> --run-dir <NEW_PREVIEW>
Read preview.md and plan.json. Apply profile-authorized capture/review only within
that approved policy. Owner-mode plans remain pending until the owner approves
that exact canonical plan digest; do not self-approve or carry forward an old hash.
  PYTHON -I -S -B VERSION/launch.py apply --binding BINDING --plan <PLAN_JSON>
For an actually owner-approved plan append --approve <EXACT_APPROVED_DIGEST>.

Run review separately with --workflow review. Each Lens independently proposes,
defers or skips; remaining work is not success or global processed state. Publish
is a separate prepare/preview: present its entire diff before prose approval.
Do not implicitly execute pure neutralization; apply it only when this deployment
includes that fixed-text maintenance action under the approved profile policy.

Keep stdout/exit status and artifacts. Query status --binding BINDING --plan
<PLAN_JSON> after uncertain effects; preserve original request bytes/keys. Report
failures, partials, pending IDs and needs_review in the existing queryable session.
Do not promise a proactive channel notification or change delivery preferences.
Never bootstrap an initialized task, edit schedules, run backup/prune scripts,
repair a ledger, or modify project files beyond this approved workflow's targets.
```

After separately authorized deployment, use a fresh supported read (`cron_read
detail` on the verified MiniMax executor, or that provider's documented equivalent)
and compare full prompt bytes plus every preserved field. Readback by reusing the
update response is insufficient. No runtime SQLite editing, delete/recreate,
deleted-review-cron restoration, backup-cron editing, or guessed provider schema.

Record the first **real** scheduled run using provider entry ID, provider run ID,
source/trigger metadata, scheduled and actual start time/timezone, executor, exit
status, retained runner run IDs and artifacts. Manual reduced-environment execution
is only a manual test. The current runner has no scheduler proof adapter: a
`MEMORYNEXUS_TRIGGER_FILE` labeled scheduler preserves evidence but still yields
`unverified_trigger` and reports `trigger.kind=manual`. Preserve that gap; correlate
the independent provider evidence without relabeling the runner result as proven.

For a missed slot, retain provider run history, expected slot and observed machine
availability. Record “missed/unknown; no proven catch-up” unless that provider's
actual later run proves catch-up. Do not add a retry daemon or infer it from the
next scheduled run. Failure/pending query:

```sh
"$PYTHON" -I -S -B "$VERSION/launch.py" status --binding "$BINDING" --plan "$PLAN"
```

If no plan exists, omit `--plan`, then read that run's summary/artifacts/stdout and
the provider history. Launcher preflight failures emit `{schema_version:1,
status:"failed",code:...}`/exit2 before workflow execution; successful dispatch
preserves the existing runner report and exit0/2/3/4/5 semantics. Neither exit0
nor `verified=true` means source content is owner-confirmed truth.

## 4. Upgrade and isolated rollback checklist

Inventory **every actual writer/reader**, including wrappers with default values,
interactive sessions, cron/launchd and backups. Record entry → exact binary/hash,
config/task ID, ledger/format/incarnation and package/binding. Instruction files
alone are insufficient: jhwg's wrapper default continued writing v1 after a new
ledger had been migrated, creating a real fork. Stop writers using their supported
controls and prove the source snapshot is current before a cutover; elapsed quiet
time or chmod labels alone are not proof against all writers.

1. Back up the current ledger consistently with a format-compatible binary. Use
   `task --ledger LIVE backup` with `{"path":"NEW_SNAPSHOT"}`; export **the
   snapshot**. With that compatible binary restore into `NEW_ISOLATED_LEDGER`
   using `task --ledger NEW_ISOLATED_LEDGER restore` and `{"path":"SNAPSHOT"}`.
   Paths are absolute, private and previously nonexistent. Compute hashes after
   all export/read subprocesses exit. Compare full logical exports/receipts,
   not only counts or file hashes. For v2, expect a **new incarnation** and
   `recovery_required`; reconcile those documented differences, do not erase them.
2. Install a new version alongside the old one, then verify its manifest and a
   candidate private binding. A v1-only backup binary cannot read v2; capability
   help is not enough—run backup/export/isolated restore with the selected binary
   on the relevant format. Leave old tools/data intact, with no fallback or downgrade.
3. Rehearse the new version on the isolated recovery path. Old approvals cannot
   survive restore; reconcile and freshly approve a recovery binding. Rehearse
   selecting the previous package **only if its CLI supports the current format**,
   with an explicit confirmed rebind, then read state. Package rollback does not
   roll back the ledger, recreate a task, reactivate old plans or undo withdrawals.
4. Present the exact production binding request and caller diffs for approval.
   After cutover independently read every entry, task ID, ledger incarnation and
   workflow status; no old wrapper may continue writing a retired ledger.
5. List the backup entry separately. Until it has an approved compatible binary
   and ledger mapping plus a real run, mark recurring backup coverage **NOT RUN /
   GAP** and keep an explicit temporary manual backup. Do not claim that a successful
   manual backup updated the old cron. This issue does not collect maintenance
   scripts, edit them, or authorize a production schedule change.

Passing repository tests proves isolated installation/launcher behavior only.
Real MiniMax↔Codex handoff, provider trigger evidence and the full #313/#301 gates
remain separate. Use the acceptance template to make those limits visible.
