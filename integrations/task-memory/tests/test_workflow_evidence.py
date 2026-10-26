"""Auxiliary source evidence is not limited by the 20-decision work batch."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import unittest
import uuid
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from workflow_cli import status_all
from workflow_fixture import Fixture
from test_workflow_publish_supersede import phase, reviews


class AuxiliaryEvidence(unittest.TestCase):
    def test_28_sync_only_sources_support_independent_lens_dependencies(self):
        f = Fixture()
        self.addCleanup(f.close)
        f.capture('primary')
        base = f.profile['collections'][0]
        extra = []
        for n in range(1, 28):
            c = copy.deepcopy(base)
            c['collection_id'] = str(uuid.uuid5(uuid.UUID(base['collection_id']), str(n)))
            c['path'] = 'auxiliary-%s.md' % n
            (f.root / c['path']).write_text('Attributed auxiliary evidence %s.\n' % n)
            extra.append(c)
        f.rebind(lambda p: p['collections'].extend(extra))
        # Metadata registration is allowed without inventing capture records.
        _, _, prepared = f.prepare('capture', 'bounded-capture')
        self.assertEqual(len(json.loads(prepared.read_text())['items']), 20)
        phase(f, 'capture', lambda p: [], 'metadata-only')
        self.assertEqual(len(status_all(f.ctx, 'inputs')), 1)
        self.assertEqual(len(status_all(f.ctx, 'sources')), 28)
        _, _, prepared = f.prepare('review', 'all-evidence')
        data = json.loads(prepared.read_text())
        self.assertEqual(len(data['items']), 2)
        self.assertEqual(len(data['evidence']), 28)
        self.assertEqual(data['remaining'], [])
        last = extra[-1]['collection_id']
        dependency = next(d for d in data['snapshot']['dependencies'] if d['id'] == last)
        self.assertEqual(dependency['sha256'], hashlib.sha256((f.root / extra[-1]['path']).read_bytes()).hexdigest())
        _, _, repeated = f.prepare('review', 'all-evidence-again')
        self.assertEqual([e['source_id'] for e in json.loads(repeated.read_text())['evidence']],
                         [e['source_id'] for e in data['evidence']])
        b = f.profile['consumers'][1]['consumer_id']
        def with_auxiliary(material):
            items = reviews(material)
            for item in items:
                if item['consumer_id'] == b:
                    item['dependencies'].append(dependency)
            return items
        _, result = phase(f, 'review', with_auxiliary, 'review-28')
        self.assertEqual(result['status'], 'ok', result)
        (f.root / extra[-1]['path']).write_text('Corrected auxiliary evidence.\n')
        phase(f, 'capture', lambda p: [], 'metadata-correction')
        self.assertEqual(len(status_all(f.ctx, 'inputs')), 1)
        decisions = {d['consumer_id']: d for d in status_all(f.ctx, 'decisions')}
        self.assertEqual(decisions[b]['validity'], 'stale')
        self.assertEqual(decisions[f.profile['consumers'][0]['consumer_id']]['validity'], 'current')
        _, _, prepared = f.prepare('review', 'selective-recheck')
        self.assertEqual([i['consumer_id'] for i in json.loads(prepared.read_text())['items']], [b])


if __name__ == '__main__':
    unittest.main()
