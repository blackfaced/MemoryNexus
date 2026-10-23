import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'scripts'))
from workflow_fixture import Fixture
from workflow_types import digest
import hashlib

class EndToEnd(unittest.TestCase):
    def test_real_prepare_is_readonly_and_returns_capture_work(self):
        fixture=Fixture()
        self.addCleanup(fixture.close)
        before=fixture.cli(['--config',fixture.config,'workflow-status'],{},fixture.ledger)['revision']
        code,result,prepared=fixture.prepare()
        self.assertEqual(code,3,result)
        self.assertTrue(prepared.is_file())
        material=json.loads(prepared.read_text())
        self.assertTrue(material['items'])
        after=fixture.cli(['--config',fixture.config,'workflow-status'],{},fixture.ledger)['revision']
        self.assertEqual(before,after)

    def test_capture_two_independent_lenses_then_publish(self):
        f=Fixture(); self.addCleanup(f.close)
        def phase(workflow, items_builder, label):
            code,result,prepared=f.prepare(workflow,label+'-prepare')
            self.assertEqual(code,3,result)
            data=json.loads(prepared.read_text())
            items=items_builder(data)
            submission=f.state/(label+'-submission.json')
            f.write(submission,{'schema_version':1,'prepared_sha256':hashlib.sha256(prepared.read_bytes()).hexdigest(),'items':items})
            directory=f.state/(label+'-preview')
            code,result=f.run('preview','--binding',f.binding_path,'--prepared',prepared,'--submission',submission,'--run-dir',directory)
            self.assertIn(code,(0,3),result)
            plan_path=directory/'plan.json'; plan=json.loads(plan_path.read_text())
            argv=['apply','--binding',f.binding_path,'--plan',plan_path]
            if plan['authorization']['mode']=='owner': argv+=['--approve',digest(plan)]
            code,result=f.run(*argv)
            self.assertEqual(code,0,result)
            return plan
        phase('capture',lambda p:[{'unit_id':x['id'],'text':'Source-reported failure; verification pending.','occurred_at':'2026-10-23T00:00:00Z'} for x in p['items']],'capture')
        def reviews(p):
            return [{'input_record_id':x['input_record_id'],'consumer_id':x['consumer_id'],'decision':'proposed','reason':'Use the original source pointer; retain uncertainty.','dependencies':x['dependencies'],'output_text':'Source pointer: incident.md; verification pending.'} for x in p['items']]
        review=phase('review',reviews,'review')
        self.assertEqual(len(review['items']),2)
        publish=phase('publish',lambda p:[],'publish')
        self.assertEqual(len(publish['items']),2)
        for target in f.binding['targets']:
            body=Path(target['path']).read_text()
            self.assertIn('Source pointer: incident.md; verification pending.',body)
            self.assertIn('<!-- memorynexus:',body)
        # A fresh publication preparation must not re-publish settled decisions.
        code,result,prepared=f.prepare('publish','settled-prepare')
        self.assertIn(code,(0,3),result)
        self.assertEqual(json.loads(prepared.read_text())['items'],[])

if __name__=='__main__': unittest.main()
