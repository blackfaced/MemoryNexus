# Binary First Self Install Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make agent self-install profile-aware and binary-first for Trial and Local One-click installs.

**Architecture:** Add a shared Rust install model used by both `memorynexus-cli` and `memorynexus-mcp`. The model reports detected OS/arch, release target, binary availability, API health, smoke commands, fallback rules, and profile-specific upgrade/install plans.

**Tech Stack:** Rust, serde_json, CLI unit tests, MCP unit tests, Markdown docs.

---

### Task 1: Failing Tests

**Files:**
- Modify: `src/bin/memorynexus-cli.rs`
- Modify: `src/bin/memorynexus-mcp.rs`

- [x] **Step 1: Add CLI tests for install profiles**

Add tests proving `install status` and upgrade plans report Trial, Local One-click, Production, and Developer profiles, and that Trial/Local plans do not include cargo.

- [x] **Step 2: Add MCP tests for install profiles**

Add tests proving MCP schemas accept profile fields and local tools return the same profile-aware plans.

- [x] **Step 3: Run focused tests**

Run: `cargo test --bin memorynexus-cli --bin memorynexus-mcp install`
Expected: FAIL because profile-aware install fields are not implemented yet.

### Task 2: Shared Install Model

**Files:**
- Create: `src/install.rs`
- Modify: `src/lib.rs`
- Modify: `src/bin/memorynexus-cli.rs`
- Modify: `src/bin/memorynexus-mcp.rs`

- [x] **Step 1: Implement `InstallProfile` and `InstallOptions`**

Profiles are `trial`, `local-one-click`, `production`, and `developer`.

- [x] **Step 2: Implement target detection**

Map macOS arm64, macOS x86_64, and Linux x86_64 to the existing release targets. Unsupported targets must report an explicit fallback reason.

- [x] **Step 3: Implement plan JSON**

Trial uses `memorynexus-mcp`, hosted API env, MCP initialize, and tools/list. Local One-click uses release archive, checksum, bin install, Docker PostgreSQL/Qdrant, API health, MCP config, and tools/list. Developer keeps cargo test/build. Production remains hosted/stable deployment, not Supabase-only.

- [x] **Step 4: Wire CLI and MCP to the shared model**

Both surfaces should expose the same status and plan shape.

### Task 3: Documentation And Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/agent-self-install.md`
- Modify: `docs/mcp.md`
- Modify: `docs/cli.md`
- Modify: `docs/TODO.md`

- [x] **Step 1: Update docs**

Make binary-first the default for Trial and Local One-click, and keep Developer Profile as the source-build path.

- [x] **Step 2: Run full verification**

Run:
`cargo fmt --check`
`cargo test`
`cargo clippy --all-targets --all-features -- -D clippy::all`

- [x] **Step 3: Clean generated outputs and publish**

Remove generated `target/`, `*.profraw`, `playwright-report`, and `test-results` if present, then commit, push, and create a PR.
