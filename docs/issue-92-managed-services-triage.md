# Issue #92 Managed Services Triage

Date: 2026-06-06

Source issue: https://github.com/blackfaced/MemoryNexus/issues/92

## Current Issue

Current title:

```text
[infra] 简化安装流程：移除 Docker 依赖，改用托管服务
```

Current proposal summary:

- Replace local Docker-based PostgreSQL, Qdrant, and Redis dependency with
  managed services.
- Use Supabase or Neon with pgvector so one database handles persistence and
  vector search.
- Remove Redis because the Rust-first backend does not use it.

## Triage Conclusion

Keep the issue open, but rename and narrow it to an optional managed-service
install path. Do not close it outright, because reducing install friction is a
valid Distribution and Agent Install goal.

The current wording conflicts with accepted architecture if interpreted as:

- deleting the local Docker path;
- replacing Qdrant with pgvector inside this issue;
- making Supabase or Neon a new backend path;
- weakening the local-first runtime direction from ADR-016.

It does not conflict if scoped as:

- release binary plus managed PostgreSQL plus managed Qdrant;
- local Docker Compose remains the default local-first developer and fallback
  path;
- Rust + Axum remains the only main backend;
- `CognitiveSpace` permissions stay inside the Rust service;
- Supabase remains a managed PostgreSQL compatibility target under ADR-015.

Redis removal does not need to be a migration goal for this issue. The Rust-first
backend does not currently depend on Redis, so any Redis cleanup should be a
small documentation/config hygiene task if stale references remain.

## Recommendation

Recommendation: keep, rename, and split.

Use #92 as the umbrella for optional managed-service installation, then split
implementation follow-ups:

1. Supabase or Neon PostgreSQL compatibility smoke through existing
   `DATABASE_URL` and SQLx migrations.
2. Qdrant Cloud compatibility through the existing Qdrant vector backend,
   including API key support if required.
3. Binary-first install documentation that supports both local Docker services
   and managed services.
4. A separate research issue and ADR only if replacing Qdrant with pgvector is
   still desired later.

Do not implement pgvector replacement, remove Qdrant, remove local Docker, or
introduce a second backend under #92.

## Suggested Title

```text
[infra] Add optional managed-service install path without removing local Docker
```

## Suggested Body

````markdown
## Problem

Current user and agent install docs assume local Docker Compose for PostgreSQL
and Qdrant. That remains the local-first developer path, but some users should
be able to run release binaries against managed infrastructure without running
Docker locally.

## Goal

Add an optional managed-service install path:

download release binary -> set service connection env vars -> run Rust API,
CLI, or MCP server.

The managed path should coexist with, not replace, the local Docker path.

## Proposed scope

- Validate managed PostgreSQL through the existing Rust + SQLx
  `DATABASE_URL` path.
- Treat Supabase Postgres compatibility according to ADR-015: managed
  PostgreSQL first, not Supabase as a backend replacement.
- Validate managed Qdrant / Qdrant Cloud through the existing Qdrant vector
  backend.
- Add Qdrant Cloud API-key support if the current REST integration needs it.
- Document the managed-service env vars next to the local Docker setup.
- Keep local deterministic embeddings available for smoke tests.

## Non-goals

- Do not delete Docker Compose or the local-first install path.
- Do not replace Qdrant with pgvector in this issue.
- Do not route core data operations through Supabase REST or PostgREST.
- Do not introduce a second backend, BFF, or Supabase Edge Functions backend.
- Do not migrate auth, storage, or realtime here.
- Do not move `CognitiveSpace` membership or authorization out of the Rust
  service.

## Acceptance criteria

- A release-binary install guide can be followed with managed PostgreSQL and
  managed Qdrant.
- The same guide still links to the local Docker Compose path for offline,
  local-first, and developer use.
- `DATABASE_URL`, `QDRANT_URL`, `QDRANT_COLLECTION`, embedding provider, and any
  required Qdrant Cloud credential are documented.
- Smoke checklist covers health, register/login, create space, create memory,
  semantic search, and Lens Run.
- No Rust API or repository path bypasses the existing Rust + Axum backend.
````

## Suggested Issue Comment

````markdown
Triage recommendation: keep this issue, but rename and narrow it.

The installation-friction problem is real, so I would not close #92. However,
the current title/body is too broad for the accepted architecture. Taken
literally, "remove Docker dependency" + "replace Qdrant with pgvector" conflicts
with the current local-first direction and with the accepted PostgreSQL/Qdrant
backend shape:

- ADR-009 keeps Rust + Axum as the only main backend.
- ADR-003 already chose Qdrant as the vector backend, with both local and cloud
  deployment paths.
- ADR-015 says Supabase is first a managed PostgreSQL compatibility target, not
  a replacement backend, and explicitly does not replace Qdrant with Supabase /
  Postgres vector without a separate ADR.
- ADR-016 keeps MemoryNexus local-first and trace-driven, while allowing future
  local/cloud routing as an optional runtime/deployment choice.

Suggested direction: make #92 an optional managed-service install path:

- release binary + managed PostgreSQL + managed Qdrant;
- local Docker Compose remains the local-first developer/offline path;
- Supabase/Neon are validated through `DATABASE_URL` and SQLx migrations;
- Qdrant Cloud is validated through the existing Qdrant vector backend;
- add Qdrant Cloud API-key support if the current REST integration needs it;
- leave any pgvector replacement as a separate research issue + ADR.

Suggested new title:

```text
[infra] Add optional managed-service install path without removing local Docker
```

Suggested split:

1. Managed PostgreSQL compatibility smoke: Supabase/Neon via `DATABASE_URL`.
2. Managed Qdrant/Qdrant Cloud compatibility smoke, including credentials if
   needed.
3. Binary-first install docs covering both local Docker and managed services.
4. Separate pgvector evaluation issue only if we still want to consider replacing
   Qdrant later.

I would keep this issue open as the umbrella after renaming, but remove the
Qdrant -> pgvector replacement from its acceptance scope.
````
