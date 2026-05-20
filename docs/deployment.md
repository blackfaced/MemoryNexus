# Deployment

MemoryNexus currently deploys as one Rust API binary plus external services.

## Required Services

- PostgreSQL
- Qdrant, when semantic search is enabled
- S3/MinIO compatible storage, when media upload is enabled

## Environment

```bash
DATABASE_URL=postgresql://postgres:postgres@postgres:5432/memorynexus
QDRANT_URL=http://qdrant:6333
QDRANT_COLLECTION=memorynexus_memories
MEMORYNEXUS_EMBEDDING_PROVIDER=openai
OPENAI_API_KEY=<secret>
JWT_SECRET=<secret>
```

For local or staging smoke tests, set:

```bash
MEMORYNEXUS_EMBEDDING_PROVIDER=local
```

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
