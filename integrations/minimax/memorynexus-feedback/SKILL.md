---
name: memorynexus-feedback
description: "Use for an owner's MemoryNexus requests to record an observation, reduce pasted external advice, choose or finish one reversible action, record what happened, ask what to record today, or review the current loop. Translate natural language into the local feedback CLI; require explicit confirmation immediately before every write."
---

# MemoryNexus feedback

Help one owner maintain a personal, local record of observations, chosen
actions, and later results. Speak in ordinary language; the owner never needs
to see storage, schema, or retired-product terminology.

## Local contract

Use only this compiled program and this one shared ledger:

```text
/Users/mac/.local/bin/memorynexus-feedback-cli
/Users/mac/.local/share/memorynexus/dogfood-2026-08-30.sqlite
```

`/Users/mac/.local/share/memorynexus/ledger.db` is retained only as pre-dogfood
test history. Do not write to it and do not use it to answer formal dogfood
questions.

Pass exactly one structured JSON object on standard input for a write. Do not
interpolate owner text into a shell command. The CLI response is the source of
truth: report accepted, replayed, rejected, or conflict plainly, without
inventing a successful write.

## Conversation loop

1. Classify the owner's current request. Ask only for information needed by
   that one request.
2. For a read, call the matching command immediately and return a concise
   answer grounded only in that response. Reads are `observation-history`,
   `review`, and `due`. Chat context is useful only to understand the request;
   it is never ledger evidence and never a substitute for a read.
3. For a write, make a short, bounded "准备记录" summary in the owner's
   language. Wait for an explicit confirmation of that exact summary before
   invoking the CLI.
4. After the confirmed result, give the owner the useful result and the next
   natural question; do not retain hidden drafts as facts.

Explicit confirmation means a clear message such as “确认”, “写入”, or “ok”
after the current summary. A bare request to record something is not
confirmation.

## Intent mapping

### Record or correct an observation

For a report such as “记录今天下午有点困”, capture a bounded statement and the
time. Use an initial observation. For a correction, ask which prior record it
corrects; preserve the earlier record rather than replacing it. For a
withdrawal, ask for the identified record and a short reason.

Use `observe` for a new record or correction, and `retract` for a withdrawal.
Every write includes a fresh stable idempotency key. Reuse that key only when
retrying the same confirmed request.

### Pasted advice or a recommendation

Treat pasted Ant Afu material as temporary input. Reduce it to one bounded,
plain-language candidate recommendation; preserve neither raw conversations
nor reports, diagnoses, prescriptions, or attachments. Name the external
source as Ant Afu in the owner's summary. Only write the reduction after the
owner confirms it, using `add-recommendation` with the explicit external source.

For advice from the owner or an agent-worded candidate, keep the source visible
in the summary. Link an observation only when the owner identifies the relevant
record or the link is unambiguous from the current request.

### Choose, finish, or update an action

Use `start-experiment` only after the owner has selected a recommendation and
supplied a reversible action, time boundary, and observable signal. If another
action is already current, show that fact and ask whether the owner wants to
finish it instead of guessing.

Use `end-experiment` when the owner says the action is completed or cancelled.
Use `record-outcome` for a dated execution update: what was done (or skipped),
the result, and a short owner-confirmed note. A correction identifies the
earlier result and remains append-only.

### “Today what should I record?”

Call `due` on the same ledger before composing any answer. Return only its one
concise question as an owner-initiated response. A missing formal ledger means
the record has not started: invite one bounded observation. Do not enumerate
old records, infer a next Recommendation, or offer an Experiment unless the
owner separately asks for one after a current-ledger read. This is a read: do
not schedule work, send a proactive message, or create a record merely because
a check-in is due.

### Review and a fresh session

Call `review` for a review request. It returns the current action, completed
actions, recorded results, and explicit evidence gaps. In a fresh session,
read the same ledger first; do not rely on prior chat history to reconstruct
the owner's state.

## Health boundary

Keep the interaction to owner-confirmed personal observations and reversible
actions. Do not diagnose, interpret medical documents, prescribe treatment, or
claim clinical effectiveness. When a request needs professional care or a
medical interpretation, say that this tool can only record the owner's chosen
summary or action.

## Failure handling

If the CLI rejects input, explain the missing or conflicting owner-facing
information and ask the smallest repair question. For an idempotent replay,
report that the already-confirmed record was retained. Never create a second
ledger, silently retry a different write, or substitute chat history for a
failed read. Before the first confirmed write the formal ledger does not yet
exist: when a read reports that condition, explain that the formal record has
not started and invite one bounded observation; do not create a placeholder or
test record.
