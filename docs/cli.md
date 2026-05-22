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
| `MEMORYNEXUS_SUMMARY_PROVIDER` | inferred from keys, else `openai` | Use `openai`, `openrouter`, or `none` |
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
cargo run --bin memorynexus-cli -- config
```

`config` reads the running API process configuration. Use it to confirm which
embedding and summary providers the API actually started with.

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

cargo run --bin memorynexus-cli -- search "cognitive lens" \
  --lens "$MEMORYNEXUS_LENS_ID" \
  --limit 5

RUN_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens run "$MEMORYNEXUS_LENS_ID" \
  --query "Summarize the current project direction" \
  --limit 5)

printf '%s\n' "$RUN_JSON" | jq

export MEMORYNEXUS_RUN_ID=$(printf '%s' "$RUN_JSON" | jq -r '.data.id')

cargo run --bin memorynexus-cli -- lens run get "$MEMORYNEXUS_RUN_ID"

cargo run --bin memorynexus-cli -- lens run list \
  --lens "$MEMORYNEXUS_LENS_ID" \
  --limit 5
```

Expected Lens Run signs:

- `data.status` is `completed`.
- `data.input_memory_ids` contains the memories used for the run.
- `data.output.query` echoes your query.
- `data.output.lens` records the Lens configuration used.
- `data.output.memories` contains the retrieved memory snippets.
- `data.output.key_points`, `open_questions`, `suggested_next_actions`, and
  `citations` provide the structured MVP interpretation.
- `data.output.summary` is AI-generated when a summary provider is configured; otherwise it is a deterministic fallback summary.
- `data.output.summary_provider`, `summary_source`, `summary_model`, and `summary_fallback_reason` record summary provenance.

## Lens Templates

Built-in Lens templates provide reusable interpretation defaults. They are local
CLI templates that expand into the same Lens create API payload.

List templates:

```bash
cargo run --bin memorynexus-cli -- lens templates | jq
```

Create a Lens from a template:

```bash
LENS_JSON=$(cargo run --quiet --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --template project_context)

export MEMORYNEXUS_LENS_ID=$(printf '%s' "$LENS_JSON" | jq -r '.data.id')
```

Available templates:

| Template | Strategy | Output | Use |
|----------|----------|--------|-----|
| `project_context` | `project_context` | `brief` | Planning and project direction |
| `learning_review` | `learning_review` | `bullets` | Learning progress, gaps, and next steps |
| `family_growth` | `family_growth` | `brief` | Family growth moments and continuity |
| `risk_review` | `risk_review` | `bullets` | Risks, contradictions, and unresolved concerns |

You can override template fields:

```bash
cargo run --bin memorynexus-cli -- lens create \
  --space "$MEMORYNEXUS_SPACE_ID" \
  --template risk_review \
  --name "Launch Risk Review" \
  --output brief
```

## IDs And Space Boundary

`SPACE_ID` is the UUID primary key of a Cognitive Space. It looks like:

```text
24f7166f-3a6f-475b-a409-bd11a00a5734
```

A Cognitive Space is the memory universe where a command operates. MemoryNexus
does not make an agent own memory; memories, lenses, searches, and Lens Runs are
scoped to a Cognitive Space.

Common ways to get a `SPACE_ID`:

```bash
SPACE_JSON=$(cargo run --quiet --bin memorynexus-cli -- space create \
  --name "Project Space" \
  --description "MemoryNexus project notes")

export MEMORYNEXUS_SPACE_ID=$(printf '%s' "$SPACE_JSON" | jq -r '.data.id')
```

or list existing spaces:

```bash
cargo run --quiet --bin memorynexus-cli -- space list | jq
export MEMORYNEXUS_SPACE_ID='<id-from-space-list>'
```

Use the same `SPACE_ID` when adding memories, searching, creating lenses, and
running lenses:

```bash
cargo run --bin memorynexus-cli -- memory add --space "$MEMORYNEXUS_SPACE_ID" --content "..."
cargo run --bin memorynexus-cli -- search "..." --space "$MEMORYNEXUS_SPACE_ID"
cargo run --bin memorynexus-cli -- lens create --space "$MEMORYNEXUS_SPACE_ID" --name "Project Context"
```

`LENS_ID` is the UUID returned by `lens create` or `lens list`. `RUN_ID` is the
UUID returned by `lens run`.

Search can also use a Lens directly:

```bash
cargo run --bin memorynexus-cli -- search "project direction" \
  --lens "$MEMORYNEXUS_LENS_ID" \
  --limit 5
```

When `--lens` is used, the API searches inside the Lens's Cognitive Space and
returns Lens provenance in `data.lens`.

## Lens Run Troubleshooting

### API Process Does Not See Provider Keys

The API process reads provider environment variables only when it starts. Setting
`OPENROUTER_API_KEY` in a different CLI terminal does not update an already
running API process.

Check the API-visible configuration:

```bash
cargo run --bin memorynexus-cli -- config | jq
```

For OpenRouter, a configured API should report `summary_enabled=true`,
`summary_provider=openrouter`, and the expected `summary_model`.

In the API terminal, stop the server and restart it with:

```bash
export QDRANT_URL=http://localhost:6333
export QDRANT_COLLECTION=memorynexus_openrouter_smoke
export MEMORYNEXUS_EMBEDDING_PROVIDER=local
export OPENROUTER_API_KEY='sk-or-v1-your-real-key'
export MEMORYNEXUS_SUMMARY_MODEL=openrouter/free
export LENS_RUN_SUMMARY_MAX_WORDS=120

printf 'OPENROUTER_API_KEY length: %s\n' "${#OPENROUTER_API_KEY}"

cargo run --bin memorynexus
```

The key length should be far greater than the length of placeholder text such as
`<your-key>`. Do not commit keys to the repository.

### Verify OpenRouter Outside MemoryNexus

Before debugging Lens Run, verify the key can call OpenRouter directly:

```bash
curl -sS https://openrouter.ai/api/v1/chat/completions \
  -H "Authorization: Bearer $OPENROUTER_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"openrouter/free","messages":[{"role":"user","content":"Say hi in Chinese."}],"max_tokens":80}' | jq
```

If this returns `401`, fix the API key or shell environment before testing
MemoryNexus.

### Read Summary Provenance

Inspect Lens Run summary provenance with:

```bash
printf '%s\n' "$RUN_JSON" | jq '{
  status: .data.status,
  provider: .data.output.summary_provider,
  source: .data.output.summary_source,
  model: .data.output.summary_model,
  fallback: .data.output.summary_fallback_reason,
  summary: .data.output.summary
}'
```

Expected successful AI summary:

```json
{
  "status": "completed",
  "provider": "openrouter",
  "source": "ai",
  "model": "openrouter/free",
  "fallback": null
}
```

Useful fallback patterns:

| Output | Meaning | Fix |
|--------|---------|-----|
| `summary_provider=deterministic`, `summary_fallback_reason=summary provider not configured` | API started without a summary key | Restart API with `OPENAI_API_KEY` or `OPENROUTER_API_KEY` |
| `summary_provider=openrouter`, `summary_source=deterministic`, `fallback` contains `401` | API reached OpenRouter but auth failed | Check the key in the API terminal and restart |
| `summary_provider=openrouter`, `summary_source=deterministic`, `fallback=summary provider returned empty output` | Provider returned HTTP 2xx but no parseable text | Try another `:free` model or increase `max_tokens` in code later |

OpenRouter's free router can route to reasoning models where useful text may be
in `message.reasoning` instead of `message.content`; MemoryNexus handles both
forms.

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
      "key_points": [
        {
          "memory_id": "memory-uuid",
          "title": "Phase 2 direction",
          "point": "Phase 2 turns Lens from configuration into a runnable interpretation strategy with provenance"
        }
      ],
      "open_questions": [
        "What additional memories would make this interpretation more reliable?"
      ],
      "suggested_next_actions": [
        "Review the cited memories before acting on this interpretation.",
        "Run another Lens if you need a different perspective."
      ],
      "citations": [
        {
          "memory_id": "memory-uuid",
          "title": "Phase 2 direction",
          "relevance": 0.83
        }
      ],
      "summary": "Lens 'Project Context' interpreted 1 memories for query 'Summarize the current project direction' using strategy 'project_context'.",
      "summary_provider": "deterministic",
      "summary_source": "deterministic",
      "summary_model": null,
      "summary_fallback_reason": "summary provider not configured"
    }
  }
}
```

## Commands

```bash
memorynexus-cli health
memorynexus-cli config

memorynexus-cli auth register --email <EMAIL> --name <NAME> --password <PASSWORD>
memorynexus-cli auth login --email <EMAIL> --password <PASSWORD>

memorynexus-cli space create --name <NAME> [--description <TEXT>]
memorynexus-cli space list

memorynexus-cli lens create --space <SPACE_ID> --name <NAME> \
  [--description <TEXT>] [--strategy <NAME>] [--output <FORMAT>] [--retrieval <MODE>]
memorynexus-cli lens create --space <SPACE_ID> --template <TEMPLATE_ID> \
  [--name <NAME>] [--description <TEXT>] [--strategy <NAME>] [--output <FORMAT>] [--retrieval <MODE>]
memorynexus-cli lens templates
memorynexus-cli lens list --space <SPACE_ID>
memorynexus-cli lens get <LENS_ID>
memorynexus-cli lens run <LENS_ID> --query <TEXT> [--limit <N>]
memorynexus-cli lens run get <RUN_ID>
memorynexus-cli lens run list [--lens <LENS_ID>] [--space <SPACE_ID>] [--limit <N>]

memorynexus-cli memory add --content <TEXT> [--space <SPACE_ID>] [--title <TEXT>] \
  [--tags <COMMA_SEPARATED_TAGS>] [--type text|image|audio|video] [--shared]
memorynexus-cli memory list [--space <SPACE_ID>] [--limit <N>] [--offset <N>]
memorynexus-cli memory get <MEMORY_ID>
memorynexus-cli memory delete <MEMORY_ID>

memorynexus-cli search <QUERY> [--space <SPACE_ID>] [--lens <LENS_ID>] [--semantic] [--limit <N>]
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
