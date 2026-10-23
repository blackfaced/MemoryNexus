import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'scripts'))
from workflow_fixture import Fixture
from workflow_cli import status_all

class Jsonl(unittest.TestCase):
    def fixture(self,raw):
        f=Fixture(); self.addCleanup(f.close)
        (f.root/'events.jsonl').write_bytes(raw)
        def change(p):
            p['collections'][0].update(collection_id='50000000-0000-4000-8000-000000000099',format='append_jsonl',path='events.jsonl',selector={'kind':'fields','keys':['text']})
        f.rebind(change)
        return f

    def test_offsets_identity_nonmonotonic_time_and_noop(self):
        row=b'{"text":"same","ts":"2026-10-23"}\n'
        f=self.fixture(row+b'{"text":"earlier","ts":"2020-01-01"}\n'+row)
        code,result,prepared=f.capture('first'); self.assertEqual(code,0,result)
        sources=status_all(f.ctx,'sources'); self.assertEqual(len(sources),3)
        self.assertEqual(len({s['source_id'] for s in sources}),3)
        self.assertEqual(sources[0]['sha256'],sources[2]['sha256'])
        before=f.cli(['--config',f.config,'workflow-status'],{},f.ledger)['revision']
        code,result,p=f.capture('again'); self.assertEqual(code,0,result); self.assertEqual(p['items'],[])
        self.assertEqual(before,f.cli(['--config',f.config,'workflow-status'],{},f.ledger)['revision'])

    def test_twenty_row_batches_partial_and_eof_refresh(self):
        row=b'{"text":"same"}\n'
        f=self.fixture(row*22+b'{"text":')
        code,result,p=f.capture('first'); self.assertEqual(len(p['items']),20)
        self.assertEqual(len(status_all(f.ctx,'sources')),20)
        code,result,p=f.capture('second'); self.assertEqual(len(p['items']),2)
        sources=status_all(f.ctx,'sources'); self.assertEqual(len(sources),22)
        original={x['source_id'] for x in sources}; cursor=next(x for x in status_all(f.ctx,'collections') if x['active'])['cursor']
        self.assertEqual(cursor['offset'],len(row)*22)
        f.rebind(lambda p:p['collections'][0].update(extractor_version='bounded-v2'))
        code,result,p=f.capture('refresh'); self.assertEqual(len(p['items']),20)
        current=status_all(f.ctx,'sources')
        self.assertEqual(original,{x['source_id'] for x in current})
        self.assertTrue(all(x['source_revision']==2 for x in current))
        self.assertEqual(cursor,next(x for x in status_all(f.ctx,'collections') if x['active'])['cursor'])
        code,result,p=f.capture('refresh-tail'); self.assertEqual(len(p['items']),2)

    def test_blocked_selected_type_does_not_advance(self):
        first=b'{"text":"valid"}\n'; f=self.fixture(first+b'{"text":42}\n'+b'{"text":"later"}\n')
        code,result,p=f.capture('first')
        self.assertEqual(len(p['items']),1)
        self.assertTrue(any(x['reason']=='blocked_unit' for x in p['remaining']))
        self.assertEqual(next(x for x in status_all(f.ctx,'collections') if x['active'])['cursor']['offset'],len(first))
        self.assertEqual(len(status_all(f.ctx,'sources')),1)
        code,result,p=f.capture('retry')
        self.assertEqual(p['items'],[])
        self.assertEqual(next(x for x in status_all(f.ctx,'collections') if x['active'])['cursor']['offset'],len(first))

if __name__=='__main__': unittest.main()
