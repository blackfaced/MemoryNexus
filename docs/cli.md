# MemoryNexus CLI

`memorynexus-cli` is a thin stateless client for the Rust API. It outputs JSON
by default so humans, scripts, and agents can parse the same responses.

## Configuration

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `MEMORYNEXUS_API_URL` | No | `http://localhost:8080` | Rust API base URL |
| `MEMORYNEXUS_TOKEN` | Yes, except `health` and `auth` | - | JWT bearer token |

Backend-only variables for semantic search:

| Variable | Default | Purpose |
|----------|---------|---------|
| `QDRANT_URL` | - | Enables Qdrant vector indexing and search |
| `QDRANT_COLLECTION` | `memorynexus_memories` | Qdrant collection |
| `MEMORYNEXUS_EMBEDDING_PROVIDER` | `openai` | Use `local` for deterministic smoke tests |

## Run Locally

Start PostgreSQL:

```bash
docker compose up -d postgres
```

Run the API in one terminal:

```bash
cargo run --bin memorynexus
```

Run the CLI in another terminal:

```bash
cargo run --bin memorynexus-cli -- health
```

## Basic Smoke Test

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080

cargo run --bin memorynexus-cli -- auth register \
  --email cli-smoke@example.com \
  --name CliSmoke \
  --password secret123

export MEMORYNEXUS_TOKEN=<token-from-auth-response>

cargo run --bin memorynexus-cli -- space list

cargo run --bin memorynexus-cli -- space create \
  --name "CLI Smoke Space" \
  --description "Local CLI verification"

export MEMORYNEXUS_SPACE_ID=<space-id-from-space-create>

cargo run --bin memorynexus-cli -- memory add \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --title "CLI smoke memory" \
  --content "Rust cognitive lens memory CLI smoke test" \
  --tags "cli,smoke"

cargo run --bin memorynexus-cli -- memory list \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --limit 5

cargo run --bin memorynexus-cli -- search "cognitive lens" \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --limit 5

cargo run --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --name "Project Context" \
  --description "Interpret project memory for planning" \
  --strategy project_context \
  --output brief \
  --retrieval semantic

cargo run --bin memorynexus-cli -- lens list \
  --space "$MEMORYNEXUS_SPACE_ID"

cargo run --bin memorynexus-cli -- lens get <lens-id>

cargo run --bin memorynexus-cli -- lens run <lens-id> \
  --query "Summarize the current project direction" \
  --limit 5

cargo run --bin memorynexus-cli -- lens run get <run-id>
```

## Semantic Smoke Test

Start PostgreSQL and Qdrant:

```bash
docker compose up -d postgres qdrant
```

Run the API with local deterministic embeddings:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

In another terminal:

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080
export MEMORYNEXUS_TOKEN=<token-from-auth-response>
export MEMORYNEXUS_SPACE_ID=<space-id>

cargo run --bin memorynexus-cli -- memory add \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --title "Semantic smoke memory" \
  --content "phase1b semantic qdrant smoke memory" \
  --tags "phase1b,semantic"

cargo run --bin memorynexus-cli -- search "phase1b semantic qdrant" \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --semantic \
  --limit 5
```

Expected result: semantic search returns the memory created in the same
Cognitive Space.

## Commands

```bash
memorynexus-cli health

memorynexus-cli auth register --email <EMAIL> --name <NAME> --password <PASSWORD>
memorynexus-cli auth login --email <EMAIL> --password <PASSWORD>

memorynexus-cli space create --name <NAME> [--description <TEXT>]
memorynexus-cli space list

memorynexus-cli lens create --space <SPACE_ID> --name <NAME> \
  [--description <TEXT>] [--strategy <NAME>] [--output <FORMAT>] [--retrieval <MODE>]
memorynexus-cli lens list --space <SPACE_ID>
memorynexus-cli lens get <LENS_ID>
memorynexus-cli lens run <LENS_ID> --query <TEXT> [--limit <N>]
memorynexus-cli lens run get <RUN_ID>

memorynexus-cli memory add --content <TEXT> [--space <SPACE_ID>] [--title <TEXT>] \
  [--tags <COMMA_SEPARATED_TAGS>] [--type text|image|audio|video] [--shared]
memorynexus-cli memory list [--space <SPACE_ID>] [--limit <N>] [--offset <N>]
memorynexus-cli memory get <MEMORY_ID>
memorynexus-cli memory delete <MEMORY_ID>

memorynexus-cli search <QUERY> [--space <SPACE_ID>] [--semantic] [--limit <N>]
```

## Output

Successful responses pass through the backend JSON:

```json
{"ok": true, "data": {}}
```

CLI-side errors are also JSON:

```json
{"ok": false, "error": {"message": "MEMORYNEXUS_TOKEN is required"}}
```
