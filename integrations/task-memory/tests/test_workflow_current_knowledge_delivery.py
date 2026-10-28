"""G4: exact input bytes and a fresh child's observed action, never client labels."""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from current_knowledge_fixture import CurrentFixture, RepositoryFixture, install_bridge, attach_workflow
from test_workflow_publish_supersede import phase, reviews


class Delivery(unittest.TestCase):
    def fixture(self, repository=False):
        f = RepositoryFixture() if repository else CurrentFixture()
        self.addCleanup(f.close); return f

    def ok(self, result):
        code, value, run = result
        self.assertEqual((code, value['code']), (0, 'ok'), value)
        return run

    def failure(self, result, expected):
        code, value, run = result
        self.assertIn(code, (2, 3), value); self.assertEqual(value['code'], expected, value)
        self.assertFalse((run / 'material.json').exists()); return run

    def trace(self, f):
        path = f.private / 'bridge-trace.jsonl'
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def instruction(self, f, name='b'): return f.record('Read workspace-' + name + '/marker.txt')

    def launch(self, f, read): return f.run('launch', evidence=read / 'evidence.json')

    def test_exact_bytes_fresh_action_auth_environment_and_one_attempt(self):
        f = self.fixture(); install_bridge(f); self.instruction(f)
        read = self.ok(f.run()); raw = (read / 'material.json').read_bytes()
        args, launch = f.command('launch', evidence=read / 'evidence.json')
        p = subprocess.run(args, capture_output=True, timeout=20, env=dict(os.environ, MN_SYNTHETIC_AUTH='synthetic-secret', MEMORYNEXUS_TASK_LEDGER='/wrong.sqlite'))
        self.ok((p.returncode, json.loads(p.stdout), launch))
        trace = self.trace(f); self.assertEqual(len(trace), 1)
        self.assertEqual((trace[0]['sha256'], trace[0]['bytes']), (hashlib.sha256(raw).hexdigest(), len(raw)))
        self.assertEqual(trace[0]['opened'], 'workspace-b/marker.txt'); self.assertEqual(trace[0]['argc'], 1)
        self.assertEqual(trace[0]['cwd'], str(f.root)); self.assertTrue(trace[0]['ledger_matches']); self.assertTrue(trace[0]['inherited_auth'])
        delivery = f.payload(launch, 'delivery.json'); self.assertEqual(delivery['status'], 'acknowledged')
        self.assertEqual(delivery['material_sha256'], trace[0]['sha256']); self.assertFalse((read / 'material.json').exists())
        self.assertFalse((launch / 'material.json').exists())
        for path in launch.iterdir(): self.assertNotIn(b'synthetic-secret', path.read_bytes())
        self.failure(self.launch(f, read), 'delivery_already_attempted'); self.assertEqual(len(self.trace(f)), 1)

    def test_correct_a_to_b_across_task_and_both_repository_consumers(self):
        f = self.fixture(); w = attach_workflow(f); install_bridge(f)
        old_id = self.instruction(f, 'a')
        f.mutation('review', source_ids=[old_id], text='Old workspace-a/marker.txt candidate')
        def decisions(prepared):
            return [{**row, 'output_text': 'Read workspace-a/marker.txt'} for row in reviews(prepared)]
        phase(w, 'review', decisions, 'a-review'); phase(w, 'publish', lambda p: [], 'a-publish')
        old_task = self.ok(f.run())
        repo_request = dict(schema_version=1, mode='repository', purpose='current', consumer_ids=['lessons', 'mechanism'])
        f.write(f.request_path, repo_request); old_repo = self.ok(f.run())
        corrected = f.mutation('correct', target_id=old_id, text='Read workspace-b/marker.txt', confirmation='confirmed')['result']['record_id']
        self.failure(self.launch(f, old_task), 'stale_snapshot'); self.failure(self.launch(f, old_repo), 'stale_snapshot')
        self.assertEqual(self.trace(f), [])
        f.write_request(); task = self.ok(f.run()); self.ok(self.launch(f, task))
        def new_decisions(prepared):
            rows = reviews(prepared)
            for row, original in zip(rows, prepared['items']):
                row['output_text'] = 'Read workspace-b/marker.txt'
                if 'previous_decision_id' in original: row['previous_decision_id'] = original['previous_decision_id']
            return rows
        phase(w, 'review', new_decisions, 'b-review'); phase(w, 'publish', lambda p: [], 'b-publish')
        f.write(f.request_path, repo_request); repo = self.ok(f.run())
        material = f.payload(repo); self.assertEqual(len(material['outputs']), 2)
        self.assertTrue(all(o['input_record_id'] == corrected for o in material['outputs']))
        self.ok(self.launch(f, repo))
        trace = self.trace(f); self.assertEqual(len(trace), 2)
        self.assertEqual({t['opened'] for t in trace}, {'workspace-b/marker.txt'})
        marker_sha = hashlib.sha256((f.root / 'workspace-b/marker.txt').read_bytes()).hexdigest()
        self.assertTrue(all(t['marker_sha256'] == marker_sha for t in trace))
        # The separately initialized task X never receives A's instruction bytes.
        x = f.cli('context', {'limit': 20}, config=f.other)
        self.assertNotIn('workspace-', json.dumps(x))

    def test_missing_or_changed_bridge_never_claims_or_spawns(self):
        for changed in (False, True):
            with self.subTest(changed=changed):
                f = self.fixture(); self.instruction(f)
                if changed: path = install_bridge(f)
                read = self.ok(f.run())
                if changed: path.write_text(path.read_text() + '\n# changed after read\n')
                self.failure(self.launch(f, read), 'bridge_unavailable')
                self.assertFalse((read / 'delivery-started.json').exists()); self.assertEqual(self.trace(f), [])
                self.assertFalse((read / 'material.json').exists())

    def test_bad_or_missing_ack_is_unknown_and_cannot_retry(self):
        for mode in ('wrong_hash', 'wrong_id', 'extra', 'long_session', 'no_ack', 'nonzero', 'duplicate', 'overflow', 'stderr'):
            with self.subTest(mode=mode):
                f = self.fixture(); install_bridge(f, mode); self.instruction(f)
                read = self.ok(f.run()); launch = self.failure(self.launch(f, read), 'delivery_unknown')
                self.assertEqual(f.payload(launch, 'delivery.json')['status'], 'unknown')
                self.assertNotIn('session_ref', f.payload(launch, 'delivery.json'))
                self.assertFalse((read / 'material.json').exists())
                self.failure(self.launch(f, read), 'delivery_already_attempted'); self.assertEqual(len(self.trace(f)), 1)
                for path in launch.iterdir(): self.assertNotIn(b'synthetic-secret', path.read_bytes())

    def test_timeout_never_restarts_child(self):
        f = self.fixture(); install_bridge(f, 'timeout'); f.binding['timeout_seconds'] = 1; f.save_binding(); self.instruction(f)
        read = self.ok(f.run()); before = time.monotonic()
        self.failure(self.launch(f, read), 'delivery_unknown'); self.assertLess(time.monotonic() - before, 5)
        self.failure(self.launch(f, read), 'delivery_already_attempted'); self.assertEqual(len(self.trace(f)), 1)

    def wait_trace(self, f, child):
        until = time.monotonic() + 12
        while not self.trace(f):
            self.assertIsNone(child.poll()); self.assertLess(time.monotonic(), until); time.sleep(.01)
        return self.trace(f)[0]

    def test_concurrent_launches_claim_original_read_directory(self):
        f = self.fixture(); install_bridge(f, 'barrier'); self.instruction(f); read = self.ok(f.run())
        args, run = f.command('launch', evidence=read / 'evidence.json')
        child = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.addCleanup(lambda: child.kill() if child.poll() is None else None)
        self.wait_trace(f, child)
        claim = f.payload(read, 'delivery-started.json'); self.assertEqual(claim['run_dir'], str(run))
        self.assertEqual((read / 'delivery-started.json').stat().st_mode & 0o777, 0o600)
        self.failure(self.launch(f, read), 'delivery_already_attempted')
        (f.private / 'bridge-release').touch(); out, err = child.communicate(timeout=15)
        self.ok((child.returncode, json.loads(out), run)); self.assertEqual(len(self.trace(f)), 1)

    def test_crash_after_claim_leaves_a_permanent_nonretryable_attempt(self):
        f = self.fixture(); install_bridge(f, 'barrier'); self.instruction(f); read = self.ok(f.run())
        args, run = f.command('launch', evidence=read / 'evidence.json')
        child = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        row = self.wait_trace(f, child)
        child.kill(); child.communicate(timeout=5)
        try:
            self.failure(self.launch(f, read), 'delivery_already_attempted'); self.assertEqual(len(self.trace(f)), 1)
            self.assertFalse((read / 'material.json').exists())
            self.assertFalse((run / 'delivery.json').exists())
        finally:
            try: os.killpg(row['pid'], signal.SIGKILL)
            except ProcessLookupError: pass

    def test_preexisting_claim_symlink_never_follows_or_spawns(self):
        f = self.fixture(); install_bridge(f); self.instruction(f); read = self.ok(f.run())
        sentinel = f.private / 'do-not-touch'; sentinel.write_text('owner data')
        (read / 'delivery-started.json').symlink_to(sentinel)
        self.failure(self.launch(f, read), 'delivery_already_attempted')
        self.assertEqual(sentinel.read_text(), 'owner data'); self.assertEqual(self.trace(f), [])

    def test_history_changed_material_and_source_gap_never_launch(self):
        f = self.fixture(); install_bridge(f); rid = self.instruction(f)
        f.write_request(purpose='history', record_ids=[rid]); read = self.ok(f.run())
        self.failure(self.launch(f, read), 'history_only'); self.assertFalse((read / 'delivery-started.json').exists())
        f.write_request(); read = self.ok(f.run()); (read / 'material.json').write_text('{}')
        self.failure(self.launch(f, read), 'stale_snapshot'); self.assertEqual(self.trace(f), [])
        w = attach_workflow(f)
        phase(w, 'review', lambda p: [{**r, 'output_text': 'Read workspace-b/marker.txt'} for r in reviews(p)], 'review')
        phase(w, 'publish', lambda p: [], 'publish')
        f.write(f.request_path, dict(schema_version=1, mode='repository', purpose='current', consumer_ids=['lessons', 'mechanism']))
        read = self.ok(f.run()); target = Path(w.binding['targets'][0]['path']); target.write_text(target.read_text() + '<!-- memorynexus:unknown:begin -->\nstale\n<!-- memorynexus:unknown:end -->\n')
        self.failure(self.launch(f, read), 'stale_snapshot'); self.assertEqual(self.trace(f), [])

    def test_retraction_and_restore_refuse_prior_material_before_claim(self):
        f = self.fixture(); install_bridge(f); rid = self.instruction(f); read = self.ok(f.run())
        f.cli('retract', dict(idempotency_key='retract', run_id='60000000-0000-4000-8000-000000000001', client='synthetic', target_id=rid, reason='Fixture withdrawal.', confirmation='confirmed'))
        self.failure(self.launch(f, read), 'stale_snapshot'); self.assertFalse((read / 'delivery-started.json').exists())
        self.instruction(f); read = self.ok(f.run())
        backup = f.base / 'data/after-withdrawal.sqlite'; restored = f.base / 'data/restored.sqlite'
        f.storage('backup', {'path': str(backup)}); f.storage('restore', {'path': str(backup)}, restored)
        for suffix in ('-wal', '-shm'): Path(str(f.ledger) + suffix).unlink(missing_ok=True)
        os.replace(restored, f.ledger)
        self.failure(self.launch(f, read), 'stale_snapshot'); self.assertEqual(self.trace(f), [])

    def test_spawn_failure_is_claimed_unknown_without_a_retry(self):
        f = self.fixture(); bridge = install_bridge(f); self.instruction(f)
        bridge.write_text('This is not an executable format.\n')
        f.binding['bridge']['sha256'] = hashlib.sha256(bridge.read_bytes()).hexdigest(); f.save_binding()
        read = self.ok(f.run()); launch = self.failure(self.launch(f, read), 'delivery_unknown')
        self.assertEqual(f.payload(launch, 'delivery.json')['status'], 'unknown')
        self.failure(self.launch(f, read), 'delivery_already_attempted'); self.assertEqual(self.trace(f), [])

    def test_actual_repository_source_drift_blocks_before_spawn(self):
        f = self.fixture(repository=True); install_bridge(f); f.seed(); read = self.ok(f.run())
        (f.root / 'incident.md').write_text('Changed upstream without sync.\n')
        self.failure(self.launch(f, read), 'source_drift')
        self.assertFalse((read / 'delivery-started.json').exists()); self.assertEqual(self.trace(f), [])
        self.assertFalse((read / 'material.json').exists())

    def test_external_old_copy_change_blocks_covered_launch(self):
        f = self.fixture(); install_bridge(f); self.instruction(f); read = self.ok(f.run())
        old = f.private / 'external-cache'; old.write_text('Read workspace-a/marker.txt')
        f.binding['copies'] = [dict(id='external_cache', path=str(old), role='external')]; f.save_binding()
        self.failure(self.launch(f, read), 'stale_snapshot')
        self.failure(f.run(), 'injection_unknown'); self.assertEqual(self.trace(f), [])
        self.assertEqual(old.read_text(), 'Read workspace-a/marker.txt')

    def test_direct_bridge_bypass_is_explicitly_outside_adapter_coverage(self):
        f = self.fixture(); bridge = install_bridge(f); old_id = self.instruction(f, 'a')
        read = self.ok(f.run()); independently_cached = (read / 'material.json').read_bytes()
        f.mutation('correct', target_id=old_id, text='Read workspace-b/marker.txt', confirmation='confirmed')
        self.failure(self.launch(f, read), 'stale_snapshot')
        # Deliberate bypass with independently retained bytes: the adapter cannot
        # enforce freshness in a caller that never invokes its entry point.
        child = subprocess.run([str(bridge)], input=independently_cached, capture_output=True, cwd=f.root, timeout=10)
        self.assertEqual(child.returncode, 0); self.assertEqual(self.trace(f)[0]['opened'], 'workspace-a/marker.txt')
        self.assertFalse((read / 'delivery-started.json').exists())
        self.assertFalse(any(f.state.glob('*/delivery.json')))


if __name__ == '__main__': unittest.main()
