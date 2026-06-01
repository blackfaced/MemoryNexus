# Subagent Issue Workflow

This document is the standard handoff template for giving a GitHub issue to a
subagent or parallel worktree.

## Default Prompt

```text
Please implement GitHub issue #XX in the MemoryNexus repository.

Before changing files:
1. Read AGENTS.md.
2. Read README.md and docs/TODO.md.
3. Read the issue body and all issue comments.
4. Read any ADRs referenced by the issue or AGENTS.md.

If the issue context is insufficient, stop and report the missing context. Do
not guess the product direction or broaden the scope.

Development rules:
- Do not work directly on main.
- Use the current branch/worktree assigned for this issue.
- Do not introduce a second backend.
- Do not introduce a new frontend stack unless the issue or an ADR explicitly
  requires it.
- Keep Rust-first behavior in the Rust + Axum service.
- For Phase 4 UI work, build on the Rust-served static Thought Review UI in
  web/thought_review.html unless instructed otherwise.

When complete:
1. Run the verification commands required by AGENTS.md.
2. Report changed files.
3. Map the implementation back to the issue acceptance criteria.
4. List validation commands and results.
5. Call out remaining risks or follow-up issues.
```

## Issue Context Checklist

An issue is ready for direct subagent execution when it has:

- Clear goal or user story.
- Concrete acceptance criteria.
- Relevant files or modules.
- Explicit non-goals.
- Required verification commands.
- References to ADRs or docs when architecture/product direction matters.

If any item is missing, first add an issue comment with the missing context or
split the issue into smaller tasks.

## When To Stop Before Coding

Stop and ask for clarification or add an issue-context comment if:

- The issue only says "build X" without acceptance criteria.
- It could require a new frontend stack or second backend.
- It changes ownership, permission, or persistence boundaries.
- It mixes multiple product domains, such as learning, piano, chess, drawing,
  and personal review in one task.
- It is unclear whether the work belongs in Phase 4 Thought Review UI or Phase 5
  Namespace / FeedbackLoop.

## Current Project Defaults

- `CognitiveSpace` is the ownership and permission boundary.
- `Namespace` is a domain partition inside a `CognitiveSpace`, not a permission
  model.
- Thought Review is the first reflective namespace product entry point.
- Phase 4 UI work should continue from `web/thought_review.html`.
- Phase 5 work should start with design and minimal model/API plans before
  schema or UI expansion.

## Example: UI Issue Prompt

```text
Please implement GitHub issue #25.

Use the standard subagent workflow in docs/subagent-issue-workflow.md.

Specific scope:
- Build on web/thought_review.html.
- Add active Cognitive Space selection.
- Apply active space_id to memory create, Lens list/create, Lens Run, review
  reports, memory list, and search calls.
- Do not treat Namespace as a Space replacement.

Verification:
- cargo fmt --check
- cargo test
- UI smoke: register/login, switch active space, save a thought in each space.
```

## Example: Design Issue Prompt

```text
Please work on GitHub issue #52.

Use the standard subagent workflow in docs/subagent-issue-workflow.md.

Specific scope:
- Produce a design/modeling pass for Namespace and FeedbackLoop.
- Start from ADR-014 and docs/cognitive-concepts.md.
- Do not implement a full learning product UI.
- Do not move permissions from CognitiveSpace to Namespace.

Expected output:
- Proposed minimal model.
- API/filter implications.
- Provenance relationship to Memory, Lens Run, Review Report, and Profile.
- Follow-up implementation issue breakdown.
```
