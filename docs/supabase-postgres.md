# Supabase Postgres Compatibility

Supabase is supported as a managed PostgreSQL target for MemoryNexus. This path
keeps the Rust + Axum API as the only backend and keeps `CognitiveSpace`
membership checks inside MemoryNexus.

This guide covers only Supabase Postgres through `DATABASE_URL`. Supabase Auth,
Storage, Realtime, Edge Functions, PostgREST, and Row Level Security are out of
scope for this compatibility slice.

## Recommended Connection Mode

Use one of these connection modes for the MemoryNexus Rust API:

| Mode | Use for MemoryNexus | Notes |
| --- | --- | --- |
| Direct connection | Preferred when the API host can reach the project endpoint | Best fit for migrations and long-running Rust API processes. Supabase direct endpoints are IPv6 by default unless the project has the IPv4 add-on. |
| Shared pooler, session mode | Preferred fallback for IPv4-only API hosts | Suitable for persistent backend processes when direct IPv6 is not available. |
| Transaction pooler | Not the default | Use only after SQLx prepared statement compatibility is handled. Supabase documents that transaction mode does not support prepared statements. |

Supabase's connection guide describes direct connections as appropriate for
migrations and long-lived backends, session pooler for persistent backends on
IPv4-only networks, and transaction pooler for serverless or edge workloads:
<https://supabase.com/docs/guides/database/connecting-to-postgres>.

MemoryNexus is a long-running Rust service with an application-side SQLx pool,
so direct connection or session pooler is the first choice.

## Environment

Use the normal MemoryNexus `DATABASE_URL` path:

```bash
export DATABASE_URL='postgresql://postgres:<password>@db.<project-ref>.supabase.co:5432/postgres?sslmode=require'
export JWT_SECRET='<long-random-secret>'

export QDRANT_URL='https://<qdrant-endpoint>'
export QDRANT_COLLECTION='memorynexus_supabase'
export QDRANT_API_KEY='<qdrant-api-key>'

export MEMORYNEXUS_EMBEDDING_PROVIDER='local'
export MEMORYNEXUS_BIND_ADDR='0.0.0.0:8080'
```

For an IPv4-only host without the Supabase IPv4 add-on, use the session pooler
connection string from the Supabase dashboard instead:

```bash
export DATABASE_URL='postgresql://postgres.<project-ref>:<password>@aws-<region>.pooler.supabase.com:5432/postgres?sslmode=require'
```

Keep `sslmode=require` for managed Supabase Postgres connections. Use the
connection strings from the Supabase dashboard's Connect flow and do not commit
real credentials to the repository.

## Migration Validation

The MemoryNexus API runs SQLx migrations on startup from `migrations/`.
Validate a Supabase project with a disposable or staging database before using
it for long-term memory:

```bash
export DATABASE_URL='postgresql://...supabase.../postgres?sslmode=require'
export JWT_SECRET='<long-random-secret>'
export MEMORYNEXUS_EMBEDDING_PROVIDER='local'

./memorynexus
```

Expected startup behavior:

1. The API connects through `DATABASE_URL`.
2. SQLx applies migrations from `migrations/001_initial_schema.sql` through the
   latest migration.
3. The API starts listening on `MEMORYNEXUS_BIND_ADDR` or `0.0.0.0:8080`.
4. `memorynexus-cli health` succeeds against the API.

If you validate from source instead of a release binary:

```bash
cargo run --bin memorynexus
```

Do not use Supabase REST, PostgREST, or the Supabase JavaScript client to create
or modify MemoryNexus core tables. The Rust API owns migrations, writes, and
permission checks.

## Transaction Pooler Caveat

Supabase transaction mode is useful for serverless and edge functions, but it
does not support prepared statements. MemoryNexus currently uses SQLx through a
long-running Rust API and should not default to transaction pooling.

Only use a transaction pooler after a dedicated follow-up verifies SQLx
configuration for prepared statement compatibility. Until that exists, prefer:

1. Direct connection for migrations and persistent API hosts that can reach it.
2. Shared pooler session mode for persistent API hosts on IPv4-only networks.
3. Transaction mode only as an explicitly tested exception.

## Smoke Checklist

After the API starts against Supabase Postgres, run this checklist through the
Rust API or `memorynexus-cli`:

1. `memorynexus-cli health`
2. Register a test user.
3. Log in as that user and export `MEMORYNEXUS_TOKEN`.
4. Create a `CognitiveSpace`.
5. Add a Memory in that Space.
6. List memories in that Space.
7. Run keyword search for the Memory content.
8. If Qdrant is configured, run semantic search and confirm results stay scoped
   to the Space.
9. Create a Lens in the Space.
10. Run the Lens and confirm a persisted Lens Run with source memory citations.

Example source-build smoke commands:

```bash
export MEMORYNEXUS_API_URL='http://localhost:8080'

AUTH_JSON=$(cargo run --quiet --bin memorynexus-cli -- auth register \
  --email "supabase-smoke-$(date +%s)@example.com" \
  --name SupabaseSmoke \
  --password secret123)

export MEMORYNEXUS_TOKEN=$(printf '%s' "$AUTH_JSON" | jq -r '.data.token')

SPACE_JSON=$(cargo run --quiet --bin memorynexus-cli -- space create \
  --name "Supabase Smoke Space")

export MEMORYNEXUS_SPACE_ID=$(printf '%s' "$SPACE_JSON" | jq -r '.data.id')

cargo run --quiet --bin memorynexus-cli -- memory add \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --title "Supabase compatibility smoke" \
  --content "MemoryNexus can persist through Supabase Postgres while keeping Rust API permissions."

cargo run --quiet --bin memorynexus-cli -- memory list \
  --space "$MEMORYNEXUS_SPACE_ID"

cargo run --quiet --bin memorynexus-cli -- search \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --query "Supabase compatibility"

LENS_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --name "Deployment Review" \
  --strategy project_context)

LENS_ID=$(printf '%s' "$LENS_JSON" | jq -r '.data.id')

cargo run --quiet --bin memorynexus-cli -- lens run "$LENS_ID" \
  --query "Summarize the Supabase deployment compatibility signal."
```

Release-binary installs can run the same commands with `memorynexus-cli` instead
of `cargo run --quiet --bin memorynexus-cli --`.

## Out Of Scope

- Supabase Auth is not used for MemoryNexus login in this slice.
- Supabase Storage is not an object storage adapter in this slice.
- Supabase Realtime is not used for UI refreshes in this slice.
- Supabase Edge Functions are not a MemoryNexus backend.
- Supabase RLS is not the primary permission model for MemoryNexus data.
- Qdrant remains the vector search backend. Do not replace it with pgvector
  without a separate issue and ADR.
