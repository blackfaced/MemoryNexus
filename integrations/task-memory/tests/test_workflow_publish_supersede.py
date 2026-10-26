"""A superseded generation must publish over its own block instead of neutralising it.

Real incident (jhwg, 2026-10-26): a published entry was corrected with the
public `consumer-review` action and `reconsider=true`.  The runner's publish
preparation then classified the superseded generation as `neutralize`
(workflow_engine.py:137) and put it first in the per-target queue, so the
corrected generation was never scheduled.

Two independent defects produced that:

1. A decision whose block is superseded by a newer generation was still
   classified `neutralize`.  The newer generation publishes over the same
   `block_id` - `workflow_io.render` replaces that block region in place and
   the Rust `consumer-approve` for a non-maintenance publication does not
   require a prior neutralisation.  A superseded generation is not "source
   evidence changed", so it must not neutralise its own block.
2. A candidate that failed its own precondition still consumed the per-target
   slot (`queued.add(target)` ran before the precondition check), so a later
   candidate for the same target could not be scheduled even when it was
   valid.  In a target file that holds more than one block the neutralise
   precondition can never hold (it compares the whole-file digest with the
   digest recorded when that block was published), which made the queue
   permanently stuck.

These tests pin the contract: one publish item per target per plan, the newest
generation of a block wins, untouched entries stay untouched, and a genuine
"source evidence changed" case still neutralises.
"""
import hashlib
import json
import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from workflow_fixture import Fixture
from workflow_types import digest


def phase(f, workflow, items_builder, label):
    code, result, prepared = f.prepare(workflow, label + '-prepare')
    if code not in (0, 3):
        raise AssertionError(result)
    data = json.loads(prepared.read_text())
    submission = f.state / (label + '-submission.json')
    f.write(submission, {'schema_version': 1,
                          'prepared_sha256': hashlib.sha256(prepared.read_bytes()).hexdigest(),
                          'items': items_builder(data)})
    directory = f.state / (label + '-preview')
    code, result = f.run('preview', '--binding', f.binding_path, '--prepared', prepared,
                         '--submission', submission, '--run-dir', directory)
    if code not in (0, 3):
        raise AssertionError(result)
    plan_path = directory / 'plan.json'
    plan = json.loads(plan_path.read_text())
    argv = ['apply', '--binding', f.binding_path, '--plan', plan_path]
    if plan['authorization']['mode'] == 'owner':
        argv += ['--approve', digest(plan)]
    code, result = f.run(*argv)
    if code not in (0, 3, 4):
        raise AssertionError(result)
    return plan, result


def decision_id_of(report, item_id):
    """The decision id the apply step created for one review item."""
    for item in report['items']:
        if item.get('id') == item_id and item.get('receipt_ids'):
            return item['receipt_ids'][0]
    raise AssertionError((item_id, report))


def reviews(p):
    return [{'input_record_id': x['input_record_id'],
             'consumer_id': x['consumer_id'],
             'decision': 'proposed',
             'reason': 'Use the original source pointer; retain uncertainty.',
             'dependencies': x['dependencies'],
             'output_text': 'Source pointer: incident.md; verification pending.'}
            for x in p['items']]


def reconsider(f, item, body_text, label='reconsider'):
    """Public CLI consumer-review with reconsider=true, plus the output artifact."""
    raw = body_text.encode()
    body = {'idempotency_key': 'reconsider-' + label,
            'run_id': '60000000-0000-4000-8000-000000000002',
            'client': 'synthetic',
            'workflow_id': f.profile['workflow_id'],
            'profile_revision': f.profile['revision'],
            'incarnation': f.incarnation,
            'consumer_id': item['consumer_id'],
            'input_record_id': item['input_record_id'],
            'decision': 'proposed',
            'reason': 'Correct the published wording; keep the source pointer.',
            'dependencies': item['dependencies'],
            'output_sha256': hashlib.sha256(raw).hexdigest(),
            'previous_decision_id': item['decision_id'],
            'reconsider': True}
    receipt = f.cli(['--config', f.config, 'consumer-review'], body, f.ledger)
    decision_id = receipt['result']['decision_id']
    run = f.state / label
    run.mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(run, 0o700)
    directory = run / 'outputs'
    directory.mkdir(mode=0o700, exist_ok=True)
    os.chmod(directory, 0o700)
    artifact = directory / (decision_id + '.txt')
    artifact.write_bytes(raw)
    os.chmod(artifact, 0o600)
    return decision_id


def published_text(f, target_id):
    return Path(next(t['path'] for t in f.binding['targets'] if t['target_id'] == target_id)).read_text()


def managed_body(text, block_id):
    """The managed block body exactly as the renderer wrote it."""
    begin = '<!-- memorynexus:' + block_id + ':begin -->\n'
    end = '<!-- memorynexus:' + block_id + ':end -->'
    start = text.index(begin) + len(begin)
    stop = text.index(end)
    return text[start:stop]


class SupersededPublish(unittest.TestCase):

    def test_corrected_generation_publishes_instead_of_neutralising_its_own_block(self):
        """The owner's actual case: correct one published entry, keep the rest."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        review, review_report = phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish')
        item = next(x for x in review['items'] if x['consumer_id'] == 'mechanism')
        item = dict(item, decision_id=decision_id_of(review_report, item['id']))
        corrected = reconsider(f, item, 'Corrected entry: source pointer; verification pending.\n')

        code, result, prepared = f.prepare('publish', 'corrected-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        self.assertEqual([x['purpose'] for x in data['items']], ['publish'],
                         'a superseded generation must not neutralise its own block')
        self.assertEqual(data['items'][0]['decision_id'], corrected)
        # The superseded generation must leave the queue entirely: it may not be
        # scheduled as a neutralisation and it may not sit in `remaining` as a
        # pending neutralisation either.
        self.assertNotIn(item['decision_id'], [x['decision_id'] for x in data['items']])
        self.assertNotIn(item['decision_id'], [x['id'] for x in data['remaining']])
        self.assertNotIn('awaiting_approval', [x['reason'] for x in data['remaining']])

        plan, _ = phase(f, 'publish', lambda p: [], 'corrected-publish')
        self.assertEqual(len(plan['items']), 1)
        body = published_text(f, 'mechanism_doc')
        self.assertIn('Corrected entry: source pointer; verification pending.', body)
        self.assertNotIn('Source pointer: incident.md; verification pending.', body)

    def test_superseded_generation_publishes_even_when_the_target_file_holds_more_blocks(self):
        """A drifted multi-block target must not deadlock the publish queue."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        review, review_report = phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish')
        item = next(x for x in review['items'] if x['consumer_id'] == 'mechanism')
        item = dict(item, decision_id=decision_id_of(review_report, item['id']))
        # a second block lands in the same target file afterwards: the file digest no
        # longer equals the digest recorded when the first block was published
        path = Path(next(t['path'] for t in f.binding['targets'] if t['target_id'] == 'mechanism_doc'))
        with path.open('a') as handle:
            handle.write('<!-- memorynexus:0000000000000000000000000000000000000000000000000000000000000001:begin -->\n'
                         'Another published entry.\n'
                         '<!-- memorynexus:0000000000000000000000000000000000000000000000000000000000000001:end -->\n')
        corrected = reconsider(f, item, 'Corrected entry: source pointer; verification pending.\n')

        code, result, prepared = f.prepare('publish', 'drifted-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        self.assertEqual([x['purpose'] for x in data['items']], ['publish'], data)
        self.assertEqual(data['items'][0]['decision_id'], corrected)

    def test_unrelated_published_entries_are_not_rescheduled(self):
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        review, review_report = phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish')
        lessons_before = published_text(f, 'lessons_doc')
        item = next(x for x in review['items'] if x['consumer_id'] == 'mechanism')
        item = dict(item, decision_id=decision_id_of(review_report, item['id']))
        reconsider(f, item, 'Corrected entry: source pointer; verification pending.\n')

        code, result, prepared = f.prepare('publish', 'single-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        self.assertEqual(len(data['items']), 1)
        self.assertEqual(data['items'][0]['consumer_id'], 'mechanism')
        self.assertEqual(data['items'][0]['purpose'], 'publish',
                         'the corrected entry publishes; the untouched entry is not rescheduled')
        self.assertEqual(published_text(f, 'lessons_doc'), lessons_before)

    def test_genuine_source_change_still_neutralises_the_block(self):
        """A stale generation with no newer generation keeps the neutralize path."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish')
        f.rebind(lambda profile: profile.update({'purpose': 'changed purpose'}))

        code, result, prepared = f.prepare('publish', 'neutralize-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        self.assertTrue(data['items'], 'a rebind must still schedule neutralisation')
        for item in data['items']:
            self.assertEqual(item['purpose'], 'neutralize')

    def test_never_published_generation_is_not_neutralised(self):
        """A proposal that was never published has nothing to neutralise."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        review, review_report = phase(f, 'review', reviews, 'review')
        item = next(x for x in review['items'] if x['consumer_id'] == 'mechanism')
        item = dict(item, decision_id=decision_id_of(review_report, item['id']))
        corrected = reconsider(f, item, 'Corrected entry: source pointer; verification pending.\n')

        code, result, prepared = f.prepare('publish', 'unpublished-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        purposes = [x['purpose'] for x in data['items']]
        self.assertNotIn('neutralize', purposes, data)
        self.assertIn(corrected, [x['decision_id'] for x in data['items']], data)


    def test_manual_text_outside_the_block_region_survives_a_publish(self):
        """Contract runner-v1.md:266 - preserve every other byte including human text."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        review, review_report = phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish')
        item = next(x for x in review['items'] if x['consumer_id'] == 'mechanism')
        item = dict(item, decision_id=decision_id_of(review_report, item['id']))
        path = Path(next(t['path'] for t in f.binding['targets'] if t['target_id'] == 'mechanism_doc'))
        with path.open('a') as handle:
            handle.write('Human note: keep this line verbatim.\n')
        reconsider(f, item, 'Corrected entry: source pointer; verification pending.\n')

        phase(f, 'publish', lambda p: [], 'corrected-publish')
        body = path.read_text()
        self.assertIn('Human note: keep this line verbatim.', body)
        self.assertIn('Corrected entry: source pointer; verification pending.', body)

    def test_edited_block_is_not_erased_by_neutralisation(self):
        """Contract runner-v1.md:280 - an edited block needs owner review."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        review, review_report = phase(f, 'review', reviews, 'review')
        phase(f, 'publish', lambda p: [], 'publish')
        item = next(x for x in review['items'] if x['consumer_id'] == 'mechanism')
        item = dict(item, decision_id=decision_id_of(review_report, item['id']))
        path = Path(next(t['path'] for t in f.binding['targets'] if t['target_id'] == 'mechanism_doc'))
        edited = path.read_text().replace('Source pointer: incident.md; verification pending.',
                                          'Human edit: I changed this line by hand.')
        path.write_text(edited)
        # A rebind invalidates every decision, so the published generation becomes a
        # neutralisation candidate with no newer generation to supersede it.
        f.rebind(lambda profile: profile.update({'purpose': 'changed purpose'}))

        code, result, prepared = f.prepare('publish', 'edited-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        self.assertNotIn(('mechanism_doc', 'neutralize'),
                         [(x['target_id'], x['purpose']) for x in data['items']])
        self.assertIn(item['decision_id'], [x['id'] for x in data['remaining']])
        self.assertIn('awaiting_approval', [x['reason'] for x in data['remaining']])
        self.assertIn('Human edit: I changed this line by hand.', path.read_text())

    def test_published_block_body_ends_with_exactly_one_lf(self):
        """Contract runner-v1.md:255 - the managed body ends in exactly one LF."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        phase(f, 'review', reviews, 'review')
        plan, _ = phase(f, 'publish', lambda p: [], 'publish')
        item = next(x for x in plan['items'] if x['target_id'] == 'mechanism_doc')
        body = managed_body(published_text(f, 'mechanism_doc'), item['block_id'])
        self.assertEqual(body, body.rstrip('\n') + '\n')
        self.assertFalse(body.endswith('\n\n'))
        self.assertEqual(item['output_text'], item['output_text'].rstrip('\n') + '\n')
        self.assertEqual(item['output_sha256'], hashlib.sha256(item['output_text'].encode()).hexdigest())

    def test_submitted_trailing_newlines_normalise_to_exactly_one_lf(self):
        """The review plan carries the canonical single-LF body."""
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')

        def trailing(p):
            return [{'input_record_id': x['input_record_id'],
                     'consumer_id': x['consumer_id'],
                     'decision': 'proposed',
                     'reason': 'Use the original source pointer; retain uncertainty.',
                     'dependencies': x['dependencies'],
                     'output_text': 'Trailing newline probe.\n\n\n'} for x in p['items']]

        review_plan, _ = phase(f, 'review', trailing, 'review')
        for item in review_plan['items']:
            self.assertEqual(item['output_text'], 'Trailing newline probe.\n')
        publish_plan, _ = phase(f, 'publish', lambda p: [], 'publish')
        item = next(x for x in publish_plan['items'] if x['target_id'] == 'mechanism_doc')
        body = managed_body(published_text(f, 'mechanism_doc'), item['block_id'])
        self.assertEqual(body, 'Trailing newline probe.\n')


if __name__ == '__main__':
    unittest.main()
