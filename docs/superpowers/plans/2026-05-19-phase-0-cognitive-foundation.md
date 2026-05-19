# Phase 0 Cognitive Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate the cognitive theory notes into repository documentation that can guide the Rust refactor.

**Architecture:** Phase 0 is documentation-only. It adds a manifesto, an architecture guide, and navigation links while preserving the Rust-first implementation path for later phases.

**Tech Stack:** Markdown, existing `docs/` and `decisions/` structure.

---

### Task 1: Add Cognitive Manifesto

**Files:**
- Create: `docs/cognitive-manifesto.md`

- [ ] **Step 1: Create the manifesto document**

Write the project direction, theory base, Lens model, Cognitive Space ownership model, and MVP thesis into `docs/cognitive-manifesto.md`.

- [ ] **Step 2: Verify no placeholders**

Run: `rg -n "TBD|FIXME|TODO" docs/cognitive-manifesto.md`

Expected: no matches.

### Task 2: Add Cognitive Architecture

**Files:**
- Create: `docs/cognitive-architecture.md`

- [ ] **Step 1: Create the architecture document**

Write the Functional Core + Imperative Shell architecture, domain module target, API direction, persistence direction, and Phase 1 Rust target into `docs/cognitive-architecture.md`.

- [ ] **Step 2: Verify no placeholders**

Run: `rg -n "TBD|FIXME|TODO" docs/cognitive-architecture.md`

Expected: no matches.

### Task 3: Update Documentation Navigation

**Files:**
- Modify: `docs/README.md`
- Modify: `docs/cognitive-lens-roadmap.md`

- [ ] **Step 1: Link the new docs**

Add `docs/cognitive-manifesto.md` and `docs/cognitive-architecture.md` to the docs index.

- [ ] **Step 2: Add Phase 0 to roadmap**

Add a Phase 0 foundation section to `docs/cognitive-lens-roadmap.md` and point it to the manifesto, concepts, and architecture docs.

### Task 4: Verify Doc-Only Diff

**Files:**
- Inspect repository diff.

- [ ] **Step 1: Check changed files**

Run: `git status --short`

Expected: only Markdown files are modified or added.

- [ ] **Step 2: Check formatting**

Run: `git diff --check`

Expected: no whitespace errors.
