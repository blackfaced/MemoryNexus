"""Synthetic files and real compiled CLI; no production ledger or fake transport."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from workflow_cli import load_context, package_inventory
from workflow_types import canonical, digest

REPO=Path(__file__).resolve().parents[3]
SCRIPTS=REPO/'integrations/task-memory/scripts'
BINARY=Path(os.environ.get('MEMORYNEXUS_TEST_BINARY',str(Path(os.environ.get('CARGO_TARGET_DIR',str(REPO/'target')))/'debug/memorynexus-feedback-cli')))

class Fixture:
    def __init__(self):
        self.temp=tempfile.TemporaryDirectory(prefix='mn-runner-',dir='/private/tmp')
        self.base=Path(self.temp.name); os.chmod(self.base,0o700)
        self.root=self.base/'repo'; self.root.mkdir(mode=0o700)
        self.private=self.base/'private'; self.private.mkdir(mode=0o700)
        self.package=self.base/'package'; self.package.mkdir(mode=0o700)
        self.state=self.private/'runs'; self.state.mkdir(mode=0o700)
        self.config=self.root/'.memorynexus.json'; self.ledger=self.private/'v2.sqlite'
        v1=self.private/'v1.sqlite'; backup=self.private/'backup.sqlite'
        init=self.cli(['--config',self.config,'init'],{'idempotency_key':'init','display_name':'synthetic runner','confirmation':'confirmed','automatic_capture':['execution','review']},v1)
        self.task_id=init['task_id']
        self.cli(['--ledger',v1,'backup'],{'path':str(backup)})
        migrated=self.cli(['--ledger',v1,'migrate'],{'path':str(self.ledger),'backup_path':str(backup),'backup_sha256':hashlib.sha256(backup.read_bytes()).hexdigest(),'expected_version':1,'target_version':2,'confirmation':'confirmed'})
        self.incarnation=migrated['result']['incarnation']
        self.profile=json.loads((REPO/'docs/repository-memory/examples/sequence-v1.json').read_text())['profile']
        self.profile['collections']=self.profile['collections'][:1]
        collection=self.profile['collections'][0]; collection['path']='incident.md'; collection['selector']={'kind':'full'}
        (self.root/'incident.md').write_text('A source report with attributed uncertainty.\n')
        self.rule=b'Use source pointers and preserve uncertainty.\n'
        for consumer in self.profile['consumers']: consumer['rules_sha256']=hashlib.sha256(self.rule).hexdigest()
        self.profile_path=self.private/'profile.json'; self.write(self.profile_path,self.profile)
        self.binding_path=self.private/'binding.json'
        self.binding={'schema_version':1,'workflow_id':self.profile['workflow_id'],'task_id':self.task_id,'task_root':str(self.root),'config':str(self.config),'ledger':str(self.ledger),'binary':str(BINARY),'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'python':str(Path(sys.executable).resolve()),'package':str(self.package),'package_sha256':'a'*64,'profile':str(self.profile_path),'state_root':str(self.state),'targets':[{'target_id':c['target_id'],'path':str(self.private/(c['target_id']+'.md'))} for c in self.profile['consumers']],'timeout_seconds':60}
        self.refresh_package()
        self.ctx=load_context(self.binding_path)
        self.cli(['--config',self.config,'workflow-bind'],{'idempotency_key':'bind','run_id':'60000000-0000-4000-8000-000000000001','client':'synthetic','workflow_id':self.profile['workflow_id'],'profile_revision':1,'incarnation':self.incarnation,'expected_profile_revision':0,'profile':self.profile,'profile_sha256':digest(self.profile),'binding_sha256':digest(self.binding),'confirmation':'confirmed'},self.ledger)

    def write(self,path,value):
        path.write_bytes(canonical(value)); os.chmod(path,0o600)

    def cli(self,args,body,ledger=None):
        env=dict(os.environ)
        if ledger: env['MEMORYNEXUS_TASK_LEDGER']=str(ledger)
        r=subprocess.run([str(BINARY),'task',*map(str,args)],input=canonical(body),capture_output=True,env=env)
        if r.returncode: raise AssertionError(r.stdout)
        return json.loads(r.stdout)

    def refresh_package(self):
        for source in SCRIPTS.glob('*.py'):
            shutil.copyfile(source,self.package/source.name); os.chmod(self.package/source.name,0o600)
        (self.package/'lens.txt').write_bytes(self.rule); os.chmod(self.package/'lens.txt',0o600)
        self.binding['package_sha256']=digest(package_inventory(self.package))
        self.write(self.binding_path,self.binding)

    def run(self,*args):
        env=dict(os.environ); env['PYTHONDONTWRITEBYTECODE']='1'
        r=subprocess.run([self.binding['python'],str(self.package/'workflow.py'),*map(str,args)],capture_output=True,env=env)
        try: value=json.loads(r.stdout)
        except ValueError: raise AssertionError((r.returncode,r.stdout,r.stderr))
        return r.returncode,value

    def prepare(self,workflow='capture',label='prepare'):
        run=self.state/label
        code,result=self.run('prepare','--binding',self.binding_path,'--workflow',workflow,'--client','synthetic','--run-dir',run)
        return code,result,run/'prepared.json'

    def close(self): self.temp.cleanup()

    def rebind(self,mutate):
        previous=self.profile['revision']; mutate(self.profile); self.profile['revision']=previous+1
        self.write(self.profile_path,self.profile)
        self.ctx=load_context(self.binding_path)
        self.cli(['--config',self.config,'workflow-bind'],{'idempotency_key':'bind-'+str(previous+1),'run_id':'60000000-0000-4000-8000-000000000001','client':'synthetic','workflow_id':self.profile['workflow_id'],'profile_revision':previous+1,'incarnation':self.incarnation,'expected_profile_revision':previous,'profile':self.profile,'profile_sha256':digest(self.profile),'binding_sha256':digest(self.binding),'confirmation':'confirmed'},self.ledger)

    def capture(self,label):
        code,report,prepared=self.prepare('capture',label+'-prepare')
        if code not in (0,3): raise AssertionError(report)
        material=json.loads(prepared.read_text())
        submission=self.state/(label+'-submission.json')
        self.write(submission,{'schema_version':1,'prepared_sha256':hashlib.sha256(prepared.read_bytes()).hexdigest(),'items':[{'unit_id':w['id'],'text':'Source-reported bounded row, verification pending.','occurred_at':'2026-10-23T00:00:00Z'} for w in material['items']]})
        directory=self.state/(label+'-preview')
        code,report=self.run('preview','--binding',self.binding_path,'--prepared',prepared,'--submission',submission,'--run-dir',directory)
        if code not in (0,3): raise AssertionError(report)
        plan=directory/'plan.json'; value=json.loads(plan.read_text()); args=['apply','--binding',self.binding_path,'--plan',plan]
        if value['authorization']['mode']=='owner': args+=['--approve',digest(value)]
        code,report=self.run(*args)
        if code not in (0,3,4): raise AssertionError(report)
        return code,report,material
