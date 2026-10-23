"""Pinned subprocess transport. The CLI remains the state authority."""
import hashlib
import os
from pathlib import Path
import subprocess
import sys
from workflow_io import WorkflowError, read_regular, safe_path
from workflow_types import (canonical, digest, strict_loads, fields, validate_binding,
                            validate_profile, uuid, number, hash_value, name, fail)

ACTIONS = {'workflow-bind','source-sync','source-capture','consumer-review',
           'consumer-approve','publication-result','workflow-status'}


def _inside(path, parent):
    return path == parent or parent in path.parents


def package_inventory(path):
    """Canonical sorted inventory; no runtime symlinks or hidden digest recursion."""
    root=safe_path(str(path))
    files=[]
    for entry in sorted(root.rglob('*')):
        safe_path(str(entry))
        if entry.is_file():
            files.append({'path':entry.relative_to(root).as_posix(),
                          'sha256':hashlib.sha256(read_regular(entry, 8*1024*1024)).hexdigest()})
    return files


def load_context(binding_path):
    path=safe_path(str(binding_path),private=True)
    safe_path(str(path.parent),private=True)
    b=validate_binding(strict_loads(read_regular(path,32768,private=True)))
    paths={k:safe_path(b[k]) for k in ('task_root','config','ledger','binary','python','package','profile','state_root')}
    root=paths['task_root']
    safe_path(str(paths['state_root']),private=True)
    safe_path(str(paths['ledger']),private=True)
    safe_path(str(paths['ledger'].parent),private=True)
    if paths['python'] != Path(sys.executable).resolve(): fail('stale_snapshot')
    if not os.access(paths['binary'],os.X_OK) or not os.access(paths['python'],os.X_OK): fail('unsafe_path')
    if paths['config']!=root/'.memorynexus.json': fail('scope_mismatch')
    for k in ('ledger','package','state_root'):
        if _inside(paths[k],root): fail('scope_mismatch')
    if not paths['ledger'].is_file() or not paths['state_root'].is_dir() or not paths['package'].is_dir(): fail('not_found')
    config=strict_loads(read_regular(paths['config'],32768,private=True))
    fields(config,('schema_version','task_id','ledger_env'))
    if config['schema_version'] != 1: fail('unsupported_version')
    if config['task_id']!=b['task_id'] or config['ledger_env']!='MEMORYNEXUS_TASK_LEDGER': fail('scope_mismatch')
    if hashlib.sha256(read_regular(paths['binary'],128*1024*1024)).hexdigest()!=b['binary_sha256']: fail('stale_snapshot')
    if digest(package_inventory(paths['package']))!=b['package_sha256']: fail('stale_snapshot')
    p=validate_profile(strict_loads(read_regular(paths['profile'],32768,private=True)))
    if p['workflow_id']!=b['workflow_id']: fail('scope_mismatch')
    if {t['target_id'] for t in b['targets']}!={c['target_id'] for c in p['consumers']}: fail('scope_mismatch')
    targets=[]
    for target in b['targets']:
        t=safe_path(target['path'])
        if not t.parent.is_dir(): fail('not_found')
        for k in ('config','ledger','binary','python','package','state_root','profile'):
            if _inside(t,paths[k]): fail('scope_mismatch')
        for c in p['collections']:
            s=safe_path(str(root/c['path']))
            if s==t and c['selector']['kind']!='marked': fail('scope_mismatch')
        targets.append(t)
    # Every rule digest must identify an actual bounded UTF-8 package file.
    hashes=set()
    for item in package_inventory(paths['package']):
        candidate=paths['package']/item['path']
        try:
            raw=read_regular(candidate,32768)
            raw.decode('utf-8')
            hashes.add(hashlib.sha256(raw).hexdigest())
        except (WorkflowError, UnicodeError):
            pass
    if any(c['rules_sha256'] not in hashes for c in p['consumers']): fail('stale_snapshot')
    # Capability discovery is storage-free and pinned; never probe legacy aliases.
    try:
        capability=subprocess.run([b['binary'],'task','--help'],capture_output=True,timeout=b['timeout_seconds'],env={'PATH':'/usr/bin:/bin','LANG':'C.UTF-8'},check=False)
        help_value=strict_loads(capability.stdout)
        commands=help_value.get('result',{}).get('commands',[])
        if capability.returncode or not ACTIONS.issubset(commands): fail('unsupported_version')
    except (subprocess.TimeoutExpired,OSError,AttributeError,TypeError): fail('unsupported_version')
    return {'binding':b,'profile':p,'binding_sha256':digest(b),'profile_sha256':digest(p)}


def call(ctx, action, body):
    if action not in ACTIONS: fail()
    b=ctx['binding']; raw=canonical(body)
    if len(raw)>32768: fail()
    env={'HOME':os.environ.get('HOME','/'), 'PATH':str(Path(b['package'])/'bin')+':/usr/bin:/bin',
         'LANG':'C.UTF-8','MEMORYNEXUS_TASK_LEDGER':b['ledger']}
    if os.environ.get('TMPDIR'): env['TMPDIR']=os.environ['TMPDIR']
    try:
        result=subprocess.run([b['binary'],'task','--config',b['config'],action],input=raw,
                              capture_output=True,timeout=b['timeout_seconds'],env=env,check=False)
    except subprocess.TimeoutExpired: fail('transport_timeout')
    except OSError: fail('io_error')
    if len(result.stdout)>131072: fail('invalid_response')
    try: response=strict_loads(result.stdout)
    except WorkflowError: fail('invalid_response')
    if not isinstance(response,dict) or response.get('schema_version')!=1 or response.get('workflow_contract')!=1: fail('invalid_response')
    if result.returncode!=0:
        if result.returncode!=2 or response.get('status')!='error' or not isinstance(response.get('code'),str): fail('invalid_response')
        raise WorkflowError(response['code'])
    if response.get('status') not in ('accepted','replayed','noop','ok') or response.get('task_id')!=b['task_id']: fail('invalid_response')
    fields(response,('schema_version','workflow_contract','status','task_id','revision','incarnation','result'))
    uuid(response.get('incarnation')); number(response.get('revision'))
    if not isinstance(response.get('result'),dict): fail('invalid_response')
    _validate_result(action,response['result'])
    if 'incarnation' in body and response['incarnation']!=body['incarnation']: fail('recovery_required')
    return response


def status_all(ctx, view='summary'):
    body={'workflow_id':ctx['binding']['workflow_id'],'view':view,'limit':20}
    items=[]; seen=set(); revision=None
    while True:
        r=call(ctx,'workflow-status',body)
        if revision is None: revision=r['revision']
        if r['revision']!=revision: fail('conflict')
        result=r['result']
        if not isinstance(result.get('items'),list) or type(result.get('complete')) is not bool: fail('invalid_response')
        items.extend(result['items'])
        if result['complete']: return items
        after=result.get('next_after'); number(after,1)
        if after in seen: fail('invalid_response')
        seen.add(after); body.update(after=after,at_revision=revision)


def _validate_result(action, result):
    required={
        'workflow-bind':('workflow_id','profile_revision','profile_sha256','binding_sha256','state'),
        'source-sync':('collection_id','scan','sources','invalidated'),
        'source-capture':('record_id','root_record_id','source_id','source_revision','state','kind','trust'),
        'consumer-review':('decision_id','generation','root_record_id','decision','validity','dependency_sha256','decision_sha256','block_id'),
        'consumer-approve':('approval_id','decision_id','decision','validity','publication','proposal'),
        'publication-result':('approval_id','attempt_id','outcome','validity','publication'),
        'workflow-status':('items','complete'),
    }[action]
    optional={'source-sync':('cursor',),'workflow-status':('next_after',)}.get(action,())
    fields(result,required,optional)
    for key,value in result.items():
        if key in ('workflow_id','collection_id','record_id','root_record_id','source_id','decision_id','approval_id','attempt_id'): uuid(value)
        elif key in ('profile_revision','scan','source_revision','generation'): number(value,1)
        elif key.endswith('_sha256') or key=='block_id': hash_value(value)
    if action=='source-sync':
        if not isinstance(result['sources'],list) or len(result['sources'])>20: fail('invalid_response')
        for source in result['sources']:
            fields(source,('source_id','source_revision','sha256','availability'))
            uuid(source['source_id']); number(source['source_revision'],1); hash_value(source['sha256'])
        fields(result['invalidated'],('records','decisions','approvals'))
        for value in result['invalidated'].values(): number(value)
    if action=='consumer-approve':
        fields(result['proposal'],('target_id','block_id','base_exists','base_sha256','after_sha256','output_sha256','plan_sha256'))
        name(result['proposal']['target_id'])
        for key,value in result['proposal'].items():
            if key.endswith('_sha256') or key=='block_id': hash_value(value)
        if type(result['proposal']['base_exists']) is not bool: fail('invalid_response')
