"""Faults injected only at file write and real CLI subprocess boundaries."""
import json
import os
from pathlib import Path
import subprocess
import unittest
import test_workflow_recovery as recovery_fixture
from workflow_types import digest


class FailureTests(unittest.TestCase):
    def setup_intent(self):
        return recovery_fixture.RecoveryTests.fixture_with_intent(self)

    def driver(self,f,run,plan,patch):
        driver=f.base/'driver.py'
        driver.write_text("import sys\nsys.path.insert(0,"+repr(str(f.package))+ ")\nimport workflow,workflow_engine as e\n"+patch+"\nsys.argv=['workflow.py','apply','--binding',"+repr(str(f.binding_path))+",'--plan',"+repr(str(run/'plan.json'))+",'--approve',"+repr(digest(plan))+"]\nsys.exit(workflow.main())\n")
        process=subprocess.run([f.binding['python'],str(driver)],capture_output=True,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'))
        return process,json.loads(process.stdout)

    def test_branch_report_failure_does_not_skip_second_target(self):
        f,ctx,run,plan,item=self.setup_intent()
        patch="""original=e.write_new
def broken(path,raw):
    if path.name=='result-u0-0.json':raise OSError('injected')
    return original(path,raw)
e.write_new=broken
"""
        process,report=self.driver(f,run,plan,patch)
        self.assertNotEqual(process.returncode,0,report)
        self.assertEqual(len(report['items']),2,report)
        self.assertEqual(report['items'][1]['status'],'ok',report)
        for target in f.binding['targets']:
            self.assertIn('incident.md source',Path(target['path']).read_text())

    def test_unknown_result_response_replays_original_request(self):
        f,ctx,run,plan,item=self.setup_intent()
        patch="""original=e.call
lost=[False]
def drop(ctx,action,body):
    response=original(ctx,action,body)
    if action=='publication-result' and not lost[0]:
        lost[0]=True
        raise OSError('response dropped after real CLI effect')
    return response
e.call=drop
"""
        process,report=self.driver(f,run,plan,patch)
        self.assertNotEqual(process.returncode,0,report)
        request=run/'requests'/'publication-result-0-0.json'
        original=request.read_bytes()
        before=Path(f.binding['targets'][0]['path']).read_bytes()
        process,report=self.driver(f,run,plan,'')
        self.assertEqual(process.returncode,0,report)
        self.assertEqual(request.read_bytes(),original)
        self.assertFalse((run/'requests'/'publication-result-0-1.json').exists())
        self.assertEqual(Path(f.binding['targets'][0]['path']).read_bytes(),before)

    def test_source_changes_after_replace_records_actual_stale_effect(self):
        f,ctx,run,plan,item=self.setup_intent()
        patch="""original=e.publish
changed=[False]
def alter_after_replace(path,*args):
    outcome=original(path,*args)
    if not changed[0]:
        changed[0]=True
        source=__import__('pathlib').Path("""+repr(str(f.root/'incident.md'))+""")
        source.write_text('Corrected source report; earlier attribution withdrawn.\\n')
        from workflow_cli import load_context,status_all,call
        c=load_context("""+repr(str(f.binding_path))+""")
        e.header(c)
        scans,_,_=e.scans(c,status_all(c,'collections'),status_all(c,'sources'))
        body={'idempotency_key':'fault-source-correction','run_id':'60000000-0000-4000-8000-000000000099','client':'synthetic','workflow_id':c['binding']['workflow_id'],'profile_revision':c['profile']['revision'],'incarnation':c['incarnation']}
        body.update(scans[0]);call(c,'source-sync',body)
    return outcome
e.publish=alter_after_replace
"""
        process,report=self.driver(f,run,plan,patch)
        self.assertNotEqual(process.returncode,0,report)
        from workflow_cli import status_all
        publication=next(p for p in status_all(ctx,'publications') if p['decision_id']==item['decision_id'])
        self.assertEqual(publication['publication'],'applied',report)
        self.assertEqual(publication['validity'],'stale',report)
        self.assertEqual(Path(f.binding['targets'][0]['path']).read_bytes(),(run/item['rendered']['path']).read_bytes())

    def test_receipted_baseline_conflict_permanently_stops_old_plan(self):
        f,ctx,run,plan,item=self.setup_intent()
        target=Path(next(t['path'] for t in f.binding['targets'] if t['target_id']==item['target_id']))
        target.write_text('Owner changed the baseline.')
        process,report=self.driver(f,run,plan,'')
        self.assertNotEqual(process.returncode,0,report)
        receipt=run/'receipts'/'publication-result-0-0.json'
        self.assertEqual(json.loads(receipt.read_text())['response']['result']['outcome'],'conflict')
        retained=receipt.read_bytes()
        target.unlink()  # Restore the original absent baseline, never authorization.
        process,report=self.driver(f,run,plan,'')
        self.assertNotEqual(process.returncode,0,report)
        self.assertFalse(target.exists())
        self.assertEqual(receipt.read_bytes(),retained)
        self.assertFalse((run/'requests'/'publication-result-0-1.json').exists())

    def test_metadata_only_capture_retains_unextracted_work_pending(self):
        from workflow_fixture import Fixture
        f=Fixture();self.addCleanup(f.close)
        _,_,prepared=f.prepare('capture','metadata-prepare')
        data=json.loads(prepared.read_text());self.assertTrue(data['items'])
        submission=f.state/'metadata-submission.json'
        f.write(submission,{'schema_version':1,'prepared_sha256':__import__('hashlib').sha256(prepared.read_bytes()).hexdigest(),'items':[]})
        run=f.state/'metadata-preview'
        code,result=f.run('preview','--binding',f.binding_path,'--prepared',prepared,'--submission',submission,'--run-dir',run)
        self.assertIn(code,(0,3),result)
        plan=json.loads((run/'plan.json').read_text())
        process,report=self.driver(f,run,plan,'')
        self.assertNotEqual(process.returncode,0,report)
        self.assertTrue(report['pending'],report)
        from workflow_cli import status_all
        self.assertTrue(status_all(f.ctx,'sources'))
        self.assertEqual(status_all(f.ctx,'inputs'),[])

    def test_profile_rebind_after_replace_records_actual_stale_effect(self):
        f,ctx,run,plan,item=self.setup_intent()
        patch="""original=e.publish
changed=[False]
def rebind_after_replace(path,*args):
    outcome=original(path,*args)
    if not changed[0]:
        changed[0]=True
        from workflow_cli import load_context,call
        c=load_context("""+repr(str(f.binding_path))+""")
        e.header(c)
        profile=dict(c['profile']);profile['revision']=2
        body={'idempotency_key':'fault-profile-rebind','run_id':'60000000-0000-4000-8000-000000000098','client':'synthetic','workflow_id':c['binding']['workflow_id'],'profile_revision':2,'incarnation':c['incarnation'],'expected_profile_revision':1,'profile':profile,'profile_sha256':e.sha(e.canonical(profile)),'binding_sha256':c['binding_sha256'],'confirmation':'confirmed'}
        call(c,'workflow-bind',body)
    return outcome
e.publish=rebind_after_replace
"""
        process,report=self.driver(f,run,plan,patch)
        self.assertNotEqual(process.returncode,0,report)
        from workflow_cli import status_all
        publication=next(p for p in status_all(ctx,'publications') if p['decision_id']==item['decision_id'])
        self.assertEqual(publication['publication'],'applied',report)
        self.assertEqual(publication['validity'],'stale',report)
        self.assertEqual(Path(f.binding['targets'][0]['path']).read_bytes(),(run/item['rendered']['path']).read_bytes())

    def test_lost_failed_result_replayed_before_next_actual_attempt(self):
        f,ctx,run,plan,item=self.setup_intent()
        patch="""original_call=e.call
failed=[False]
lost=[False]
original_publish=e.publish
def fail_once(path,*args):
    if not failed[0]:
        failed[0]=True
        raise e.WorkflowError('io_error')
    return original_publish(path,*args)
def drop_failed(ctx,action,body):
    response=original_call(ctx,action,body)
    if action=='publication-result' and body['outcome']=='failed' and not lost[0]:
        lost[0]=True
        raise OSError('lost committed failed result')
    return response
e.publish=fail_once
e.call=drop_failed
"""
        process,report=self.driver(f,run,plan,patch)
        self.assertNotEqual(process.returncode,0,report)
        request=run/'requests'/'publication-result-0-0.json'
        original=request.read_bytes()
        self.assertEqual(json.loads(original)['outcome'],'failed')
        self.assertFalse((run/'receipts'/request.name).exists())
        process,report=self.driver(f,run,plan,'')
        self.assertEqual(process.returncode,0,report)
        self.assertEqual(request.read_bytes(),original)
        first=json.loads((run/'receipts'/request.name).read_text())['response']['result']
        second=json.loads((run/'receipts'/'publication-result-0-1.json').read_text())['response']['result']
        self.assertEqual(first['outcome'],'failed')
        self.assertEqual(second['outcome'],'applied')
        self.assertEqual(Path(f.binding['targets'][0]['path']).read_bytes(),(run/item['rendered']['path']).read_bytes())
