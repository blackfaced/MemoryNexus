# Issue #92 Managed Services Triage

Date: 2026-06-08

Source issue: https://github.com/blackfaced/MemoryNexus/issues/92

## Current Issue

Current title:

```text
Define hosted Production Profile without local Docker
```

Current proposal summary:

- Define a Production Profile where MemoryNexus runs against stable hosted or
  self-hosted services instead of requiring local Docker on every user or agent
  machine.
- Keep PostgreSQL as the persistence backend, supplied by Supabase, Neon, RDS,
  self-hosted Postgres, or equivalent.
- Keep vector search on Qdrant Cloud or self-hosted Qdrant for the first hosted
  path.
- Treat pgvector as a future optional vector backend evaluation, not as part of
  this issue.
- Document environment variables, smoke checks, TLS, backups, monitoring,
  secret management, and upgrade process.

## Triage Conclusion

Keep the issue open. The updated issue description resolves the main triage
concern from 2026-06-06: it no longer treats avoiding local Docker and replacing
Qdrant with pgvector as one change.

The issue no longer conflicts with accepted architecture as long as execution
stays within the new scope:

- Production Profile is an install/deployment profile, not a backend rewrite;
- local Docker Compose remains the Local One-click Profile and local-first
  developer/offline path;
- PostgreSQL can be managed or self-hosted through the existing `DATABASE_URL`
  path;
- Qdrant remains the first vector search backend, using Qdrant Cloud or
  self-hosted Qdrant;
- pgvector remains a separate future evaluation and would need its own issue and
  likely ADR before replacing or supplementing Qdrant;
- Rust + Axum remains the only main backend;
- `CognitiveSpace` permissions stay inside the Rust service;
- Supabase remains a managed PostgreSQL compatibility target under ADR-015.

Redis removal does not need to be a migration goal for this issue. The Rust-first
backend does not currently depend on Redis, so any Redis cleanup should be a
small documentation/config hygiene task if stale references remain.

## Recommendation

Recommendation: keep as currently rewritten. No close is needed, and the current
title is acceptable.

Use #92 as a docs-first Production Profile issue. The implementation should be
limited to documenting and validating hosted/self-hosted service configuration.
Likely follow-ups remain:

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

## Suggested Remaining Adjustment

No title/body rewrite is required now. If the issue body is edited again, the
only useful clarification would be to say explicitly that the Production Profile
may run the MemoryNexus API as a binary, service, or container, while "without
local Docker" refers to not requiring local Docker-managed dependencies on the
user or agent machine.

## Current Title Is Acceptable

```text
Define hosted Production Profile without local Docker
```

No body rewrite is needed after the 2026-06-08 issue update.

## Suggested Issue Comment

````markdown
The updated issue description resolves the main conflict from the earlier
triage. It now separates:

1. avoiding local Docker as an install/deployment profile; and
2. replacing Qdrant with pgvector as a separate architecture decision.

I would keep #92 open with the current title/body. The implementation should be
docs-first: define the Production Profile, required env vars, and smoke
checklist for managed PostgreSQL + Qdrant Cloud or equivalent.

Architecture boundaries still look correct:

- ADR-009 keeps Rust + Axum as the only main backend.
- ADR-003 already chose Qdrant as the vector backend, with both local and cloud
  deployment paths.
- ADR-015 says Supabase is first a managed PostgreSQL compatibility target, not
  a replacement backend, and explicitly does not replace Qdrant with Supabase /
  Postgres vector without a separate ADR.
- ADR-016 keeps MemoryNexus local-first and trace-driven, while allowing future
  local/cloud routing as an optional runtime/deployment choice.

Remaining implementation split I would keep:

1. Managed PostgreSQL compatibility smoke: Supabase/Neon via `DATABASE_URL`.
2. Managed Qdrant/Qdrant Cloud compatibility smoke, including credentials if
   needed.
3. Production Profile docs covering binary/service/container API execution,
   managed dependencies, TLS, backups, monitoring, secrets, upgrades, and smoke
   checks.
4. Separate pgvector evaluation issue/ADR only if we still want to consider it
   later.

No Rust implementation change is implied by this triage.
````
