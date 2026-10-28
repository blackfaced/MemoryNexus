"""Private synthetic tasks and actual CLI/adapter processes for current knowledge."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import uuid

from workflow_cli import package_inventory
from workflow_types import canonical, digest
from workflow_fixture import BINARY, REPO, SCRIPTS


class CurrentFixture:
    def __init__(self):
        if not BINARY.is_file(): raise AssertionError('Build the real CLI; skipped tests are forbidden')
        self.temp = tempfile.TemporaryDirectory(prefix='mn-current-', dir=Path(tempfile.gettempdir()).resolve())
        self.base = Path(self.temp.name); os.chmod(self.base, 0o700)
        for name in ('task', 'other', 'private', 'data', 'reads', 'package'):
            (self.base / name).mkdir(mode=0o700)
        self.root = self.base / 'task'; self.config = self.root / '.memorynexus.json'
        self.other = self.base / 'other/.memorynexus.json'
        self.private = self.base / 'private'; self.state = self.base / 'reads'
        self.ledger = self.base / 'data/v2.sqlite'; self.v1 = self.base / 'data/v1.sqlite'
        self.package = self.base / 'package'
        for source in SCRIPTS.glob('*.py'):
            dest = self.package / source.name
            shutil.copyfile(source, dest); os.chmod(dest, 0o600)
        self.task_id = self.cli('init', {'idempotency_key': 'a', 'display_name': 'Task A',
                                       'confirmation': 'confirmed', 'automatic_capture': ['execution', 'review']}, ledger=self.v1)['task_id']
        self.other_id = self.cli('init', {'idempotency_key': 'x', 'display_name': 'Task X',
                                        'confirmation': 'confirmed', 'automatic_capture': ['execution', 'review']}, ledger=self.v1, config=self.other)['task_id']
        backup = self.base / 'data/before.sqlite'
        self.storage('backup', {'path': str(backup)}, self.v1)
        self.storage('migrate', {'path': str(self.ledger), 'backup_path': str(backup),
                                'backup_sha256': hashlib.sha256(backup.read_bytes()).hexdigest(),
                                'expected_version': 1, 'target_version': 2, 'confirmation': 'confirmed'}, self.v1)
        self.binding = {'schema_version': 1, 'task_id': self.task_id, 'task_root': str(self.root),
                        'config': str(self.config), 'ledger': str(self.ledger), 'binary': str(BINARY),
                        'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
                        'python': str(Path(sys.executable).resolve()), 'package': str(self.package),
                        'package_sha256': digest(package_inventory(self.package)), 'state_root': str(self.state),
                        'timeout_seconds': 5, 'client': 'synthetic', 'copies': []}
        self.binding_path = self.private / 'binding.json'; self.save_binding()
        self.request = {'schema_version': 1, 'mode': 'task', 'purpose': 'current', 'limit': 20}
        self.request_path = self.private / 'request.json'; self.write(self.request_path, self.request)
        self.entry = self.package / 'current_knowledge.py'

    def close(self): self.temp.cleanup()

    def write(self, path, value):
        path.write_bytes(canonical(value)); os.chmod(path, 0o600)

    def save_binding(self): self.write(self.binding_path, self.binding)

    def invoke_cli(self, args, body, ledger=None, expected=0):
        env = dict(os.environ)
        if ledger: env['MEMORYNEXUS_TASK_LEDGER'] = str(ledger)
        p = subprocess.run([str(BINARY), 'task', *map(str, args)], input=canonical(body), capture_output=True, env=env, timeout=15)
        if p.returncode != expected: raise AssertionError((p.returncode, p.stdout, p.stderr))
        return json.loads(p.stdout)

    def cli(self, action, body, ledger=None, config=None, expected=0):
        return self.invoke_cli(['--config', config or self.config, action], body, ledger or self.ledger, expected)

    def storage(self, action, body, ledger=None):
        return self.invoke_cli(['--ledger', ledger or self.ledger, action], body)

    def write_request(self, **changes):
        self.write(self.request_path, {**self.request, **changes})

    def mutation(self, action, **fields):
        body = {'idempotency_key': str(uuid.uuid4()), 'run_id': str(uuid.uuid4()), 'client': 'synthetic',
                'occurred_at': '2026-10-27T00:00:00Z', **fields}
        return self.cli(action, body)

    def record(self, text='Work in workspace-a.', kind='instruction', **fields):
        return self.mutation('record', kind=kind, text=text, confirmation='confirmed', **fields)['result']['record_id']

    def command(self, command='read', label=None, evidence=None):
        run = self.state / (label or str(uuid.uuid4()))
        args = [self.binding['python'], '-B', str(self.entry), command, '--binding', str(self.binding_path), '--run-dir', str(run)]
        args += ['--request', str(self.request_path)] if command == 'read' else ['--evidence', str(evidence)]
        return args, run

    def run(self, command='read', label=None, evidence=None):
        args, run = self.command(command, label, evidence)
        # An unrelated inherited ledger must never affect the reader.
        env = dict(os.environ, MEMORYNEXUS_TASK_LEDGER='/nonexistent/unrelated.sqlite', PYTHONDONTWRITEBYTECODE='1')
        p = subprocess.run(args, capture_output=True, env=env, timeout=30)
        if not p.stdout: raise AssertionError((p.returncode, p.stderr))
        return p.returncode, json.loads(p.stdout), run

    def payload(self, run, name='material.json'): return json.loads((run / name).read_bytes())

    def interpose(self, mode):
        """Instrument the *real* CLI boundary; do not synthesize CLI responses."""
        wrapper = self.private / 'cli-barrier'
        script = f'''#!{self.binding['python']}
import os,sys,json,time,subprocess
from pathlib import Path
real={str(BINARY)!r}
base=Path({str(self.private)!r})
mode={mode!r}
raw=sys.stdin.buffer.read()
if sys.argv[-1]=='context':
 count=base/'count'
 n=int(count.read_text())+1 if count.exists() else 1
 count.write_text(str(n))
 if mode=='timeout': time.sleep(30)
 if mode=='overflow':
  sys.stdout.write('x'*140000); sys.exit(0)
 if mode=='stderr':
  sys.stderr.write('x'*5000); sys.exit(1)
 if mode=='barrier':
  (base/('ready-'+str(n))).touch()
  until=time.monotonic()+10
  while not (base/('release-'+str(n))).exists():
   if time.monotonic()>until: sys.exit(2)
   time.sleep(.005)
p=subprocess.run([real,*sys.argv[1:]],input=raw,capture_output=True)
sys.stdout.buffer.write(p.stdout);sys.stderr.buffer.write(p.stderr);sys.exit(p.returncode)
'''
        wrapper.write_text(script); os.chmod(wrapper, 0o700)
        self.binding.update(binary=str(wrapper), binary_sha256=hashlib.sha256(wrapper.read_bytes()).hexdigest())
        self.save_binding()


class RepositoryFixture(CurrentFixture):
    """Existing public workflow fixture plus a distinct read-only binding/state."""
    def __init__(self, collections=1, jsonl=False):
        from workflow_fixture import Fixture
        self.workflow = w = Fixture(collections)
        self.temp = w.temp; self.base = w.base; self.root = w.root; self.config = w.config
        self.private = w.private; self.ledger = w.ledger; self.package = w.package; self.task_id = w.task_id
        self.state = self.base / 'reads'; self.state.mkdir(mode=0o700)
        self.binding = {k: w.binding[k] for k in ('schema_version', 'task_id', 'task_root', 'config', 'ledger',
                       'binary', 'binary_sha256', 'python', 'package', 'package_sha256', 'timeout_seconds')}
        self.binding.update(state_root=str(self.state), client='synthetic', copies=[], repository_binding=str(w.binding_path))
        self.binding_path = self.private / 'current-binding.json'; self.save_binding()
        self.request = dict(schema_version=1, mode='repository', purpose='current',
                            consumer_ids=[c['consumer_id'] for c in w.profile['consumers']])
        self.request_path = self.private / 'current-request.json'; self.write(self.request_path, self.request)
        self.entry = self.package / 'current_knowledge.py'
        if jsonl:
            def change(p):
                c = p['collections'][0]; c.update(collection_id='50000000-0000-4000-8000-000000000099', format='append_jsonl', path='stream.jsonl', selector={'kind': 'fields', 'keys': ['text']})
            w.rebind(change)
            (self.root / 'stream.jsonl').write_text('{"text":"first source row"}\n')

    def seed(self):
        from test_workflow_publish_supersede import phase, reviews
        w = self.workflow
        w.capture('capture')
        self.review, self.review_report = phase(w, 'review', reviews, 'review')
        self.plan, _ = phase(w, 'publish', lambda p: [], 'publish')
        if len(w.profile['collections']) > 1:
            phase(w, 'publish', lambda p: [], 'publish-more')

    def status(self, view='summary'):
        return self.cli('workflow-status', {'workflow_id': self.workflow.profile['workflow_id'], 'view': view})

    def page_barrier(self):
        """Pause a real pinned CLI before a selected status page; never fake data."""
        wrapper = self.private / 'page-cli'
        script = f'''#!{self.binding['python']}
import sys,json,subprocess,time
from pathlib import Path
base=Path({str(self.private)!r})
raw=sys.stdin.buffer.read()
body=json.loads(raw) if raw else {{}}
if (base/'armed').exists() and sys.argv[-1]=='workflow-status':
 with (base/'calls.jsonl').open('a') as out: out.write(json.dumps(body)+'\\n')
 if body.get('view')=='inputs' and body.get('after'):
  count=base/'page-count'; n=int(count.read_text())+1 if count.exists() else 1
  count.write_text(str(n)); (base/('page-ready-'+str(n))).touch()
  until=time.monotonic()+10
  while not (base/('page-release-'+str(n))).exists():
   if time.monotonic()>until: sys.exit(2)
   time.sleep(.005)
p=subprocess.run([{str(BINARY)!r},*sys.argv[1:]],input=raw,capture_output=True)
sys.stdout.buffer.write(p.stdout);sys.stderr.buffer.write(p.stderr);sys.exit(p.returncode)
'''
        wrapper.write_text(script); os.chmod(wrapper, 0o700)
        for b in (self.binding, self.workflow.binding):
            b.update(binary=str(wrapper), binary_sha256=hashlib.sha256(wrapper.read_bytes()).hexdigest())
        self.save_binding(); self.workflow.write(self.workflow.binding_path, self.workflow.binding)
        self.workflow.rebind(lambda p: None)


def install_bridge(f, mode='ack'):
    """Fresh synthetic client: derive a marker path solely from material stdin."""
    bridge = f.private / 'synthetic-bridge'
    script = f'''#!{f.binding['python']}
import hashlib,json,os,re,sys,time,uuid
from pathlib import Path
raw=sys.stdin.buffer.read(65537)
if len(raw)>65536: sys.exit(2)
material=json.loads(raw)
if not material['current_use']: sys.exit(2)
texts=[r['text'] for r in material['entries'] if 'text' in r]+[o['text'] for o in material['outputs']]
paths=set(re.findall(r'workspace-[a-z]+/marker.txt','\\n'.join(texts)))
if len(paths)!=1: sys.exit(2)
chosen=next(iter(paths)); value=Path(chosen).read_bytes()
trace=Path({str(f.private / 'bridge-trace.jsonl')!r})
row=dict(pid=os.getpid(),read_id=material['read_id'],sha256=hashlib.sha256(raw).hexdigest(),bytes=len(raw),
         opened=chosen,marker_sha256=hashlib.sha256(value).hexdigest(),argc=len(sys.argv),cwd=os.getcwd(),
         ledger_matches=os.environ.get('MEMORYNEXUS_TASK_LEDGER')=={str(f.ledger)!r},
         inherited_auth=os.environ.get('MN_SYNTHETIC_AUTH')=='synthetic-secret')
with trace.open('a') as out:
 out.write(json.dumps(row)+'\\n');out.flush();os.fsync(out.fileno())
mode={mode!r}
if mode=='timeout': time.sleep(30)
if mode=='barrier':
 release=Path({str(f.private / 'bridge-release')!r})
 until=time.monotonic()+15
 while not release.exists():
  if time.monotonic()>until: sys.exit(2)
  time.sleep(.005)
ack=dict(schema_version=1,read_id=material['read_id'],material_sha256=hashlib.sha256(raw).hexdigest(),session_ref=str(uuid.uuid4()))
if mode=='wrong_hash': ack['material_sha256']='a'*64
if mode=='wrong_id': ack['read_id']=str(uuid.uuid4())
if mode=='extra': ack['unapproved']='field'
if mode=='long_session': ack['session_ref']='x'*129
if mode=='no_ack': sys.exit(0)
if mode=='nonzero': print(json.dumps(ack));sys.exit(2)
if mode=='duplicate': print('{{"schema_version":1,"schema_version":1}}');sys.exit(0)
if mode=='overflow': print('x'*4097);sys.exit(0)
if mode=='stderr': sys.stderr.write('synthetic-secret'*400);sys.stderr.flush();sys.exit(2)
print(json.dumps(ack))
'''
    bridge.write_text(script); os.chmod(bridge, 0o700)
    f.binding['bridge'] = dict(executable=str(bridge), sha256=hashlib.sha256(bridge.read_bytes()).hexdigest())
    f.save_binding()
    for name in ('workspace-a', 'workspace-b'):
        directory = f.root / name; directory.mkdir(exist_ok=True)
        (directory / 'marker.txt').write_text('Harmless marker: ' + name)
    return bridge


def attach_workflow(f):
    """Bind a real workflow to CurrentFixture's pre-migrated two-task ledger."""
    from workflow_fixture import Fixture
    w = Fixture.__new__(Fixture)
    for key in ('temp', 'base', 'root', 'private', 'package', 'config', 'ledger', 'task_id'):
        setattr(w, key, getattr(f, key))
    w.state = f.base / 'workflow-runs'; w.state.mkdir(mode=0o700)
    w.incarnation = f.cli('workflow-status', {})['incarnation']
    w.profile = json.loads((REPO / 'docs/repository-memory/examples/sequence-v1.json').read_text())['profile']
    w.profile['collections'] = w.profile['collections'][:1]
    w.profile['collections'][0].update(path='incident.md', selector={'kind': 'full'})
    (w.root / 'incident.md').write_text('A synthetic source report.\n')
    w.rule = b'Preserve provenance and uncertainty.\n'
    for c in w.profile['consumers']: c['rules_sha256'] = hashlib.sha256(w.rule).hexdigest()
    w.profile_path = f.private / 'workflow-profile.json'; w.write(w.profile_path, w.profile)
    w.binding_path = f.private / 'workflow-binding.json'
    w.binding = {k: f.binding[k] for k in ('schema_version', 'task_id', 'task_root', 'config', 'ledger', 'binary',
                                           'binary_sha256', 'python', 'package', 'package_sha256', 'timeout_seconds')}
    w.binding.update(workflow_id=w.profile['workflow_id'], profile=str(w.profile_path), state_root=str(w.state),
                     targets=[dict(target_id=c['target_id'], path=str(f.private / (c['target_id'] + '.md'))) for c in w.profile['consumers']])
    w.refresh_package()
    from workflow_cli import load_context
    w.ctx = load_context(w.binding_path)
    f.cli('workflow-bind', dict(idempotency_key='workflow-bind', run_id=str(uuid.uuid4()), client='synthetic',
          workflow_id=w.profile['workflow_id'], profile_revision=1, incarnation=w.incarnation,
          expected_profile_revision=0, profile=w.profile, profile_sha256=digest(w.profile), binding_sha256=digest(w.binding),
          confirmation='confirmed'))
    f.binding.update(package_sha256=w.binding['package_sha256'], repository_binding=str(w.binding_path)); f.save_binding()
    return w
