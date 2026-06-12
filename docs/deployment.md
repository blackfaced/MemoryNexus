# Deployment

MemoryNexus currently deploys as one Rust API binary plus external services.
Local personal-agent use can also run the `memorynexus-mcp` stdio binary next to
the agent client; it calls the same Rust API and is not a second backend.

## Required Services

- PostgreSQL
- Qdrant, when semantic search is enabled
- S3/MinIO compatible storage, when media upload is enabled

For hosted deployments that should not require local Docker-managed
dependencies, use the [Production Profile](production-profile.md).

For Supabase managed PostgreSQL, use the normal `DATABASE_URL` path described in
[Supabase Postgres Compatibility](supabase-postgres.md). Supabase is a managed
PostgreSQL target for the Rust API, not a replacement backend.

## Environment

```bash
DATABASE_URL=postgresql://postgres:postgres@postgres:5432/memorynexus
QDRANT_URL=http://qdrant:6333
QDRANT_COLLECTION=memorynexus_memories
# Required for Qdrant Cloud; leave unset for local unauthenticated Qdrant.
QDRANT_API_KEY=<secret>
MEMORYNEXUS_EMBEDDING_PROVIDER=openai
OPENAI_API_KEY=<secret>
JWT_SECRET=<secret>
```

For local or staging smoke tests, set:

```bash
MEMORYNEXUS_EMBEDDING_PROVIDER=local
```

Managed database connections should use SSL. For Supabase, prefer the direct
connection string or shared pooler session mode for the long-running Rust API.
Use transaction pooling only after SQLx prepared statement compatibility has
been explicitly validated.

## Docker Build

```bash
docker build -t memorynexus:local .
```

## Docker Compose Infrastructure

The repository `docker-compose.yml` starts local infrastructure:

```bash
docker compose up -d postgres qdrant minio
```

Run the API locally with Cargo or add your own deployment wrapper around the
root `Dockerfile`.

## MCP Sidecar

For Claw, Hermes, or another local MCP client, build the stdio adapter:

```bash
cargo build --bin memorynexus-mcp
```

Then configure the agent client with:

```json
{
  "mcpServers": {
    "memorynexus": {
      "command": "/path/to/MemoryNexus/target/debug/memorynexus-mcp",
      "env": {
        "MEMORYNEXUS_API_URL": "http://localhost:8080",
        "MEMORYNEXUS_TOKEN": "<jwt-token>"
      }
    }
  }
}
```

Do not store production JWTs or API keys in committed config files.
