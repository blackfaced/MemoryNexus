#!/usr/bin/env python3
"""Synthetic public-CLI preflight. Never invokes a model or a real task ledger."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tempfile
import uuid


class CheckFailed(Exception):
    """A content-free failure suitable for the public preflight report."""


def require(condition, code):
    if not condition:
        raise CheckFailed(code)


def envelope(client, text, **fields):
    return {
        "idempotency_key": str(uuid.uuid4()),
        "run_id": str(uuid.uuid4()),
        "client": client,
        "text": text,
        "occurred_at": "2026-10-21T10:00:00+08:00",
        **fields,
    }


class Smoke:
    def __init__(self, binary, root):
        self.binary = binary
        self.root = root
        self.ledger = root / "task.sqlite"
        self.configs = [root / "task-a.json", root / "task-b.json"]
        self.identities = {}
        self.checks = []

    def invoke(self, args, request=None, ledger=None, error=None):
        env = os.environ.copy()
        env.pop("MEMORYNEXUS_TASK_LEDGER", None)
        if ledger is not None:
            env["MEMORYNEXUS_TASK_LEDGER"] = str(ledger)
        data = request if isinstance(request, bytes) else json.dumps(request or {}).encode()
        try:
            result = subprocess.run(
                [str(self.binary), "task", *map(str, args)],
                input=data, capture_output=True, env=env, timeout=30,
            )
        except subprocess.TimeoutExpired:
            raise CheckFailed("cli_timeout") from None
        except OSError:
            raise CheckFailed("cli_launch_failed") from None
        try:
            response = json.loads(result.stdout)
        except (ValueError, UnicodeError):
            raise CheckFailed("invalid_json_response") from None
        require(isinstance(response, dict), "invalid_response_shape")
        require(type(response.get("schema_version")) is int and response["schema_version"] == 1,
                "unsupported_response_schema")
        if error:
            require(result.returncode == 2 and response.get("status") == "error", "expected_error")
            require(response.get("code") == error, "wrong_error_code")
        else:
            require(result.returncode == 0, "unexpected_cli_error")
            require(response.get("status") in ("accepted", "replayed", "ok"), "invalid_status")
            require(isinstance(response.get("result"), dict), "missing_result")
            require(type(response.get("revision")) is int, "missing_revision")
        return response

    def task(self, index, action, request=None, error=None, ledger=None):
        result = self.invoke(
            ["--config", self.configs[index], action], request,
            ledger=self.ledger if ledger is None else ledger, error=error,
        )
        if not error and index in self.identities:
            require(result.get("task_id") == self.identities[index], "task_identity_mismatch")
        return result

    def recovery(self, action, request=None, ledger=None, error=None):
        return self.invoke(["--ledger", ledger or self.ledger, action], request, error=error)

    def check(self, name, operation):
        try:
            operation()
        except CheckFailed as failure:
            self.checks.append({"name": name, "status": "failed", "code": str(failure)})
            raise
        self.checks.append({"name": name, "status": "passed"})

    def capabilities_and_binding(self):
        help_result = self.invoke(["--help"])
        require(help_result["result"].get("command") == "task", "wrong_command")
        commands = help_result["result"].get("commands")
        require(isinstance(commands, list) and all(
            isinstance(command, str) and re.fullmatch(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*", command)
            for command in commands
        ), "invalid_commands")
        require({
            "init", "record", "context", "review", "correct", "retract",
            "export", "backup", "restore",
        }.issubset(commands), "missing_commands")
        for index in (0, 1):
            response = self.task(index, "init", {
                "idempotency_key": str(uuid.uuid4()), "display_name": "Synthetic task",
                "confirmation": "confirmed", "automatic_capture": ["execution", "review"],
            })
            binding = json.loads(self.configs[index].read_bytes())
            require(binding["task_id"] == response.get("task_id"), "config_identity_mismatch")
            self.identities[index] = binding["task_id"]
        require(self.identities[0] != self.identities[1], "duplicate_task_identity")

    def reverse_writes_and_isolation(self):
        self.checkpoint = self.task(0, "record", envelope(
            "minimax", "演示：素材编号完成。", kind="checkpoint",
        ))["result"]["record_id"]
        context = self.task(0, "context")
        require(context["result"]["entries"][0]["id"] == self.checkpoint, "forward_readback")
        self.execution_request = envelope("codex", "Synthetic: selected opening clip.", kind="execution")
        self.execution = self.task(0, "record", self.execution_request)["result"]["record_id"]
        entries = {row["id"]: row for row in self.task(0, "context")["result"]["entries"]}
        require(entries[self.checkpoint]["client"] == "minimax", "forward_provenance")
        require(entries[self.execution]["client"] == "codex", "reverse_provenance")
        limited = self.task(0, "context", {"limit": 1})["result"]
        require(len(limited["entries"]) == 1 and limited["omitted_count"] == 1 and limited["truncated"], "bounded_context_gap_missing")
        isolated = self.task(1, "context")["result"]
        require(isolated["entries"] == [] and isolated["candidates"] == [], "task_leak")
        self.task(1, "context", {"record_ids": [self.execution]}, error="scope_mismatch")

    def pending_retry_and_failures(self):
        pending_dir = self.root / "pending"
        pending_dir.mkdir(mode=0o700)
        pending = pending_dir / (str(uuid.uuid4()) + ".json")
        request = envelope("minimax", "Synthetic: pending execution.", kind="execution")
        payload = json.dumps(request, ensure_ascii=False).encode()
        with os.fdopen(os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        require(stat.S_IMODE(pending.stat().st_mode) == 0o600, "pending_permissions")
        self.invoke(["--config", self.configs[0], "record"], pending.read_bytes(), error="invalid_input")
        require(pending.read_bytes() == payload, "failed_write_lost_pending")
        missing = self.root / "missing.sqlite"
        self.task(0, "context", ledger=missing, error="not_found")
        require(not missing.exists(), "substitute_ledger_created")
        accepted = self.task(0, "record", pending.read_bytes())
        # Deliberately retain the request to simulate losing a successful response.
        require(accepted["status"] == "accepted", "expected_first_accept")
        replay = self.task(0, "record", pending.read_bytes())
        require(replay["status"] == "replayed", "expected_replay")
        require(replay["result"] == accepted["result"], "replay_identity_changed")
        require(replay["result"]["state"] == "current", "unexpected_write_state")
        uuid.UUID(replay["result"]["record_id"])
        pending.unlink()  # Only after exit/status/schema/task/record validation above.
        before = self.task(0, "context")
        self.task(0, "record", {**request, "text": "Changed same-key content."}, error="conflict")
        require(self.task(0, "context") == before, "conflict_changed_state")
        require(sum(row["id"] == replay["result"]["record_id"] for row in before["result"]["entries"]) == 1, "duplicate_replay")
        require(not pending.exists(), "successful_pending_not_cleaned")

    def review_correction_and_withdrawal(self):
        review = self.task(0, "review", envelope(
            "minimax", "Synthetic candidate: use the selected clip.", source_ids=[self.execution],
        ))["result"]["record_id"]
        context = self.task(0, "context")["result"]
        require([row["id"] for row in context["candidates"]] == [review], "candidate_not_separate")
        candidate = context["candidates"][0]
        require(candidate["trust"] == "review_candidate" and candidate["source_ids"] == [self.execution], "candidate_source_or_trust")
        self.correction_request = envelope(
            "codex", "Synthetic correction: selection still needs approval.",
            target_id=self.execution, confirmation="confirmed",
        )
        self.corrected = self.task(0, "correct", self.correction_request)["result"]["record_id"]
        context = self.task(0, "context")["result"]
        require(context["candidates"] == [], "invalidated_candidate_visible")
        require(self.execution not in [row["id"] for row in context["entries"]], "old_source_visible")
        marker = self.task(0, "context", {"record_ids": [review]})["result"]["candidates"][0]
        require(marker["state"] == "stale" and "text" not in marker and "evidence_refs" not in marker, "stale_marker_leaks_content")
        fresh = self.task(0, "review", envelope(
            "minimax", "Synthetic candidate: request approval before editing.", source_ids=[self.corrected],
        ))["result"]["record_id"]
        endorsed = self.task(0, "correct", envelope(
            "codex", "Synthetic owner-endorsed lesson: request approval.",
            target_id=fresh, confirmation="confirmed",
        ))["result"]["record_id"]
        entries = self.task(0, "context")["result"]["entries"]
        require(any(row["id"] == endorsed and row["trust"] == "owner_confirmed" for row in entries), "endorsement_missing")
        self.task(0, "retract", {
            "idempotency_key": str(uuid.uuid4()), "run_id": str(uuid.uuid4()), "client": "codex",
            "target_id": self.execution, "reason": "Synthetic withdrawal.", "confirmation": "confirmed",
        })
        current = self.task(0, "context")["result"]
        require(not {self.execution, self.corrected, endorsed}.intersection(row["id"] for row in current["entries"]), "withdrawn_or_stale_entry_visible")
        chain = self.task(0, "context", {"record_ids": [self.execution, self.corrected, endorsed]})["result"]["entries"]
        require({row["state"] for row in chain} == {"retracted", "stale"}, "withdrawal_chain_state")
        require(all("text" not in row and "evidence_refs" not in row for row in chain), "withdrawal_marker_leaks_content")
        exported = self.recovery("export")["result"]
        withdrawn = [row for row in exported["records"] if row["id"] in (self.execution, self.corrected)]
        require(len(withdrawn) == 2 and all("text" not in row and "evidence_refs" not in row for row in withdrawn), "withdrawal_export_leaks_content")

    def backup_restore(self):
        backup, restored = self.root / "backup.sqlite", self.root / "restored.sqlite"
        before = [self.task(index, "context") for index in (0, 1)]
        exported = self.recovery("export")
        result = self.recovery("backup", {"path": str(backup)})["result"]
        require(result["format"] == "memorynexus-task-ledger" and result["version"] == 1, "backup_format")
        self.recovery("restore", {"path": str(backup)}, ledger=restored)
        require(self.recovery("export", ledger=restored) == exported, "restored_state_changed")
        require([self.task(index, "context", ledger=restored) for index in (0, 1)] == before, "restored_context_changed")
        for action, request, identity in (
            ("record", self.execution_request, self.execution),
            ("correct", self.correction_request, self.corrected),
        ):
            replay = self.task(0, action, request, ledger=restored)
            require(replay["status"] == "replayed" and replay["result"] == {"record_id": identity, "state": "retracted"}, "restored_receipt_changed")
            self.task(0, action, {**request, "text": "Changed after restore."}, ledger=restored, error="conflict")
        self.recovery("restore", {"path": str(backup)}, ledger=restored, error="conflict")
        require(self.recovery("export", ledger=restored) == exported, "restore_retry_changed_state")

    def run(self):
        self.check("schema_and_distinct_task_bindings", self.capabilities_and_binding)
        self.check("fresh_process_reverse_writes_and_scope", self.reverse_writes_and_isolation)
        self.check("pending_exact_retry_conflict_and_missing_history", self.pending_retry_and_failures)
        self.check("sourced_review_correction_and_withdrawal", self.review_correction_and_withdrawal)
        self.check("backup_restore_identity_state_and_receipts", self.backup_restore)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, help="Explicit compiled memorynexus-feedback-cli path")
    args = parser.parse_args()
    report = {"schema_version": 1, "mode": "simulated_cli", "real_client_acceptance": "not_run", "checks": []}
    smoke = None
    try:
        binary = Path(args.binary).resolve(strict=True)
        require(binary.is_file() and os.access(binary, os.X_OK), "binary_not_executable")
        report["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
        with tempfile.TemporaryDirectory(prefix="memorynexus-task-smoke-") as directory:
            smoke = Smoke(binary, Path(directory))
            smoke.run()
    except CheckFailed as failure:
        report["status"] = "failed"
        report["code"] = str(failure)
    except (OSError, ValueError, KeyError, TypeError, IndexError):
        report["status"] = "failed"
        report["code"] = "preflight_environment_or_response_error"
    else:
        report["status"] = "passed"
    if smoke is not None:
        report["checks"] = smoke.checks
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
