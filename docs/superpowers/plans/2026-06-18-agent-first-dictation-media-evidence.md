# Agent-First Dictation And Media Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align MemoryNexus documentation around an Agent-first Dictation Coach loop where Agents perform OCR/ASR, MemoryNexus processes confirmed text, and original media remains traceable through provider-neutral evidence references.

**Architecture:** Add ADR-021 as the durable decision and a detailed media evidence contract as the single field-level source of truth. Existing Dictation, Agent, Surface, Trace, architecture, roadmap, and contributor documents should link to those sources and state only their local consequences. ADR-002 remains valid for optional MemoryNexus-managed object storage, while ADR-021 governs external references and resolver behavior.

**Tech Stack:** Markdown, ADRs, Rust/Axum architecture documentation, GitHub Issues, `rg`, `git diff --check`, `gh` CLI.

---

## File Map

**Create:**

- `decisions/ADR-021-external-media-evidence-references.md`: durable architecture decision and relationship to ADR-002, ADR-019, and ADR-020.
- `docs/media-evidence-contract.md`: canonical `EvidenceRef`, `EvidenceRefInput`, resolver, lifecycle, security, and failure semantics.

**Modify:**

- `decisions/ADR-002-storage-abstraction.md`: constrain the older storage trait to optional managed storage and point external evidence to ADR-021.
- `decisions/README.md`: list ADR-021.
- `docs/README.md`: link the media evidence contract.
- `docs/dictation-coach-mvp.md`: allow Agent-side OCR/ASR, add confirmed-text and optional evidence-reference provenance.
- `docs/architecture/surfaces-and-adapters.md`: define Agent preprocessing and resolver as Adapter/integration responsibilities.
- `docs/trace-contract.md`: link Trace to media evidence without storing media bytes.
- `docs/agent-integration.md`: give Claw/Hermes the OCR/ASR confirmation and safe-reference policy.
- `docs/architecture/README.md`: show external providers and optional managed object storage rather than a fixed S3 path.
- `README.md`: add one concise architecture boundary sentence and a contract link.
- `AGENTS.md`: add the durable media boundary and remove the duplicate Dictation Coach copy rule.
- `docs/TODO.md`: prioritize MCP/chat Agent testing before a dedicated App and add the evidence-reference foundation.
- `docs/issues.md`: align issue mirrors 5.2, 5.3, 5.7, and 6.2 with the approved boundary.

**External coordination after merge:**

- GitHub issue `#155`: Capture Dictation Word List.
- GitHub issue `#156`: Submit Dictation Attempt.
- GitHub issue `#160`: Minimal Dictation Adapter.
- GitHub issue `#162`: Chat / Agent Adapter Surface Flow.
- One new issue for the `EvidenceRefInput` contract and validation foundation.

### Task 1: Add The Durable Decision And Canonical Contract

**Files:**
- Create: `decisions/ADR-021-external-media-evidence-references.md`
- Create: `docs/media-evidence-contract.md`
- Modify: `decisions/ADR-002-storage-abstraction.md`
- Modify: `decisions/README.md`
- Modify: `docs/README.md`

- [ ] **Step 1: Add ADR-021**

Write the ADR with these exact decisions:

```markdown
# ADR-021: External Media Evidence References

## 状态
✅ 已接受

## 决策

- Agent / App 负责 OCR、ASR、媒体获取和用户确认。
- MemoryNexus 的反馈主链只依赖确认后的文字。
- 原始媒体通过 provider-neutral `EvidenceRef` 关联到 Trace / Surface provenance。
- `EvidenceResolver` 是可选 integration abstraction；第一版只登记引用，不要求解析或读取媒体。
- S3、OSS、WebDAV、移动硬盘、本地文件和未来的 MemoryNexus managed storage 都是 provider，不是 Engine 核心。
- `CognitiveSpace` 仍是 ownership / permission boundary；Namespace 仍只是领域分区。
- 未来独立 Dictation Coach 仓库只通过 Surface Gateway / MCP 使用 Engine，不直接访问内部表或对象。
```

Include background, failure behavior, security consequences, non-goals, and
links to ADR-002, ADR-019, and ADR-020. State that inaccessible media must not
invalidate confirmed text or completed feedback.

- [ ] **Step 2: Add the canonical contract**

Define these conceptual shapes in `docs/media-evidence-contract.md`:

```text
EvidenceRef {
  id
  space_id
  provider
  locator
  media_type
  content_hash?
  original_name?
  captured_at?
  transcript?
  transcript_source?
  metadata
}

EvidenceRefInput {
  provider
  locator
  media_type
  content_hash?
  original_name?
  captured_at?
  transcript?
  transcript_source?
  metadata
}
```

Specify:

- Gateway derives `space_id` from authorized context; callers cannot claim ownership.
- `locator` cannot contain credentials or short-lived signed query parameters.
- Provider examples are illustrative, not a closed enum.
- Confirmed Surface text is canonical for feedback; `transcript` is provenance.
- Resolver operations are availability, authorized resolution, relocation, and hash verification only.
- Failure codes are `evidence_unavailable`, `evidence_mismatch`, `evidence_forbidden`, and `invalid_evidence_reference`.
- The first slice has no media upload, download, resolver execution, schema, or repository.

- [ ] **Step 3: Narrow ADR-002 without rewriting its history**

Change its status line to:

```markdown
✅ 已接受；适用于 MemoryNexus 托管对象。外部媒体证据引用由 ADR-021 补充。
```

Add a scope note saying `StorageBackend` applies only when MemoryNexus manages
bytes. It does not require external evidence to be copied into S3/MinIO and does
not replace `EvidenceRef` or `EvidenceResolver`.

- [ ] **Step 4: Add navigation links**

Add ADR-021 to `decisions/README.md` and add `Media Evidence Contract` to the
architecture/contracts section in `docs/README.md`.

- [ ] **Step 5: Validate and commit Task 1**

Run:

```bash
rg -n "ADR-021|EvidenceRef|EvidenceResolver" decisions docs/media-evidence-contract.md docs/README.md
git diff --check
```

Expected: ADR-021 appears in the index and contract references; `git diff
--check` prints nothing.

Commit:

```bash
git add decisions/ADR-021-external-media-evidence-references.md \
  decisions/ADR-002-storage-abstraction.md decisions/README.md \
  docs/media-evidence-contract.md docs/README.md
git commit -m "docs: define external media evidence references"
```

### Task 2: Align Dictation And Agent Contracts

**Files:**
- Modify: `docs/dictation-coach-mvp.md`
- Modify: `docs/architecture/surfaces-and-adapters.md`
- Modify: `docs/agent-integration.md`

- [ ] **Step 1: Correct Dictation non-goals**

Replace absolute OCR/ASR prohibitions with:

```markdown
- MemoryNexus does not perform OCR, handwriting recognition, audio
  transcription, or raw-media interpretation in this MVP.
- An Agent or App Adapter may perform OCR/ASR, confirm normalized text with the
  user, and submit that text with optional media evidence references.
```

Keep multi-child management, broad education platform, full curriculum, and
cloud-only generation as non-goals.

- [ ] **Step 2: Add provenance to Dictation task and attempt inputs**

Add optional `evidence_refs` to `DictationTask`, `DictationAttempt`, Capture
request, and Performance request conceptual shapes. Extend source semantics to
include `agent_ocr` and `agent_transcribed`, while retaining `typed`, `pasted`,
and `test_fixture`.

Add these rules:

```markdown
- Feedback and deterministic classification use confirmed normalized text, not
  inaccessible media.
- `evidence_refs` preserve provenance and must follow the Media Evidence
  Contract.
- OCR/ASR confidence is Adapter context; uncertain text must be confirmed before
  submission.
```

- [ ] **Step 3: Add the Agent preprocessing flow to Surface architecture**

Add this flow to `docs/architecture/surfaces-and-adapters.md`:

```text
media -> Agent/App OCR or ASR -> user-confirmed text
      -> Surface Gateway(text + optional EvidenceRefInput)
      -> Engine feedback objects and Trace
```

State that `EvidenceResolver` belongs to integration/Adapter infrastructure and
does not perform cognitive analysis.

- [ ] **Step 4: Add Claw/Hermes operating rules**

In `docs/agent-integration.md`, tell Agents to:

1. extract text outside MemoryNexus;
2. show or otherwise confirm uncertain text;
3. submit confirmed text through Surface Gateway MCP tools;
4. attach an evidence reference only when future inspection matters;
5. never persist tokens, credentials, or signed URLs in locator or metadata;
6. continue the text loop when media is unavailable.

Also state that the current MCP Surface tools are pending and this policy is the
target contract for issues #160 and #162.

- [ ] **Step 5: Validate and commit Task 2**

Run:

```bash
rg -n "No OCR|No handwriting recognition|No audio transcription" \
  docs/dictation-coach-mvp.md docs/architecture/surfaces-and-adapters.md \
  docs/agent-integration.md
rg -n "agent_ocr|agent_transcribed|evidence_refs|confirmed" \
  docs/dictation-coach-mvp.md docs/architecture/surfaces-and-adapters.md \
  docs/agent-integration.md
git diff --check
```

Expected: the first search finds no absolute MemoryNexus-wide prohibition; the
second finds the new provenance and confirmation language; diff check is clean.

Commit:

```bash
git add docs/dictation-coach-mvp.md \
  docs/architecture/surfaces-and-adapters.md docs/agent-integration.md
git commit -m "docs: align dictation with agent media preprocessing"
```

### Task 3: Align Trace And Top-Level Architecture

**Files:**
- Modify: `docs/trace-contract.md`
- Modify: `docs/architecture/README.md`
- Modify: `README.md`

- [ ] **Step 1: Add Trace evidence links**

Add `related_evidence_ref_ids` to the full conceptual Trace shape and field
table. State explicitly:

```markdown
Trace links media evidence; it does not own media bytes. A missing or unavailable
reference affects provenance inspection, not the validity of confirmed text or
completed feedback.
```

Mark the field as conceptual and not present in the current persistent Trace
foundation.

- [ ] **Step 2: Correct the architecture diagram**

Replace the fixed storage branch with:

```text
      +--> External media providers
      |      local disk, removable drive, WebDAV, object storage
      |      referenced through EvidenceRef
      |
      +--> Optional managed object storage
             S3 / MinIO compatible provider when MemoryNexus owns bytes
```

Add `evidence/` as a future contract/integration responsibility only if the
layout section explicitly labels it conceptual; do not claim a Rust module
already exists.

- [ ] **Step 3: Add a concise README boundary**

Add one paragraph near the Dictation Coach or architecture overview:

```markdown
Agents and Apps may perform OCR or speech-to-text before calling MemoryNexus.
The Engine works from confirmed text and can retain provider-neutral media
evidence references for later inspection without requiring media ingestion.
```

Link `docs/media-evidence-contract.md` without duplicating its field list.

- [ ] **Step 4: Validate and commit Task 3**

Run:

```bash
rg -n "related_evidence_ref_ids|External media providers|confirmed text" \
  docs/trace-contract.md docs/architecture/README.md README.md
git diff --check
```

Expected: all three concepts are found and diff check prints nothing.

Commit:

```bash
git add docs/trace-contract.md docs/architecture/README.md README.md
git commit -m "docs: connect trace to external media provenance"
```

### Task 4: Update Contributor Rules And Roadmap

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/TODO.md`
- Modify: `docs/issues.md`

- [ ] **Step 1: Add the durable AGENTS rule**

Add ADR-021 to the project-mainline and architecture-decision sections. Add one
development rule:

```markdown
- OCR、ASR 和媒体采集属于 Agent / App Adapter。MemoryNexus 第一版只处理确认后的
  文字，并通过 `EvidenceRef` 保存可选的原始媒体追溯信息；不要让媒体不可用阻断文字
  Trace、反馈或计划。
```

Delete the duplicated Dictation Coach copy-guidance bullet already present in
`AGENTS.md`.

- [ ] **Step 2: Update TODO sequencing**

In Milestone 5, add evidence-reference contract/validation before Dictation
Capture and state that OCR/ASR may happen in the Agent Adapter. Change the final
adapter step to MCP/chat Agent first. In Milestone 6, state that the first smoke
is one learner, manual or Agent-confirmed text, and no dedicated App dependency.

- [ ] **Step 3: Align issue mirrors**

Update `docs/issues.md`:

- Issue 5.2: accepts typed, pasted, `agent_ocr`, and `agent_transcribed`
  confirmed text plus optional evidence references; MemoryNexus does no OCR.
- Issue 5.3: attempts are text-first and may link original media evidence;
  media availability is not needed for evaluation.
- Issue 5.7: choose MCP/chat Agent, remove CLI/Web as equal first choices, and
  cover Capture, Performance, Reflection, Planning, and Observation.
- Issue 6.2: require OCR/ASR preprocessing outside MemoryNexus, user confirmation,
  generic Surface calls, Trace provenance, and graceful unavailable evidence.
- Add one foundation issue for `EvidenceRefInput` validation only. Its non-goals
  are persistence, resolver execution, upload/download, OCR, and provider SDKs.

- [ ] **Step 4: Validate and commit Task 4**

Run:

```bash
rg -n "ADR-021|EvidenceRef|MCP/chat|agent_ocr|agent_transcribed" \
  AGENTS.md docs/TODO.md docs/issues.md
test "$(rg -c "Dictation Coach 文案优先" AGENTS.md)" -eq 1
git diff --check
```

Expected: new boundaries are found, the duplicate rule count is exactly one,
and diff check prints nothing.

Commit:

```bash
git add AGENTS.md docs/TODO.md docs/issues.md
git commit -m "docs: prioritize agent-first dictation delivery"
```

### Task 5: Repository-Wide Consistency Check And PR

**Files:**
- Review: all Markdown files changed in Tasks 1-4
- External: GitHub PR and issues #155, #156, #160, #162

- [ ] **Step 1: Scan for conflicting claims**

Run:

```bash
rg -n "No OCR|No audio transcription|Do not OCR|must upload|S3 / MinIO compatible storage" \
  README.md AGENTS.md docs decisions
```

Expected: remaining matches either explicitly scope the prohibition to
MemoryNexus Engine behavior or describe optional managed storage. Fix any
unqualified conflicting wording in the nearest canonical document and stage it
with the task that owns that document.

- [ ] **Step 2: Check links, whitespace, and scope**

Run:

```bash
test -f decisions/ADR-021-external-media-evidence-references.md
test -f docs/media-evidence-contract.md
rg -n "media-evidence-contract.md|ADR-021-external-media-evidence-references.md" \
  README.md AGENTS.md docs decisions
git diff --check
git status --short
```

Expected: both files exist, links are referenced from navigation/context docs,
diff check is silent, and status contains only intended Markdown changes.

- [ ] **Step 3: Skip Rust tests with an explicit reason**

Record in the PR body:

```text
Tests not run: documentation-only change; no Rust code, schema, API behavior, or
UI behavior changed. Validation: repository-wide terminology scans and
`git diff --check`.
```

- [ ] **Step 4: Push and open the PR**

Run:

```bash
git push -u origin codex/agent-first-dictation-evidence-docs
gh pr create --title "Docs: define agent-first dictation media evidence boundary" \
  --body $'## Summary\n\n- define provider-neutral external media evidence references\n- align Dictation Coach and Agent adapters around confirmed text\n- prioritize the MCP/chat Agent loop before a dedicated App\n\n## Validation\n\n- git diff --check\n- repository-wide terminology scans\n\nTests not run: documentation-only change; no Rust code, schema, API behavior, or UI behavior changed.'
```

Expected: GitHub returns a PR URL. Wait for required Format, Clippy, Build, and
Test checks even though the patch is documentation-only.

- [ ] **Step 5: Merge after green CI**

Run:

```bash
gh pr checks "$(gh pr view --json number --jq .number)" --watch
gh pr merge "$(gh pr view --json number --jq .number)" --squash --delete-branch
```

Expected: all required checks pass and the PR merges with a squash commit.

- [ ] **Step 6: Synchronize live GitHub issues after merge**

Edit issue bodies `#155`, `#156`, `#160`, and `#162` so they match the exact
scope and acceptance criteria written in `docs/issues.md`. Create one new P1
foundation issue titled:

```text
Foundation: Validate External Media Evidence References
```

Its acceptance criteria must require provider-neutral `EvidenceRefInput`,
Space ownership derived from authorized Surface context, locator secret
rejection/redaction, optional references on Surface requests, and tests for
valid, invalid, and unavailable-reference metadata. Its non-goals must exclude
persistence, resolver execution, upload/download, OCR/ASR, and provider SDKs.

Verify:

```bash
gh issue view 155 --json body
gh issue view 156 --json body
gh issue view 160 --json body
gh issue view 162 --json body
gh issue list --state open --search \
  'Foundation: Validate External Media Evidence References in:title'
```

Expected: issue bodies match the merged planning mirror and exactly one open
foundation issue is returned.
