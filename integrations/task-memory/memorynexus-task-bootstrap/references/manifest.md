# Bootstrap manifest v1

`preview` takes an authored JSON draft with exactly these fields. Paths must be
canonical absolute paths without symlink components. Select 1–30 existing local
text source files, each at most 2 MiB; their contents stay in the repository.

```json
{
  "schema_version": 1,
  "task_root": "/absolute/task",
  "binding": {
    "config": "/absolute/task/.memorynexus.json",
    "ledger": "/absolute/owner-data/task.sqlite",
    "display_name": "My task",
    "automatic_capture": ["execution", "review"]
  },
  "sources": ["README.md", "docs/TODO.md"],
  "conflicts": [],
  "gaps": ["Actual client startup behavior has not been verified."],
  "entries": [
    {"id": "progress", "kind": "checkpoint", "text": "Current progress and next step.", "sources": ["docs/TODO.md"]},
    {"id": "decision", "kind": "execution", "text": "The current README describes the selected approach.", "sources": ["README.md"]},
    {"id": "constraint", "kind": "instruction", "text": "Proposed constraint requiring your explicit approval.", "sources": ["README.md"]},
    {"id": "lesson", "kind": "lesson", "text": "A proposed lesson, not an established fact.", "source_entries": ["decision"]}
  ]
}
```

- 1–12 entries, at most one checkpoint; each text is 1–1,000 Unicode characters.
  Total entry text plus display name must fit 12,000 characters.
- IDs are unique `[a-z][a-z0-9_-]{0,39}`; `init` is reserved. Base entries cite 1–5 selected paths;
  lessons cite 1–12 earlier **base** entry IDs. No candidate-to-candidate chains.
- Sources are relative paths without `..`, absolute paths or URI schemes.
- `conflicts` and `gaps` are bounded lists. Conflicts block application; resolve
  them explicitly in a new draft. A documented observation about uncertain
  tracker evidence can remain a gap, without asserting a resolved conclusion.
- The proposed policy allows automatic execution reports and candidate retention.
  It is part of the preview requiring approval. Do not infer confirmation from
  documents. Only exact approval later permits instruction confirmation.
- New-task-only: existing configs cannot be seeded, even when empty. The sole
  exception is resuming this same import's own initialization.

Preview writes `manifest.json` and `preview.md` into a **new**, owner-only state
directory outside the task. The manifest wraps the draft with schema 1, generated
`import_id`, fixed `created_at`, selected file hashes and, for a Git checkout,
HEAD plus bounded porcelain status (excluding only the generated root config).
The digest is SHA-256 of the exact UTF-8 manifest bytes, including its newline.
The preview exposes this entire binding/snapshot and every proposed record.

The helper reads no ledger and calls no CLI during preview. Missing ledger parent
directories may be proposed; create them owner-only after approval and before
apply. It never chooses a fallback path or creates parent directories implicitly.
Preview cannot certify that a proposed existing ledger is the right one; the
compiled CLI validates its format during authorized initialization.

Commands, from the complete Skill bundle:

```sh
python3 scripts/bootstrap.py preview --draft /absolute/draft.json --state-dir /absolute/new-seed
# STOP: show preview.md and manifest.json; obtain the human's exact-content approval.
python3 scripts/bootstrap.py apply --state-dir /absolute/new-seed \
  --approve EXACT_MANIFEST_SHA256 --config /absolute/task/.memorynexus.json \
  --ledger /absolute/owner-data/task.sqlite --binary /absolute/memorynexus-feedback-cli
```

`--approve` is an operator guard, not authentication. Never supply it merely
because an owner asked to install or implement the tool. Changing any draft,
source snapshot or binding requires a new preview and confirmation. A pending
import keeps its original envelopes; do not mutate it or regenerate keys.

## Apply result and retained state

The helper is a POSIX adapter (Python 3.9+), using owner-only files and an advisory
lock. Keep other task writers idle during this short serial import. Each request
uses `client: "task-bootstrap"`, `run_id: import_id`, the fixed `created_at`, and
key `bootstrap:<import_id>:<entry-id>` (`init` for initialization). Source hashes
become evidence references; lesson source IDs come from validated receipts.

`attempted` records accepted/replayed writes in this invocation. `receipts`
includes previous acknowledgements; an exact completed rerun has no new attempts.
`pending` lists unacknowledged entries; a pending status with an empty list means
cleanup/readback still needs attention. CLI failure diagnostics never include
request bodies. Requests stay in `pending-<id>.json` until validated acknowledgement;
content-free `receipt-<id>.json` files retain identity/revision/status. Neither is
ledger evidence until the CLI has acknowledged it. Keep state outside Git.

A proven OS launch failure marks init as not started and permits explicit retry
of the original envelope. Timeout or process failure has an uncertain outcome;
missing/replaced previously observed ledger state stops recovery. Do not create
a substitute ledger or edit/remove receipts to bypass that check. After diagnosing
a failure, rerun the same command with the same digest and binding. A different
verified binary path is allowed; it does not change the staged request.

The manifest and preview intentionally retain the original approved draft for
owner review; acknowledged pending payloads are deleted. Retraction does not
automatically erase these local artifacts. The owner controls their retention.
