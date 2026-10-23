"""Public runner exercises dependency-sensitive independent consumer freshness."""
import hashlib
import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'scripts'))
from workflow_fixture import Fixture
from workflow_types import digest
from workflow_cli import status_all

class Selective(unittest.TestCase):
    def test_auxiliary_document_change_invalidates_only_its_consumer(self):
        f=Fixture(); self.addCleanup(f.close)
        aux_id='50000000-0000-4000-8000-000000000098'
        (f.root/'correction.md').write_text('Follow-up remains unverified.\n')
        f.rebind(lambda p:p['collections'].append(dict(collection_id=aux_id,format='text',path='correction.md',selector={'kind':'full'},extractor_version='bounded-v1',authority='Auxiliary correction document reports.')))
        code,result,_=f.capture('seed'); self.assertEqual(code,0,result)
        code,result,prepared=f.prepare('review','review-prepare'); self.assertEqual(code,3,result)
        material=json.loads(prepared.read_text())
        primary=next(x for x in status_all(f.ctx,'inputs') if x['origin']['source_id']!=aux_id)
        aux=next(x for x in material['snapshot']['dependencies'] if x['id']==aux_id)
        submissions=[]
        consumers=f.profile['consumers']
        for w in material['items']:
            if w['input_record_id']!=primary['id']:continue
            dependencies=list(w['dependencies'])
            if w['consumer_id']==consumers[1]['consumer_id']:dependencies.append(aux)
            submissions.append(dict(input_record_id=w['input_record_id'],consumer_id=w['consumer_id'],decision='proposed',reason='Use declared evidence only.',dependencies=dependencies,output_text='A bounded source pointer.'))
        submission=f.state/'review-submission.json';f.write(submission,dict(schema_version=1,prepared_sha256=hashlib.sha256(prepared.read_bytes()).hexdigest(),items=submissions))
        directory=f.state/'review-preview'
        code,result=f.run('preview','--binding',f.binding_path,'--prepared',prepared,'--submission',submission,'--run-dir',directory); self.assertIn(code,(0,3),result)
        plan=directory/'plan.json'
        code,result=f.run('apply','--binding',f.binding_path,'--plan',plan);self.assertIn(code,(0,4),result)
        before={d['consumer_id']:d for d in status_all(f.ctx,'decisions')};self.assertEqual(len(before),2)
        (f.root/'correction.md').write_text('Follow-up now contradicts the earlier explanation.\n')
        code,result,_=f.capture('corrected'); self.assertEqual(code,0,result)
        after={d['consumer_id']:d for d in status_all(f.ctx,'decisions')}
        a=consumers[0]['consumer_id'];b=consumers[1]['consumer_id']
        self.assertEqual(after[a]['decision_id'],before[a]['decision_id'])
        self.assertEqual(after[a]['validity'],'current')
        self.assertEqual(after[b]['validity'],'stale')
        code,result,prepared=f.prepare('review','review-again')
        new=json.loads(prepared.read_text())
        primary_work=[x for x in new['items'] if x['root_record_id']==primary['root_record_id']]
        self.assertEqual([x['consumer_id'] for x in primary_work],[b])

class Unsynchronized(unittest.TestCase):
    def test_changed_source_requires_capture_before_review_evidence(self):
        f=Fixture();self.addCleanup(f.close)
        code,result,_=f.capture('seed');self.assertEqual(code,0,result)
        (f.root/'incident.md').write_text('New and not-yet-synchronized evidence.\n')
        code,result,path=f.prepare('review','changed-review');self.assertEqual(code,3,result)
        value=json.loads(path.read_text());self.assertEqual(value['items'],[])
        self.assertEqual(value['evidence'],[])
        self.assertEqual(value['snapshot']['dependencies'],[])
        self.assertIn('awaiting_extraction',[x['reason'] for x in value['remaining']])

    def test_missing_source_requires_invalidation_before_review(self):
        f=Fixture();self.addCleanup(f.close)
        code,result,_=f.capture('seed');self.assertEqual(code,0,result)
        (f.root/'incident.md').unlink()
        code,result,path=f.prepare('review','missing-review');self.assertEqual(code,3,result)
        value=json.loads(path.read_text());self.assertEqual(value['items'],[])
        self.assertEqual(value['evidence'],[])
        self.assertEqual(value['snapshot']['dependencies'],[])
        self.assertIn('source_unavailable',[x['reason'] for x in value['remaining']])
        self.assertEqual(len(status_all(f.ctx,'inputs')),1)
        self.assertEqual(status_all(f.ctx,'inputs')[0]['state'],'current')

if __name__=='__main__':unittest.main()
