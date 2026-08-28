# Current Architecture

The supported target architecture is a local personal experiment feedback kernel
on a Mac mini:

```text
MiniMax Skill / WeChat input / owner-initiated conversation query
                    |
                    v
          compiled local CLI (stable JSON)
                    |
                    v
     SQLite WAL single-file authoritative ledger
                    |
                    v
 Observation -> Recommendation -> Experiment -> Outcome
```

[ADR-027](../../decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md) is the
source of truth. [#274](https://github.com/blackfaced/MemoryNexus/issues/274)
has demonstrated that normal chat and an independent native scheduled session
share local state. Cron output has no channel context, so the owner retrieves a
`due` result by querying the existing MiniMax conversation; the system does
not claim proactive MiniMax App or WeChat delivery and does not rely on chat
history for state.

## Main seam

The target CLI will be the primary interface for product behavior and
verification after #274 and #276–#281. It will accept explicit use cases and
stable structured input/output, not generic Surface dispatch or private table
operations:

```text
observe | retract | add-recommendation | start-experiment | record-outcome | review | due
```

SQLite in WAL mode is the accepted authoritative first-version store. It must
support deterministic migrations, consistent backup, JSON export, and restore.
The first version has one fixed owner and at most one active Experiment. The
current runnable code remains the frozen legacy runtime; no target CLI command
is claimed to be available before the implementation tickets complete.

## Responsibilities

| Layer | Owns | Does not own |
| --- | --- | --- |
| MiniMax | natural-language intent, clarifying questions, unconfirmed drafts, native task wake-up and owner-initiated result query | authoritative history, medical analysis, or reliable proactive delivery |
| CLI + SQLite | confirmation, source, time, selected action, execution, result, review and due state | chat-session state, scheduler, channel delivery, generic recall |
| external advisor | analysis and a candidate suggestion | authoritative write, final action selection, factual result |

Every authoritative write requires an owner-confirmed bounded summary. Raw
external conversations and medical documents are ephemeral by default.

## Explicit exclusions

The first path has no PostgreSQL, Qdrant, embeddings, vector search, Axum REST
service, MCP server, Surface Gateway, `CognitiveSpace`, Namespace, Trace,
Sleep/Dreaming, Dictation, source Adapter, daemon, retry worker, channel
framework, multi-user permission model, or historical data migration.

A later third-party memory system may only hold a rebuildable recall projection;
it cannot own confirmations, provenance, Experiments, or Outcomes. Do not add an
abstraction until two real implementations require it.

## Expand–contract status

The repository currently still contains the legacy Rust runtime while the new
path is verified. It is frozen, not supported as the current product. It may be
deleted only after the fixed fourteen-day gate passes; see [TODO](../TODO.md).

The remaining files in this directory describe that frozen legacy architecture.
They are historical reference, not supported architecture or an active roadmap.
