"""Selected publication freshness and explicit renewal through real CLI processes."""
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import unittest
from test_workflow_publish_supersede import phase, reviews
from workflow_fixture import Fixture
from workflow_cli import status_all
from workflow_engine import envelope
from workflow_types import digest


class PublicationPreflight(unittest.TestCase):
    def fixture(self):
        f = Fixture(collections=2)
        self.addCleanup(f.close)
        f.capture('capture')
        phase(f, 'review', reviews, 'review')
        self.decision = status_all(f.ctx, 'decisions')[0]
        return f

    def preview(self, f, label, decisions=None):
        selection = f.state / (label + '-selection.json')
        f.write(selection, dict(schema_version=1, items=[dict(
            decision_id=d['decision_id'], purpose='publish') for d in decisions or [self.decision]]))
        run = f.state / (label + '-prepare')
        code, result = f.run('prepare', '--binding', f.binding_path, '--workflow', 'publish',
                             '--client', 'synthetic', '--publish-selection', selection, '--run-dir', run)
        self.assertEqual(code, 3, result)
        prepared = run / 'prepared.json'
        submission = f.state / (label + '-submission.json')
        f.write(submission, dict(schema_version=1,
                                prepared_sha256=hashlib.sha256(prepared.read_bytes()).hexdigest(), items=[]))
        run = f.state / (label + '-preview')
        code, result = f.run('preview', '--binding', f.binding_path, '--prepared', prepared,
                             '--submission', submission, '--run-dir', run)
        self.assertEqual(code, 3, result)
        path = run / 'plan.json'
        return path, json.loads(path.read_text())

    def approval_request(self, plan):
        item = plan['items'][0]
        body = envelope(plan, 'consumer-approve', 0)
        proposal = {k: item[k] for k in ('target_id', 'block_id', 'base_exists',
                                        'base_sha256', 'after_sha256', 'output_sha256')}
        proposal['plan_sha256'] = digest(plan)
        body.update(decision_id=item['decision_id'], decision_sha256=item['decision_sha256'],
                    proposal=proposal, confirmation='confirmed')
        if 'previous_approval_id' in item:
            body['previous_approval_id'] = item['previous_approval_id']
        return body

    def apply(self, f, path, plan):
        return f.run('apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan))

    def export(self, f):
        return f.cli(['--ledger', f.ledger, 'export'], {})

    def target(self, f):
        target = next(c['target_id'] for c in f.profile['consumers']
                      if c['consumer_id'] == self.decision['consumer_id'])
        return Path(next(t['path'] for t in f.binding['targets']
                         if t['target_id'] == target))

    def test_unrelated_collection_change_does_not_block_selected_publish(self):
        f = self.fixture()
        path, plan = self.preview(f, 'selected')
        unrelated = f.root / f.profile['collections'][1]['path']
        unrelated.write_text('Unrelated work continued after preview.\n')
        code, result = self.apply(f, path, plan)
        self.assertIn(code, (0, 4), result)
        self.assertEqual(result['items'][0]['status'], 'ok', result)
        pubs = status_all(f.ctx, 'publications')
        self.assertEqual(len(pubs), 1)
        self.assertEqual(pubs[0]['publication'], 'applied')
        self.assertEqual(self.target(f).read_bytes(), (path.parent / 'rendered/u0.md').read_bytes())

    def test_related_source_or_target_drift_refuses_before_new_intent(self):
        for change in ('source', 'target'):
            with self.subTest(change=change):
                f = self.fixture()
                path, plan = self.preview(f, 'selected')
                if change == 'source':
                    (f.root / 'incident.md').write_text('Related evidence changed.\n')
                else:
                    self.target(f).write_text('Owner text.\n')
                before = self.export(f)
                code, result = self.apply(f, path, plan)
                self.assertNotEqual(code, 0, result)
                self.assertEqual(result['items'][0]['code'], 'stale_snapshot')
                self.assertEqual(self.export(f), before)
                if change == 'target':
                    self.assertEqual(self.target(f).read_text(), 'Owner text.\n')
                else:
                    self.assertFalse(self.target(f).exists())

    def test_pending_intent_renews_with_fresh_digest_and_retains_history(self):
        f = self.fixture()
        old_path, old_plan = self.preview(f, 'old')
        request = self.approval_request(old_plan)
        old_id = f.cli(['--config', f.config, 'consumer-approve'], request, f.ledger)['result']['approval_id']
        self.target(f).write_text('Human text to preserve.\n')
        path, plan = self.preview(f, 'renew')
        self.assertEqual(plan['items'][0].get('previous_approval_id'), old_id)
        before = self.export(f)
        for approval in ([], ['--approve', digest(old_plan)]):
            code, result = f.run('apply', '--binding', f.binding_path, '--plan', path, *approval)
            self.assertNotEqual(code, 0, result)
            self.assertEqual(self.export(f), before)
        forged = copy.deepcopy(plan)
        forged['authorization'] = dict(mode='profile', profile_revision=f.profile['revision'])
        forged_path = path.parent / 'profile-plan.json'; f.write(forged_path, forged)
        code, result = f.run('apply', '--binding', f.binding_path, '--plan', forged_path)
        self.assertEqual(result['items'][0]['code'], 'approval_required')
        self.assertEqual(self.export(f), before)
        code, result = self.apply(f, path, plan)
        self.assertIn(code, (0, 4), result)
        self.assertEqual(result['items'][0]['status'], 'ok', result)
        pubs = status_all(f.ctx, 'publications')
        self.assertEqual(len(pubs), 2)
        old, new = pubs
        self.assertEqual(old['approval_id'], old_id)
        self.assertEqual(old['validity'], 'stale')
        self.assertEqual(old['publication'], 'not_applied')
        self.assertEqual(new['publication'], 'applied')
        self.assertEqual(status_all(f.ctx, 'summary')[0]['counts']['pending_publications'], 0)
        from workflow_recovery import prepare_recovery
        self.assertEqual(prepare_recovery(f.ctx, pubs), ([], []))
        after = self.export(f)
        self.assertEqual(len(after['result']['consumer_decisions']), len(before['result']['consumer_decisions']))
        self.assertEqual(f.cli(['--config', f.config, 'consumer-approve'], request, f.ledger)['status'], 'replayed')
        changed = copy.deepcopy(request)
        changed['proposal']['plan_sha256'] = 'a' * 64
        with self.assertRaisesRegex(AssertionError, 'conflict'):
            f.cli(['--config', f.config, 'consumer-approve'], changed, f.ledger)
        self.assertEqual(after, self.export(f))
        raw = self.target(f).read_bytes()
        self.assertNotEqual(self.apply(f, old_path, old_plan)[0], 0)
        self.assertEqual(raw, self.target(f).read_bytes())
        self.assertEqual(after, self.export(f))
        read_state = f.base / 'read-runs'; read_state.mkdir(mode=0o700)
        read_binding = {k: f.binding[k] for k in ('schema_version', 'task_id', 'task_root', 'config',
                       'ledger', 'binary', 'binary_sha256', 'python', 'package', 'package_sha256', 'timeout_seconds')}
        read_binding.update(state_root=str(read_state), client='synthetic', copies=[],
                            repository_binding=str(f.binding_path))
        read_config = f.private / 'read-binding.json'; f.write(read_config, read_binding)
        read_request = f.private / 'read-request.json'
        f.write(read_request, dict(schema_version=1, mode='repository', purpose='current',
                                   consumer_ids=[self.decision['consumer_id']]))
        read_run = read_state / 'renewed'
        read = subprocess.run([f.binding['python'], '-B', str(f.package / 'current_knowledge.py'),
                               'read', '--binding', str(read_config), '--request', str(read_request),
                               '--run-dir', str(read_run)], capture_output=True,
                              env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))
        self.assertEqual(read.returncode, 0, read.stdout + read.stderr)
        material = json.loads((read_run / 'material.json').read_text())
        self.assertEqual(len(material['outputs']), 1)
        self.assertEqual(material['outputs'][0]['approval_id'], new['approval_id'])
        self.assertEqual(after, self.export(f))
        backup = f.private / 'renewed.snapshot.sqlite'
        restored = f.private / 'renewed.restored.sqlite'
        f.cli(['--ledger', f.ledger, 'backup'], dict(path=str(backup)))
        f.cli(['--ledger', restored, 'restore'], dict(path=str(backup)))
        recovered = f.cli(['--ledger', restored, 'export'], {})
        expected_publications = copy.deepcopy(after['result']['publications'])
        for p in expected_publications:
            p['data']['validity'] = 'stale'
        self.assertEqual(recovered['result']['publications'], expected_publications)
        self.assertEqual(recovered['result']['receipts'], after['result']['receipts'])
        rejected = self.approval_request(plan)
        rejected['idempotency_key'] += '-restored'
        rejected['incarnation'] = recovered['result']['ledger_meta'][0]['incarnation']
        with self.assertRaisesRegex(AssertionError, 'recovery_required'):
            f.cli(['--config', f.config, 'consumer-approve'], rejected, restored)
        self.assertEqual(recovered, f.cli(['--ledger', restored, 'export'], {}))

    def test_recorded_failed_or_conflicted_intent_can_be_explicitly_renewed(self):
        for outcome in ('failed', 'conflict'):
            with self.subTest(outcome=outcome):
                f = self.fixture()
                _, old = self.preview(f, 'old')
                request = self.approval_request(old)
                aid = f.cli(['--config', f.config, 'consumer-approve'], request, f.ledger)['result']['approval_id']
                self.target(f).write_text('Owner baseline after interrupted operation.\n')
                result = envelope(old, 'publication-result', 0)
                result.update(approval_id=aid, outcome=outcome, observed_at=old['created_at'],
                              error_code='io_error' if outcome == 'failed' else 'baseline_changed',
                              observed_sha256=hashlib.sha256(self.target(f).read_bytes()).hexdigest())
                f.cli(['--config', f.config, 'publication-result'], result, f.ledger)
                path, plan = self.preview(f, 'renew')
                self.assertEqual(plan['items'][0]['previous_approval_id'], aid)
                self.assertEqual(self.apply(f, path, plan)[1]['items'][0]['status'], 'ok')
                pubs = status_all(f.ctx, 'publications')
                self.assertEqual([p['publication'] for p in pubs], [outcome, 'applied'])
                self.assertEqual(pubs[0]['validity'], 'stale')

    def test_competing_and_invalid_renewals_cannot_replace_authority(self):
        f = self.fixture()
        old_path, old_plan = self.preview(f, 'old')
        old_request = self.approval_request(old_plan)
        old_id = f.cli(['--config', f.config, 'consumer-approve'], old_request, f.ledger)['result']['approval_id']
        path, plan = self.preview(f, 'successor')
        _, competitor = self.preview(f, 'competitor')
        request = self.approval_request(plan)
        before = self.export(f)
        bad = [dict(previous_approval_id='10000000-0000-4000-8000-000000000099'),
               dict(incarnation='10000000-0000-4000-8000-000000000099'),
               dict(profile_revision=999)]
        for n, changes in enumerate(bad):
            invalid = copy.deepcopy(request)
            invalid.update(changes, idempotency_key=request['idempotency_key'] + '-bad-' + str(n))
            with self.assertRaises(AssertionError):
                f.cli(['--config', f.config, 'consumer-approve'], invalid, f.ledger)
            self.assertEqual(before, self.export(f))
        changed = copy.deepcopy(request)
        changed['proposal']['output_sha256'] = 'a' * 64
        with self.assertRaisesRegex(AssertionError, 'conflict'):
            f.cli(['--config', f.config, 'consumer-approve'], changed, f.ledger)
        self.assertEqual(before, self.export(f))
        new_id = f.cli(['--config', f.config, 'consumer-approve'], request, f.ledger)['result']['approval_id']
        after = self.export(f)
        pubs = status_all(f.ctx, 'publications')
        self.assertEqual(pubs[0]['superseded_by'], new_id)
        self.assertEqual(pubs[1]['previous_approval_id'], old_id)
        with self.assertRaisesRegex(AssertionError, 'conflict'):
            f.cli(['--config', f.config, 'consumer-approve'], self.approval_request(competitor), f.ledger)
        same = copy.deepcopy(request); same['idempotency_key'] += '-same'
        self.assertEqual(f.cli(['--config', f.config, 'consumer-approve'], same, f.ledger)['status'], 'noop')
        self.assertEqual(after, self.export(f))
        # Even when successor bytes equal predecessor bytes, the old plan is
        # retired: no late file write or publication-result under its authority.
        self.assertEqual(self.apply(f, path, plan)[1]['items'][0]['status'], 'ok')
        final = self.export(f)
        raw = self.target(f).read_bytes()
        self.assertNotEqual(self.apply(f, old_path, old_plan)[0], 0)
        self.assertEqual(final, self.export(f))
        self.assertEqual(raw, self.target(f).read_bytes())
        applied = copy.deepcopy(request)
        applied.update(previous_approval_id=new_id, idempotency_key=request['idempotency_key'] + '-applied')
        applied['proposal']['plan_sha256'] = digest(competitor)
        with self.assertRaisesRegex(AssertionError, 'conflict'):
            f.cli(['--config', f.config, 'consumer-approve'], applied, f.ledger)
        self.assertEqual(final, self.export(f))

    def test_after_bytes_without_intent_do_not_bypass_initial_preflight(self):
        f = self.fixture()
        path, plan = self.preview(f, 'selected')
        self.target(f).write_bytes((path.parent / 'rendered/u0.md').read_bytes())
        before = self.export(f)
        self.assertNotEqual(self.apply(f, path, plan)[0], 0)
        self.assertEqual(before, self.export(f))

    def test_auxiliary_dependency_is_checked_only_for_its_consumer(self):
        f = Fixture(collections=2); self.addCleanup(f.close)
        f.capture('capture')
        primary = status_all(f.ctx, 'inputs')[0]
        auxiliary = status_all(f.ctx, 'sources')[1]
        consumers = f.profile['consumers']
        def build(data):
            items = []
            for w in data['items']:
                if w['input_record_id'] != primary['id']:
                    continue
                deps = list(w['dependencies'])
                if w['consumer_id'] == consumers[1]['consumer_id']:
                    deps.append(dict(type='source', id=auxiliary['source_id'],
                                     revision=auxiliary['source_revision'], sha256=auxiliary['sha256']))
                items.append(dict(input_record_id=w['input_record_id'], consumer_id=w['consumer_id'],
                                  decision='proposed', reason='Selected evidence.', dependencies=deps,
                                  output_text='A source pointer; uncertainty retained.'))
            return items
        phase(f, 'review', build, 'review')
        decisions = status_all(f.ctx, 'decisions')
        self.decision = decisions[0]
        path, plan = self.preview(f, 'both-consumers', decisions)
        (f.root / f.profile['collections'][1]['path']).write_text('Auxiliary correction changed.\n')
        code, report = self.apply(f, path, plan)
        self.assertEqual(code, 4, report)
        self.assertEqual(report['items'][0]['status'], 'ok')
        self.assertEqual(report['items'][1]['code'], 'stale_snapshot')
        self.assertEqual(len(status_all(f.ctx, 'publications')), 1)
        self.assertEqual(status_all(f.ctx, 'publications')[0]['decision_id'], decisions[0]['decision_id'])

    def test_jsonl_registered_tail_append_and_prefix_identity(self):
        for mutation in ('append', 'rewrite'):
            with self.subTest(mutation=mutation):
                f = Fixture(); self.addCleanup(f.close)
                source = f.root / 'events.jsonl'
                raw = b''.join(json.dumps(dict(text='row-%02d' % i)).encode() + b'\n' for i in range(22))
                source.write_bytes(raw)
                f.rebind(lambda p: p['collections'][0].update(collection_id='50000000-0000-4000-8000-000000000099',
                                                            format='append_jsonl', path='events.jsonl',
                                                            selector=dict(kind='fields', keys=['text']),
                                                            extractor_version='jsonl-v1'))
                f.capture('first'); f.capture('second')
                rec = status_all(f.ctx, 'inputs')[-1]
                out = f.state / 'retained' / 'outputs'; out.mkdir(parents=True, mode=0o700)
                out.parent.chmod(0o700)
                sources = status_all(f.ctx, 'sources')
                for n, consumer in enumerate(f.profile['consumers']):
                    text = 'Registered row 21 pointer, uncertainty retained; lens ' + str(n)
                    deps = [dict(type='record', id=rec['id'], sha256=hashlib.sha256(rec['text'].encode()).hexdigest())]
                    deps += [dict(type='source', id=s['source_id'], revision=s['source_revision'], sha256=s['sha256'])
                             for s in (sources[:18] if n == 0 else sources[-18:])]
                    result = f.cli(['--config', f.config, 'consumer-review'], dict(
                        idempotency_key='tail-review-' + str(n), run_id='60000000-0000-4000-8000-000000000099',
                        client='synthetic', workflow_id=f.profile['workflow_id'],
                        profile_revision=f.profile['revision'], incarnation=f.incarnation,
                        consumer_id=consumer['consumer_id'], input_record_id=rec['id'],
                        decision='proposed', reason='Registered tail row.', dependencies=deps,
                        output_sha256=hashlib.sha256(text.encode()).hexdigest()), f.ledger)
                    p = out / (result['result']['decision_id'] + '.txt'); p.write_text(text); p.chmod(0o600)
                decisions = status_all(f.ctx, 'decisions')
                self.decision = decisions[0]
                path, plan = self.preview(f, 'tail', decisions)
                units = plan['snapshot']['scans'][0]['units']
                self.assertEqual(len(units), 22)
                self.assertGreater(max(u['start'] for u in units), len(raw) * 20 // 22)
                source.write_bytes(raw + b'{"text":"appended"}\n' if mutation == 'append' else raw.replace(b'row-00', b'row-XX'))
                before = self.export(f)
                code, report = self.apply(f, path, plan)
                if mutation == 'append':
                    self.assertEqual(report['items'][0]['status'], 'ok', report)
                else:
                    self.assertEqual(report['items'][0]['code'], 'stale_snapshot')
                    self.assertEqual(before, self.export(f))
if __name__ == '__main__':
    unittest.main()
