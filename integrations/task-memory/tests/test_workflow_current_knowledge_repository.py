"""G3: actual ledger, workflow, reader processes and synthetic file effects."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from current_knowledge_fixture import RepositoryFixture
from test_workflow_publish_supersede import phase, reconsider, decision_id_of, reviews
from workflow_types import digest


class RepositoryRead(unittest.TestCase):
    def fixture(self, **kwargs):
        f = RepositoryFixture(**kwargs); self.addCleanup(f.close); return f

    def ok(self, result):
        code, value, run = result
        self.assertEqual((code, value['code']), (0, 'ok'), value)
        return run

    def failure(self, result, expected=None):
        code, value, run = result
        self.assertIn(code, (2, 3), value)
        if expected: self.assertEqual(value['code'], expected, value)
        self.assertFalse((run / 'material.json').exists())
        return run

    def test_both_consumers_attribution_copies_and_readonly_check(self):
        f = self.fixture(); f.seed(); before = f.status()['revision']
        run = self.ok(f.run()); material = f.payload(run); evidence = f.payload(run, 'evidence.json')
        self.assertEqual({o['consumer_id'] for o in material['outputs']}, {'lessons', 'mechanism'})
        self.assertEqual(material['entries'], []); self.assertEqual(material['candidates'], [])
        self.assertEqual({i['trust'] for o in material['outputs'] for i in o['inputs']}, {'source_reported'})
        self.assertTrue(all(o['classification'] == 'consumer_output' for o in material['outputs']))
        self.assertEqual({c['id'] for c in evidence['copies']}, {'lessons_doc', 'mechanism_doc'})
        self.assertTrue(all(c['status'] == 'verified' for c in evidence['copies']))
        for t in f.workflow.binding['targets']:
            self.assertEqual(next(c['sha256'] for c in evidence['copies'] if c['id'] == t['target_id']), hashlib.sha256(Path(t['path']).read_bytes()).hexdigest())
        checked = self.ok(f.run('check', evidence=run / 'evidence.json'))
        self.assertFalse((checked / 'material.json').exists()); self.assertEqual(f.status()['revision'], before)
        (run / 'material.json').unlink()
        self.ok(f.run('check', evidence=run / 'evidence.json'))

    def test_source_change_delete_and_no_implicit_sync(self):
        f = self.fixture(); f.seed(); source = f.root / 'incident.md'; raw = source.read_bytes()
        before = f.status('sources'); run = self.ok(f.run())
        source.write_bytes(b'Changed source without a sync.\n')
        self.failure(f.run(), 'source_drift'); self.failure(f.run('check', evidence=run / 'evidence.json'), 'source_drift')
        self.assertFalse((run / 'material.json').exists()); self.assertEqual(f.status('sources'), before)
        source.unlink(); self.failure(f.run(), 'source_unavailable')
        source.write_bytes(raw); self.ok(f.run())

    def test_target_modified_unidentified_duplicate_blocks_and_human_regions(self):
        f = self.fixture(); f.seed(); target = Path(f.workflow.binding['targets'][0]['path'])
        original = target.read_bytes(); human = b'# Owner notes\nDo not change these.\n'
        target.write_bytes(human + original); self.ok(f.run())
        for changed in (original.replace(b'verification pending', b'false claim'), original + original,
                        original + b'<!-- memorynexus:unknown:begin -->\nstale\n<!-- memorynexus:unknown:end -->\n'):
            target.write_bytes(human + changed)
            failed = self.failure(f.run(), 'target_drift')
            self.assertIn('stale', [c['status'] for c in f.payload(failed, 'evidence.json')['copies']])
            self.assertEqual(target.read_bytes(), human + changed)
        target.write_bytes(human + original); self.ok(f.run())

    def test_multiple_blocks_and_newest_generation_do_not_require_old_whole_file_hash(self):
        f = self.fixture(collections=2); f.seed(); initial = self.ok(f.run())
        self.assertEqual(len(f.payload(initial)['outputs']), 4)
        item = f.review['items'][0]; item = dict(item, decision_id=decision_id_of(f.review_report, item['id']))
        changed = reconsider(f.workflow, item, 'Updated source interpretation; preserve uncertainty.\n')
        self.failure(f.run(), 'target_drift')
        phase(f.workflow, 'publish', lambda p: [], 'replacement')
        run = self.ok(f.run()); outputs = f.payload(run)['outputs']
        self.assertEqual(len(outputs), 4); self.assertIn(changed, [o['decision_id'] for o in outputs])
        self.assertNotIn(item['decision_id'], [o['decision_id'] for o in outputs])
        self.failure(f.run('check', evidence=initial / 'evidence.json'), 'stale_snapshot')

    def test_retained_plan_receipt_request_and_rendered_bytes_are_required(self):
        f = self.fixture(); f.seed(); root = f.workflow.state / 'publish-preview'
        for relative in ('plan.json', 'receipts/consumer-approve-0-0.json', 'receipts/publication-result-0-0.json'):
            path = root / relative; raw = path.read_bytes(); path.unlink()
            self.failure(f.run(), 'source_unavailable'); path.write_bytes(raw); os.chmod(path, 0o600)
        request = root / 'requests/consumer-approve-0-0.json'; raw = request.read_bytes()
        body = json.loads(raw); body['confirmation'] = 'unconfirmed'; f.write(request, body)
        self.failure(f.run(), 'stale_snapshot'); request.write_bytes(raw)
        plan = json.loads((root / 'plan.json').read_bytes()); artifact = root / plan['items'][0]['rendered']['path']
        original = artifact.read_bytes(); artifact.write_bytes(original + b'changed')
        self.failure(f.run(), 'stale_snapshot')

    def test_jsonl_prefix_complete_new_row_and_partial_tail(self):
        f = self.fixture(jsonl=True); f.seed(); self.ok(f.run())
        source = f.root / 'stream.jsonl'; raw = source.read_bytes()
        for changed, code in ((raw.replace(b'first', b'other'), 'source_drift'),
                              (raw + b'{"text":"new row"}\n', 'source_drift'),
                              (raw + b'{"text":', 'incomplete')):
            source.write_bytes(changed); self.failure(f.run(), code)
        source.write_bytes(raw); self.ok(f.run())

    def test_marked_source_selection_and_missing_markers(self):
        f = self.fixture(); w = f.workflow
        w.rebind(lambda p: p['collections'][0].update(collection_id='50000000-0000-4000-8000-000000000099', selector={'kind': 'marked', 'begin': '<source>', 'end': '</source>'}))
        source = f.root / 'incident.md'; source.write_text('outside\n<source>\nselected facts\n</source>\n')
        f.seed(); self.ok(f.run())
        source.write_text('changed outside\n<source>\nselected facts\n</source>\n'); self.ok(f.run())
        source.write_text('changed outside\n<source>\nchanged facts\n</source>\n'); self.failure(f.run(), 'source_drift')
        source.write_text('no selector markers'); self.failure(f.run(), 'source_unavailable')

    def test_external_copies_and_managed_identity_conflicts(self):
        f = self.fixture(); f.seed(); t = f.workflow.binding['targets'][0]
        cid = next(c['consumer_id'] for c in f.workflow.profile['consumers'] if c['target_id'] == t['target_id'])
        f.binding['copies'] = [dict(id=t['target_id'], path=t['path'], role='managed', consumer_id=cid)]; f.save_binding()
        self.ok(f.run())
        f.binding['copies'][0]['id'] = 'other_id'; f.save_binding(); self.failure(f.run(), 'scope_mismatch')
        f.binding['copies'] = [dict(id='old', path=str(f.private / 'unknown-cache'), role='external')]; f.save_binding()
        run = self.failure(f.run(), 'injection_unknown')
        self.assertEqual(f.payload(run, 'evidence.json')['coverage'], 'unknown')

    def test_profile_binding_and_installation_disagreement(self):
        f = self.fixture(); f.seed(); old = self.ok(f.run()); w = f.workflow
        profile = json.loads(w.profile_path.read_bytes()); profile['revision'] += 1; f.write(w.profile_path, profile)
        self.failure(f.run(), 'stale_snapshot'); f.write(w.profile_path, w.profile)
        self.ok(f.run())
        w.rebind(lambda p: None)
        self.failure(f.run('check', evidence=old / 'evidence.json'))
        self.assertFalse((old / 'material.json').exists())
        f.binding['task_id'] = '10000000-0000-4000-8000-000000000001'; f.save_binding()
        self.failure(f.run(), 'scope_mismatch')

    def test_correction_withdrawal_and_old_authority_do_not_expose_output(self):
        f = self.fixture(); f.seed(); old = self.ok(f.run())
        rid = f.payload(old)['outputs'][0]['input_record_id']
        new = f.mutation('correct', target_id=rid, text='Corrected source interpretation.', confirmation='confirmed')['result']['record_id']
        self.failure(f.run(), 'target_drift')
        self.failure(f.run('check', evidence=old / 'evidence.json')); self.assertFalse((old / 'material.json').exists())
        f.cli('retract', dict(idempotency_key='withdraw', run_id='60000000-0000-4000-8000-000000000001', client='synthetic', target_id=new, reason='Fixture withdrawal.', confirmation='confirmed'))
        result = self.failure(f.run())
        for path in result.iterdir(): self.assertNotIn(b'Source pointer:', path.read_bytes())

    def test_auxiliary_record_and_source_dependency_closure(self):
        f = self.fixture(collections=2); f.seed(); w = f.workflow
        inputs = f.status('inputs')['result']['items']; auxiliary = inputs[1]
        item = next(x for x in f.review['items'] if x['input_record_id'] != auxiliary['id'])
        item = dict(item, decision_id=decision_id_of(f.review_report, item['id']))
        item['dependencies'].append(dict(type='record', id=auxiliary['id'], sha256=hashlib.sha256(auxiliary['text'].encode()).hexdigest()))
        reconsider(w, item, 'Interpretation with auxiliary evidence.\n')
        phase(w, 'publish', lambda p: [], 'auxiliary')
        run = self.ok(f.run()); outputs = f.payload(run)['outputs']
        self.assertTrue(any(len(o['inputs']) == 2 for o in outputs))
        (f.root / w.profile['collections'][1]['path']).write_text('Auxiliary evidence changed.')
        self.failure(f.run(), 'source_drift')

    def test_paged_inputs_and_strict_repository_request(self):
        f = self.fixture(); f.seed()
        for i in range(24): f.record(text='Additional current record ' + str(i))
        self.ok(f.run())
        for update in ({'limit': 20}, {'consumer_ids': ['lessons', 'lessons']}, {'purpose': 'history'}):
            f.write_request(**update); self.failure(f.run(), 'invalid_input')
        f.write_request(consumer_ids=['unknown']); self.failure(f.run(), 'scope_mismatch')

    def test_source_and_target_symlinks_are_never_followed(self):
        f = self.fixture(); f.seed(); source = f.root / 'incident.md'; saved = f.private / 'saved-source'
        source.rename(saved); source.symlink_to(saved); self.failure(f.run(), 'invalid_input')
        source.unlink(); saved.rename(source)
        target = Path(f.workflow.binding['targets'][0]['path']); target.rename(saved); target.symlink_to(saved)
        self.failure(f.run())
        self.assertTrue(saved.is_file())

    def test_revision_change_between_pages_retries_once_then_stops(self):
        f = self.fixture(); f.page_barrier(); f.seed()
        for i in range(21): f.record(text='Paging record ' + str(i))
        (f.private / 'armed').touch()
        args, run = f.command()
        child = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.addCleanup(lambda: child.kill() if child.poll() is None else None)
        for n in (1, 2):
            deadline = time.monotonic() + 12
            while not (f.private / ('page-ready-' + str(n))).exists():
                self.assertIsNone(child.poll()); self.assertLess(time.monotonic(), deadline); time.sleep(.01)
            f.record(text='Concurrent commit ' + str(n))
            (f.private / ('page-release-' + str(n))).touch()
        out, err = child.communicate(timeout=15)
        self.failure((child.returncode, json.loads(out), run), 'stale_snapshot')
        calls = [json.loads(line) for line in (f.private / 'calls.jsonl').read_text().splitlines()]
        self.assertEqual(sum('after' in c for c in calls), 2)
        self.assertTrue(all('at_revision' in c for c in calls if c.get('view')))

    def test_task_mode_with_repository_binding_still_checks_kernel_agreement(self):
        f = self.fixture(); f.record()
        f.write(f.request_path, dict(schema_version=1, mode='task', purpose='current', limit=20))
        self.ok(f.run())
        w = f.workflow; w.binding['timeout_seconds'] -= 1; w.write(w.binding_path, w.binding)
        self.failure(f.run(), 'stale_snapshot')

    def test_restored_incarnation_cannot_reuse_repository_approval(self):
        f = self.fixture(); f.seed(); read = self.ok(f.run())
        before = f.payload(read, 'evidence.json')
        backup = f.private / 'backup-current.sqlite'; restored = f.private / 'restored.sqlite'
        f.storage('backup', {'path': str(backup)}); f.storage('restore', {'path': str(backup)}, restored)
        for suffix in ('-wal', '-shm'): Path(str(f.ledger) + suffix).unlink(missing_ok=True)
        os.replace(restored, f.ledger)
        header = f.cli('workflow-status', {})
        self.assertEqual(header['revision'], before['revision']); self.assertNotEqual(header['incarnation'], before['incarnation'])
        self.failure(f.run('check', evidence=read / 'evidence.json'), 'stale_snapshot')
        self.assertFalse((read / 'material.json').exists())
        self.failure(f.run(), 'stale_snapshot')

    def test_candidate_dependency_retains_trust_and_complete_record_closure(self):
        f = self.fixture(); f.seed()
        execution = f.record('An attributed execution.', kind='execution')
        candidate = f.mutation('review', source_ids=[execution], text='CANDIDATE_BODY_NOT_DELIVERED')['result']['record_id']
        item = f.review['items'][0]; item = dict(item, decision_id=decision_id_of(f.review_report, item['id']))
        item['dependencies'].append(dict(type='record', id=candidate, sha256=hashlib.sha256(b'CANDIDATE_BODY_NOT_DELIVERED').hexdigest()))
        reconsider(f.workflow, item, 'Approved interpretation with attributed dependencies.\n')
        phase(f.workflow, 'publish', lambda p: [], 'candidate-publication')
        run = self.ok(f.run()); material = f.payload(run)
        inputs = next(o['inputs'] for o in material['outputs'] if o['consumer_id'] == item['consumer_id'])
        self.assertIn(candidate, [i['id'] for i in inputs]); self.assertIn(execution, [i['id'] for i in inputs])
        self.assertEqual(next(i['trust'] for i in inputs if i['id'] == candidate), 'review_candidate')
        self.assertNotIn('CANDIDATE_BODY_NOT_DELIVERED', (run / 'material.json').read_text())


if __name__ == '__main__': unittest.main()
