# MemoryNexus Vision

MemoryNexus is a local-first personal experiment feedback kernel: a MiniMax
Skill invokes a local CLI, and one SQLite ledger preserves confirmed history
across sessions.

Its question is deliberately narrow:

```text
What did I choose to try, did I actually try it, what happened, and is the
adjustment worth keeping?
```

This replaces the prior default vision of a namespace-based long-term feedback
Engine. [ADR-027](../decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)
is authoritative for the new product direction; prior ADRs and architecture
documents remain historical context during the expand–contract transition.

## Product boundary

The authoritative core contains only:

```text
Observation -> Recommendation -> Experiment -> Outcome
```

An Observation is a confirmed bounded report. A Recommendation is a sourced
bounded suggestion. An Experiment is one selected reversible action. An Outcome
records execution and the confirmed result. Reviews reconstruct only confirmed
evidence and explicitly show gaps.

MiniMax handles natural-language understanding. The owner can conveniently use
WeChat for input and actively query an existing MiniMax conversation for a
native scheduled task result. SQLite, rather than Agent chat memory, is the
continuity mechanism. Proactive MiniMax App or WeChat reminder delivery is not
available in the verified first path.

## Non-goals

- No health diagnosis, treatment, medical-document interpretation, or clinical
  claim. Ant Afu is an external advice source, not a system to replicate.
- No PostgreSQL, Qdrant, embeddings, vector search, Axum service, MCP server,
  Surface Gateway, daemon, scheduler, or channel framework in the first path.
- No historical-data migration, dual write, or compatibility layer.
- No automatic retention of raw external conversations or sensitive documents.
- No preemptive memory-backend abstraction. A future backend can only be a
  rebuildable non-authoritative recall projection from SQLite.

## Evidence before expansion

[#274](https://github.com/blackfaced/MemoryNexus/issues/274) has proved that
MiniMax can execute an owner-approved local command from a normal WeChat session
and that an independent native scheduled session can read the same shared state.
The observed cron output lacks channel context, so the owner must actively
query the existing conversation; MiniMax App and WeChat proactive delivery are
not first-version capabilities.

After implementation and clean installation, a fixed fourteen-day owner gate
decides whether the legacy runtime can be deleted. It requires ten valid
Observations, one Experiment, five execution updates, one evidence-backed
review, cross-session state recovery, at most one manual system intervention,
and an owner day-fifteen continuation decision. See ADR-027 and
[roadmap](TODO.md) for the exact ordering.
