# MemoryNexus Docs

## Start Here

- [ADR-028: MiniMax / Codex shared task memory](../decisions/ADR-028-shared-task-memory.md)
- [Task memory implementation contract](task-memory-contract.md)
- [Task memory onboarding on another machine](task-memory-onboarding.md)
- [Shared client protocol](../integrations/task-memory/PROTOCOL.md)
- [Real-client acceptance TODO](acceptance/task-memory-cross-client.md)
- [Existing feedback ledger recovery](feedback-ledger-recovery.md)
- [Vision](vision.md)
- [Architecture](architecture/README.md)
- [TODO](TODO.md)
- [ADR-027: SQLite CLI MiniMax Personal Experiment Feedback Kernel](../decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)
- [Architecture Decisions](../decisions/README.md)

## Frozen legacy documentation

The legacy documents listed below describe the frozen pre-ADR-027 runtime. They are
preserved for historical context during expand–contract, but are not supported
setup, architecture, or roadmap instructions.

- [Legacy Engine](architecture/memorynexus-engine.md)
- [Legacy Surfaces and Adapters](architecture/surfaces-and-adapters.md)
- [Legacy Surface Gateway](architecture/surface-gateway.md)
- [Legacy Reference Adapter Runtime](reference-adapter-runtime.md)
- [Legacy Sleep-driven Feedback Loop](architecture/sleep-driven-feedback-loop.md)
- [Legacy API / CLI / MCP and deployment material](api.md)

## Notes

#274 validated MiniMax local-command and shared-state assumptions, with an
owner-initiated pull constraint for scheduled output. The feedback CLI now has
source implementation and recovery commands; clean-install acceptance remains
separate. ADR-028 task commands and simulated CLI preflight exist in source;
their own real-client gate #301 remains NOT RUN. Do not use legacy docs as setup guidance.
