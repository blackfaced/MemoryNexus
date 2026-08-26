# MemoryNexus

MemoryNexus is a local-first personal experiment feedback kernel for a Mac mini:
MiniMax Skill + local CLI + one authoritative SQLite ledger.

It helps one owner keep a confirmed record of what they observed, which advice
they chose, what reversible adjustment they tried, and what happened later. It
is not a generic AI memory store, a medical-analysis product, a cloud service,
or an Agent runtime.

The current default direction is [ADR-027](decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)
and its parent [spec #273](https://github.com/blackfaced/MemoryNexus/issues/273).
The implementation gate is [#274](https://github.com/blackfaced/MemoryNexus/issues/274): before
the product path is implemented, a real MiniMax cross-session local-command
tracer must prove shared local state works as assumed.

## The feedback kernel

```text
confirmed Observation
  -> sourced Recommendation
  -> one active, reversible Experiment
  -> confirmed Outcome
  -> evidence-backed review
```

- `Observation`: a confirmed, bounded fact or subjective report.
- `Recommendation`: a bounded suggestion from the owner, an external advisor,
  or explicitly marked Agent wording.
- `Experiment`: the one selected reversible action, its period, and expected
  observable signal.
- `Outcome`: whether it was performed, skipped, or not evaluable, plus the
  confirmed result.

SQLite in WAL mode is the only first-version authoritative store. The compiled
CLI is the primary behavior seam and returns stable JSON for `observe`,
`add-recommendation`, `start-experiment`, `record-outcome`, `review`, and
`due`. It does not expose generic CRUD or internal database objects.

## MiniMax and health boundary

The owner can use an existing MiniMax Agent through WeChat for convenient input.
MiniMax Skill turns natural-language intent into explicit local CLI use cases
and asks for confirmation before every authoritative write. MiniMax native
scheduled tasks will invoke `due` in an independent session and initially show
the result in the MiniMax App; the SQLite ledger, not chat history, provides
continuity. Proactive WeChat delivery is not a first-version requirement.

MemoryNexus does not diagnose, interpret medical documents, prescribe treatment,
or make clinical claims. Ant Afu and other tools are external Recommendation
sources. Their raw conversations, reports, diagnoses, and prescriptions remain
ephemeral by default; only an owner-confirmed bounded summary is retained.

## Transition

The former Rust + Axum, PostgreSQL/Qdrant, Surface Gateway, MCP, Sleep/Dreaming,
Dictation, Thought Review, and source Adapter runtime is frozen during the
replacement. There is no data migration, dual write, compatibility promise, or
new generic backend abstraction. Git history remains the record of the retired
implementation.

The old runtime is deleted only after the fixed fourteen-calendar-day owner
dogfood gate passes. Until then, this repository contains the frozen legacy
implementation while the new SQLite/CLI path is added and validated. A future
third-party memory backend may only be a rebuildable, non-authoritative recall
projection from the SQLite ledger.

## Roadmap and documentation

- [Current architecture decision](decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)
- [Current roadmap and tracker reconciliation](docs/TODO.md)
- [Current architecture](docs/architecture/README.md)
- [Vision](docs/vision.md)
- [All architecture decisions](decisions/README.md)
- [GitHub Issues](https://github.com/blackfaced/MemoryNexus/issues)

The older documents under `docs/architecture/` describe the frozen legacy
runtime. They remain available for historical context and must not be treated
as the supported product path.

## Verification

This change is documentation-only. Rust tests are intentionally not run because
it changes neither Rust sources nor runtime behavior.

## License

MIT. See [LICENSE](LICENSE).
