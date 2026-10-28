"""Explicit selection reaches publish work behind neutralization without applying it."""
import json
import unittest
from pathlib import Path
from test_workflow_publish_supersede import phase, reviews, publish_prepared
from workflow_fixture import Fixture
from workflow_cli import status_all
from workflow_types import digest


class Selection(unittest.TestCase):
    def fixture(self):
        f = Fixture(collections=2)
        self.addCleanup(f.close)
        f.capture('capture')
        phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish-first')
        pubs = status_all(f.ctx, 'publications')
        published = {p['decision_id'] for p in pubs}
        ds = status_all(f.ctx, 'decisions')
        self.selected = [d for d in ds if d['decision_id'] not in published]
        self.old = [d for d in ds if d['decision_id'] in published]
        for d in self.old:
            f.cli(['--config', f.config, 'consumer-review'], dict(
                idempotency_key='defer-' + d['decision_id'], run_id='60000000-0000-4000-8000-000000000004',
                client='synthetic', workflow_id=f.profile['workflow_id'], profile_revision=1,
                incarnation=f.incarnation, consumer_id=d['consumer_id'], input_record_id=d['input_record_id'],
                decision='deferred', reason='Duplicate; do not neutralize merely to unblock queue.',
                dependencies=d['dependencies'], previous_decision_id=d['decision_id'], reconsider=True), f.ledger)
        return f

    def select(self, f, items, label='selected', *extra):
        selection = f.state / (label + '.json')
        f.write(selection, dict(schema_version=1, items=items))
        run = f.state / (label + '-prepare')
        code, report = f.run('prepare', '--binding', f.binding_path, '--workflow', 'publish',
                             '--client', 'synthetic', '--publish-selection', selection, '--run-dir', run, *extra)
        return code, report, run / 'prepared.json'

    def test_selected_publications_bypass_both_neutralizations(self):
        f = self.fixture()
        _, _, default = f.prepare('publish', 'default')
        self.assertEqual([x['purpose'] for x in json.loads(default.read_text())['items']], ['neutralize', 'neutralize'])
        before = status_all(f.ctx, 'publications')
        items = [dict(decision_id=d['decision_id'], purpose='publish') for d in self.selected]
        code, report, prepared = self.select(f, items)
        self.assertEqual(code, 3, report)
        data = json.loads(prepared.read_text())
        self.assertEqual({x['decision_id'] for x in data['items']}, {x['decision_id'] for x in items})
        self.assertEqual({x['reason'] for x in data['remaining']}, {'unselected'})
        plan, code, report = publish_prepared(f, prepared, 'selected')
        self.assertEqual(plan['authorization']['mode'], 'owner')
        self.assertIn(code, (0, 4), report)  # Unselected work remains pending.
        after = status_all(f.ctx, 'publications')
        self.assertEqual(after[:2], before)
        self.assertEqual(len(after), 4)
        for t in f.binding['targets']:
            text = Path(t['path']).read_text()
            self.assertNotIn('review required.', text)
            self.assertEqual(text.count(':begin -->'), 2)
        _, _, repeated = f.prepare('publish', 'repeated')
        again = json.loads(repeated.read_text())
        self.assertEqual([x['purpose'] for x in again['items']], ['neutralize', 'neutralize'])
        self.assertFalse({x['decision_id'] for x in items} & {x['decision_id'] for x in again['items']})

    def test_invalid_selection_never_falls_back_to_fifo(self):
        f = self.fixture()
        chosen = dict(decision_id=self.selected[0]['decision_id'], purpose='publish')
        other = next(d for d in self.old if d['consumer_id'] == self.selected[0]['consumer_id'])
        bad = [[], [chosen, chosen], [dict(decision_id='10000000-0000-4000-8000-000000000099', purpose='publish')],
               [dict(chosen, purpose='remove')], [chosen, dict(decision_id=other['decision_id'], purpose='neutralize')]]
        before = f.cli(['--ledger', f.ledger, 'export'], {})
        targets = [Path(t['path']).read_bytes() for t in f.binding['targets']]
        for i, items in enumerate(bad):
            with self.subTest(items=items):
                code, report, prepared = self.select(f, items, 'bad-' + str(i))
                self.assertEqual(code, 2, report)
                self.assertFalse(prepared.exists())
        self.assertEqual(before, f.cli(['--ledger', f.ledger, 'export'], {}))
        self.assertEqual(targets, [Path(t['path']).read_bytes() for t in f.binding['targets']])

    def test_missing_output_is_not_bypassed(self):
        f = self.fixture()
        d = self.selected[0]
        for p in f.state.glob('*/outputs/' + d['decision_id'] + '.txt'):
            p.unlink()  # Isolated retained-output loss.
        code, report, prepared = self.select(f, [dict(decision_id=d['decision_id'], purpose='publish')])
        self.assertEqual(code, 2, report)
        self.assertFalse(prepared.exists())

    def test_remove_duplicates_then_publish_from_fresh_baselines(self):
        from test_workflow_removal import Removal
        f = self.fixture()
        _, _, prepared = Removal.prepare(self, f)
        path, plan = Removal.preview(self, f, prepared)
        code, report = f.run('apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan))
        self.assertEqual(code, 0, report)
        selection = [dict(decision_id=d['decision_id'], purpose='publish') for d in self.selected]
        code, report, prepared = self.select(f, selection)
        self.assertEqual(code, 3, report)
        _, code, report = publish_prepared(f, prepared, 'replacement')
        self.assertEqual(code, 0, report)
        for t in f.binding['targets']:
            self.assertEqual(Path(t['path']).read_text().count(':begin -->'), 1)
        self.assertEqual(len(status_all(f.ctx, 'publications')), 6)
        _, _, again = f.prepare('publish', 'done')
        self.assertEqual(json.loads(again.read_text())['items'], [])


if __name__ == '__main__':
    unittest.main()
