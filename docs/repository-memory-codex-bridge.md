# Pinned read-only Codex bridge

Implementation ticket: #337, following the local real-client probe in #336.
[Acceptance evidence](acceptance/codex-bridge.md) separates deterministic fixtures
from the generated bridge's real local Codex probe.
This bridge supports the observed `codex-cli 0.162.0` JSON event protocol. It starts
one fresh, ephemeral, read-only Codex session from current material supplied by the
frozen adapter. It is not a general client launcher or a writable coding session.
A different version requires protocol validation and an implementation update;
changing the expected version string does not enable it.

## Install alongside a verified version

Install a fixed Git revision containing #337 through the [official installer](repository-memory-installation.md).
The manifest includes `install_codex_bridge.py` and `codex_bridge.py` at VERSION
root, outside the package. The generator verifies the full installation before
importing its template. Python is the interpreter pinned by that installation.

Create an owner-only spec outside the task and installation. Exact fields:

```json
{
  "schema_version": 1,
  "task_id": "00000000-0000-4000-8000-000000000001",
  "task_root": "/ABS/OWNER/TASK",
  "home": "/ABS/OWNER",
  "codex_home": "/ABS/OWNER/.codex",
  "client": {"path": "/ABS/REVIEWED/CODEX_ENTRY", "sha256": "64-lowercase-hex"},
  "client_version": "codex-cli 0.162.0",
  "files": [{"path": "/ABS/OWNER/.codex/config.toml", "sha256": "64-lowercase-hex"}],
  "timeout_seconds": 180
}
```

Replace all placeholders with reviewed values. `files` must include the selected
Codex home's `config.toml`, and may contain up to 64 distinct regular-file pins.
Each pinned file is bounded to 512 MiB and hashed in chunks.
Explicitly inventory any wrapper registry/payload, actual native executable,
project/layered configuration, rules, and configuration references involved in
the selected entry. Include those files before approving the spec; the generator
does not discover a wrapper's dependency closure or judge completeness. Paths
must be absolute, owned by the caller/root and not writable by
other users (sticky temporary ancestors are allowed only for isolated tests).
By default symlinks are rejected. For a reviewed symlinked client/dependency, add
`resolved: ABS_REAL_PATH` to that pin: both the resolved selection and target bytes
are checked before/after execution. This also detects a link retargeted to a
different file with identical bytes. Python/home/task paths remain symlink-free. Do not bypass an existing startup policy to make a test pass.

`home` and `codex_home` select existing authentication/configuration locations;
no authentication files are copied or changed by the generator. Runtime retains
only the environment keys OPENAI_API_KEY, OPENAI_ORG_ID, OPENAI_PROJECT_ID,
HTTPS_PROXY, HTTP_PROXY, ALL_PROXY, NO_PROXY and MEMORYNEXUS_TASK_LEDGER, if present.
It fixes HOME/CODEX_HOME from the spec and PATH/LANG to `/usr/bin:/bin` / `C.UTF-8`.
Inherited Python/Node paths, model overrides and arbitrary config environment
variables are not forwarded. Credential refresh and proxy routing are outside
file-pin guarantees. The reviewed client or enabled tools may have their own
network/authentication behavior.

```sh
"$PYTHON" -I -S -B "$VERSION/install_codex_bridge.py" \
  --spec "$PRIVATE_SPEC" --destination "$NEW_BRIDGE_DIRECTORY"
```

The destination must be a new child of an existing owner-only directory, outside
VERSION. The generator checks declared hashes and the storage-free client
`--version`, then creates a 0700 directory containing one 0700 `codex-bridge`.
Its spec and Python pin are embedded, so no mutable sidecar can change its target.
There is no overwrite, active-version switch, bind, model session or scheduler
operation during generation. An interrupted directory is retained; select a new
one after review. Stdout returns `bridge:{executable,sha256}`, source revision,
client version and `coverage:declared_pins_only`; failure is exit2 with a fixed
non-sensitive code. The generated executable uses the pinned Python with `-I`;
the Python standard library/system dependencies remain an external trust boundary.

Put the returned bridge object into the **separate reader binding**, retaining
its strict schema, task identity and package pins. A changed reader binding
invalidates old read evidence. Obtain a fresh read; do not reuse a prior read
under a new bridge hash. Keep production activation and any workflow rebind under
the existing reviewed installation procedure.

## Execute through the current adapter

Use `VERSION/launch.py current read/check/launch` as documented in the installation
guide. Do not invoke the bridge directly with cached material: freshness and the
permanent delivery claim belong to the adapter. The bridge accepts no argv and
requires the bound cwd/task ID, canonical current material with no gaps, and a
matching Python/client/config/dependency snapshot.

Before client execution it verifies pins, probes the exact supported version,
and rechecks pins. Its fixed invocation is:

```text
PINNED_CODEX exec --ephemeral --json --sandbox read-only
  --skip-git-repo-check -C PINNED_TASK_ROOT FIXED_INSTRUCTION
```

The fixed instruction says to use current stdin material while preserving trust,
perform only justified read-only actions, and report observations without writes.
It contains no task-specific answer. The original material bytes go unchanged to
process stdin; Codex may wrap them when constructing model input. No shell,
resume/fork, caller-provided flags, automatic model fallback or retry is added.
Existing rules/hooks are not disabled. `--sandbox read-only` constrains shell
execution; it is not proof that configured remote tools have no side effects.
Review enabled client capabilities as part of activation.

Client stdout is bounded to 1 MiB, stderr to 64 KiB, each version probe to 10 s,
and the model call to the spec's 1–240 s. Choose an adapter timeout with room for
preflight (normally 300 s); a smaller outer timeout can interrupt first. The client starts in the adapter's process group, so cleanup covers the bridge,
client and descendants that remain in that group. Client-created detached processes
are outside this guarantee. There are no internal attempts/retries.

Ack requires exit0, exactly one fresh UUID thread, one ordered started/completed
turn, valid supported JSON events and matching post-run pins. Error/failed/aborted
turns, malformed/unknown events, missing completion, drift and timeout fail closed.
Only the frozen read_id/material hash/session_ref ack is emitted; no model output,
command output, raw prompt, stderr, or credential values are persisted by this
bridge. Runtime failure exits2 with fixed stderr; after a claimed attempt the
adapter records delivery_unknown and **never automatically retries**, even if
preflight may have failed before a session began.

## Evidence limits and acceptance

An ack proves the trusted client's completed-turn report for the supplied process
input. It does not prove every instruction was followed, task success, independent
server-side input, or semantic freshness of all native memory. Actual actions
need separate client evidence; never substitute self-reported hashes for those.
Declared pins are finite before/after observations, not filesystem locks or an
exhaustive dependency graph; change-and-restore or post-check changes remain limits.

AGENTS/skills/native memory/cache input coverage remains UNKNOWN unless separately
observed. Do not remove external-copy entries to force a successful read: v1 still
blocks every declared external copy. A deployable executable does not by itself
establish safe activation for a particular private project. #301/#313 and parent
#317 remain separate gates; #283 is unchanged.

Automated `test_workflow_codex_bridge.py` uses a **synthetic Codex event emitter**
for deterministic transport failures and a real MemoryNexus CLI/installed adapter
for claims and exact byte delivery. It is not real-model acceptance. Real local
Codex probes use only fresh private synthetic tasks/markers and are recorded
separately, with installed source, binary/bridge/client/config hashes, session,
input/receipt digests and actual action limits.
