"""Real compiled CLI tests for pinned transport and current ledger responses."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'scripts'))
from workflow_cli import call, status_all
from workflow_types import strict_loads, canonical, digest, validate_profile
from workflow_io import WorkflowError

REPO=Path(__file__).resolve().parents[3]
BIN=Path(os.environ.get('MEMORYNEXUS_TEST_BINARY',str(Path(os.environ.get('CARGO_TARGET_DIR',str(REPO/'target')))/'debug/memorynexus-feedback-cli')))

class Bridge(unittest.TestCase):
    def test_strict_values(self):
        for raw in ('{"x":null}','{"x":1,"x":2}','{"x":1.5}','{"x":9007199254740992}'):
            with self.assertRaises(WorkflowError): strict_loads(raw)
        self.assertEqual(canonical({'z':1,'a':'value'}),b'{"a":"value","z":1}')

    def test_real_migration_status_bind_and_scope(self):
        self.assertTrue(BIN.is_file(),'compiled test binary required')
        with tempfile.TemporaryDirectory(prefix='wf-bridge-',dir=Path(tempfile.gettempdir()).resolve()) as tmp:
            root=Path(tmp); config=root/'.memorynexus.json'; old=root/'v1.sqlite'; new=root/'v2.sqlite'; backup=root/'backup.sqlite'
            def invoke(args,body,ledger=None):
                env=dict(os.environ)
                if ledger: env['MEMORYNEXUS_TASK_LEDGER']=str(ledger)
                p=subprocess.run([str(BIN),'task',*map(str,args)],input=canonical(body),capture_output=True,env=env,check=False)
                self.assertEqual(p.returncode,0,p.stdout)
                return json.loads(p.stdout)
            init=invoke(['--config',config,'init'],{'idempotency_key':'init','display_name':'bridge','confirmation':'confirmed','automatic_capture':['execution','review']},old)
            invoke(['--ledger',old,'backup'],{'path':str(backup)})
            migration=invoke(['--ledger',old,'migrate'],{'path':str(new),'backup_path':str(backup),'backup_sha256':hashlib.sha256(backup.read_bytes()).hexdigest(),'expected_version':1,'target_version':2,'confirmation':'confirmed'})
            profile=json.loads((Path(__file__).resolve().parents[3]/'docs/repository-memory/examples/sequence-v1.json').read_text())['profile']
            validate_profile(profile)
            ctx={'binding':{'binary':str(BIN),'config':str(config),'ledger':str(new),'package':str(root),'task_id':init['task_id'],'timeout_seconds':60,'workflow_id':profile['workflow_id']}}
            header=call(ctx,'workflow-status',{})
            self.assertEqual(header['result']['items'][0]['task_id'],init['task_id'])
            request={'idempotency_key':'bind','run_id':'60000000-0000-4000-8000-000000000001','client':'bridge','workflow_id':profile['workflow_id'],'profile_revision':1,'incarnation':migration['result']['incarnation'],'profile':profile,'profile_sha256':digest(profile),'binding_sha256':'c'*64,'expected_profile_revision':0,'confirmation':'confirmed'}
            self.assertEqual(call(ctx,'workflow-bind',request)['status'],'accepted')
            self.assertEqual(call(ctx,'workflow-bind',request)['status'],'replayed')
            self.assertEqual(status_all(ctx)[0]['profile_revision'],1)
            self.assertEqual(status_all(ctx,'sources'),[])
            ctx['binding']['task_id']='10000000-0000-4000-8000-000000000099'
            with self.assertRaises(WorkflowError) as error: call(ctx,'workflow-status',{})
            self.assertEqual(error.exception.code,'invalid_response')

    def test_private_binding_pins_and_capability_are_enforced(self):
        from workflow_fixture import Fixture
        from workflow_cli import load_context
        f=Fixture(); self.addCleanup(f.close)
        self.assertEqual(load_context(f.binding_path)['binding']['task_id'],f.task_id)
        before=f.ledger.read_bytes()
        for field,value in [('binary_sha256','0'*64),('package_sha256','0'*64),('task_id','10000000-0000-4000-8000-000000000099'),('python','/bin/sh')]:
            original=f.binding[field]; f.binding[field]=value; f.write(f.binding_path,f.binding)
            with self.assertRaises(WorkflowError): load_context(f.binding_path)
            f.binding[field]=original
        f.write(f.binding_path,f.binding)
        self.assertEqual(before,f.ledger.read_bytes())

if __name__=='__main__': unittest.main()
