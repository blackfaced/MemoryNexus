"""Recovery exercises real ledger intents and retained immutable runner plans."""
import hashlib
import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'scripts'))
from workflow_fixture import Fixture
from workflow_cli import load_context, status_all
from workflow_engine import envelope, staged, header
from workflow_io import publish, resolve_ref
from workflow_recovery import prepare_recovery, apply_recovery
from workflow_types import digest


class RecoveryTests(unittest.TestCase):
    def fixture_with_intent(self):
        f=Fixture();self.addCleanup(f.close)
        for workflow in ('capture','review','publish'):
            _,_,prepared=f.prepare(workflow,workflow+'-p')
            data=json.loads(prepared.read_text())
            if workflow=='capture':
                items=[{'unit_id':x['id'],'text':'Attributed source; verification pending.','occurred_at':'2026-10-23T00:00:00Z'} for x in data['items']]
            elif workflow=='review':
                items=[{'input_record_id':x['input_record_id'],'consumer_id':x['consumer_id'],'decision':'proposed','reason':'Retain source uncertainty.','dependencies':x['dependencies'],'output_text':'incident.md source; unverified.'} for x in data['items']]
            else:items=[]
            submission=f.state/(workflow+'-s.json');f.write(submission,dict(schema_version=1,prepared_sha256=hashlib.sha256(prepared.read_bytes()).hexdigest(),items=items))
            run=f.state/(workflow+'-v')
            code,result=f.run('preview','--binding',f.binding_path,'--prepared',prepared,'--submission',submission,'--run-dir',run)
            self.assertIn(code,(0,3),result)
            plan=json.loads((run/'plan.json').read_text())
            if workflow!='publish':
                argv=['apply','--binding',f.binding_path,'--plan',run/'plan.json']
                if plan['authorization']['mode']=='owner':argv+=['--approve',digest(plan)]
                code,result=f.run(*argv);self.assertEqual(code,0,result)
        for n in ('requests','receipts','outputs'):(run/n).mkdir(mode=0o700)
        ctx=load_context(f.binding_path);header(ctx);item=plan['items'][0]
        proposal={k:item[k] for k in ('target_id','block_id','base_exists','base_sha256','after_sha256','output_sha256')};proposal['plan_sha256']=digest(plan)
        body=envelope(plan,'consumer-approve',0);body.update(decision_id=item['decision_id'],decision_sha256=item['decision_sha256'],proposal=proposal,confirmation='confirmed')
        staged(ctx,run,plan,'consumer-approve',0,body)
        return f,ctx,run,plan,item

    def test_before_recovery_uses_original_intent_and_keys(self):
        f,ctx,run,plan,original=self.fixture_with_intent()
        items,remaining=prepare_recovery(ctx,status_all(ctx,'publications'))
        self.assertEqual(remaining,[]);self.assertEqual(items[0]['classification'],'before')
        items[0]['id']='r0';result=apply_recovery(ctx,run,plan,items[0],0)
        self.assertEqual(result['status'],'ok')
        self.assertTrue((run/'requests'/'publication-result-0-0.json').exists())
        self.assertEqual(status_all(ctx,'publications')[0]['publication'],'applied')

    def test_after_conflict_and_restored_classification(self):
        f,ctx,run,plan,original=self.fixture_with_intent()
        path=next(t['path'] for t in f.binding['targets'] if t['target_id']==original['target_id'])
        raw=resolve_ref(run,original['rendered'],f.state)
        publish(path,original['base_exists'],original['base_sha256'],raw,original['after_sha256'])
        items,_=prepare_recovery(ctx,status_all(ctx,'publications'));self.assertEqual(items[0]['classification'],'after')
        Path(path).write_text('manual edit')
        items,_=prepare_recovery(ctx,status_all(ctx,'publications'));self.assertEqual(items[0]['classification'],'conflict')
        items[0]['id']='r0';self.assertEqual(apply_recovery(ctx,run,plan,items[0],0)['status'],'needs_review')
        self.assertEqual(Path(path).read_text(),'manual edit')
        ctx['incarnation']='ffffffff-ffff-4fff-8fff-ffffffffffff'
        items,_=prepare_recovery(ctx,status_all(ctx,'publications'));self.assertEqual(items[0]['classification'],'restored')

    def test_missing_retained_plan_never_reconstructed_from_ledger(self):
        f,ctx,run,plan,original=self.fixture_with_intent()
        publications=status_all(ctx,'publications')
        (run/'plan.json').unlink()
        items,remaining=prepare_recovery(ctx,publications)
        self.assertEqual(items,[])
        self.assertEqual(remaining,[{'id':publications[0]['approval_id'],'reason':'plan_missing'}])
