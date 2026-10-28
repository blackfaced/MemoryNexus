"""Codex protocol fixtures are synthetic; the task ledger/adapter are real processes."""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
import unittest
import uuid

import test_workflow_install as installation
from current_knowledge_fixture import CurrentFixture


def pin(path):
    return {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}


class CodexBridge(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        installation.Installation.setUpClass()

    @classmethod
    def tearDownClass(cls):
        installation.Installation.tearDownClass()

    def setUp(self):
        self.i = installation.Installation(methodName='runTest')
        self.i.setUp()
        self.addCleanup(self.i.doCleanups)
        code, self.installed = self.i.install()
        self.assertEqual(code, 0, self.installed)
        self.f = CurrentFixture()
        self.addCleanup(self.f.close)
        self.home = self.f.private / 'codex-home'; self.home.mkdir(mode=0o700)
        self.config = self.home / 'config.toml'; self.config.write_text('# synthetic\n'); self.config.chmod(0o600)
        self.client = self.f.private / 'synthetic-client'
        self.trace = self.f.private / 'stdin-digest'
        self.spec_path = self.f.private / 'bridge-spec.json'
        self.out = self.f.private / 'bridge'
        self.client_mode('ok')
        self.spec = {'schema_version': 1, 'task_id': self.f.task_id, 'task_root': str(self.f.root),
                     'home': str(self.f.private), 'codex_home': str(self.home),
                     'client': pin(self.client), 'client_version': 'codex-cli 0.162.0',
                     'files': [pin(self.config)], 'timeout_seconds': 3}
        self.read_id = str(uuid.uuid4())
        self.material = {'schema_version': 1, 'task_id': self.f.task_id, 'read_id': self.read_id,
                         'mode': 'task', 'purpose': 'current', 'current_use': True, 'gaps': []}

    def client_mode(self, mode):
        script = '''#!PYTHON
import hashlib,json,os,sys,time
from pathlib import Path
mode=MODE
if sys.argv[1:] == ['--version']:
 print('codex-cli 0.162.0' if mode!='version' else 'codex-cli 0.1.0');sys.exit(0)
raw=sys.stdin.buffer.read()
Path(TRACE).write_text(json.dumps({'sha256':hashlib.sha256(raw).hexdigest(),'argv':sys.argv[1:],'pgid':os.getpgrp(),'auth':os.getenv('OPENAI_API_KEY')=='synthetic-auth','pythonpath':os.getenv('PYTHONPATH')}))
if mode=='timeout':time.sleep(10)
if mode=='overflow':print('x'*1048577);sys.exit(0)
if mode=='stderr':sys.stderr.write('secret'*12000);sys.exit(2)
if mode=='drift':Path(CONFIG).write_text('# changed during client run')
rows=[{'type':'thread.started','thread_id':'12345678-1234-4234-8234-123456789abc'}, {'type':'turn.started'}, {'type':'turn.completed'}]
if mode=='failed':rows[-1]={'type':'turn.failed'}
if mode=='missing':rows=rows[:-1]
if mode=='duplicate':rows.insert(1, rows[0])
if mode=='order':rows=[rows[0],rows[2],rows[1]]
if mode=='unknown':rows.insert(2,{'type':'unrecognized'})
if mode=='malformed':print('{broken');sys.exit(0)
for row in rows:print(json.dumps(row))
sys.exit(2 if mode=='nonzero' else 0)
'''
        script = script.replace('PYTHON', installation.PYTHON, 1).replace('MODE',repr(mode)).replace('TRACE',repr(str(self.trace))).replace('CONFIG',repr(str(self.config)))
        self.client.write_text(script); self.client.chmod(0o700)

    def generate(self):
        self.f.write(self.spec_path, self.spec)
        p = subprocess.run([installation.PYTHON, '-I', '-S', '-B', str(self.i.version / 'install_codex_bridge.py'),
                            '--spec', str(self.spec_path), '--destination', str(self.out)], capture_output=True, timeout=30)
        return p.returncode, json.loads(p.stdout)

    def generated(self):
        code, result = self.generate(); self.assertEqual(code, 0, result)
        self.bridge = Path(result['bridge']['executable'])
        self.assertEqual(pin(self.bridge)['sha256'], result['bridge']['sha256'])
        self.assertEqual(self.bridge.stat().st_mode & 0o777, 0o700)
        return result['bridge']

    def raw(self):
        return json.dumps(self.material,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()+b'\n'

    def invoke(self, raw=None, cwd=None, args=()):
        return subprocess.run([str(self.bridge), *args], input=self.raw() if raw is None else raw,
                              cwd=cwd or self.f.root, capture_output=True, timeout=20,
                              env=dict(os.environ, OPENAI_API_KEY='synthetic-auth', PYTHONPATH='/untrusted'))

    def test_exact_bytes_ack_private_install_no_overwrite_and_environment(self):
        self.generated(); p = self.invoke()
        self.assertEqual(p.returncode, 0, p.stderr)
        ack = json.loads(p.stdout); trace = json.loads(self.trace.read_text())
        self.assertEqual(ack['read_id'], self.read_id)
        self.assertEqual(ack['material_sha256'], hashlib.sha256(self.raw()).hexdigest())
        self.assertEqual(trace['sha256'], ack['material_sha256'])
        self.assertTrue(trace['auth']); self.assertIsNone(trace['pythonpath'])
        self.assertEqual(trace['argv'][:8], ['exec','--ephemeral','--json','--sandbox','read-only','--skip-git-repo-check','-C',str(self.f.root)])
        self.assertEqual(self.generate()[0], 2)
        self.assertNotIn(b'synthetic-auth', self.bridge.read_bytes())

    def test_config_client_dependency_and_version_drift_refuse_before_spawn(self):
        dependency = self.f.private/'dependency'; dependency.write_text('pinned'); dependency.chmod(0o600)
        self.spec['files'].append(pin(dependency)); self.generated()
        for changed in (self.client, self.config, dependency):
            original = changed.read_bytes(); changed.write_bytes(original+b'\n# drift')
            p = self.invoke(); self.assertEqual(p.returncode, 2)
            self.assertFalse(self.trace.exists()); changed.write_bytes(original)
        self.client_mode('version'); self.spec['client']=pin(self.client)
        self.out = self.f.private/'bad-version'
        self.assertEqual(self.generate()[0], 2)
        self.assertFalse(self.out.exists())

    def test_spec_and_source_validation(self):
        original = dict(self.spec)
        for change in ({'extra': True}, {'timeout_seconds': True}, {'files': []}, {'client_version':'future'}):
            self.spec = {**original, **change}
            self.assertEqual(self.generate()[0], 2); self.assertFalse(self.out.exists())
        self.spec = original
        template = self.i.version/'codex_bridge.py'; template.write_text('raise RuntimeError("must not import")')
        self.assertEqual(self.generate()[0], 2)
        self.assertFalse(self.out.exists())

    def test_extra_argv_wrong_cwd_task_history_and_invalid_material_refuse(self):
        self.generated()
        self.assertEqual(self.invoke(args=('ignored',)).returncode, 2)
        self.assertEqual(self.invoke(cwd=self.f.private).returncode, 2)
        for change in ({'purpose':'history'}, {'current_use':False}, {'task_id':str(uuid.uuid4())}, {'gaps':['unknown']}):
            old = self.material; self.material = {**old, **change}
            self.assertEqual(self.invoke().returncode, 2); self.material = old
        for raw in (b'{}', b'{' , b'x'*65537, b'{"schema_version":1,"schema_version":1}\n'):
            self.assertEqual(self.invoke(raw=raw).returncode, 2)
        self.assertFalse(self.trace.exists())

    def test_event_failure_limits_and_postrun_drift_never_ack(self):
        for mode in ('failed','missing','duplicate','order','unknown','malformed','nonzero','overflow','stderr','timeout','drift'):
            with self.subTest(mode=mode):
                self.config.write_text('# synthetic\n')
                self.client_mode(mode); self.spec['client']=pin(self.client); self.spec['files']=[pin(self.config)]
                self.spec['timeout_seconds']=1
                self.out=self.f.private/('bridge-'+mode); self.generated()
                p=self.invoke(); self.assertEqual(p.returncode,2)
                self.assertEqual(p.stdout,b''); self.assertEqual(p.stderr,b'codex_bridge_failed\n')

    def test_real_installed_adapter_claim_and_child_process_group(self):
        bridge=self.generated(); f=self.f
        for k in ('binary','binary_sha256','package','package_sha256','python'): f.binding[k]=self.installed[k]
        f.binding['bridge']=bridge; f.binding['timeout_seconds']=30; f.save_binding(); f.record('Read the current marker.')
        def adapter(action, label, **extra):
            args=['current',action,'--binding',f.binding_path,'--run-dir',f.state/label]
            for k,v in extra.items():args+=['--'+k,v]
            return self.i.run_launcher(*args)
        self.assertEqual(adapter('read','read',request=f.request_path)[0],0)
        evidence=f.state/'read/evidence.json'
        self.assertEqual(adapter('launch','launch',evidence=evidence)[0],0)
        trace=self.trace.read_bytes()
        self.assertEqual(adapter('launch','repeat',evidence=evidence)[1]['code'],'delivery_already_attempted')
        self.assertEqual(trace,self.trace.read_bytes())
        self.assertFalse((f.state/'read/material.json').exists())
        self.assertEqual(json.loads((f.state/'launch/delivery.json').read_text())['status'],'acknowledged')

    def test_parent_timeout_kills_client_with_bridge(self):
        self.client_mode('timeout');self.spec['client']=pin(self.client);self.spec['timeout_seconds']=10
        self.generated()
        process=subprocess.Popen([str(self.bridge)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                                 cwd=self.f.root,start_new_session=True)
        process.stdin.write(self.raw());process.stdin.close()
        try:
            until=time.monotonic()+10
            while not self.trace.exists():
                self.assertIsNone(process.poll());self.assertLess(time.monotonic(),until);time.sleep(.02)
            self.assertEqual(json.loads(self.trace.read_text())['pgid'],process.pid)
        finally:
            os.killpg(process.pid,signal.SIGKILL);process.wait()
            process.stdout.close();process.stderr.close()


if __name__ == '__main__':
    unittest.main()
