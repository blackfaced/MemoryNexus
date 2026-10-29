"""Daily entry public-process acceptance with isolated tasks and the real CLI."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
import uuid

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO / 'integrations/task-memory/scripts'))
from workflow_cli import package_inventory
from workflow_types import digest

BINARY = Path(os.environ.get('MEMORYNEXUS_TEST_BINARY', str(REPO / 'target/debug/memorynexus-feedback-cli'))).resolve()
PYTHON = str(Path(sys.executable).resolve())


class DailyFixture:
    def __init__(self):
        self.temp = tempfile.TemporaryDirectory(prefix='mn-daily-', dir=Path(tempfile.gettempdir()).resolve())
        self.base = Path(self.temp.name)
        self.root = self.base / 'task'; self.root.mkdir(mode=0o700)
        self.private = self.base / 'private'; self.private.mkdir(mode=0o700)
        self.data = self.private / 'data'; self.data.mkdir(mode=0o700)
        self.state = self.private / 'state'; self.state.mkdir(mode=0o700)
        self.package = self.base / 'package'
        shutil.copytree(REPO / 'integrations/task-memory/scripts', self.package,
                        ignore=shutil.ignore_patterns('__pycache__', '*.pyc'))
        self.config = self.root / '.memorynexus.json'
        self.ledger = self.data / 'task.sqlite'
        result = self.cli('init', {'idempotency_key': 'init', 'display_name': 'Synthetic daily task',
                                 'automatic_capture': ['execution', 'review'], 'confirmation': 'confirmed'})
        self.binding = {'schema_version': 1, 'task_id': result['task_id'], 'task_root': str(self.root),
                        'config': str(self.config), 'ledger': str(self.ledger), 'binary': str(BINARY),
                        'binary_sha256': self.sha(BINARY), 'python': PYTHON, 'package': str(self.package),
                        'package_sha256': digest(package_inventory(self.package)), 'state_root': str(self.state),
                        'timeout_seconds': 5, 'client': 'synthetic', 'copies': []}
        self.binding_path = self.private / 'binding.json'; self.save_binding()
        self.mode = self.private / 'fault.txt'; self.mode.write_text('none'); self.mode.chmod(0o600)

    @staticmethod
    def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()

    def write(self, path, value):
        path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n'); path.chmod(0o600)
        return path

    def save_binding(self): self.write(self.binding_path, self.binding)

    def cli(self, action, body, config=None, ok=True):
        r = subprocess.run([str(BINARY), 'task', '--config', str(config or self.config), action],
                           input=json.dumps(body).encode(), capture_output=True,
                           env={**os.environ, 'MEMORYNEXUS_TASK_LEDGER': str(self.ledger)})
        if ok and r.returncode: raise AssertionError(r.stdout)
        return json.loads(r.stdout)

    def body(self, text='A bounded execution report.', kind='execution'):
        return {'idempotency_key': str(uuid.uuid4()), 'run_id': str(uuid.uuid4()), 'client': 'synthetic',
                'kind': kind, 'text': text, 'occurred_at': '2026-10-29T08:00:00+00:00'}

    def run(self, command, **kwargs):
        args = [PYTHON, '-I', '-S', '-B', str(self.package / 'daily.py'), command,
                '--binding', str(self.binding_path)]
        for key, value in kwargs.items(): args += ['--' + key.replace('_', '-'), str(value)]
        r = subprocess.run(args, cwd='/', capture_output=True, timeout=30,
                           env={'PATH': '/not-found', 'MEMORYNEXUS_TASK_LEDGER': '/wrong.sqlite'})
        if not r.stdout: raise AssertionError(r.stderr)
        return r.returncode, json.loads(r.stdout)

    def stage(self, body=None, action='record'):
        run = self.state / str(uuid.uuid4())
        request = self.write(self.private / (str(uuid.uuid4()) + '.json'), body or self.body())
        code, result = self.run('stage', action=action, request=request, run_dir=run)
        if code: raise AssertionError(result)
        return run, request

    def fault_proxy(self):
        # Inject transport faults AFTER a real child process, never emulate ledger responses.
        proxy = self.private / 'cli-proxy'
        proxy.write_text('#!' + PYTHON + '\n' + '''import json,os,subprocess,sys,time
from pathlib import Path
binary,mode_file,capture = ''' + repr((str(BINARY), str(self.mode), str(self.private / 'captured.json'))) + '''
a=sys.argv[1:]; raw=sys.stdin.buffer.read(); mode=Path(mode_file).read_text()
if '--help' in a:
 r=subprocess.run([binary,*a],input=raw,capture_output=True)
 sys.stdout.buffer.write(r.stdout); sys.exit(r.returncode)
write=a[-1] in ('record','review','correct','retract')
if write: Path(capture).write_bytes(raw)
r=subprocess.run([binary,*a],input=raw,capture_output=True)
if (write and mode in ('lost','partial','wrong-task','wrong-revision','exit','large','timeout')) or (not write and mode=='read-fail'):
 if mode=='timeout': time.sleep(10)
 elif mode=='partial': sys.stdout.buffer.write(r.stdout[:20])
 elif mode=='wrong-task':
  v=json.loads(r.stdout); v['task_id']='00000000-0000-4000-8000-000000000001'; print(json.dumps(v))
 elif mode=='wrong-revision':
  v=json.loads(r.stdout); v['revision']=True; print(json.dumps(v))
 elif mode=='large': sys.stdout.write('x'*200000)
 elif mode=='exit': sys.stdout.buffer.write(r.stdout); sys.exit(7)
 else: sys.stderr.write('PRIVATE_SENTINEL do not disclose')
 sys.exit(0)
sys.stdout.buffer.write(r.stdout); sys.exit(r.returncode)
''')
        proxy.chmod(0o700)
        self.binding.update(binary=str(proxy), binary_sha256=self.sha(proxy), timeout_seconds=5)
        self.save_binding()

    def close(self): self.temp.cleanup()


class Daily(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not BINARY.is_file() or not os.access(BINARY, os.X_OK):
            raise RuntimeError('Build MEMORYNEXUS_TEST_BINARY before running daily acceptance; no skips')

    def setUp(self):
        self.f = DailyFixture(); self.addCleanup(self.f.close)

    def test_success_preserves_provenance_candidates_and_completed_readback(self):
        f = self.f
        self.assertEqual(f.run('verify')[0], 0)
        run, request = f.stage()
        note = json.loads((run / 'pending.json').read_text())
        self.assertEqual((run / note['request']).read_bytes(), request.read_bytes())
        code, result = f.run('apply', run_dir=run)
        self.assertEqual((code, result['status']), (0, 'synchronized'), result)
        record = result['receipt']['record_id']
        self.assertEqual(result['readback']['result']['entries'][0]['trust'], 'execution_reported')
        self.assertFalse((run / 'pending.json').exists())
        self.assertNotIn('bounded', (run / 'receipt.json').read_text())
        self.assertEqual(f.run('apply', run_dir=run)[1]['receipt']['record_id'], record)
        review = f.body('A proposed lesson.'); review.pop('kind'); review['source_ids'] = [record]
        candidate, _ = f.stage(review, 'review')
        candidate_result = f.run('apply', run_dir=candidate)[1]
        self.assertEqual(candidate_result['readback']['result']['candidates'][0]['trust'], 'review_candidate')
        current = f.run('read')[1]['context']['result']
        self.assertEqual(current['candidate_count'], 1)
        self.assertEqual(current['candidates'][0]['source_ids'], [record])

    def test_lost_response_explicit_cross_client_retry_uses_identical_bytes_once(self):
        f = self.f; f.fault_proxy()
        run, request = f.stage()
        before = request.read_bytes(); f.mode.write_text('lost')
        code, failed = f.run('apply', run_dir=run)
        self.assertEqual((code, failed['status']), (2, 'pending'))
        self.assertNotIn('PRIVATE_SENTINEL', json.dumps(failed))
        note = json.loads((run / 'pending.json').read_text())
        self.assertEqual((run / note['request']).read_bytes(), before)
        self.assertEqual((f.private / 'captured.json').read_bytes(), before)
        f.mode.write_text('none'); f.binding['client'] = 'other-client'; f.save_binding()
        code, result = f.run('apply', run_dir=run)
        self.assertEqual(code, 0, result)
        self.assertEqual(result['receipt']['status'], 'replayed')
        self.assertEqual((f.private / 'captured.json').read_bytes(), before)
        entries = f.run('read')[1]['context']['result']['entries']
        self.assertEqual(len(entries), 1)
        self.assertEqual(entries[0]['client'], 'synthetic')
        self.assertEqual(entries[0]['run_id'], json.loads(before)['run_id'])
        self.assertEqual(entries[0]['occurred_at'], json.loads(before)['occurred_at'])

    def test_transport_failures_never_claim_sync_and_keep_exact_request(self):
        f = self.f; f.fault_proxy()
        for fault in ('partial', 'wrong-task', 'wrong-revision', 'exit', 'large', 'timeout'):
            with self.subTest(fault=fault):
                f.mode.write_text('none'); run, original = f.stage()
                note = json.loads((run / 'pending.json').read_text()); f.mode.write_text(fault)
                code, result = f.run('apply', run_dir=run)
                self.assertEqual((code, result['status']), (2, 'pending'), result)
                self.assertEqual((run / note['request']).read_bytes(), original.read_bytes())
                self.assertFalse((run / 'receipt.json').exists())
                f.mode.write_text('none')
                self.assertEqual(f.run('apply', run_dir=run)[1]['receipt']['status'], 'replayed')

    def test_wrong_task_missing_ledger_and_package_drift_fail_closed(self):
        f = self.f
        other = f.root / 'other.json'
        second = f.cli('init', {'idempotency_key': 'second', 'display_name': 'Other task',
                              'automatic_capture': [], 'confirmation': 'confirmed'}, config=other)
        original = f.config.read_bytes(); f.config.write_bytes(other.read_bytes())
        self.assertEqual(f.run('read')[1]['status'], 'history_not_loaded')
        f.config.write_bytes(original)
        f.binding['ledger'] = str(f.data / 'absent.sqlite'); f.save_binding()
        self.assertEqual(f.run('read')[0], 2); self.assertFalse((f.data / 'absent.sqlite').exists())
        f.binding['ledger'] = str(f.ledger); f.save_binding()
        (f.package / 'unexpected.txt').write_text('drift')
        self.assertEqual(f.run('read')[1]['code'], 'stale_snapshot')
        self.assertNotEqual(second['task_id'], f.binding['task_id'])

    def test_wrong_record_id_and_altered_pending_never_write(self):
        f = self.f
        req = f.write(f.private / 'read.json', {'record_ids': [str(uuid.uuid4())]})
        self.assertEqual(f.run('read', request=req)[1]['status'], 'history_not_loaded')
        run, _ = f.stage(); note = json.loads((run / 'pending.json').read_text())
        (run / note['request']).write_text('{}')
        self.assertEqual(f.run('apply', run_dir=run)[1]['code'], 'stale_snapshot')
        self.assertEqual(f.cli('context', {})['result']['entries'], [])

    def test_omissions_are_visible_and_explicit_historical_read_works(self):
        f = self.f
        old = f.cli('record', f.body('Old progress.', 'checkpoint'))['result']['record_id']
        f.cli('record', f.body('New progress.', 'checkpoint'))
        f.cli('record', f.body('Independent acceptance: failed.'))
        request = f.write(f.private / 'read.json', {'limit': 1})
        code, result = f.run('read', request=request)
        self.assertEqual((code, result['status']), (3, 'incomplete'))
        self.assertTrue(result['context']['result']['truncated'])
        self.assertEqual(result['context']['result']['omitted_count'], 1)
        f.write(request, {'record_ids': [old]})
        self.assertEqual(f.run('read', request=request)[1]['context']['result']['entries'][0]['state'], 'historical')

    def test_confirmation_is_not_filled_and_competing_correction_conflicts(self):
        f = self.f
        request = f.write(f.private / 'instruction.json', f.body('An owner instruction.', 'instruction'))
        code, result = f.run('stage', action='record', request=request, run_dir=f.state / 'bad')
        self.assertEqual((code, result['status']), (2, 'not_staged'))
        self.assertFalse((f.state / 'bad').exists())
        first = f.cli('record', f.body())['result']['record_id']
        body = f.body('The exact owner-approved correction.'); body.pop('kind')
        body.update(target_id=first, confirmation='confirmed')
        a, _ = f.stage(body, 'correct')
        body['idempotency_key'] = str(uuid.uuid4()); body['text'] = 'A competing correction.'
        b, _ = f.stage(body, 'correct')
        self.assertEqual(f.run('apply', run_dir=a)[0], 0)
        code, result = f.run('apply', run_dir=b)
        self.assertEqual((code, result['status'], result['code']), (2, 'pending', 'conflict'))
        self.assertTrue((b / 'pending.json').exists())

    def test_replay_after_retraction_and_read_failure_do_not_restore_text(self):
        f = self.f; f.fault_proxy()
        run, _ = f.stage(); f.mode.write_text('lost'); f.run('apply', run_dir=run)
        first = f.cli('context', {})['result']['entries'][0]['id']
        retract = {'idempotency_key': str(uuid.uuid4()), 'run_id': str(uuid.uuid4()), 'client': 'synthetic',
                   'target_id': first, 'reason': 'Synthetic owner withdrawal.', 'confirmation': 'confirmed'}
        f.mode.write_text('none'); withdrawal, _ = f.stage(retract, 'retract')
        self.assertEqual(f.run('apply', run_dir=withdrawal)[0], 0)
        result = f.run('apply', run_dir=run)[1]
        self.assertEqual(result['receipt']['status'], 'replayed')
        self.assertEqual(result['readback']['result']['entries'][0]['state'], 'retracted')
        self.assertNotIn('text', result['readback']['result']['entries'][0])
        f.mode.write_text('read-fail')
        self.assertEqual(f.run('read')[1]['status'], 'history_not_loaded')

    def test_readback_failure_retains_pending_and_cleanup_crash_is_recoverable(self):
        f = self.f; f.fault_proxy()
        # One-shot proxy commits and then causes the following readback to fail.
        proxy = Path(f.binding['binary'])
        text = proxy.read_text().replace("if write: Path(capture).write_bytes(raw)",
            "if write: Path(capture).write_bytes(raw)\nif write and mode=='readback': Path(mode_file).write_text('read-fail')")
        proxy.write_text(text); f.binding['binary_sha256'] = f.sha(proxy); f.save_binding()
        # Stage only after pinning the test proxy; never alter an existing pending pin.
        run, _ = f.stage(); f.mode.write_text('readback')
        code, result = f.run('apply', run_dir=run)
        self.assertEqual((code, result['status']), (2, 'pending'))
        self.assertFalse((run / 'receipt.json').exists())
        note_bytes = (run / 'pending.json').read_bytes()
        f.mode.write_text('none')
        self.assertEqual(f.run('apply', run_dir=run)[1]['receipt']['status'], 'replayed')
        # Crash between removing the payload and removing the note.
        (run / 'pending.json').write_bytes(note_bytes); (run / 'pending.json').chmod(0o600)
        self.assertEqual(f.run('apply', run_dir=run)[0], 0)
        self.assertFalse((run / 'pending.json').exists())

    def test_permission_boundary_and_preserved_external_copies(self):
        f = self.f
        f.binding['copies'] = [{'id': 'external_notes', 'path': str(f.private / 'external.md'), 'role': 'external'}]
        f.save_binding(); self.assertEqual(f.run('verify')[0], 0)
        run, _ = f.stage()
        f.binding['copies'] = []; f.save_binding()
        self.assertEqual(f.run('apply', run_dir=run)[1]['code'], 'scope_mismatch')
        f.private.chmod(0o755)
        self.assertEqual(f.run('read')[1]['code'], 'unsafe_path')
        f.private.chmod(0o700)


if __name__ == '__main__': unittest.main()
