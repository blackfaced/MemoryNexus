"""G2 public-process task current/history and use-before-check acceptance."""
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from current_knowledge_fixture import CurrentFixture
from workflow_fixture import Fixture
from workflow_types import canonical, digest


class CurrentKnowledge(unittest.TestCase):
    def setUp(self):
        self.f = CurrentFixture(); self.addCleanup(self.f.close)

    def ok(self, result):
        code, report, run = result
        self.assertEqual((code, report['code']), (0, 'ok'), report)
        return run

    def failure(self, result, expected):
        code, report, run = result
        self.assertIn(code, (2, 3)); self.assertEqual(report['code'], expected, report)
        self.assertFalse((run / 'material.json').exists())
        return run

    def test_correction_replay_competition_current_and_history(self):
        f = self.f; old = f.record()
        candidate = f.mutation('review', text='UNCONFIRMED secret inference', source_ids=[old])['result']['record_id']
        first = self.ok(f.run()); initial = f.payload(first)
        self.assertEqual(initial['candidates'][0]['id'], candidate)
        self.assertNotIn('UNCONFIRMED', (first / 'material.json').read_text())
        body = {'idempotency_key': 'correct', 'run_id': '60000000-0000-4000-8000-000000000001', 'client': 'synthetic',
                'target_id': old, 'text': 'Work in workspace-b.', 'occurred_at': '2026-10-27T00:01:00Z', 'confirmation': 'confirmed'}
        corrected = f.cli('correct', body); new = corrected['result']['record_id']
        self.assertEqual(f.cli('correct', body)['status'], 'replayed')
        self.assertEqual(f.cli('correct', {**body, 'idempotency_key': 'competing'}, expected=2)['code'], 'conflict')
        fresh = self.ok(f.run()); text = (fresh / 'material.json').read_text()
        self.assertIn('workspace-b', text); self.assertNotIn('workspace-a', text); self.assertNotIn('UNCONFIRMED', text)
        self.failure(f.run('check', evidence=first / 'evidence.json'), 'stale_snapshot')
        self.assertFalse((first / 'material.json').exists())
        f.write_request(purpose='history', record_ids=[old, new, candidate])
        history = self.ok(f.run()); data = f.payload(history)
        self.assertFalse(data['current_use']); self.assertIn('workspace-a', (history / 'material.json').read_text())
        self.assertNotIn('UNCONFIRMED', (history / 'material.json').read_text())
        self.failure(f.run('check', evidence=history / 'evidence.json'), 'history_only')

    def test_check_uses_original_identity_and_rebuilds_without_cached_body(self):
        f = self.f; f.record(); read = self.ok(f.run()); e = f.payload(read, 'evidence.json')
        (read / 'material.json').unlink()
        check = self.ok(f.run('check', evidence=read / 'evidence.json'))
        checked = f.payload(check, 'evidence.json')
        self.assertEqual((checked['read_id'], checked['created_at']), (e['read_id'], e['created_at']))
        self.assertNotIn('material', checked); self.assertFalse((check / 'material.json').exists())
        self.assertEqual(checked['previous_evidence_sha256'], hashlib.sha256((read / 'evidence.json').read_bytes()).hexdigest())
        self.failure(f.run('check', evidence=check / 'evidence.json'), 'invalid_input')
        self.failure(f.run('read', label=read.name), 'io_error')

    def test_competing_real_corrections_only_one_wins(self):
        f = self.f; old = f.record()
        def correct(label):
            body = {'idempotency_key': label, 'run_id': '60000000-0000-4000-8000-000000000001', 'client': 'synthetic',
                    'target_id': old, 'text': label, 'occurred_at': '2026-10-27T00:00:00Z', 'confirmation': 'confirmed'}
            p = subprocess.run([str(Path(f.binding['binary'])), 'task', '--config', str(f.config), 'correct'],
                               input=canonical(body), capture_output=True, env=dict(os.environ, MEMORYNEXUS_TASK_LEDGER=str(f.ledger)))
            return p.returncode, json.loads(p.stdout)
        with ThreadPoolExecutor(2) as pool: results = list(pool.map(correct, ['one', 'two']))
        self.assertEqual(sorted(r[0] for r in results), [0, 2])
        self.assertEqual([r[1]['code'] for r in results if r[0]], ['conflict'])
        self.assertEqual(len(f.payload(self.ok(f.run()))['entries']), 1)

    def test_empty_truncation_and_unrelated_contradictory_records(self):
        f = self.f; self.failure(f.run(), 'empty')
        f.record('Direction is A.'); f.record('Direction is B.')
        self.assertEqual(len(f.payload(self.ok(f.run()))['entries']), 2)
        f.write_request(limit=1); self.failure(f.run(), 'incomplete')
        f.write_request()
        for _ in range(19): f.record('Bounded report')
        self.failure(f.run(), 'incomplete')

    def test_character_budget_is_not_silent_success(self):
        for _ in range(13): self.f.record('x' * 1000)
        run = self.failure(self.f.run(), 'incomplete')
        self.assertTrue(self.f.payload(run, 'evidence.json')['truncated'])

    def test_material_byte_limit_preserves_metadata_without_partial_body(self):
        f = self.f
        for _ in range(4): f.record('x' * 1000, evidence_refs=[{'path': 'x' * 16000}])
        run = self.failure(f.run(), 'incomplete')
        evidence = f.payload(run, 'evidence.json')
        self.assertEqual(len(evidence['selected']), 4)
        self.assertNotIn('material', evidence)
        self.assertFalse(evidence['truncated'])

    def test_withdrawal_tombstones_export_and_real_restore(self):
        f = self.f; old = f.record('WITHDRAWN_SENTINEL'); read = self.ok(f.run())
        body = {'idempotency_key': 'withdraw', 'run_id': '60000000-0000-4000-8000-000000000001', 'client': 'synthetic',
                'target_id': old, 'reason': 'Synthetic removal', 'confirmation': 'confirmed'}
        f.cli('retract', body)
        self.failure(f.run('check', evidence=read / 'evidence.json'), 'stale_snapshot')
        self.assertFalse((read / 'material.json').exists())
        self.assertNotIn('WITHDRAWN_SENTINEL', json.dumps(f.storage('export', {})))
        f.write_request(purpose='history', record_ids=[old]); hist = self.ok(f.run())
        self.assertNotIn('WITHDRAWN_SENTINEL', (hist / 'material.json').read_text())
        self.assertNotIn('text', f.payload(hist)['entries'][0])
        backup = f.base / 'data/withdrawn.sqlite'; restored = f.base / 'data/restored.sqlite'
        f.storage('backup', {'path': str(backup)})
        original_i = f.cli('workflow-status', {})['incarnation']
        f.storage('restore', {'path': str(backup)}, restored)
        f.binding['ledger'] = str(restored); f.save_binding()
        newhist = self.ok(f.run()); self.assertNotEqual(f.payload(newhist)['incarnation'], original_i)
        self.assertNotIn('WITHDRAWN_SENTINEL', (newhist / 'material.json').read_text())

    def test_cross_task_history_and_binding(self):
        f = self.f
        other = f.cli('record', {'idempotency_key': 'x', 'run_id': '60000000-0000-4000-8000-000000000001',
                                'client': 'synthetic', 'kind': 'instruction', 'text': 'FOREIGN_SENTINEL',
                                'occurred_at': '2026-10-27T00:00:00Z', 'confirmation': 'confirmed'}, config=f.other)['result']['record_id']
        f.write_request(purpose='history', record_ids=[other]); run = self.failure(f.run(), 'scope_mismatch')
        self.assertNotIn('FOREIGN_SENTINEL', (run / 'result.json').read_text())
        f.binding['task_id'] = f.other_id; f.save_binding(); self.failure(f.run(), 'scope_mismatch')

    def test_strict_schema_symlinks_and_missing_ledger(self):
        f = self.f; f.record()
        for data in (b'{"schema_version":1,"schema_version":1}', b'{"schema_version":null}', b'x' * 32769,
                     canonical({**f.request, 'record_ids': []}), canonical({**f.request, 'limit': True}), b'[' * 1500):
            f.request_path.write_bytes(data); self.failure(f.run(), 'invalid_input')
        f.write_request(); original = f.private / 'real.json'; f.request_path.rename(original); f.request_path.symlink_to(original)
        self.failure(f.run(), 'invalid_input'); f.request_path.unlink(); original.rename(f.request_path)
        f.binding['ledger'] = str(f.base / 'data/missing.sqlite'); f.save_binding()
        self.failure(f.run(), 'io_error'); self.assertFalse(Path(f.binding['ledger']).exists())

    def test_unknown_copies_and_unsupported_slices(self):
        f = self.f; f.record(); old_file = f.root / 'old.md'; old_file.write_text('Uncoordinated old claim')
        f.binding['copies'] = [{'id': 'old', 'path': str(old_file), 'role': 'external'}]; f.save_binding()
        run = self.failure(f.run(), 'injection_unknown'); e = f.payload(run, 'evidence.json')
        self.assertEqual((e['coverage'], e['copies'][0]['status']), ('unknown', 'unknown'))
        old_file.unlink(); self.failure(f.run(), 'injection_unknown')
        f.binding['copies'][0]['path'] = str(f.root / 'missing-parent/missing.md'); f.save_binding()
        self.failure(f.run(), 'injection_unknown')
        f.binding['copies'] = []; f.save_binding()
        self.failure(f.run('launch', evidence=f.private / 'absent'), 'unsupported_version')
        f.write(f.request_path, {'schema_version': 1, 'mode': 'repository', 'purpose': 'current', 'consumer_ids': ['lessons']})
        self.failure(f.run(), 'unsupported_version')
        f.write_request()
        f.binding['repository_binding'] = str(f.private / 'repository.json'); f.save_binding()
        self.failure(f.run(), 'unsupported_version')

    def test_tampered_material_result_and_install(self):
        f = self.f; f.record(); run = self.ok(f.run())
        (run / 'material.json').write_text('UNTRUSTED')
        self.failure(f.run('check', evidence=run / 'evidence.json'), 'stale_snapshot')
        self.assertFalse((run / 'material.json').exists())
        run = self.ok(f.run()); r = f.payload(run, 'result.json'); r['artifacts'][1]['sha256'] = 'f' * 64; f.write(run / 'result.json', r)
        self.failure(f.run('check', evidence=run / 'evidence.json'), 'stale_snapshot')
        fresh = self.ok(f.run())
        (f.package / 'new-file').write_text('Changed package')
        self.failure(f.run('check', evidence=fresh / 'evidence.json'), 'stale_snapshot')
        self.assertFalse((fresh / 'material.json').exists())
        self.failure(f.run(), 'stale_snapshot')

    def test_expiry_and_cleanup_do_not_delete_damaged_artifacts(self):
        f = self.f; f.record(); run = self.ok(f.run()); e = f.payload(run, 'evidence.json')
        e['created_at'] = (datetime.now(timezone.utc) - timedelta(days=2)).strftime('%Y-%m-%dT%H:%M:%SZ')
        f.write(run / 'evidence.json', e)
        r = f.payload(run, 'result.json'); raw = (run / 'evidence.json').read_bytes()
        r['artifacts'][1].update(sha256=hashlib.sha256(raw).hexdigest(), bytes=len(raw)); f.write(run / 'result.json', r)
        damaged = f.state / 'unowned'; damaged.mkdir(mode=0o700); f.write(damaged / 'material.json', {'keep': True})
        self.failure(f.run('check', evidence=run / 'evidence.json'), 'stale_snapshot')
        self.assertFalse((run / 'material.json').exists()); self.assertTrue((damaged / 'material.json').exists())

    def test_real_cli_timeout_and_bounded_pipes(self):
        f = self.f; f.record(); f.binding['timeout_seconds'] = 1
        for mode in ('timeout', 'overflow', 'stderr'):
            f.interpose(mode); self.failure(f.run(), 'io_error')

    def test_read_race_retries_once_and_persistent_change_fails(self):
        f = self.f; f.record(); f.interpose('barrier')
        def wait_ready(n):
            until = time.monotonic() + 8
            while not (f.private / ('ready-' + str(n))).exists():
                self.assertLess(time.monotonic(), until); time.sleep(.005)
        with ThreadPoolExecutor(1) as pool:
            future = pool.submit(f.run)
            wait_ready(1); f.record('Changed after first header'); (f.private / 'release-1').touch()
            wait_ready(2); (f.private / 'release-2').touch()
            run = self.ok(future.result()); self.assertEqual(len(f.payload(run)['entries']), 2)
            future = pool.submit(f.run)
            for n in (3, 4):
                wait_ready(n); f.record('Another concurrent write'); (f.private / ('release-' + str(n))).touch()
            self.failure(future.result(), 'stale_snapshot')
        self.assertEqual((f.private / 'count').read_text(), '4')

    def test_v1_private_modes_and_unhashed_evidence_reference(self):
        f = self.f
        f.record(evidence_refs=[{'path': 'local-marker.txt'}])
        run = self.ok(f.run())
        self.assertEqual(f.payload(run)['entries'][0]['evidence_refs'], [{'path': 'local-marker.txt'}])
        self.assertEqual(run.stat().st_mode & 0o777, 0o700)
        for p in run.iterdir(): self.assertEqual(p.stat().st_mode & 0o777, 0o600)
        os.chmod(f.binding_path, 0o644); self.failure(f.run(), 'invalid_input'); os.chmod(f.binding_path, 0o600)
        f.binding['ledger'] = str(f.v1); f.save_binding(); self.failure(f.run(), 'unsupported_version')

    def test_check_rejects_restore_even_at_same_task_revision(self):
        f = self.f; f.record(); read = self.ok(f.run()); original = f.payload(read, 'evidence.json')
        backup = f.base / 'data/before-restore.sqlite'; restored = f.base / 'data/restored.sqlite'
        f.storage('backup', {'path': str(backup)}); f.storage('restore', {'path': str(backup)}, restored)
        # Isolated test only: put the publicly restored file at the bound path.
        # No connections are alive and no SQL or fabricated ledger bytes are used.
        for suffix in ('-wal', '-shm'):
            Path(str(f.ledger) + suffix).unlink(missing_ok=True)
        os.replace(restored, f.ledger)
        self.assertEqual(f.cli('workflow-status', {})['revision'], original['revision'])
        self.failure(f.run('check', evidence=read / 'evidence.json'), 'stale_snapshot')
        self.assertFalse((read / 'material.json').exists())

    def test_corrected_source_draft_stays_out_of_task_material(self):
        # Existing workflow fixture provides actual capture; no direct DB mutation.
        f = self.f; w = Fixture(); self.addCleanup(w.close); w.capture('source')
        rows = w.cli(['--config', w.config, 'workflow-status'], {'workflow_id': w.profile['workflow_id'], 'view': 'inputs'}, w.ledger)['result']['items']
        draft = rows[0]['id']
        w.cli(['--config', w.config, 'review'], {'idempotency_key': 'draft-candidate', 'run_id': '60000000-0000-4000-8000-000000000001', 'client': 'synthetic',
             'source_ids': [draft], 'text': 'CANDIDATE_ONLY_SENTINEL', 'occurred_at': '2026-10-27T00:00:00Z'}, w.ledger)
        f.binding.update(task_id=w.task_id, task_root=str(w.root), config=str(w.config), ledger=str(w.ledger)); f.save_binding()
        candidate_only = self.failure(f.run(), 'empty')
        self.assertEqual(f.payload(candidate_only, 'evidence.json')['candidate_count'], 1)
        self.assertNotIn('CANDIDATE_ONLY_SENTINEL', (candidate_only / 'evidence.json').read_text())
        w.cli(['--config', w.config, 'correct'], {'idempotency_key': 'correct-draft', 'run_id': '60000000-0000-4000-8000-000000000001', 'client': 'synthetic',
             'target_id': draft, 'text': 'CONFIRMED_DRAFT_SENTINEL', 'occurred_at': '2026-10-27T00:00:00Z', 'confirmation': 'confirmed'}, w.ledger)
        f.binding.update(task_id=w.task_id, task_root=str(w.root), config=str(w.config), ledger=str(w.ledger)); f.save_binding()
        self.failure(f.run(), 'empty')
        self.assertNotIn('CONFIRMED_DRAFT_SENTINEL', '\n'.join(p.read_text() for p in f.state.glob('*/evidence.json')))


if __name__ == '__main__': unittest.main()
