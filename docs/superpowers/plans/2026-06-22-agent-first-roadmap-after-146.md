# Agent-first Roadmap After Issue 146 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Synchronize the repository roadmap and GitHub backlog around the fastest text-first Dictation Agent loop after Issue #146.

**Architecture:** Keep the serialized Surface Gateway dispatcher chain, but move typed/pasted Dictation and generic text MCP delivery ahead of media-reference completion. Preserve the generic `Trace -> FeedbackLoop -> GrowthModel -> PracticePlan` Engine path, while moving event publication, seven-day trends, release, and deployment off the first Agent-smoke critical path.

**Tech Stack:** Markdown roadmap documents, GitHub Issues, GitHub CLI, Rust test command references.

---

### Task 1: Synchronize The Executable Roadmap

**Files:**
- Modify: `docs/TODO.md:146-345`
- Reference: `docs/superpowers/specs/2026-06-22-agent-first-roadmap-after-146-design.md`

- [ ] **Step 1: Update Milestone 3 status and verification gate**

Change the status to say Reflection is implemented in PR #176 but remains pending review/merge. Add PostgreSQL integration verification before #147 starts.

- [ ] **Step 2: Replace the Milestone 5 dependency graph**

Encode these delivery edges explicitly:

```text
#146 review/merge -> #147 -> #148
#148 -> #155 typed/pasted path -> #156 typed/pasted path -> #157
#148 -> #162 generic text Surface tools
#148 -> #175 -> media extensions in #155, #156, and #162
#157 -> #152 -> #153 -> #158
#155 through #158 + text-capable #162 -> initial #160 Agent smoke
#159 -> extended seven-day Agent acceptance
```

- [ ] **Step 3: Add the CI and distribution waves**

State that #177 required PostgreSQL Surface integration CI is started in parallel with #147/#148. Keep #128, #129, and #130 after initial #160 acceptance, and identify #130 as P1 for the Mac mini deployment path.

- [ ] **Step 4: Verify roadmap consistency**

Run:

```bash
rg -n "#146|#147|#148|#152|#153|#155|#156|#157|#158|#159|#160|#162|#175|CI" docs/TODO.md
git diff --check
```

Expected: every listed issue has one unambiguous place in the dependency graph and `git diff --check` produces no output.

### Task 2: Synchronize Canonical Issue Definitions

**Files:**
- Modify: `docs/issues.md:474-534`
- Modify: `docs/issues.md:694-1210`
- Modify: `docs/issues.md:1250-1310`

- [ ] **Step 1: Clarify #146 completion state and #147/#148 sequencing**

Keep the dispatcher ownership chain serialized and add PostgreSQL integration verification to the completion expectations for shared-dispatcher issues.

- [ ] **Step 2: Split typed/pasted delivery from media extension in #155 and #156**

Use this contract:

```text
Typed or pasted confirmed text can ship after the preceding Dictation issue.
agent_ocr, agent_transcribed, mixed, input_confirmation, and evidence_refs
remain disabled until Foundation F1 (#175) lands.
```

Add a negative acceptance test proving a caller cannot label media-derived content as typed/pasted to bypass confirmation.

- [ ] **Step 3: Thread the generic Engine loop through #152, #153, and #158**

Make #152 follow #157 for the Dictation fixture, make #153 consume #152 output, and make #158 shape #153's PracticePlan instead of introducing a second plan model.

- [ ] **Step 4: Split initial and extended Agent acceptance**

Make initial #160 depend on #155, #156, #157, #158, and the text-capable portion of #162. Keep #159 as an extended acceptance dependency for the seven-day trend, not a blocker for same-day smoke.

- [ ] **Step 5: Stage #162 delivery**

Specify generic text Surface MCP tools after #148 and media mapping after #175. Preserve the rule that #162 never edits `src/api/surfaces.rs` and never adds Dictation-specific Engine actions.

- [ ] **Step 6: Verify canonical definitions**

Run:

```bash
rg -n "typed|pasted|media-derived|initial Agent|extended.*seven-day|#152|#153|#175" docs/issues.md
git diff --check
```

Expected: text-first delivery and media gating are both explicit, with no contradictory dependency statements.

### Task 3: Update Existing GitHub Issues

**Files:**
- No repository files
- Update GitHub issue bodies from canonical sections for: `#146`, `#147`, `#148`, `#150`, `#152`, `#153`, `#155`, `#156`, `#157`, `#158`, `#159`, `#160`, `#162`, `#175`
- Update scheduling and priority metadata for: `#130`
- Keep `#128` and `#129` as post-#160 distribution issues without body sync in this pass.

- [ ] **Step 1: Update issue bodies from `docs/issues.md`**

Use `gh issue edit <number> --body-file <temporary-body-file>` with bodies copied from the synchronized canonical issue sections. Do not hand-maintain a second wording variant.

- [ ] **Step 2: Update priorities**

Apply `priority:p1` to #160, #162, and #130. Remove `priority:p2` from those issues. Leave #150 at P1 but state that it is parallel rather than an initial-smoke dependency.

- [ ] **Step 3: Add coordinator comments for cross-milestone dependencies**

Comment on #152, #153, #158, and #160 with the new critical-path relationship so workers do not rely only on milestone ordering.

- [ ] **Step 4: Verify live issue state**

Run:

```bash
gh issue view 160 --json number,state,labels,body,url
gh issue view 162 --json number,state,labels,body,url
gh issue view 130 --json number,state,labels,body,url
```

Expected: all three issues carry `priority:p1`, and #160/#162 show staged initial-versus-media acceptance.

### Task 4: Verify The Required Integration CI Issue

**Files:**
- Modify: `docs/TODO.md`
- Modify: `docs/issues.md`
- Existing GitHub issue: #177

- [ ] **Step 1: Confirm #177 has explicit scope**

The issue title is:

```text
CI: Require PostgreSQL Surface integration tests on pull requests
```

Its scope must include a pinned PostgreSQL service, deterministic database isolation, dynamic enumeration of all `tests/surface_*_postgres_integration.rs` files, exact execution-set equality, and branch-protection enrollment only after the Coordinator observes five consecutive eligible successful runs across at least two PRs and one main push with no flake reruns.

- [ ] **Step 2: Keep external systems out of the required job**

State that OpenRouter and other credentialed provider tests remain manual or scheduled. Qdrant tests use a pinned image and run when vector behavior changes or in scheduled full acceptance.

- [ ] **Step 3: Define acceptance criteria**

Require a deliberately failing integration fixture to block a PR, a passing rerun to unblock it, no use of `qdrant:latest`, documentation of the local equivalent command, and failure if enumerated ignored Surface integration tests are not executed.

- [ ] **Step 4: Verify issue state**

Run:

```bash
gh issue view 177 --json number,title,state,labels,milestone,body,url
```

Expected: #177 is open, P0, testable, in the Surface Gateway milestone, and names the exact required job boundary.

### Task 5: Review, Commit, Push, And Open The Planning PR

**Files:**
- Modify: `docs/TODO.md`
- Modify: `docs/issues.md`
- Existing: `docs/superpowers/specs/2026-06-22-agent-first-roadmap-after-146-design.md`
- Existing: `docs/superpowers/plans/2026-06-22-agent-first-roadmap-after-146.md`

- [ ] **Step 1: Review the complete diff**

Run:

```bash
git diff --check
git diff --stat main...HEAD
git diff main...HEAD -- docs/TODO.md docs/issues.md docs/superpowers
```

Expected: only planning documentation changes appear; no Rust, migrations, workflows, or generated files are modified.

- [ ] **Step 2: Commit the synchronized roadmap**

Run:

```bash
git add docs/TODO.md docs/issues.md docs/superpowers
git commit -m "docs: prioritize text-first dictation agent loop"
```

- [ ] **Step 3: Push and open a pull request**

Run:

```bash
git push -u origin codex/agent-first-roadmap-after-146
gh pr create --base main --head codex/agent-first-roadmap-after-146 --title "Prioritize text-first Dictation Agent loop" --body-file <prepared-pr-body>
```

The PR body must summarize the dependency changes, list the new CI issue, state that no Rust tests were run because the diff is documentation-only, and link PR #176 as the immediate predecessor.

- [ ] **Step 4: Watch required CI**

Run:

```bash
gh pr checks <new-pr-number> --watch
```

Expected: Format, Clippy, Build, and Test pass before merge review.
