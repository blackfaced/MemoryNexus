# Domain Docs

How engineering skills should consume MemoryNexus domain documentation.

## Layout

This is a single-context repository.

- Read root `AGENTS.md` first; it is the active project and agent operating guide.
- Read root `CONTEXT.md` if it exists; otherwise proceed silently.
- Read relevant architecture decisions from `decisions/`. This repository uses `decisions/` for ADRs, not `docs/adr/`.
- Read `README.md`, `docs/TODO.md`, and focused docs under `docs/` for the area being changed.

## Architecture decisions

Long-term decisions belong in `decisions/`, using the `ADR-00X-short-title.md` naming pattern. New ADRs must also update `decisions/README.md`.

Relevant starting ADRs include Rust-first backend (ADR-009), Thought Review MVP (ADR-013), namespace feedback loops (ADR-014), Trace runtime (ADR-016), Sleep/Dreaming (ADR-017), product positioning (ADR-018), Surface/Adapter/Engine layering (ADR-019), Dictation Coach (ADR-020), and media evidence boundaries (ADR-021).

## Vocabulary and conflicts

Use established terms such as `CognitiveSpace`, `Namespace`, `Trace`, `FeedbackLoop`, `GrowthModel`, `PracticePlan`, Surface, Adapter, Engine, Thought Review, and Dictation Coach when applicable.

Do not introduce a competing agent-memory-retrieval system, generic memory cloud, second backend, or full local inference runtime unless an ADR explicitly reopens that direction.

If work conflicts with an ADR or `AGENTS.md` boundary, flag the conflict explicitly rather than silently overriding it.
