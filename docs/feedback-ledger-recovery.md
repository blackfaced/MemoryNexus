# Feedback ledger export and recovery

The authoritative ledger is one local SQLite file. The public recovery commands
are use-case commands on `memorynexus-feedback-cli`; they do not require direct
table access.

## Export

```text
memorynexus-feedback-cli --ledger <ledger.sqlite> export
```

The command emits one JSON document with:

```json
{
  "format": "memorynexus-feedback-ledger",
  "version": 1,
  "observations": [],
  "observation_retractions": [],
  "recommendations": [],
  "recommendation_observations": [],
  "experiments": [],
  "outcomes": []
}
```

It contains confirmed authoritative records and their links only. Raw external
conversations, unsupported documents, idempotency keys, and internal request
payloads are not exported.

## Backup and restore

Create a SQLite-consistent backup while WAL mode is enabled:

```text
memorynexus-feedback-cli --ledger <ledger.sqlite> backup
{"path":"/safe/location/ledger-backup.sqlite"}
```

Restore only to a new, non-existent ledger path. This avoids silently replacing
an existing authoritative history:

```text
memorynexus-feedback-cli --ledger <new-ledger.sqlite> restore
{"path":"/safe/location/ledger-backup.sqlite"}
```

After restore, run `review` and `observation-history` against the new path to
perform a public read check.
