"""Renew removal through public CLI/runner; preserve failed intent and receipts."""
import copy
import hashlib
import json
import unittest
from pathlib import Path
import test_workflow_removal as removal
from workflow_cli import status_all
from workflow_engine import envelope
from workflow_types import digest


class Reapproval(unittest.TestCase):
    fixture = removal.Removal.fixture

    def plan(self, f, approval, label):
        code, result, prepared = removal.Removal.prepare(self, f, label, [approval])
        self.assertEqual(code, 3, result)
        submission = f.state / (label + '-submission.json')
        f.write(submission, dict(schema_version=1, prepared_sha256=hashlib.sha256(prepared.read_bytes()).hexdigest(), items=[]))
        run = f.state / (label + '-preview')
        code, result = f.run('preview', '--binding', f.binding_path, '--prepared', prepared,
                             '--submission', submission, '--run-dir', run)
        self.assertEqual(code, 3, result)
        path = run / 'plan.json'
        return path, json.loads(path.read_text())

    def apply(self, f, path, plan):
        return f.run('apply', '--binding', f.binding_path, '--plan', path, '--approve', digest(plan))

    def request(self, plan):
        item = plan['items'][0]
        body = envelope(plan, 'consumer-approve', 0)
        proposal = {k: item[k] for k in ('target_id', 'block_id', 'base_exists', 'base_sha256', 'after_sha256', 'output_sha256')}
        proposal['plan_sha256'] = digest(plan)
        body.update(decision_id=item['decision_id'], decision_sha256=item['decision_sha256'], proposal=proposal,
                    maintenance='remove', confirmation='confirmed', previous_approval_id=item['previous_approval_id'],
                    reason=item['reason'], prior_block_sha256=item['prior_block_sha256'])
        return body

    def test_multiple_conflicts_fresh_owner_digest_and_historical_replay(self):
        f = self.fixture()
        prior = status_all(f.ctx, 'publications')[0]
        target = Path(next(t['path'] for t in f.binding['targets'] if t['target_id'] == prior['proposal']['target_id']))
        old_plans = []
        for i in range(2):
            path, plan = self.plan(f, prior, 'conflict-' + str(i))
            target.write_bytes(target.read_bytes() + f'Human {i}\n'.encode())
            raw = target.read_bytes()
            code, result = self.apply(f, path, plan)
            self.assertNotEqual(code, 0, result)
            self.assertEqual(result['items'][0]['code'], 'baseline_changed')
            self.assertEqual(target.read_bytes(), raw)
            old_plans.append((path, plan))
        history = status_all(f.ctx, 'publications')
        removals = [p for p in history if p.get('maintenance') == 'remove']
        self.assertEqual(len(removals), 2)
        self.assertEqual([p['publication'] for p in removals], ['conflict', 'conflict'])
        path, plan = self.plan(f, prior, 'success')
        before = f.cli(['--ledger', f.ledger, 'export'], {})
        for approval in ([], ['--approve', digest(old_plans[-1][1])]):
            code, result = f.run('apply', '--binding', f.binding_path, '--plan', path, *approval)
            self.assertNotEqual(code, 0, result)
            self.assertEqual(before, f.cli(['--ledger', f.ledger, 'export'], {}))
        self.assertEqual(self.apply(f, path, plan)[0], 0)
        self.assertEqual(target.read_bytes(), b'Human 0\nHuman 1\n')
        after = status_all(f.ctx, 'publications')
        self.assertEqual(after[:len(history)], history)
        self.assertEqual(after[-1]['publication'], 'applied')
        self.assertNotIn(after[-1]['approval_id'], {p['approval_id'] for p in history})
        final = f.cli(['--ledger', f.ledger, 'export'], {})
        for old_path, old_plan in old_plans:
            # Same old key replays its old approval, not a renewed authorization.
            req = json.loads((old_path.parent / 'requests/consumer-approve-0-0.json').read_text())
            replay = f.cli(['--config', f.config, 'consumer-approve'], req, f.ledger)
            self.assertEqual(replay['status'], 'replayed')
            changed = copy.deepcopy(req); changed['reason'] = 'Changed under same key'
            with self.assertRaisesRegex(AssertionError, 'conflict'):
                f.cli(['--config', f.config, 'consumer-approve'], changed, f.ledger)
            # Matching an older failure with a new key cannot bypass applied removal.
            changed = copy.deepcopy(req); changed['idempotency_key'] += '-new'
            with self.assertRaisesRegex(AssertionError, 'conflict'):
                f.cli(['--config', f.config, 'consumer-approve'], changed, f.ledger)
            self.assertNotEqual(self.apply(f, old_path, old_plan)[0], 0)
        self.assertEqual(self.apply(f, path, plan)[0], 0)
        self.assertEqual(final, f.cli(['--ledger', f.ledger, 'export'], {}))
        self.assertEqual(target.read_bytes(), b'Human 0\nHuman 1\n')

    def test_pending_blocks_competition_failed_allows_new_approval(self):
        f = self.fixture()
        prior = status_all(f.ctx, 'publications')[0]
        p1, plan1 = self.plan(f, prior, 'first')
        r1 = self.request(plan1)
        a1 = f.cli(['--config', f.config, 'consumer-approve'], r1, f.ledger)['result']['approval_id']
        p2, plan2 = self.plan(f, prior, 'second')
        r2 = self.request(plan2)
        before = f.cli(['--ledger', f.ledger, 'export'], {})
        with self.assertRaisesRegex(AssertionError, 'conflict'):
            f.cli(['--config', f.config, 'consumer-approve'], r2, f.ledger)
        self.assertEqual(before, f.cli(['--ledger', f.ledger, 'export'], {}))
        result = envelope(plan1, 'publication-result', 0)
        result.update(approval_id=a1, outcome='failed', observed_at=plan1['created_at'], error_code='io_error')
        f.cli(['--config', f.config, 'publication-result'], result, f.ledger)
        a2 = f.cli(['--config', f.config, 'consumer-approve'], r2, f.ledger)['result']['approval_id']
        self.assertNotEqual(a1, a2)
        old = copy.deepcopy(r1); old['idempotency_key'] += '-different-key'
        with self.assertRaisesRegex(AssertionError, 'conflict'):
            f.cli(['--config', f.config, 'consumer-approve'], old, f.ledger)
        # Original bytes/key still replay while the new approval is pending.
        self.assertEqual(f.cli(['--config', f.config, 'consumer-approve'], r1, f.ledger)['status'], 'replayed')
        self.assertEqual(self.apply(f, p2, plan2)[0], 0)
        pubs = status_all(f.ctx, 'publications')
        self.assertEqual([p['publication'] for p in pubs if p.get('maintenance') == 'remove'], ['failed', 'applied'])


if __name__ == '__main__':
    unittest.main()
