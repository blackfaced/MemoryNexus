"""Public CLI and runner removal, with retained history and exact authorization."""
import hashlib
import json
import os
import unittest
from pathlib import Path
from test_workflow_publish_supersede import phase, reviews, reconsider, decision_id_of
from workflow_fixture import Fixture
from workflow_cli import status_all
from workflow_types import digest


class Removal(unittest.TestCase):
    def fixture(self):
        f = Fixture()
        self.addCleanup(f.close)
        f.capture('capture')
        self.review, self.report = phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish')
        return f

    def prepare(self, f, label='remove', approvals=None):
        pubs = status_all(f.ctx, 'publications')
        request = f.state / (label + '.json')
        f.write(request, dict(schema_version=1, items=[dict(approval_id=p['approval_id'], reason='Duplicate conclusion; retain history.') for p in (pubs if approvals is None else approvals)]))
        directory = f.state / (label + '-prepare')
        code, result = f.run('prepare', '--binding', f.binding_path, '--workflow', 'publish', '--client', 'synthetic', '--removals', request, '--run-dir', directory)
        return code, result, directory / 'prepared.json'

    def preview(self, f, prepared):
        submission = f.state / 'removal-submission.json'
        f.write(submission, dict(schema_version=1, prepared_sha256=hashlib.sha256(prepared.read_bytes()).hexdigest(), items=[]))
        directory = f.state / 'removal-preview'
        code, result = f.run('preview', '--binding', f.binding_path, '--prepared', prepared, '--submission', submission, '--run-dir', directory)
        self.assertEqual(code, 3, result)
        path = directory / 'plan.json'
        return path, json.loads(path.read_text())

    def test_owner_removal_history_replay_and_no_resurrection(self):
        f = self.fixture()
        for t in f.binding['targets']:
            p = Path(t['path'])
            p.write_bytes(b'Human preface\n' + p.read_bytes() + b'Human suffix\n')
        code, result, prepared = self.prepare(f)
        self.assertEqual(code, 3, result)
        path, plan = self.preview(f, prepared)
        self.assertEqual(plan['authorization'], {'mode': 'owner'})
        code, result = f.run('apply', '--binding', f.binding_path, '--plan', path)
        self.assertEqual(code, 3, result)
        self.assertEqual(len(status_all(f.ctx, 'publications')), 2)
        args = ['apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan)]
        code, result = f.run(*args)
        self.assertEqual(code, 0, result)
        for t in f.binding['targets']:
            self.assertEqual(Path(t['path']).read_bytes(), b'Human preface\nHuman suffix\n')
        pubs = status_all(f.ctx, 'publications')
        self.assertEqual(len(pubs), 4)
        self.assertEqual(sum(p.get('maintenance') == 'remove' for p in pubs), 2)
        request = json.loads((path.parent / 'requests/consumer-approve-0-0.json').read_text())
        request['reason'] = 'Different intent on the same key'
        with self.assertRaisesRegex(AssertionError, 'conflict'):
            f.cli(['--config', f.config, 'consumer-approve'], request, f.ledger)
        request['idempotency_key'] = 'unconfirmed-removal'
        del request['confirmation']
        with self.assertRaisesRegex(AssertionError, 'invalid_input|approval_required'):
            f.cli(['--config', f.config, 'consumer-approve'], request, f.ledger)
        before = f.cli(['--ledger', f.ledger, 'export'], {})
        self.assertEqual(f.run(*args)[0], 0)
        self.assertEqual(before, f.cli(['--ledger', f.ledger, 'export'], {}))
        _, _, again = f.prepare('publish', 'again')
        self.assertEqual(json.loads(again.read_text())['items'], [])
        item = dict(self.review['items'][0], decision_id=decision_id_of(self.report, self.review['items'][0]['id']))
        new_id = reconsider(f, item, 'Explicitly reviewed new generation.\n')
        _, _, new = f.prepare('publish', 'new-generation')
        self.assertEqual([x['decision_id'] for x in json.loads(new.read_text())['items']], [new_id])

    def test_last_block_leaves_empty_file(self):
        f = self.fixture()
        _, _, prepared = self.prepare(f)
        path, plan = self.preview(f, prepared)
        self.assertEqual(f.run('apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan))[0], 0)
        for t in f.binding['targets']:
            self.assertTrue(Path(t['path']).exists())
            self.assertEqual(Path(t['path']).read_bytes(), b'')

    def test_edited_and_missing_blocks_rejected(self):
        for mode in ('edited', 'missing', 'duplicate'):
            with self.subTest(mode=mode):
                f = self.fixture()
                target = Path(f.binding['targets'][0]['path'])
                raw = target.read_bytes()
                target.write_bytes(raw.replace(b'Source pointer', b'Human pointer') if mode == 'edited' else b'Human content' if mode == 'missing' else raw + raw)
                code, result, _ = self.prepare(f)
                self.assertNotEqual(code, 0)
                self.assertEqual(result['items'][0]['code'], 'target_conflict')
                self.assertEqual(len(status_all(f.ctx, 'publications')), 2)

    def test_changed_file_after_preview_preserved(self):
        f = self.fixture()
        _, _, prepared = self.prepare(f)
        path, plan = self.preview(f, prepared)
        target = Path(f.binding['targets'][0]['path'])
        raw = target.read_bytes() + b'New manual text\n'
        target.write_bytes(raw)
        code, result = f.run('apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan))
        self.assertNotEqual(code, 0, result)
        self.assertEqual(target.read_bytes(), raw)

    def test_newer_published_generation_rejects_old_owner(self):
        f = self.fixture()
        old = status_all(f.ctx, 'publications')
        item = dict(self.review['items'][0], decision_id=decision_id_of(self.report, self.review['items'][0]['id']))
        reconsider(f, item, 'Replacement generation.\n')
        phase(f, 'publish', lambda p: [], 'replacement')
        code, result, _ = self.prepare(f, approvals=old)
        self.assertEqual(result['items'][0]['code'], 'stale_snapshot')

    def test_response_loss_replays_receipt_without_second_removal(self):
        from test_workflow_failures import FailureTests
        f = self.fixture()
        _, _, prepared = self.prepare(f)
        path, plan = self.preview(f, prepared)
        patch = """original=e.call
lost=[False]
def drop(ctx,action,body):
    response=original(ctx,action,body)
    if action=='publication-result' and not lost[0]:
        lost[0]=True
        raise OSError('lost response after committed removal')
    return response
e.call=drop
"""
        process, _ = FailureTests.driver(self, f, path.parent, plan, patch)
        self.assertNotEqual(process.returncode, 0)
        before = f.cli(['--ledger', f.ledger, 'export'], {})
        process, result = FailureTests.driver(self, f, path.parent, plan, '')
        self.assertEqual(process.returncode, 0, result)
        self.assertEqual(before, f.cli(['--ledger', f.ledger, 'export'], {}))

    def test_restore_invalidates_pending_removal(self):
        f = self.fixture()
        _, _, prepared = self.prepare(f)
        path, plan = self.preview(f, prepared)
        backup, restored = f.private / 'pre.sqlite', f.private / 'restored.sqlite'
        f.cli(['--ledger', f.ledger, 'backup'], {'path': str(backup)})
        f.cli(['--ledger', restored, 'restore'], {'path': str(backup)})
        before = [Path(t['path']).read_bytes() for t in f.binding['targets']]
        for suffix in ('-wal', '-shm'):
            Path(str(f.ledger) + suffix).unlink(missing_ok=True)
        os.replace(restored, f.ledger)  # Only the isolated fixture's restored copy.
        code, result = f.run('apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan))
        self.assertEqual(result['items'][0]['code'], 'recovery_required')
        self.assertEqual(before, [Path(t['path']).read_bytes() for t in f.binding['targets']])

    def test_duplicate_deferred_successor_does_not_prevent_explicit_removal(self):
        f = self.fixture()
        for d in status_all(f.ctx, 'decisions'):
            body = dict(idempotency_key='defer-' + d['decision_id'],
                        run_id='60000000-0000-4000-8000-000000000003', client='synthetic',
                        workflow_id=f.profile['workflow_id'], profile_revision=1,
                        incarnation=f.incarnation, consumer_id=d['consumer_id'],
                        input_record_id=d['input_record_id'], decision='deferred',
                        reason='Duplicate of another root; retain historical publication.',
                        dependencies=d['dependencies'], previous_decision_id=d['decision_id'], reconsider=True)
            f.cli(['--config', f.config, 'consumer-review'], body, f.ledger)
        _, _, prepared = self.prepare(f)
        path, plan = self.preview(f, prepared)
        self.assertEqual(f.run('apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan))[0], 0)
        _, _, again = f.prepare('publish', 'after-deferred-removal')
        self.assertEqual(json.loads(again.read_text())['items'], [])


if __name__ == '__main__':
    unittest.main()
