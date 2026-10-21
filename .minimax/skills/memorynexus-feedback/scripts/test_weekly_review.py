from datetime import datetime, timedelta
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('weekly_review', Path(__file__).with_name('weekly_review.py'))
weekly = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(weekly)


def observation(key, stamp, **changes):
    record = {'id': key, 'occurred_at': stamp, 'statement': 'synthetic observation',
              'is_current': True, 'state': 'current', 'source': 'owner_report'}
    return {**record, **changes}


class WeeklyReviewTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.reports = Path(self.temp.name) / 'reports'
        self.addCleanup(patch.stopall)
        patch.object(weekly, 'REPORTS', self.reports).start()
        self.period = weekly.period_for('2026-08-17', '2026-08-23')
        self.history = {'observations': [observation('one', '2026-08-17T00:00:00+08:00')]}
        self.review = {'experiments': [], 'outcomes': [], 'evidence_gaps': ['no_confirmed_outcome']}
        patch.object(weekly, 'read_ledger', side_effect=lambda: (self.history, self.review)).start()

    def draft(self):
        data, _ = weekly.evidence(self.history, self.review, self.period)
        return {'period': self.period, 'evidence_fingerprint': data['evidence_fingerprint'],
                'source_ids': ['one'], 'markdown': 'Synthetic reading. Continue?',
                'question': 'Continue?'}

    def previous(self):
        period = weekly.period_for('2026-08-24', '2026-08-30')
        _, sources = weekly.evidence(self.history, self.review, period)
        return weekly.previous_reading(period, sources, self.history)

    def test_local_midnight_and_exclusive_next_day(self):
        self.history['observations'] = [
            observation('before', '2026-08-16T15:59:59Z'),
            observation('start', '2026-08-16T16:00:00Z'),
            observation('end', '2026-08-23T15:59:59Z'),
            observation('after', '2026-08-23T16:00:00Z'),
        ]
        data, _ = weekly.evidence(self.history, self.review, self.period)
        self.assertEqual([r['id'] for r in data['observations']], ['start', 'end'])
        self.assertEqual([r['id'] for r in data['previous_observations']], ['before'])

    def test_default_is_seven_complete_shanghai_days(self):
        period = weekly.period_for()
        today = datetime.now(weekly.ZONE).date()
        self.assertEqual(period['end'], (today - timedelta(days=1)).isoformat())
        self.assertEqual(period['start'], (today - timedelta(days=7)).isoformat())

    def test_current_corrections_only_for_observations_and_outcomes(self):
        self.history['observations'] += [
            observation('old', '2026-08-18T00:00:00+08:00', is_current=False, state='superseded'),
            observation('withdrawn', '2026-08-19T00:00:00+08:00', is_current=False, state='retracted'),
        ]
        self.review['outcomes'] = [
            {'id': 'result-1', 'supersedes_outcome_id': None},
            {'id': 'result-2', 'supersedes_outcome_id': 'result-1'},
            {'id': 'result-3', 'supersedes_outcome_id': 'result-2'},
        ]
        data, sources = weekly.evidence(self.history, self.review, self.period)
        self.assertEqual([r['id'] for r in data['observations']], ['one'])
        self.assertEqual([r['id'] for r in data['review_state']['outcomes']], ['result-3'])
        self.assertNotIn('withdrawn', sources)

    def test_rejects_partial_reversed_future_and_naive_time(self):
        for start, end in [('2026-08-17', None), ('2026-08-23', '2026-08-17'),
                           ('2026-08-17', '9999-01-01')]:
            with self.assertRaises(ValueError):
                weekly.period_for(start, end)
        with self.assertRaises(ValueError):
            weekly.local_date('2026-08-17T00:00:00')

    def test_save_and_recover_in_a_fresh_read(self):
        result = weekly.save_reading(self.draft())
        path = Path(result['path'])
        self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        report = json.loads(path.read_text())
        self.assertEqual(report['authority'], 'agent_interpretation_only')
        self.assertNotIn('statement', report)
        self.assertFalse(result['ledger_written'])
        self.assertIn('Synthetic reading', Path(result['markdown_path']).read_text())
        self.assertEqual(self.previous()['status'], 'valid')
        self.assertEqual(self.previous()['question'], 'Continue?')

    def test_retraction_invalidates_previous_reading_without_returning_text(self):
        weekly.save_reading(self.draft())
        self.history['observations'][0].update(is_current=False, state='retracted')
        prior = self.previous()
        self.assertEqual(prior['status'], 'stale')
        self.assertNotIn('markdown', prior)

    def test_backfilled_observation_invalidates_previous_reading(self):
        weekly.save_reading(self.draft())
        self.history['observations'].append(observation('late', '2026-08-20T00:00:00+08:00'))
        self.assertEqual(self.previous()['status'], 'stale')

    def test_new_week_observation_does_not_invalidate_previous_reading(self):
        weekly.save_reading(self.draft())
        self.history['observations'].append(observation('new', '2026-08-24T00:00:00+08:00'))
        self.assertEqual(self.previous()['status'], 'valid')

    def test_changed_evidence_rejects_save(self):
        draft = self.draft()
        self.history['observations'][0]['statement'] = 'corrected statement'
        with self.assertRaisesRegex(ValueError, 'Evidence changed'):
            weekly.save_reading(draft)
        self.assertFalse(self.reports.exists())

    def test_unknown_duplicate_and_unrelated_citations_rejected(self):
        self.history['observations'].append(observation('outside', '2026-08-01T00:00:00+08:00'))
        for ids in (['invented'], ['one', 'one'], ['outside'], [], 'one'):
            draft = self.draft()
            draft['source_ids'] = ids
            with self.assertRaises(ValueError):
                weekly.save_reading(draft)

    def test_empty_period_does_not_save_a_story(self):
        self.history['observations'] = []
        with self.assertRaisesRegex(ValueError, 'No observations'):
            weekly.save_reading(self.draft())

    def test_two_runs_preserve_both_readings(self):
        first = weekly.save_reading(self.draft())
        second = weekly.save_reading(self.draft())
        self.assertNotEqual(first['path'], second['path'])
        self.assertEqual(len(list(self.reports.glob('*.json'))), 2)

    def test_malformed_reading_does_not_break_next_week(self):
        self.reports.mkdir()
        (self.reports / 'incomplete.json').write_text(json.dumps({
            'kind': weekly.KIND, 'ledger': str(weekly.LEDGER), 'period': self.period,
        }))
        self.assertEqual(self.previous()['status'], 'none')

    def test_context_and_save_invoke_only_ledger_read_commands(self):
        patch.stopall()
        patch.object(weekly, 'REPORTS', self.reports).start()
        ledger = Path(self.temp.name) / 'ledger.sqlite'
        ledger.write_text('untouched synthetic ledger')
        patch.object(weekly, 'LEDGER', ledger).start()
        replies = [self.history, self.review] * 2
        calls = []

        def run(args, **kwargs):
            calls.append(args)
            return weekly.subprocess.CompletedProcess(args, 0, json.dumps({'status': 'ok', **replies.pop(0)}), '')

        patch.object(weekly.subprocess, 'run', side_effect=run).start()
        data = weekly.context('2026-08-17', '2026-08-23')
        draft = {'period': data['period'], 'evidence_fingerprint': data['evidence_fingerprint'],
                 'source_ids': ['one'], 'markdown': 'Reading. Continue?', 'question': 'Continue?'}
        weekly.save_reading(draft)
        self.assertEqual([args[-1] for args in calls], ['observation-history', 'review'] * 2)
        self.assertTrue(all(args[1:3] == ['--ledger', str(ledger)] for args in calls))
        self.assertEqual(ledger.read_text(), 'untouched synthetic ledger')

    def test_missing_ledger_is_not_created(self):
        patch.stopall()
        ledger = Path(self.temp.name) / 'missing.sqlite'
        patch.object(weekly, 'LEDGER', ledger).start()
        with self.assertRaisesRegex(ValueError, 'Formal ledger is missing'):
            weekly.read_ledger()
        self.assertFalse(ledger.exists())


if __name__ == '__main__':
    unittest.main()
