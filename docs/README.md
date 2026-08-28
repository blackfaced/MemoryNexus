# MemoryNexus Docs

## Start Here

- [Vision](vision.md)
- [Architecture](architecture/README.md)
- [TODO](TODO.md)
- [ADR-027: SQLite CLI MiniMax Personal Experiment Feedback Kernel](../decisions/ADR-027-sqlite-cli-minimax-feedback-kernel.md)
- [Architecture Decisions](../decisions/README.md)

## Frozen legacy documentation

The remaining documents describe the frozen pre-ADR-027 runtime. They are
preserved for historical context during expand–contract, but are not supported
setup, architecture, or roadmap instructions.

- [Legacy Engine](architecture/memorynexus-engine.md)
- [Legacy Surfaces and Adapters](architecture/surfaces-and-adapters.md)
- [Legacy Surface Gateway](architecture/surface-gateway.md)
- [Legacy Reference Adapter Runtime](reference-adapter-runtime.md)
- [Legacy Sleep-driven Feedback Loop](architecture/sleep-driven-feedback-loop.md)
- [Legacy API / CLI / MCP and deployment material](api.md)

## Notes

#274 has validated MiniMax local-command and shared-state assumptions, with an
owner-initiated pull constraint for scheduled output. No current installation
commands exist until the follow-on implementation tickets are complete. Do not
use legacy docs as setup guidance.
