"""Public-process acceptance for the bootstrap Skill; synthetic data only."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

HELPER = Path(__file__).resolve().parents[1] / 'integrations/task-memory/memorynexus-task-bootstrap/scripts/bootstrap.py'
BINARY = os.environ.get('MEMORYNEXUS_TEST_BINARY')


class BootstrapTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name).resolve()
        self.task = self.home / 'task'
        self.task.mkdir()
        (self.task / 'README.md').write_text('A synthetic task.\n')
        self.state = self.home / 'seed'
        self.config = self.task / '.memorynexus.json'
        self.ledger = self.home / 'task.sqlite'
        self.draft = self.home / 'draft.json'
        self.data = {
            'schema_version': 1, 'task_root': str(self.task),
            'binding': {'config': str(self.config), 'ledger': str(self.ledger),
                        'display_name': 'Synthetic task', 'automatic_capture': ['execution', 'review']},
            'sources': ['README.md'], 'conflicts': [], 'gaps': ['Real clients not run'],
            'entries': [{'id': 'progress', 'kind': 'checkpoint', 'text': 'Ready for first step.', 'sources': ['README.md']}],
        }

    def run_helper(self, *args):
        result = subprocess.run([sys.executable, str(HELPER), *map(str, args)], capture_output=True, text=True)
        self.assertEqual(result.stderr, '', result.stderr)
        return result.returncode, json.loads(result.stdout)

    def preview(self):
        self.draft.write_text(json.dumps(self.data))
        return self.run_helper('preview', '--draft', self.draft, '--state-dir', self.state)

    def test_preview_needs_no_binary_or_ledger_and_exposes_exact_draft(self):
        code, result = self.preview()
        self.assertEqual(code, 0, result)
        self.assertFalse(self.config.exists())
        self.assertFalse(self.ledger.exists())
        self.assertEqual(sorted(p.name for p in self.task.iterdir()), ['README.md'])
        manifest = (self.state / 'manifest.json').read_bytes()
        self.assertEqual(hashlib.sha256(manifest).hexdigest(), result['manifest_sha256'])
        preview = (self.state / 'preview.md').read_text()
        for expected in ('Ready for first step.', 'execution_reported', 'Real clients not run', str(self.config), str(self.ledger), result['manifest_sha256']):
            self.assertIn(expected, preview)
        self.assertEqual(self.state.stat().st_mode & 0o777, 0o700)
        self.assertEqual((self.state / 'manifest.json').stat().st_mode & 0o777, 0o600)

    def apply(self, sha=None, binary=None, **extra):
        args = ['apply', '--state-dir', self.state, '--config', self.config, '--ledger', self.ledger,
                '--binary', binary or BINARY or '/no/binary']
        if sha is not None:
            args += ['--approve', sha]
        for key, value in extra.items():
            args += ['--' + key.replace('_', '-'), value]
        return self.run_helper(*args)

    def test_missing_or_wrong_approval_and_binding_do_not_write(self):
        code, result = self.preview()
        self.assertEqual(code, 0)
        for sha in (None, '0' * 64):
            code, result = self.apply(sha)
            self.assertEqual(code, 2)
            self.assertEqual(result['status'], 'error')
            self.assertFalse(self.config.exists())
            self.assertFalse(self.ledger.exists())
        self.assertEqual(sorted(p.name for p in self.state.iterdir()), ['manifest.json', 'preview.md'])

    def seed_entries(self):
        self.data['entries'] += [
            {'id': 'report', 'kind': 'execution', 'text': 'The example source was inspected.', 'sources': ['README.md']},
            {'id': 'rule', 'kind': 'instruction', 'text': 'Keep this synthetic task local.', 'sources': ['README.md']},
            {'id': 'candidate', 'kind': 'lesson', 'text': 'Inspect source before continuing.', 'source_entries': ['report', 'rule']},
        ]

    def context(self):
        result = subprocess.run([BINARY, 'task', '--config', str(self.config), 'context'], input='{}',
                                env={**os.environ, 'MEMORYNEXUS_TASK_LEDGER': str(self.ledger)},
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stdout)
        return json.loads(result.stdout)

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_approved_seed_writes_reports_confirmed_instruction_and_candidate(self):
        self.seed_entries()
        code, result = self.preview()
        self.assertEqual(code, 0, result)
        code, applied = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 0, applied)
        self.assertEqual(applied['status'], 'complete')
        self.assertEqual(len(applied['receipts']), 5)
        context = self.context()
        self.assertEqual(context['task_id'], applied['task_id'])
        records = context['result']['entries'] + context['result']['candidates']
        by_kind = {r['kind']: r for r in records}
        self.assertEqual(len(records), 4)
        self.assertEqual(by_kind['instruction']['trust'], 'owner_confirmed')
        self.assertEqual(by_kind['execution']['trust'], 'execution_reported')
        self.assertEqual(by_kind['checkpoint']['trust'], 'execution_reported')
        self.assertEqual(by_kind['lesson']['trust'], 'review_candidate')
        self.assertEqual(by_kind['lesson']['source_ids'], [by_kind['execution']['id'], by_kind['instruction']['id']])
        self.assertTrue(all(r['state'] == 'current' for r in records))
        self.assertFalse(list(self.state.glob('pending-*.json')))

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_exact_rerun_keeps_record_ids_and_has_no_new_attempts(self):
        self.seed_entries()
        _, result = self.preview()
        sha = result['manifest_sha256']
        code, first = self.apply(sha)
        self.assertEqual(code, 0, first)
        before = self.context()
        code, second = self.apply(sha)
        self.assertEqual(code, 0, second)
        self.assertEqual(second['attempted'], [])
        self.assertEqual(second['receipts'], first['receipts'])
        self.assertEqual(self.context(), before)

    def transport_wrapper(self, action, mode='malformed'):
        wrapper = self.home / 'cli-wrapper'
        marker = self.home / 'response-lost'
        wrapper.write_text('#!' + sys.executable + '\n' +
                          'import os, subprocess, sys, time\n' +
                          'from pathlib import Path\n' +
                          'data = sys.stdin.buffer.read()\n' +
                          'result = subprocess.run([' + repr(BINARY) + '] + sys.argv[1:], input=data, capture_output=True)\n' +
                          'marker = Path(' + repr(str(marker)) + ')\n' +
                          'if sys.argv[-1] == ' + repr(action) + ' and not marker.exists():\n' +
                          '    marker.write_text("lost")\n' +
                          ('    time.sleep(4)\n' if mode == 'timeout' else '    print("broken response")\n    sys.exit(0)\n') +
                          'sys.stdout.buffer.write(result.stdout)\nsys.exit(result.returncode)\n')
        wrapper.chmod(0o700)
        return wrapper

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_lost_write_response_resumes_same_envelope_without_duplicate(self):
        self.seed_entries()
        _, result = self.preview()
        wrapper = self.transport_wrapper('record')
        code, partial = self.apply(result['manifest_sha256'], binary=wrapper)
        self.assertEqual(code, 2, partial)
        self.assertEqual(partial['status'], 'pending')
        self.assertEqual(set(partial['receipts']), {'init'})
        pending = self.state / 'pending-progress.json'
        staged = pending.read_bytes()
        before_id = self.context()['result']['entries'][0]['id']
        code, done = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 0, done)
        self.assertEqual(done['attempted'][0], {'entry_id': 'progress', 'status': 'replayed'})
        self.assertEqual(done['receipts']['progress']['record_id'], before_id)
        self.assertEqual(done['receipts']['progress']['idempotency_key'], json.loads(staged)['idempotency_key'])
        self.assertFalse(pending.exists())
        self.assertEqual(len(self.context()['result']['entries']), 3)

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_lost_init_response_retains_identity_on_resume(self):
        _, result = self.preview()
        wrapper = self.transport_wrapper('init')
        code, partial = self.apply(result['manifest_sha256'], binary=wrapper)
        self.assertEqual(code, 2, partial)
        task_id = self.context()['task_id']
        self.assertTrue((self.state / 'pending-init.json').exists())
        code, done = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 0, done)
        self.assertEqual(done['task_id'], task_id)
        self.assertEqual(done['attempted'][0], {'entry_id': 'init', 'status': 'replayed'})

    def test_reserved_entry_id_is_rejected_without_creating_preview(self):
        self.data['entries'][0]['id'] = 'init'
        code, result = self.preview()
        self.assertEqual(code, 2, result)
        self.assertFalse(self.state.exists())

    def test_changed_source_binding_or_manifest_requires_new_approval(self):
        _, result = self.preview()
        sha = result['manifest_sha256']
        code, rejected = self.run_helper('apply', '--state-dir', self.state, '--approve', sha,
                                         '--config', self.config, '--ledger', self.home / 'other.sqlite', '--binary', BINARY or '/no/binary')
        self.assertEqual(code, 2, rejected)
        (self.task / 'README.md').write_text('Changed source.')
        code, rejected = self.apply(sha)
        self.assertEqual(code, 2, rejected)
        self.assertIn('snapshot changed', rejected['message'])
        manifest = self.state / 'manifest.json'
        manifest.write_bytes(manifest.read_bytes() + b' ')
        code, rejected = self.apply(sha)
        self.assertEqual(code, 2, rejected)
        self.assertIn('approval required', rejected['message'])
        self.assertFalse(self.config.exists())
        self.assertFalse(self.ledger.exists())

    def test_conflicts_bounds_and_candidate_sources_are_rejected(self):
        invalid = [
            [self.data['entries'][0]] * 13,
            [{'id': 'large', 'kind': 'execution', 'text': 'x' * 1001, 'sources': ['README.md']}],
            [{'id': 'candidate', 'kind': 'lesson', 'text': 'Guess', 'source_entries': ['missing']}],
        ]
        for entries in invalid:
            self.data['entries'] = entries
            code, rejected = self.preview()
            self.assertEqual(code, 2, rejected)
            self.assertFalse(self.state.exists())
        self.data['entries'] = [{'id': 'report', 'kind': 'execution', 'text': 'Bounded report', 'sources': ['README.md']}]
        self.data['conflicts'] = ['Unresolved current instruction conflict']
        _, result = self.preview()
        self.assertIn(self.data['conflicts'][0], (self.state / 'preview.md').read_text())
        code, rejected = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 2, rejected)
        self.assertIn('conflicts', rejected['message'])
        self.assertFalse(self.config.exists())

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_existing_task_cannot_be_seeded_or_have_checkpoint_replaced(self):
        request = {'idempotency_key': 'other-init', 'display_name': 'Other task',
                   'confirmation': 'confirmed', 'automatic_capture': ['execution', 'review']}
        for action, body in [('init', request), ('record', {'idempotency_key': 'other-progress', 'kind': 'checkpoint',
                             'text': 'Existing progress must remain.', 'run_id': 'a0455207-c500-45ab-ad4c-13c222467e65',
                             'client': 'other-client', 'occurred_at': '2026-10-21T00:00:00Z'})]:
            result = subprocess.run([BINARY, 'task', '--config', str(self.config), action], input=json.dumps(body),
                                    env={**os.environ, 'MEMORYNEXUS_TASK_LEDGER': str(self.ledger)}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout)
        before = self.context()
        _, result = self.preview()
        code, rejected = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 2, rejected)
        self.assertEqual(self.context(), before)

    def test_symlinked_sources_state_and_in_task_ledger_are_rejected(self):
        source = self.task / 'README.md'
        source.unlink()
        source.symlink_to(self.draft)
        code, rejected = self.preview()
        self.assertEqual(code, 2, rejected)
        source.unlink()
        source.write_text('Synthetic source')
        self.state.symlink_to(self.task, target_is_directory=True)
        code, rejected = self.preview()
        self.assertEqual(code, 2, rejected)
        self.state.unlink()
        self.data['binding']['ledger'] = str(self.task / 'ledger.sqlite')
        code, rejected = self.preview()
        self.assertEqual(code, 2, rejected)
        self.assertFalse(self.config.exists())

    def test_preview_refuses_overwrite_and_apply_refuses_permissive_state(self):
        _, result = self.preview()
        original = (self.state / 'manifest.json').read_bytes()
        code, rejected = self.preview()
        self.assertEqual(code, 2, rejected)
        self.assertEqual((self.state / 'manifest.json').read_bytes(), original)
        self.state.chmod(0o755)
        code, rejected = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 2, rejected)
        self.assertIn('owner-only', rejected['message'])
        self.assertFalse(self.config.exists())

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_timeout_preserves_request_and_source_drift_blocks_resume(self):
        _, result = self.preview()
        wrapper = self.transport_wrapper('record', mode='timeout')
        code, partial = self.apply(result['manifest_sha256'], binary=wrapper, timeout='2')
        self.assertEqual(code, 2, partial)
        self.assertEqual(partial['status'], 'pending')
        pending = self.state / 'pending-progress.json'
        original = pending.read_bytes()
        (self.task / 'README.md').write_text('Different source')
        code, rejected = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 2, rejected)
        self.assertEqual(pending.read_bytes(), original)
        self.assertEqual(len(self.context()['result']['entries']), 1)

    def test_hardlinked_manifest_is_refused_before_task_access(self):
        _, result = self.preview()
        os.link(self.state / 'manifest.json', self.home / 'linked-manifest')
        code, rejected = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 2, rejected)
        self.assertIn('hardlinked', rejected['message'])
        self.assertFalse(self.config.exists())

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_concurrent_apply_is_refused_while_first_apply_holds_lock(self):
        _, result = self.preview()
        wrapper = self.transport_wrapper('init', mode='timeout')
        command = [sys.executable, str(HELPER), 'apply', '--state-dir', str(self.state),
                   '--config', str(self.config), '--ledger', str(self.ledger),
                   '--binary', str(wrapper), '--approve', result['manifest_sha256']]
        first = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic() + 10
            while not (self.home / 'response-lost').exists() and time.monotonic() < deadline:
                time.sleep(0.01)
            self.assertTrue((self.home / 'response-lost').exists())
            code, second = self.apply(result['manifest_sha256'])
            self.assertEqual(code, 2, second)
            self.assertIn('another apply', second['message'])
            out, err = first.communicate(timeout=10)
            self.assertEqual(first.returncode, 0, (out, err))
            self.assertEqual(len(self.context()['result']['entries']), 1)
        finally:
            if first.poll() is None:
                first.kill()
                first.communicate()

    def test_git_snapshot_drift_blocks_even_when_selected_files_are_unchanged(self):
        for args in (['init', '-q'], ['add', 'README.md'], ['-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'synthetic']):
            subprocess.run(['git', '-C', str(self.task), *args], check=True, capture_output=True)
        _, result = self.preview()
        (self.task / 'new-plan.txt').write_text('New plan after review')
        code, rejected = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 2, rejected)
        self.assertIn('snapshot changed', rejected['message'])
        self.assertFalse(self.config.exists())

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_boolean_cli_schema_is_not_accepted_as_schema_one(self):
        _, result = self.preview()
        wrapper = self.transport_wrapper('record')
        script = wrapper.read_text().replace('import os, subprocess, sys, time', 'import json, os, subprocess, sys, time')
        script = script.replace('print("broken response")', 'body = json.loads(result.stdout); body["schema_version"] = True; print(json.dumps(body))')
        wrapper.write_text(script)
        code, rejected = self.apply(result['manifest_sha256'], binary=wrapper)
        self.assertEqual(code, 2, rejected)
        self.assertEqual(rejected['status'], 'pending')
        self.assertTrue((self.state / 'pending-progress.json').exists())
        code, recovered = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 0, recovered)
        self.assertEqual(recovered['attempted'][0]['status'], 'replayed')

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_known_launch_failure_can_retry_first_init_without_replacing_a_ledger(self):
        _, result = self.preview()
        broken = self.home / 'bad-executable'
        broken.write_text('This is not an executable format.\n')
        broken.chmod(0o700)
        code, partial = self.apply(result['manifest_sha256'], binary=broken)
        self.assertEqual(code, 2, partial)
        self.assertEqual(partial['status'], 'pending')
        original = (self.state / 'pending-init.json').read_bytes()
        self.assertFalse(self.ledger.exists())
        code, done = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 0, done)
        self.assertEqual(done['receipts']['init']['idempotency_key'], json.loads(original)['idempotency_key'])

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_unsafe_ledger_sidecar_is_rejected_before_any_cli_write(self):
        _, result = self.preview()
        Path(str(self.ledger) + '-wal').symlink_to(self.task / 'README.md')
        code, rejected = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 2, rejected)
        self.assertFalse(self.config.exists())
        self.assertFalse(self.ledger.exists())
        self.assertEqual((self.task / 'README.md').read_text(), 'A synthetic task.\n')

    def test_non_regular_manifest_is_rejected_without_waiting_for_input(self):
        _, result = self.preview()
        manifest = self.state / 'manifest.json'
        manifest.unlink()
        os.mkfifo(manifest, 0o600)
        process = subprocess.run([sys.executable, str(HELPER), 'apply', '--state-dir', str(self.state),
                                  '--config', str(self.config), '--ledger', str(self.ledger), '--binary', BINARY or '/no/binary',
                                  '--approve', result['manifest_sha256']], text=True, capture_output=True, timeout=1)
        self.assertEqual(process.returncode, 2, process.stdout)
        self.assertEqual(json.loads(process.stdout)['status'], 'error')
        self.assertFalse(self.config.exists())

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_malformed_context_does_not_report_success_or_leak_traceback(self):
        _, result = self.preview()
        wrapper = self.home / 'context-wrapper'
        wrapper.write_text('#!' + sys.executable + '\n' +
                          'import json, subprocess, sys\n' +
                          'result = subprocess.run([' + repr(BINARY) + '] + sys.argv[1:], input=sys.stdin.buffer.read(), capture_output=True)\n' +
                          'body = json.loads(result.stdout)\n' +
                          'if sys.argv[-1] == "context" and body["result"].get("entries"):\n' +
                          '    body["result"]["entries"][0].pop("id")\n' +
                          'print(json.dumps(body))\nsys.exit(result.returncode)\n')
        wrapper.chmod(0o700)
        code, partial = self.apply(result['manifest_sha256'], binary=wrapper)
        self.assertEqual(code, 2, partial)
        self.assertEqual(partial['status'], 'pending')
        code, done = self.apply(result['manifest_sha256'])
        self.assertEqual(code, 0, done)
        self.assertEqual(done['attempted'], [])

    @unittest.skipUnless(BINARY, 'set MEMORYNEXUS_TEST_BINARY to compiled CLI')
    def test_hardlinked_compiled_cli_runs_but_symlinked_cli_is_refused(self):
        _, result = self.preview()
        executable = self.home / 'compiled-cli'
        executable.symlink_to(BINARY)
        code, rejected = self.apply(result['manifest_sha256'], binary=executable)
        self.assertEqual(code, 2, rejected)
        self.assertIn('symlink', rejected['message'])
        self.assertFalse(self.config.exists())
        self.assertFalse(self.ledger.exists())
        executable.unlink()
        os.link(BINARY, executable)
        self.assertGreater(executable.stat().st_nlink, 1)
        code, done = self.apply(result['manifest_sha256'], binary=executable)
        self.assertEqual(code, 0, done)
        self.assertEqual(done['status'], 'complete')
        self.assertEqual(len(self.context()['result']['entries']), 1)


if __name__ == '__main__':
    unittest.main()
