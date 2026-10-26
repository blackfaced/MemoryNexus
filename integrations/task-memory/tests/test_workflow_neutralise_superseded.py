"""An old neutralisation plan must not erase a newer published generation.

Independent review finding (2026-10-26, second patch review), severity P1:

The runner excluded a superseded generation from the neutralisation queue
only while *preparing* a plan (`workflow_engine.py:156-159`).  The apply step
never re-checked whether the block had since been taken over by a newer
generation.  A plan prepared before a newer generation was published could
therefore be applied afterwards and replace that newer generation's body with
the fixed "source evidence changed" text.

Reproduced through the public CLI only: prepare and preview a neutralisation
plan, publish a newer generation that carries the *same* body, then apply the
already-previewed plan.  The same body matters.  It leaves the file bytes
unchanged, so the whole-file baseline recorded in the plan still matches and
`check_snap` cannot stop the write.  A different body changes the file digest
and is refused by the baseline check, which is why the defect was invisible in
the common case.

Approval validity does not catch it either: the maintenance approval records
the validity of the decision it neutralises (stale), and stale is exactly what
a neutralisation is allowed to act on.

These tests pin the contract:

- A neutralisation write re-checks the authoritative successor slot at write
  time, not only at prepare time.
- The same body is covered; the whole-file baseline is not the protection.
- Another target in the same plan still completes the work it was allowed to
  do.
- A plan whose neutralisation intent was created before the newer generation
  appeared (the recovery branch) is still refused.
- A replay of an already-applied neutralisation stays idempotent: the
  historical effect may be recorded again, but it must not obtain a new write
  permission.
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
from test_workflow_publish_supersede import (phase, reviews, reconsider,
                                             decision_id_of, published_text)


ORIGINAL_BODY = 'Source pointer: incident.md; verification pending.\n'


def fresh_dir(f, label):
    """A run directory that does not exist yet.

    `new_run` creates the directory and refuses to reuse one, so a repeated
    step in a test has to get a name of its own.
    """
    candidate = f.state / label
    suffix = 0
    while candidate.exists():
        suffix += 1
        candidate = f.state / (label + '-' + str(suffix))
    return candidate


def preview_only(f, prepared, label):
    """Preview a prepared plan and return the plan path without applying it."""
    submission = f.state / (label + '-submission.json')
    f.write(submission, {'schema_version': 1,
                          'prepared_sha256': hashlib.sha256(prepared.read_bytes()).hexdigest(),
                          'items': []})
    directory = fresh_dir(f, label + '-preview')
    code, result = f.run('preview', '--binding', f.binding_path, '--prepared', prepared,
                         '--submission', submission, '--run-dir', directory)
    if code not in (0, 3):
        raise AssertionError(result)
    plan_path = directory / 'plan.json'
    assert plan_path.exists(), (code, result, sorted(p.name for p in directory.glob('*')))
    return plan_path


def apply_plan(f, plan_path, label):
    """Apply an already-previewed plan."""
    plan = json.loads(plan_path.read_text())
    argv = ['apply', '--binding', f.binding_path, '--plan', plan_path]
    if plan['authorization']['mode'] == 'owner':
        argv += ['--approve', digest(plan)]
    code, result = f.run(*argv)
    return plan, code, result


def published_generation(f):
    """Publish one generation for both consumers."""
    review, review_report = phase(f, 'capture', lambda p: [
        {'unit_id': x['id'],
         'text': 'Source-reported failure; verification pending.',
         'occurred_at': '2026-10-23T00:00:00Z'} for x in p['items']], 'capture')
    review, review_report = phase(f, 'review', reviews, 'review')
    phase(f, 'publish', lambda p: [], 'publish')
    item = next(x for x in review['items'] if x['consumer_id'] == 'mechanism')
    return dict(item, decision_id=decision_id_of(review_report, item['id']))


def stale_neutralisation_plan(f, label):
    """Invalidate the published generation and preview its neutralisation."""
    f.rebind(lambda profile: profile.update({'purpose': 'changed purpose'}))
    code, result, prepared = f.prepare('publish', fresh_dir(f, label + '-prepare'))
    if code not in (0, 3):
        raise AssertionError(result)
    data = json.loads(prepared.read_text())
    neutralise = [x for x in data['items'] if x['purpose'] == 'neutralize']
    assert neutralise, data
    return preview_only(f, prepared, label)


class SupersededNeutralisationPlan(unittest.TestCase):

    def test_old_neutralisation_plan_cannot_erase_the_newer_published_body(self):
        """The reported P1: same body, newer generation, old plan applied."""
        f = Fixture()
        self.addCleanup(f.close)
        item = published_generation(f)

        # The neutralisation plan is prepared and previewed, then left alone.
        old_plan = stale_neutralisation_plan(f, 'old-neutralise')

        # A newer generation is published with the *same* body.  The file bytes
        # therefore do not change and the old plan's whole-file baseline still
        # matches.
        reconsider(f, item, ORIGINAL_BODY)
        new_prepared = fresh_dir(f, 'newer-prepare')
        code, result, prepared = f.prepare('publish', new_prepared)
        self.assertIn(code, (0, 3), result)
        new_plan = preview_only(f, prepared, 'newer-publish')
        _, code, result = apply_plan(f, new_plan, 'newer-publish')
        self.assertEqual(result['status'], 'ok', result)
        body_after_new = published_text(f, 'mechanism_doc')
        self.assertIn('Source pointer: incident.md; verification pending.', body_after_new)

        # Now apply the OLD neutralisation plan.  It must not replace the body.
        plan, code, result = apply_plan(f, old_plan, 'old-neutralise-apply')
        body_now = published_text(f, 'mechanism_doc')
        self.assertNotIn('source evidence changed', body_now,
                         'the old plan must not neutralise the newer generation')
        self.assertIn('Source pointer: incident.md; verification pending.', body_now)

    def test_the_old_plan_reports_a_refusal_rather_than_success(self):
        """The refusal has to be visible, not a silent no-op."""
        f = Fixture()
        self.addCleanup(f.close)
        item = published_generation(f)
        old_plan = stale_neutralisation_plan(f, 'old-neutralise')

        reconsider(f, item, ORIGINAL_BODY)
        code, result, prepared = f.prepare('publish', fresh_dir(f, 'newer-prepare'))
        self.assertIn(code, (0, 3), result)
        new_plan = preview_only(f, prepared, 'newer-publish')
        apply_plan(f, new_plan, 'newer-publish')

        plan, code, result = apply_plan(f, old_plan, 'old-neutralise-apply')
        self.assertNotEqual(result['status'], 'ok',
                            'the old plan must not report overall success')
        refused = result['items'][0]
        self.assertIn(refused['status'], ('needs_review', 'failed'), result)
        self.assertIn(refused.get('code'), ('stale_snapshot', 'approval_required'), result)

    def test_the_other_target_still_completes_its_allowed_work(self):
        """A refused neutralisation must not block an unrelated target.

        The newer generation only concerns the mechanism block.  The lessons
        block is still a legitimate neutralisation, and it must complete on
        its own: the refusal of the mechanism item may not hold the lessons
        item hostage.
        """
        f = Fixture()
        self.addCleanup(f.close)
        item = published_generation(f)
        old_plan = stale_neutralisation_plan(f, 'old-neutralise')

        # Only the mechanism block gets a newer generation.  The lessons block
        # stays a legitimate neutralisation and must still complete.
        reconsider(f, item, ORIGINAL_BODY)
        code, result, prepared = f.prepare('publish', fresh_dir(f, 'newer-prepare'))
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        purposes = {(x['purpose'], x['target_id']) for x in data['items']}
        self.assertIn(('neutralize', 'lessons_doc'), purposes,
                      'the untouched target keeps its legitimate neutralisation')
        new_plan = preview_only(f, prepared, 'newer-publish')
        _, code, result = apply_plan(f, new_plan, 'newer-publish')
        self.assertEqual(result['status'], 'ok', result)
        self.assertIn('source evidence changed', published_text(f, 'lessons_doc'),
                      'the lessons block is a legitimate neutralisation')

        # The old plan must not undo that work either.
        plan, code, result = apply_plan(f, old_plan, 'old-neutralise-apply')
        self.assertNotIn('source evidence changed', published_text(f, 'mechanism_doc'),
                         'the old plan must not neutralise the newer generation')

    def test_neutralisation_intent_created_before_the_newer_generation_is_refused(self):
        """Recovery branch: the intent existed before the newer generation."""
        f = Fixture()
        self.addCleanup(f.close)
        item = published_generation(f)
        old_plan = stale_neutralisation_plan(f, 'old-neutralise')

        # The newer generation appears and publishes before the old plan runs.
        reconsider(f, item, ORIGINAL_BODY)
        code, result, prepared = f.prepare('publish', fresh_dir(f, 'newer-prepare'))
        self.assertIn(code, (0, 3), result)
        new_plan = preview_only(f, prepared, 'newer-publish')
        apply_plan(f, new_plan, 'newer-publish')

        # Replaying the old plan must not gain a new write permission.
        plan, code, result = apply_plan(f, old_plan, 'old-neutralise-replay')
        body_now = published_text(f, 'mechanism_doc')
        self.assertNotIn('source evidence changed', body_now)

    def test_an_applied_neutralisation_stays_idempotent_on_replay(self):
        """The historical effect may be recorded again, but not re-written."""
        f = Fixture()
        self.addCleanup(f.close)
        published_generation(f)

        f.rebind(lambda profile: profile.update({'purpose': 'changed purpose'}))
        code, result, prepared = f.prepare('publish', fresh_dir(f, 'neutralise-prepare'))
        self.assertIn(code, (0, 3), result)
        plan_path = preview_only(f, prepared, 'neutralise-apply')
        plan, code, result = apply_plan(f, plan_path, 'neutralise-apply')
        body = published_text(f, 'mechanism_doc')
        self.assertIn('source evidence changed', body)

        # The same plan replayed: the already applied effect must be reported
        # again without touching the file a second time.
        before = body
        plan2, code2, result2 = apply_plan(f, plan_path, 'neutralise-replay')
        self.assertIn(code2, (0, 3, 4), result2)
        self.assertEqual(published_text(f, 'mechanism_doc'), before,
                         'a replay must not modify the file again')

    def test_a_repeated_prepare_does_not_ask_for_the_same_approval_again(self):
        """R2: a completed neutralisation is not reported as awaiting approval.

        After a successful neutralisation the block holds the fixed
        review-required text, so the region no longer holds the published body.
        The runner must recognise its own completed work instead of asking
        the owner for the same approval again.
        """
        f = Fixture()
        self.addCleanup(f.close)
        published_generation(f)

        f.rebind(lambda profile: profile.update({'purpose': 'changed purpose'}))
        code, result, prepared = f.prepare('publish', fresh_dir(f, 'neutralise-prepare'))
        self.assertIn(code, (0, 3), result)
        plan_path = preview_only(f, prepared, 'neutralise-apply')
        apply_plan(f, plan_path, 'neutralise-apply')
        self.assertIn('source evidence changed', published_text(f, 'mechanism_doc'))

        # A fresh prepare must not report the neutralised decisions as
        # awaiting approval again.
        code, result, prepared = f.prepare('publish', fresh_dir(f, 'after-neutralise'))
        self.assertIn(code, (0, 3), result)
        data = json.loads(prepared.read_text())
        self.assertNotIn('awaiting_approval', [x['reason'] for x in data['remaining']],
                         'a completed neutralisation must not ask for approval again')
        self.assertNotIn('neutralize', [x['purpose'] for x in data['items']],
                         'a completed neutralisation must not be scheduled again')


if __name__ == '__main__':
    unittest.main()
