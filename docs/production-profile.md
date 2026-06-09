# Production Profile

The Production Profile is for running MemoryNexus against stable hosted or
self-hosted services without requiring local Docker on every user or agent
machine.

It is an install and deployment profile, not a new backend architecture and not
a Supabase-only recipe.
MemoryNexus remains one Rust + Axum API. PostgreSQL remains the persistence
backend. Qdrant remains the first vector search backend. Local Docker Compose
stays available for local-first development, offline use, and the Local
One-click Profile.

## Service Shape

Recommended first hosted shape:

- MemoryNexus API runs as a release binary, system service, or container.
- PostgreSQL comes from Supabase, Neon, RDS, self-hosted Postgres, or an
  equivalent PostgreSQL-compatible provider.
- Vector search comes from Qdrant Cloud or self-hosted Qdrant.
- Object storage is optional and uses an S3-compatible provider when media
  upload is enabled.
- OpenAI-compatible summary and embedding providers are optional. Deterministic
  local embeddings remain useful for smoke tests.
- MCP clients use the `memorynexus-mcp` stdio binary and call the same Rust API.

This profile does not use Supabase REST, PostgREST, Edge Functions, or RLS as a
replacement for the Rust API or `CognitiveSpace` permissions. Supabase is first
a managed PostgreSQL compatibility target, as defined in ADR-015.

## Environment Variables

Required for the API:

| Variable | Required | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | Yes | PostgreSQL connection string. Use SSL for managed databases. |
| `JWT_SECRET` | Yes | Secret used to sign MemoryNexus JWTs. Use a long random value. |
| `QDRANT_URL` | Yes for semantic search | Qdrant endpoint, local, self-hosted, or Qdrant Cloud. |
| `QDRANT_COLLECTION` | Recommended | Vector collection name. Defaults to `memorynexus_memories`. |
| `QDRANT_API_KEY` | Required for Qdrant Cloud | API key sent as the Qdrant `api-key` header. Leave unset for unauthenticated local Qdrant. |
| `MEMORYNEXUS_EMBEDDING_PROVIDER` | Recommended | Use `local` for deterministic smoke tests or `openai` for real semantic embeddings. |
| `OPENAI_API_KEY` | Optional | Enables OpenAI-compatible embeddings, summaries, and transcription paths that use the OpenAI key. |
| `OPENROUTER_API_KEY` | Optional | Enables OpenRouter-backed summary generation when configured by the summary provider path. |
| `MEMORYNEXUS_BIND_ADDR` | Optional | API bind address, for example `0.0.0.0:8080`. Defaults to the local API port. |

Optional S3-compatible object storage:

| Variable | Purpose |
| --- | --- |
| `S3_ENDPOINT` | S3-compatible endpoint. |
| `S3_REGION` | Region, defaulting to `us-east-1` when omitted. |
| `S3_ACCESS_KEY` | Access key. |
| `S3_SECRET_KEY` | Secret key. |

MCP sidecar:

| Variable | Required | Purpose |
| --- | --- | --- |
| `MEMORYNEXUS_API_URL` | Recommended | Rust API base URL for `memorynexus-cli` and `memorynexus-mcp`. Defaults to `http://localhost:8080`. |
| `MEMORYNEXUS_TOKEN` | Yes for authenticated MCP calls | JWT returned by `auth register` or login. |

## Example Hosted Environment

```bash
export DATABASE_URL='postgresql://user:password@db.example.com:5432/memorynexus?sslmode=require'
export JWT_SECRET='<long-random-secret>'

export QDRANT_URL='https://example.qdrant.cloud'
export QDRANT_COLLECTION='memorynexus_production'
export QDRANT_API_KEY='<qdrant-cloud-api-key>'

export MEMORYNEXUS_EMBEDDING_PROVIDER='openai'
export OPENAI_API_KEY='<openai-compatible-api-key>'

export MEMORYNEXUS_BIND_ADDR='0.0.0.0:8080'
```

For deterministic smoke tests without external embedding calls:

```bash
export MEMORYNEXUS_EMBEDDING_PROVIDER='local'
```

## PostgreSQL Notes

Use the normal `DATABASE_URL` path. The Rust API runs SQLx migrations on
startup, so the configured database user must be allowed to create and alter the
MemoryNexus schema during deployment.

Provider notes:

- Supabase Postgres is supported as managed PostgreSQL only. Do not use
  Supabase REST or PostgREST to bypass the Rust API.
- Neon and RDS are PostgreSQL compatibility targets through `DATABASE_URL`.
- For Supabase, prefer direct connection or session pooler for the long-running
  Rust API. Transaction pooler requires explicit SQLx compatibility checks.
- Use SSL for managed database connections.

## Qdrant Notes

Use Qdrant Cloud or self-hosted Qdrant through the existing Qdrant vector
backend:

```bash
export QDRANT_URL='https://example.qdrant.cloud'
export QDRANT_COLLECTION='memorynexus_production'
export QDRANT_API_KEY='<qdrant-cloud-api-key>'
```

Local or unauthenticated self-hosted Qdrant can omit `QDRANT_API_KEY`.

Do not replace Qdrant with pgvector in this profile. pgvector can be evaluated
later as a separate optional vector backend through a new issue and ADR.

## Launch

Run the API as a release binary, service, or container with the environment
above:

```bash
memorynexus
```

For local development from source, the command remains:

```bash
cargo run --bin memorynexus
```

The API health endpoint should return `OK`:

```bash
curl -fsS http://localhost:8080/health
```

## Smoke Checklist

Set the API URL for CLI and MCP smoke commands:

```bash
export MEMORYNEXUS_API_URL='https://memorynexus.example.com'
```

If testing locally against the hosted dependencies, use:

```bash
export MEMORYNEXUS_API_URL='http://localhost:8080'
```

1. Health:

   ```bash
   memorynexus-cli health
   ```

2. Register or log in, then export the returned JWT as `MEMORYNEXUS_TOKEN`:

   ```bash
   memorynexus-cli auth register \
     --email you@example.com \
     --name You \
     --password secret123
   ```

3. Create a Cognitive Space:

   ```bash
   memorynexus-cli space create --name "Production Smoke"
   ```

   Export the returned Space ID:

   ```bash
   export MEMORYNEXUS_SPACE_ID='<space-id-from-response>'
   ```

4. Create a Memory in that Space:

   ```bash
   memorynexus-cli memory add \
     --space "$MEMORYNEXUS_SPACE_ID" \
     --content "Production Profile smoke memory for managed PostgreSQL and Qdrant."
   ```

5. Run keyword search:

   ```bash
   memorynexus-cli search "Production Profile" \
     --space "$MEMORYNEXUS_SPACE_ID" \
     --limit 5
   ```

6. Run semantic search:

   ```bash
   memorynexus-cli search "managed Qdrant smoke" \
     --space "$MEMORYNEXUS_SPACE_ID" \
     --semantic \
     --limit 5
   ```

7. Create and run a Lens:

   ```bash
   memorynexus-cli lens create \
     --space "$MEMORYNEXUS_SPACE_ID" \
     --name "Production Context" \
     --strategy project_context
   ```

   Export the returned Lens ID:

   ```bash
   export MEMORYNEXUS_LENS_ID='<lens-id-from-response>'
   ```

   Then run the Lens:

   ```bash
   memorynexus-cli lens run "$MEMORYNEXUS_LENS_ID" \
     --query "Summarize the hosted Production Profile smoke."
   ```

8. Verify MCP tools are discoverable:

   ```bash
   printf '%s\n' \
     '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' \
     | MEMORYNEXUS_API_URL="$MEMORYNEXUS_API_URL" \
       MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" \
       memorynexus-mcp
   ```

Expected result:

- Health returns `OK`.
- Auth returns a JWT.
- Space and Memory creation return IDs.
- Keyword and semantic search can find the smoke Memory.
- Lens Run returns a persisted result with provenance.
- MCP `tools/list` includes MemoryNexus tools such as `create_space`,
  `add_memory`, `search_memories`, `run_lens`, and the practice-session tools.

## Operations Checklist

Before long-term production use, document and verify:

- TLS termination and allowed origins.
- Database backups and restore drill.
- Qdrant collection backup or export strategy.
- Secret storage and rotation for `JWT_SECRET`, database credentials, Qdrant
  credentials, object storage credentials, and AI provider keys.
- Monitoring for API health, database connectivity, Qdrant connectivity, and
  semantic search errors.
- Upgrade process for the API binary/container and database migrations.
- Log retention and redaction policy for personal cognitive data.

## Relationship To Local One-click

The Production Profile removes the requirement for local Docker-managed
dependencies on the user or agent machine. It does not delete Docker support.

Use local Docker Compose when you need:

- local-first development;
- offline or LAN-only operation;
- the Local One-click Profile;
- reproducible local acceptance testing.
