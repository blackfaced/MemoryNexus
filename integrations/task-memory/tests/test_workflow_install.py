"""Versioned installation through public commands, using a real compiled CLI."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from workflow_fixture import Fixture, BINARY, REPO
from workflow_types import digest

INSTALLER = REPO / 'integrations/task-memory/install.py'
PYTHON = str(Path(sys.executable).resolve())


class Installation(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source_temp = tempfile.TemporaryDirectory(prefix='mn-install-source-', dir=Path(tempfile.gettempdir()).resolve())
        cls.source = Path(cls.source_temp.name)
        integration = cls.source / 'integrations/task-memory'
        integration.mkdir(parents=True)
        original = REPO / 'integrations/task-memory'
        for name in ('scripts', 'memorynexus-task-bootstrap', 'memorynexus-repo-onboard', 'memorynexus-repository-review'):
            shutil.copytree(original / name, integration / name, ignore=shutil.ignore_patterns('__pycache__'))
        for name in ('PROTOCOL.md', 'launch.py', 'codex_bridge.py', 'install_codex_bridge.py'):
            shutil.copyfile(original / name, integration / name)
        for name in ('integrations/minimax/memorynexus-task-memory', 'integrations/codex', 'docs/repository-memory', 'docs/current-knowledge'):
            shutil.copytree(REPO / name, cls.source / name)
        for name in ('docs/current-knowledge-contract.md', 'docs/acceptance/current-knowledge.md',
                     'decisions/ADR-030-current-knowledge-maintenance.md'):
            target = cls.source / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPO / name, target)
        template = cls.source / 'empty-template'
        template.mkdir()
        cls.git('init', '--quiet', '--template=' + str(template))
        cls.git('add', 'integrations', 'docs', 'decisions')
        cls.git('-c', 'user.name=Synthetic install test', '-c', 'user.email=synthetic@example.invalid',
                '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Synthetic install fixture')
        cls.revision = cls.git('rev-parse', 'HEAD').decode().strip()

    @classmethod
    def tearDownClass(cls):
        cls.source_temp.cleanup()

    @classmethod
    def git(cls, *args):
        return subprocess.check_output(['/usr/bin/git', '-C', str(cls.source), *args])

    def setUp(self):
        self.f = Fixture()
        self.addCleanup(self.f.close)
        self.version = self.f.private / 'version-1'
        self.rule = self.f.private / 'rule.txt'
        self.rule.write_bytes(self.f.rule)
        os.chmod(self.rule, 0o600)

    def install(self, destination=None, **overrides):
        values = {'source': self.source, 'revision': self.revision, 'binary': BINARY,
                  'binary-sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
                  'python': PYTHON, 'destination': destination or self.version, 'rule-file': self.rule}
        values.update(overrides)
        args = [PYTHON, '-B', str(INSTALLER)]
        for key, value in values.items():
            args += ['--' + key, str(value)]
        result = subprocess.run(args, capture_output=True, env={'PATH': '/usr/bin:/bin'})
        self.assertTrue(result.stdout, result.stderr)
        return result.returncode, json.loads(result.stdout)

    def run_launcher(self, *args, version=None, extra_env=None):
        env = {'PATH': '/not-an-executable-path', 'MEMORYNEXUS_TASK_LEDGER': '/wrong/ledger.sqlite',
               'PYTHONPATH': '/wrong/python/path'}
        env.update(extra_env or {})
        result = subprocess.run([PYTHON, '-I', '-S', '-B', str((version or self.version) / 'launch.py'),
                                 *map(str, args)], capture_output=True, env=env, cwd='/')
        self.assertTrue(result.stdout, result.stderr)
        return result.returncode, json.loads(result.stdout)

    def bind(self):
        code, installed = self.install()
        self.assertEqual(code, 0, installed)
        f = self.f
        for key in ('binary', 'binary_sha256', 'package', 'package_sha256', 'python'):
            f.binding[key] = installed[key]
        f.write(f.binding_path, f.binding)
        f.rebind(lambda profile: None)
        return installed

    def test_install_manifest_modes_committed_bytes_and_no_overwrite(self):
        # An uncommitted checkout edit cannot be mislabeled as the pinned commit.
        file = self.source / 'integrations/task-memory/PROTOCOL.md'
        original = file.read_bytes()
        self.addCleanup(file.write_bytes, original)
        file.write_bytes(b'Uncommitted text must not be installed.\n')
        before = self.f.cli(['--config', self.f.config, 'workflow-status'], {}, self.f.ledger)
        code, installed = self.install()
        self.assertEqual(code, 0, installed)
        self.assertEqual((self.version / 'package/integrations/task-memory/PROTOCOL.md').read_bytes(), original)
        manifest = json.loads((self.version / 'manifest.json').read_text())
        self.assertEqual(manifest['source_revision'], self.revision)
        paths = [entry['path'] for entry in manifest['files']]
        self.assertEqual(paths, sorted(paths, key=Path))
        self.assertIn('package/workflow.py', paths)
        self.assertIn('package/current_knowledge.py', paths)
        self.assertIn('package/docs/current-knowledge-contract.md', paths)
        self.assertIn('package/docs/current-knowledge/examples/read-request.json', paths)
        self.assertIn('package/docs/acceptance/current-knowledge.md', paths)
        self.assertIn('package/decisions/ADR-030-current-knowledge-maintenance.md', paths)
        self.assertIn('package/integrations/task-memory/memorynexus-task-bootstrap/references/manifest.md', paths)
        self.assertIn('package/integrations/minimax/memorynexus-task-memory/SKILL.md', paths)
        self.assertIn('package/rules/lens-1.txt', paths)
        for entry in manifest['files']:
            path = self.version / entry['path']
            self.assertEqual(entry['sha256'], hashlib.sha256(path.read_bytes()).hexdigest())
            self.assertEqual(entry['mode'], '0700' if entry['path'].startswith('bin/') else '0600')
        for path in self.version.rglob('*'):
            if path.is_dir():
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o700)
        self.assertEqual(stat.S_IMODE((self.version / 'manifest.json').stat().st_mode), 0o600)
        self.assertFalse(list(self.version.rglob('*.sqlite')))
        self.assertFalse(list(self.version.rglob('.memorynexus.json')))
        self.assertEqual(self.f.cli(['--config', self.f.config, 'workflow-status'], {}, self.f.ledger), before)
        old_manifest = (self.version / 'manifest.json').read_bytes()
        code, result = self.install()
        self.assertEqual((code, result['code']), (2, 'new_directory_required'))
        self.assertEqual((self.version / 'manifest.json').read_bytes(), old_manifest)

    def current_fixture(self):
        from current_knowledge_fixture import CurrentFixture, install_bridge
        code, installed = self.install()
        self.assertEqual(code, 0, installed)
        f = CurrentFixture()
        self.addCleanup(f.close)
        install_bridge(f)
        for key in ('binary', 'binary_sha256', 'package', 'package_sha256', 'python'):
            f.binding[key] = installed[key]
        f.save_binding()
        f.record('Read workspace-b/marker.txt')
        return f

    def test_installed_current_read_check_launch_and_no_retry(self):
        f = self.current_fixture()
        read = f.state / 'read'
        code, result = self.run_launcher('current', 'read', '--binding', f.binding_path,
                                        '--request', f.request_path, '--run-dir', read)
        self.assertEqual(code, 0, result)
        raw = (read / 'material.json').read_bytes()
        for action in ('check', 'launch'):
            code, result = self.run_launcher('current', action, '--binding', f.binding_path,
                '--evidence', read / 'evidence.json', '--run-dir', f.state / action,
                extra_env={'MN_SYNTHETIC_AUTH': 'synthetic-secret'})
            self.assertEqual(code, 0, result)
        trace = json.loads((f.private / 'bridge-trace.jsonl').read_text())
        self.assertEqual(trace['sha256'], hashlib.sha256(raw).hexdigest())
        self.assertEqual(trace['opened'], 'workspace-b/marker.txt')
        self.assertTrue(trace['inherited_auth'])
        self.assertTrue(trace['ledger_matches'])
        self.assertFalse((read / 'material.json').exists())
        code, result = self.run_launcher('current', 'launch', '--binding', f.binding_path,
            '--evidence', read / 'evidence.json', '--run-dir', f.state / 'retry')
        self.assertEqual((code, result['code']), (3, 'delivery_already_attempted'))
        self.assertFalse(list(self.version.rglob('__pycache__')))
        self.assertEqual(self.run_launcher('verify')[0], 0)

    def test_current_preflight_blocks_other_install_and_tampering(self):
        f = self.current_fixture()
        second = self.f.private / 'version-2'
        self.assertEqual(self.install(second)[0], 0)
        run = f.state / 'blocked'
        args = ('current', 'read', '--binding', f.binding_path,
                '--request', f.request_path, '--run-dir', run)
        code, result = self.run_launcher(*args, version=second)
        self.assertEqual((code, result['code']), (2, 'installation_binding_mismatch'))
        self.assertFalse(run.exists())
        self.assertEqual(self.run_launcher(*args, '--binding', f.binding_path)[1]['code'],
                         'explicit_binding_required')
        self.assertEqual(self.run_launcher('current', 'apply')[1]['code'], 'invalid_command')
        self.assertEqual(self.run_launcher(*args, '--approve', 'unused')[1]['code'], 'invalid_command')
        path = self.version / 'package/current_knowledge.py'
        path.write_text('raise RuntimeError("must never import")')
        code, result = self.run_launcher(*args)
        self.assertEqual((code, result['code']), (2, 'installation_changed'))
        self.assertFalse(run.exists())

    def test_bad_binary_hash_and_non_task_executable_fail_before_creation(self):
        code, result = self.install(**{'binary-sha256': '0' * 64})
        self.assertEqual((code, result['code']), (2, 'binary_hash_mismatch'))
        self.assertFalse(self.version.exists())
        non_cli = Path('/usr/bin/true').resolve()
        code, result = self.install(binary=non_cli, **{'binary-sha256': hashlib.sha256(non_cli.read_bytes()).hexdigest()})
        self.assertEqual((code, result['code']), (2, 'unsupported_version'))
        self.assertFalse(self.version.exists())

    def test_unsafe_python_or_destination_never_changes_permissions(self):
        alias = self.f.private / 'python-alias'
        alias.symlink_to(PYTHON)
        code, result = self.install(python=alias)
        self.assertEqual((code, result['code']), (2, 'unsafe_path'))
        shared = self.f.private / 'shared'
        shared.mkdir(mode=0o755)
        os.chmod(shared, 0o755)
        code, result = self.install(shared / 'version')
        self.assertEqual((code, result['code']), (2, 'unsafe_path'))
        self.assertEqual(stat.S_IMODE(shared.stat().st_mode), 0o755)
        self.assertFalse(self.version.exists())

    def test_preflight_rejects_changed_package_rules_binary_and_extra_files(self):
        self.assertEqual(self.install()[0], 0)
        for relative in ('package/workflow.py', 'package/rules/lens-1.txt', 'bin/memorynexus-feedback-cli'):
            with self.subTest(relative=relative):
                path = self.version / relative
                before = path.read_bytes()
                path.write_bytes(before + b'\nraise RuntimeError("must not execute")\n')
                code, result = self.run_launcher('verify')
                self.assertEqual((code, result['code']), (2, 'installation_changed'))
                path.write_bytes(before)
        extra = self.version / 'package/unlisted.py'
        extra.write_text('unexpected')
        os.chmod(extra, 0o600)
        self.assertEqual(self.run_launcher('verify')[1]['code'], 'installation_changed')

    def test_installation_survives_source_checkout_and_build_artifact_removal(self):
        source = self.f.private / 'source'
        shutil.copytree(self.source, source)
        binary = self.f.private / 'input-binary'
        shutil.copyfile(BINARY, binary)
        os.chmod(binary, 0o700)
        code, result = self.install(source=source, binary=binary)
        self.assertEqual(code, 0, result)
        shutil.rmtree(source)
        binary.unlink()
        self.rule.unlink()
        self.assertEqual(self.run_launcher('verify')[0], 0)

    def test_new_version_requires_explicit_binding_and_keeps_old_installation(self):
        self.bind()
        new_version = self.f.private / 'version-2'
        self.assertEqual(self.install(new_version)[0], 0)
        code, result = self.run_launcher('verify', '--binding', self.f.binding_path, version=new_version)
        self.assertEqual((code, result['code']), (2, 'installation_binding_mismatch'))
        self.assertEqual(self.run_launcher('verify', '--binding', self.f.binding_path)[0], 0)
        self.assertEqual(self.run_launcher('prepare', '--bind', self.f.binding_path)[1]['code'], 'invalid_command')
        self.assertEqual(self.run_launcher('status', '--binding', self.f.binding_path, '--binding', self.f.binding_path)[1]['code'], 'explicit_binding_required')

    def test_same_installed_entry_two_labels_increment_review_publish_and_noop(self):
        self.bind()
        f = self.f
        # These labels exercise routing, not actual MiniMax/Codex acceptance.
        for label in ('synthetic-minimax', 'synthetic-codex'):
            code, result = self.run_launcher('prepare', '--binding', f.binding_path, '--workflow', 'capture',
                                             '--client', label, '--run-dir', f.state / label)
            self.assertEqual(code, 3, result)
        f.run = self.run_launcher
        self.assertEqual(f.capture('capture')[0], 0)
        for phase in ('review', 'publish'):
            code, result, prepared = f.prepare(phase, phase + '-prepare')
            self.assertEqual(code, 3, result)
            value = json.loads(prepared.read_text())
            items = [] if phase == 'publish' else [
                {'input_record_id': item['input_record_id'], 'consumer_id': item['consumer_id'],
                 'decision': 'proposed', 'reason': 'Documented source report, not owner confirmation.',
                 'dependencies': item['dependencies'], 'output_text': 'Source: incident.md; verification pending.'}
                for item in value['items']]
            submission = f.state / (phase + '-submission.json')
            f.write(submission, {'schema_version': 1, 'prepared_sha256': hashlib.sha256(prepared.read_bytes()).hexdigest(), 'items': items})
            directory = f.state / (phase + '-preview')
            code, result = f.run('preview', '--binding', f.binding_path, '--prepared', prepared,
                                 '--submission', submission, '--run-dir', directory)
            self.assertIn(code, (0, 3), result)
            plan = directory / 'plan.json'
            code, result = f.run('apply', '--binding', f.binding_path, '--plan', plan,
                                 '--approve', digest(json.loads(plan.read_text())))
            self.assertEqual(code, 0, result)
        for target in f.binding['targets']:
            self.assertIn('Source: incident.md', Path(target['path']).read_text())
        before = [Path(t['path']).read_bytes() for t in f.binding['targets']]
        code, result, prepared = f.prepare('publish', 'repeat-publish')
        self.assertIn(code, (0, 3), result)
        self.assertEqual(json.loads(prepared.read_text())['items'], [])
        self.assertEqual([Path(t['path']).read_bytes() for t in f.binding['targets']], before)
        self.assertFalse(list(self.version.rglob('__pycache__')))
        self.assertEqual(self.run_launcher('status', '--binding', f.binding_path)[0], 0)

    def test_trigger_evidence_is_retained_but_never_proves_a_scheduler(self):
        self.bind()
        f = self.f
        proof = f.private / 'proof.json'
        f.write(proof, {'source': 'synthetic-only'})
        trigger = f.private / 'trigger.json'
        f.write(trigger, {'kind': 'scheduler', 'provider': 'unconfigured', 'run_id': 'synthetic', 'evidence_path': str(proof)})
        run = f.state / 'scheduled-label'
        code, result = self.run_launcher('prepare', '--binding', f.binding_path, '--workflow', 'capture',
                                         '--client', 'synthetic', '--run-dir', run,
                                         extra_env={'MEMORYNEXUS_TRIGGER_FILE': str(trigger)})
        self.assertEqual(code, 3, result)
        self.assertEqual(result['trigger'], {'kind': 'manual'})
        self.assertIn('unverified_trigger', [item.get('code') for item in result['items']])
        self.assertEqual((run / 'trigger-proof.json').read_bytes(), proof.read_bytes())

    def test_compatible_package_rollback_and_pinned_backup_restore_stay_isolated(self):
        first = self.bind()
        f = self.f
        f.run = self.run_launcher
        self.assertEqual(f.capture('before-upgrade')[0], 0)
        second_version = f.private / 'version-2'
        code, second = self.install(second_version)
        self.assertEqual(code, 0, second)
        original_binding = dict(f.binding)
        for key in ('binary', 'binary_sha256', 'package', 'package_sha256', 'python'):
            f.binding[key] = second[key]
        f.write(f.binding_path, f.binding)
        f.rebind(lambda profile: None)
        self.assertEqual(self.run_launcher('status', '--binding', f.binding_path, version=second_version)[0], 0)
        # Explicit confirmed rollback of package binding, never of ledger bytes.
        f.binding = original_binding
        f.write(f.binding_path, f.binding)
        f.rebind(lambda profile: None)
        self.assertEqual(self.run_launcher('status', '--binding', f.binding_path)[0], 0)

        def task(ledger, action, body):
            result = subprocess.run([first['binary'], 'task', '--ledger', str(ledger), action],
                                    input=json.dumps(body).encode(), capture_output=True,
                                    env={'PATH': '/usr/bin:/bin'})
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            return json.loads(result.stdout)['result']
        snapshot = f.private / 'rollback-snapshot.sqlite'
        restored = f.private / 'rollback-restored.sqlite'
        task(f.ledger, 'backup', {'path': str(snapshot)})
        task(restored, 'restore', {'path': str(snapshot)})
        saved = task(snapshot, 'export', {})
        recovered = task(restored, 'export', {})
        # The installed binary handles the current v2 format; restoration must
        # preserve all ordinary records/receipts while fencing old authority.
        self.assertNotEqual(saved['ledger_meta'][0]['incarnation'], recovered['ledger_meta'][0]['incarnation'])
        expected = json.loads(json.dumps(saved))
        expected['ledger_meta'][0]['incarnation'] = recovered['ledger_meta'][0]['incarnation']
        for workflow in expected['workflows']:
            workflow['state'] = 'recovery_required'
        self.assertEqual(expected, recovered)
        self.assertEqual(f.binding['ledger'], str(f.ledger))
        self.assertEqual(self.run_launcher('status', '--binding', f.binding_path)[0], 0)


if __name__ == '__main__':
    unittest.main()
