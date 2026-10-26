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


def publish_prepared(f, prepared, label):
    """Preview and apply an already prepared publish plan."""
    submission = f.state / (label + '-submission.json')
    f.write(submission, {'schema_version': 1,
                          'prepared_sha256': hashlib.sha256(prepared.read_bytes()).hexdigest(),
                          'items': []})
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
    return plan, code, result


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


    def test_neutralisation_applies_and_replaces_the_published_body(self):
        """A neutralisation plan must be executable, not only preparable.

        The maintenance approval records the validity of the decision it
        neutralises, so it is stale by construction.  The publish path required a
        current approval, which made every neutralisation fail with
        `stale_snapshot` and left the old body in place.
        """
        f = Fixture()
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        review, review_report = phase(f, 'review', reviews, 'review')
        plan, _ = phase(f, 'publish', lambda p: [], 'publish')
        item = next(x for x in plan['items'] if x['target_id'] == 'mechanism_doc')
        self.assertIn('Source pointer: incident.md; verification pending.',
                      managed_body(published_text(f, 'mechanism_doc'), item['block_id']))
        # A rebind invalidates every decision, so the published generation becomes a
        # neutralisation candidate with no newer generation to supersede it.
        f.rebind(lambda profile: profile.update({'purpose': 'changed purpose'}))

        code, result, prepared = f.prepare('publish', 'neutralize-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        neutralize = [x for x in data['items']
                      if x['purpose'] == 'neutralize' and x['target_id'] == 'mechanism_doc']
        self.assertEqual(len(neutralize), 1, data)

        plan, code, result = publish_prepared(f, prepared, 'neutralize-publish')
        self.assertIn(code, (0, 3, 4), result)
        self.assertEqual(managed_body(published_text(f, 'mechanism_doc'), item['block_id']),
                         'MemoryNexus: source evidence changed; review required.\n')

    def test_earlier_block_of_a_multi_block_target_can_still_be_neutralised(self):
        """The neutralisation precondition must compare the block, not the whole file.

        The precondition compared the whole-file digest with the digest recorded
        when that block was published, so in a file holding more than one block
        every block except the last one could never be neutralised.
        """
        f = Fixture(collections=2)
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [{'unit_id': x['id'],
                                         'text': 'Source-reported failure; verification pending.',
                                         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
        phase(f, 'review', reviews, 'review')
        plan1, _ = phase(f, 'publish', lambda p: [], 'publish-one')
        plan2, _ = phase(f, 'publish', lambda p: [], 'publish-two')
        first = next(x for x in plan1['items'] if x['target_id'] == 'mechanism_doc')
        second = next(x for x in plan2['items'] if x['target_id'] == 'mechanism_doc')
        self.assertNotEqual(first['block_id'], second['block_id'])

        f.rebind(lambda profile: profile.update({'purpose': 'changed purpose'}))
        code, result, prepared = f.prepare('publish', 'multi-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        neutralize = [x for x in data['items']
                      if x['purpose'] == 'neutralize' and x['target_id'] == 'mechanism_doc']
        self.assertEqual(len(neutralize), 1, data)
        self.assertEqual(neutralize[0]['block_id'], first['block_id'])

        plan, code, result = publish_prepared(f, prepared, 'multi-publish')
        self.assertIn(code, (0, 3, 4), result)
        text = published_text(f, 'mechanism_doc')
        self.assertEqual(managed_body(text, first['block_id']),
                         'MemoryNexus: source evidence changed; review required.\n')
        self.assertIn('Source pointer: incident.md; verification pending.',
                      managed_body(text, second['block_id']))

    def test_superseded_generation_leaves_the_queue_without_its_local_output(self):
        """Supersession must not depend on a local artifact that can be lost.

        The corrected generation had already been published, so the published
        generation must stay out of the neutralisation queue even after the
        local output artifact of the corrected generation disappears.
        """
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
        phase(f, 'publish', lambda p: [], 'corrected-publish')
        for artifact in f.state.glob('*/outputs/' + corrected + '.txt'):
            artifact.unlink()

        code, result, prepared = f.prepare('publish', 'lost-output-prepare')
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        self.assertNotIn('neutralize', [x['purpose'] for x in data['items']])
        self.assertNotIn(item['decision_id'], [x['id'] for x in data['remaining']])


class NeutralizationLifecycle(unittest.TestCase):
    def fixture(self, collections=1):
        f = Fixture(collections=collections)
        self.addCleanup(f.close)
        phase(f, 'capture', lambda p: [dict(unit_id=x['id'], text='Attributed report.',
              occurred_at='2026-10-23T00:00:00Z') for x in p['items']], 'capture')
        phase(f, 'review', reviews, 'review')
        for i in range(collections):
            phase(f, 'publish', lambda p: [], 'publish-' + str(i))
        f.rebind(lambda p: p.update(purpose='Reassess the existing source reports.'))
        return f

    def preview(self, f, label):
        code, report, prepared = f.prepare('publish', label + '-prepare')
        self.assertIn(code, (0, 3), report)
        submission = f.state / (label + '-submission.json')
        f.write(submission, dict(schema_version=1,
              prepared_sha256=hashlib.sha256(prepared.read_bytes()).hexdigest(), items=[]))
        run = f.state / (label + '-preview')
        code, report = f.run('preview', '--binding', f.binding_path, '--prepared', prepared,
                             '--submission', submission, '--run-dir', run)
        self.assertIn(code, (0, 3), report)
        return run, json.loads((run / 'plan.json').read_text())

    def new_reviews(self, p, consumer=None):
        items = reviews(p)
        for item, work in zip(items, p['items']):
            if 'previous_decision_id' in work:
                item['previous_decision_id'] = work['previous_decision_id']
        return [i for i in items if consumer is None or i['consumer_id'] == consumer]

    def test_old_neutral_plan_cannot_erase_same_bytes_new_generation(self):
        f = self.fixture()
        run, old = self.preview(f, 'old-neutral')
        phase(f, 'review', self.new_reviews, 'new-review')
        phase(f, 'publish', lambda p: [], 'new-publish')
        before = {t['path']: Path(t['path']).read_bytes() for t in f.binding['targets']}
        for item in old['items']:
            target = next(t['path'] for t in f.binding['targets'] if t['target_id'] == item['target_id'])
            self.assertEqual(hashlib.sha256(before[target]).hexdigest(), item['base_sha256'])
        code, report = f.run('apply', '--binding', f.binding_path, '--plan', run / 'plan.json')
        self.assertNotEqual(code, 0, report)
        self.assertTrue(all(i.get('code') == 'stale_snapshot' for i in report['items']), report)
        for path, raw in before.items():
            self.assertEqual(Path(path).read_bytes(), raw)

    def test_lost_intent_response_does_not_authorize_neutralizing_successor(self):
        from workflow_cli import load_context
        from workflow_engine import envelope, header, staged
        f = self.fixture()
        run, old = self.preview(f, 'old-neutral')
        for n in ('requests', 'receipts', 'outputs'):
            (run / n).mkdir(mode=0o700)
        ctx = load_context(f.binding_path); header(ctx)
        item = old['items'][0]
        proposal = {k: item[k] for k in ('target_id', 'block_id', 'base_exists',
                    'base_sha256', 'after_sha256', 'output_sha256')}
        proposal['plan_sha256'] = digest(old)
        request = envelope(old, 'consumer-approve', 0)
        request.update(decision_id=item['decision_id'], decision_sha256=item['decision_sha256'],
                       proposal=proposal, maintenance='neutralize')
        staged(ctx, run, old, 'consumer-approve', 0, request)
        request_path = run / 'requests/consumer-approve-0-0.json'
        retained = request_path.read_bytes()
        (run / 'receipts/consumer-approve-0-0.json').unlink()
        phase(f, 'review', lambda p: self.new_reviews(p, item['consumer_id']), 'new-review')
        target = next(t['path'] for t in f.binding['targets'] if t['target_id'] == item['target_id'])
        before = Path(target).read_bytes()
        code, report = f.run('apply', '--binding', f.binding_path, '--plan', run / 'plan.json')
        self.assertNotEqual(code, 0, report)
        self.assertEqual(report['items'][0].get('code'), 'stale_snapshot', report)
        self.assertEqual(report['items'][1]['status'], 'ok', report)
        self.assertEqual(Path(target).read_bytes(), before)
        self.assertEqual(request_path.read_bytes(), retained)

    def test_applied_neutral_replay_never_rewrites_new_generation(self):
        f = self.fixture()
        run, old = self.preview(f, 'neutral')
        code, report = f.run('apply', '--binding', f.binding_path, '--plan', run / 'plan.json')
        self.assertEqual(code, 0, report)
        phase(f, 'review', self.new_reviews, 'new-review')
        phase(f, 'publish', lambda p: [], 'new-publish')
        before = {t['path']: Path(t['path']).read_bytes() for t in f.binding['targets']}
        code, report = f.run('apply', '--binding', f.binding_path, '--plan', run / 'plan.json')
        self.assertEqual(code, 0, report)
        for path, raw in before.items():
            self.assertEqual(Path(path).read_bytes(), raw)
        self.assertFalse((run / 'requests/publication-result-0-1.json').exists())

    def test_lost_result_replay_completes_without_rewriting_new_generation(self):
        from test_workflow_failures import FailureTests
        f = self.fixture()
        run, old = self.preview(f, 'neutral')
        patch = """original=e.call
lost=[False]
def drop(ctx,action,body):
    response=original(ctx,action,body)
    if action=='publication-result' and not lost[0]:
        lost[0]=True
        raise OSError('lost actual response')
    return response
e.call=drop
"""
        process, report = FailureTests.driver(self, f, run, old, patch)
        self.assertNotEqual(process.returncode, 0, report)
        request = run / 'requests/publication-result-0-0.json'
        retained = request.read_bytes()
        phase(f, 'review', self.new_reviews, 'new-review')
        phase(f, 'publish', lambda p: [], 'new-publish')
        before = {t['path']: Path(t['path']).read_bytes() for t in f.binding['targets']}
        code, report = f.run('apply', '--binding', f.binding_path, '--plan', run / 'plan.json')
        self.assertEqual(code, 0, report)
        self.assertEqual(request.read_bytes(), retained)
        for path, raw in before.items():
            self.assertEqual(Path(path).read_bytes(), raw)

    def test_completed_neutralization_is_quiet_and_preserves_human_edits(self):
        f = self.fixture()
        phase(f, 'publish', lambda p: [], 'neutral')
        target = Path(f.binding['targets'][0]['path'])
        target.write_text('Owner introduction.\n' + target.read_text())
        code, report, prepared = f.prepare('publish', 'repeat')
        data = json.loads(prepared.read_text())
        self.assertEqual(code, 0, report)
        self.assertEqual(data['items'], [])
        self.assertEqual(data['remaining'], [])
        # Quiet publication does not resolve the outstanding review decision.
        _, _, review = f.prepare('review', 'review-still-needed')
        self.assertEqual(len(json.loads(review.read_text())['items']), 2)
        target.write_text(target.read_text().replace('review required.', 'Owner edited the block.'))
        before = target.read_bytes()
        _, _, prepared = f.prepare('publish', 'edited-neutral')
        data = json.loads(prepared.read_text())
        self.assertEqual(data['items'], [])
        self.assertIn('awaiting_approval', [r['reason'] for r in data['remaining']])
        self.assertEqual(target.read_bytes(), before)

    def test_multiblock_neutralization_advances_and_then_is_quiet(self):
        f = self.fixture(collections=2)
        first, _ = phase(f, 'publish', lambda p: [], 'neutral-first')
        second, _ = phase(f, 'publish', lambda p: [], 'neutral-second')
        self.assertTrue(set(i['block_id'] for i in first['items']).isdisjoint(
                        i['block_id'] for i in second['items']))
        code, report, prepared = f.prepare('publish', 'neutral-done')
        self.assertEqual(code, 0, report)
        self.assertEqual(json.loads(prepared.read_text())['remaining'], [])

    def test_stale_successor_publication_keeps_older_generation_out_of_queue(self):
        f = self.fixture()
        phase(f, 'review', self.new_reviews, 'new-review')
        new, _ = phase(f, 'publish', lambda p: [], 'new-publish')
        f.rebind(lambda p: p.update(purpose='Reassess again.'))
        code, report, prepared = f.prepare('publish', 'latest-only')
        self.assertIn(code, (0, 3), report)
        data = json.loads(prepared.read_text())
        self.assertEqual({x['decision_id'] for x in data['items']},
                         {x['decision_id'] for x in new['items']})
        self.assertEqual(data['remaining'], [])


if __name__ == '__main__':
    unittest.main()
