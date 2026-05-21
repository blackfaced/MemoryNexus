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

Backend-only variables for Lens Run summaries:

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEMORYNEXUS_SUMMARY_PROVIDER` | `openai` | Use `openai`, `openrouter`, or `none` |
| `MEMORYNEXUS_SUMMARY_API_KEY` | `OPENAI_API_KEY` / `OPENROUTER_API_KEY` | Summary-only API key override |
| `MEMORYNEXUS_SUMMARY_MODEL` | `OPENAI_MODEL` or provider default | Chat model for Lens Run summaries |
| `MEMORYNEXUS_AI_BASE_URL` | `OPENAI_BASE_URL` or provider default | OpenAI-compatible API base URL |
| `LENS_RUN_SUMMARY_MAX_WORDS` | output-format based | Override Lens Run summary length |

## Run Locally

Start PostgreSQL and Qdrant:

```bash
docker compose up -d postgres qdrant
```

Run the API in one terminal with deterministic local embeddings:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_local
export MEMORYNEXUS_EMBEDDING_PROVIDER=local

cargo run --bin memorynexus
```

Optional: use an OpenAI-compatible provider for Lens Run summaries. For example,
OpenRouter exposes a compatible chat-completions API and may offer free-tier
models. Model availability changes, so check OpenRouter's model list before
choosing one:

```bash
export OPENROUTER_API_KEY=<your-openrouter-key>
export MEMORYNEXUS_SUMMARY_MODEL=openrouter/free
export LENS_RUN_SUMMARY_MAX_WORDS=120

cargo run --bin memorynexus
```

When `MEMORYNEXUS_SUMMARY_PROVIDER` is unset, `OPENROUTER_API_KEY` automatically
selects the OpenRouter provider and `https://openrouter.ai/api/v1` base URL. Set
`MEMORYNEXUS_SUMMARY_PROVIDER=openai` only if you intentionally want OpenAI
provider semantics with explicit base URL/model overrides.

If no summary key is configured, Lens Run still works with a deterministic
fallback summary.

Run the CLI in another terminal:

```bash
cargo run --bin memorynexus-cli -- health
```

## Cognitive Lens MVP Walkthrough

The examples below use `jq` to extract IDs from JSON responses. If `jq` is not
installed, copy the same values manually from each response.

```bash
export MEMORYNEXUS_API_URL=http://localhost:8080

AUTH_JSON=$(cargo run --quiet --bin memorynexus-cli -- auth register \
  --email "cli-smoke-$(date +%s)@example.com" \
  --name CliSmoke \
  --password secret123)

export MEMORYNEXUS_TOKEN=$(printf '%s' "$AUTH_JSON" | jq -r '.data.token')

cargo run --bin memorynexus-cli -- space list

SPACE_JSON=$(cargo run --quiet --bin memorynexus-cli -- space create \
  --name "CLI Smoke Space" \
  --description "Local CLI verification")

export MEMORYNEXUS_SPACE_ID=$(printf '%s' "$SPACE_JSON" | jq -r '.data.id')

cargo run --bin memorynexus-cli -- memory add \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --title "CLI smoke memory" \
  --content "MemoryNexus is a Rust-first cognitive lens memory system. Memory belongs to Cognitive Space, and Lens interprets one memory universe through many minds." \
  --tags "cli,smoke,lens"

cargo run --bin memorynexus-cli -- memory add \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --title "Phase 2 direction" \
  --content "Phase 2 turns Lens from configuration into a runnable interpretation strategy with provenance." \
  --tags "phase2,lens-run"

cargo run --bin memorynexus-cli -- memory list \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --limit 5

cargo run --bin memorynexus-cli -- search "cognitive lens" \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --limit 5

LENS_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --name "Project Context" \
  --description "Interpret project memory for planning" \
  --strategy project_context \
  --output brief \
  --retrieval semantic)

export MEMORYNEXUS_LENS_ID=$(printf '%s' "$LENS_JSON" | jq -r '.data.id')

cargo run --bin memorynexus-cli -- lens list \
  --space "$MEMORYNEXUS_SPACE_ID"

cargo run --bin memorynexus-cli -- lens get "$MEMORYNEXUS_LENS_ID"

RUN_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens run "$MEMORYNEXUS_LENS_ID" \
  --query "Summarize the current project direction" \
  --limit 5)

printf '%s\n' "$RUN_JSON" | jq

export MEMORYNEXUS_RUN_ID=$(printf '%s' "$RUN_JSON" | jq -r '.data.id')

cargo run --bin memorynexus-cli -- lens run get "$MEMORYNEXUS_RUN_ID"
```

Expected Lens Run signs:

- `data.status` is `completed`.
- `data.input_memory_ids` contains the memories used for the run.
- `data.output.query` echoes your query.
- `data.output.lens` records the Lens configuration used.
- `data.output.memories` contains the retrieved memory snippets.
- `data.output.summary` is AI-generated when `OPENAI_API_KEY` is configured; otherwise it is a deterministic fallback summary.
- `data.output.summary_provider`, `summary_model`, and `summary_fallback_reason` record summary provenance.

## Semantic Smoke Test

If you only want to test semantic recall without Lens Run:

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

## Example Lens Run Response

The exact IDs and timestamps will differ, but the shape should look like:

```json
{
  "ok": true,
  "data": {
    "id": "run-uuid",
    "lens_id": "lens-uuid",
    "space_id": "space-uuid",
    "query": "Summarize the current project direction",
    "input_memory_ids": ["memory-uuid"],
    "status": "completed",
    "output": {
      "lens": {
        "id": "lens-uuid",
        "name": "Project Context",
        "strategy": "project_context",
        "output_format": "brief",
        "retrieval_mode": "semantic"
      },
      "query": "Summarize the current project direction",
      "search_mode": "semantic",
      "memory_count": 1,
      "memories": [
        {
          "id": "memory-uuid",
          "title": "Phase 2 direction",
          "content": "Phase 2 turns Lens from configuration into a runnable interpretation strategy with provenance.",
          "memory_type": "text",
          "relevance": 0.83
        }
      ],
      "summary": "Lens 'Project Context' interpreted 1 memories for query 'Summarize the current project direction' using strategy 'project_context'.",
      "summary_provider": "deterministic",
      "summary_model": null,
      "summary_fallback_reason": "summary provider not configured"
    }
  }
}
```

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
