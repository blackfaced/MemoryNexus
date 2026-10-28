"""Capability regressions for the synthetic task-memory preflight."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import uuid


SCRIPT = Path(__file__).with_name("task-memory-smoke.py")
SPEC = importlib.util.spec_from_file_location("task_memory_smoke", SCRIPT)
SMOKE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SMOKE)

REQUIRED = [
    "init", "record", "context", "review", "correct", "retract",
    "export", "backup", "restore",
]


def help_response():
    return {
        "schema_version": 1, "status": "ok", "revision": 0,
        "result": {"command": "task", "commands": REQUIRED.copy()},
    }


class CapabilityTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def check_help(self, response, code=None, returncode=0):
        smoke = SMOKE.Smoke(Path("/unused-binary"), self.root)

        def init(index, action, request):
            self.assertEqual(action, "init")
            identity = str(uuid.uuid4())
            smoke.configs[index].write_text(json.dumps({"task_id": identity}))
            return {"task_id": identity}

        process = subprocess.CompletedProcess(
            [], returncode, json.dumps(response).encode(), b"",
        )
        with mock.patch.object(SMOKE.subprocess, "run", return_value=process), \
                mock.patch.object(smoke, "task", side_effect=init) as task:
            if code is None:
                smoke.check("schema_and_distinct_task_bindings", smoke.capabilities_and_binding)
                self.assertEqual(task.call_count, 2)
                self.assertEqual(len(set(smoke.identities.values())), 2)
                self.assertEqual(smoke.checks[0]["status"], "passed")
            else:
                with self.assertRaises(SMOKE.CheckFailed) as failure:
                    smoke.check("schema_and_distinct_task_bindings", smoke.capabilities_and_binding)
                self.assertEqual(str(failure.exception), code)
                task.assert_not_called()
                self.assertEqual(smoke.checks, [{
                    "name": "schema_and_distinct_task_bindings", "status": "failed", "code": code,
                }])

    def test_exact_required_commands_continue_to_distinct_bindings(self):
        self.check_help(help_response())

    def test_compatible_extensions_continue_to_distinct_bindings(self):
        response = help_response()
        response["result"]["commands"] += [
            "migrate", "workflow-bind", "source-sync", "source-capture",
            "consumer-review", "consumer-approve", "publication-result",
            "workflow-status", "read", "check", "future-command",
        ]
        self.check_help(response)

    def test_each_missing_required_command_fails(self):
        for missing in REQUIRED:
            with self.subTest(missing=missing):
                response = help_response()
                response["result"]["commands"].remove(missing)
                self.check_help(response, "missing_commands")
        response = help_response()
        response["result"]["commands"] = []
        self.check_help(response, "missing_commands")

    def test_commands_must_be_an_array(self):
        for commands in (None, " ".join(REQUIRED), {name: True for name in REQUIRED}, 9, True):
            with self.subTest(commands=commands):
                response = help_response()
                response["result"]["commands"] = commands
                self.check_help(response, "invalid_commands")
        response = help_response()
        del response["result"]["commands"]
        self.check_help(response, "invalid_commands")

    def test_malformed_entries_fail_even_with_all_required_commands(self):
        for entry in (None, 1, True, [], {}, "", " ", "source sync", "init\n", " init", "INIT", "-init"):
            with self.subTest(entry=entry):
                response = help_response()
                response["result"]["commands"].append(entry)
                self.check_help(response, "invalid_commands")

    def test_schema_must_be_integer_version_one(self):
        for schema in (None, 0, 2, "1", True, 1.0):
            with self.subTest(schema=schema):
                response = help_response()
                response["schema_version"] = schema
                self.check_help(response, "unsupported_response_schema")
        response = help_response()
        del response["schema_version"]
        self.check_help(response, "unsupported_response_schema")

    def test_wrong_or_missing_command_fails(self):
        for command in (None, "context", 1, ["task"]):
            with self.subTest(command=command):
                response = help_response()
                response["result"]["command"] = command
                self.check_help(response, "wrong_command")
        response = help_response()
        del response["result"]["command"]
        self.check_help(response, "wrong_command")

    def test_invalid_response_shape_and_result_fail(self):
        for response in (None, [], "task"):
            with self.subTest(response=response):
                self.check_help(response, "invalid_response_shape")
        for result in (None, [], "task"):
            with self.subTest(result=result):
                response = help_response()
                response["result"] = result
                self.check_help(response, "missing_result")

    def test_status_revision_and_exit_validation_remain_required(self):
        for field, value, code in (
            ("status", "error", "invalid_status"),
            ("status", None, "invalid_status"),
            ("revision", None, "missing_revision"),
            ("revision", True, "missing_revision"),
        ):
            with self.subTest(field=field, value=value):
                response = help_response()
                response[field] = value
                self.check_help(response, code)
        self.check_help(help_response(), "unexpected_cli_error", returncode=2)

    def test_public_script_failures_are_nonzero_machine_readable_reports(self):
        missing = help_response()
        missing["result"]["commands"].remove("restore")
        malformed = help_response()
        malformed["result"]["commands"].append({"name": "extension"})
        wrong_command = help_response()
        wrong_command["result"]["command"] = "context"
        wrong_schema = help_response()
        wrong_schema["schema_version"] = True
        for response, code in (
            (missing, "missing_commands"), (malformed, "invalid_commands"),
            (wrong_command, "wrong_command"), (wrong_schema, "unsupported_response_schema"),
        ):
            with self.subTest(code=code):
                binary = self.root / "fake-cli"
                binary.write_text("#!" + sys.executable + "\nprint(" + repr(json.dumps(response)) + ")\n")
                binary.chmod(0o700)
                process = subprocess.run(
                    [sys.executable, str(SCRIPT), "--binary", str(binary)],
                    capture_output=True, text=True, timeout=30,
                )
                self.assertEqual(process.returncode, 1, process.stderr)
                self.assertEqual(process.stderr, "")
                report = json.loads(process.stdout)
                self.assertEqual(report["status"], "failed")
                self.assertEqual(report["code"], code)
                self.assertEqual(report["mode"], "simulated_cli")
                self.assertEqual(report["real_client_acceptance"], "not_run")
                self.assertEqual(report["checks"], [{
                    "name": "schema_and_distinct_task_bindings", "status": "failed", "code": code,
                }])


if __name__ == "__main__":
    unittest.main()
