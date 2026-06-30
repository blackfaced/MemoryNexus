# Minimal Dictation Agent Demo

This demo proves that a one-learner Dictation Coach loop can run through the
generic Surface Gateway MCP tools without a web UI or dedicated product app.

The demo is an Adapter orchestration pattern. Dictation product wording stays
in the agent prompt and payload semantics; MemoryNexus still receives generic
Surface requests:

| Agent action | MCP tool | Surface action |
| --- | --- | --- |
| Record today's list | `surface_capture_observation` | Capture / `capture_observation` |
| Submit a dictation result | `surface_submit_attempt` | Performance / `submit_attempt` |
| Explain mistake patterns | `surface_review_evidence` | Reflection / `review_evidence` |
| Prepare tomorrow's focused practice | `surface_generate_next_task` | Planning / `generate_next_task` |
| Show a recent trend | `surface_get_state_summary` | Observation / `get_state_summary` |

The initial smoke path is text-first. Use `source: "typed"` or
`source: "pasted"` and do not include media-only fields such as
`evidence_refs`, `input_confirmation`, `locator`, `provider`, `metadata`,
`transcript`, or `transcript_source`.

## Deterministic Smoke Fixture

The automated smoke test uses
`tests/fixtures/dictation_agent/minimal_english_spelling_demo.json` and verifies
the generated MCP tool calls. It checks:

- namespace: `child.english.spelling`;
- actor;
- `payload.space_id`;
- typed or pasted sources only;
- deterministic context with `mode` and `runtime_preference`;
- no media-only fields in the initial path;
- generated Surface provenance such as `generated_trace_id`.

Run it with:

```bash
cargo test --test dictation_agent_smoke_test
```

## Live MCP Sequence

Start the MemoryNexus API and export a valid token as documented in
[MCP Server](mcp.md). Then send the five generic Surface tool calls through the
MCP stdio server:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"surface_capture_observation","arguments":{"namespace":"child.english.spelling","actor":"00000000-0000-0000-0000-000000000001","payload":{"space_id":"22222222-2222-2222-2222-222222222222","source":"typed","task_kind":"english_spelling","title":"Tuesday spelling list","prompt_items":[{"id":"word-1","item_kind":"english_word","expected_text":"because","hint":"reason"},{"id":"word-2","item_kind":"english_word","expected_text":"friend"}]},"context":{"mode":"fast","locale":"en-US","device":"agent","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"surface_submit_attempt","arguments":{"namespace":"child.english.spelling","actor":"00000000-0000-0000-0000-000000000001","payload":{"space_id":"22222222-2222-2222-2222-222222222222","source":"pasted","task_id":"task-demo-0001","submitted_items":[{"prompt_item_id":"word-1","actual_text":"becuase"},{"prompt_item_id":"word-2","actual_text":"friend"}]},"context":{"mode":"fast","locale":"en-US","device":"agent","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"surface_review_evidence","arguments":{"namespace":"child.english.spelling","actor":"00000000-0000-0000-0000-000000000001","payload":{"space_id":"22222222-2222-2222-2222-222222222222","attempt_id":"attempt-demo-0001","evaluation_id":"evaluation-demo-0001","timeframe":"today","question":"Explain the spelling mistake pattern from today'\''s dictation result"},"context":{"mode":"focused","locale":"en-US","device":"agent","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"surface_generate_next_task","arguments":{"namespace":"child.english.spelling","actor":"00000000-0000-0000-0000-000000000001","payload":{"space_id":"22222222-2222-2222-2222-222222222222","target_date":"2026-07-01","duration_minutes":10,"objective":"Prepare tomorrow'\''s focused 10-minute spelling practice"},"context":{"mode":"focused","locale":"en-US","device":"agent","runtime_preference":"deterministic"}}}}' \
  '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"surface_get_state_summary","arguments":{"namespace":"child.english.spelling","actor":"00000000-0000-0000-0000-000000000001","payload":{"space_id":"22222222-2222-2222-2222-222222222222","timeframe":"7d","summary_goal":"Show recent dictation trend and current focus"},"context":{"mode":"focused","locale":"en-US","device":"agent","runtime_preference":"deterministic"}}}}' \
  | MEMORYNEXUS_TOKEN="$MEMORYNEXUS_TOKEN" cargo run --quiet --bin memorynexus-mcp
```

Replace the actor and space IDs with IDs from the authenticated local setup for
a real run. The smoke fixture keeps fixed IDs only so request mapping stays
readable and deterministic.

## Media Prompt Policy

Media remains an Adapter concern. If an agent reads a worksheet image or audio
clip, it must first prepare normalized text outside MemoryNexus and ask the user
to accept or correct that text.

Accepted normalized text maps to:

```json
{
  "input_confirmation": {
    "status": "confirmed",
    "method": "explicit_acceptance"
  }
}
```

Corrected normalized text maps to:

```json
{
  "input_confirmation": {
    "status": "confirmed",
    "method": "explicit_correction"
  }
}
```

Optional media descriptors may be sent only through confirmed media-derived
Capture or Performance calls. This demo does not implement OCR, ASR, media
resolution, descriptor persistence, or evidence storage. Descriptors are
ephemeral request provenance until a separate persistence issue lands.
