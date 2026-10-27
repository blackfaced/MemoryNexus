"""Read-only task snapshot, pinned installation and bounded public CLI transport."""
import hashlib
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time

from workflow_cli import package_inventory
from workflow_io import WorkflowError, read_regular, safe_path
from workflow_types import (array, canonical, digest, fail, fields, hash_value,
                            name, number, strict_loads, string, unique, uuid)

JSON_LIMIT = 131072
MATERIAL_LIMIT = 65536


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def inside(path, root):
    return path == root or root in path.parents


def load_binding(path):
    path = safe_path(path, True)
    b = strict_loads(read_regular(path, 32768, True))
    fields(b, ('schema_version', 'task_id', 'task_root', 'config', 'ledger', 'binary',
               'binary_sha256', 'python', 'package', 'package_sha256', 'state_root',
               'timeout_seconds', 'client', 'copies'), ('repository_binding', 'bridge'))
    if type(b['schema_version']) is not int or b['schema_version'] != 1:
        fail('unsupported_version')
    uuid(b['task_id']); name(b['client']); number(b['timeout_seconds'], 1, 300)
    for key in ('binary_sha256', 'package_sha256'):
        hash_value(b[key])
    paths = {key: safe_path(b[key]) for key in
             ('task_root', 'config', 'ledger', 'binary', 'python', 'package', 'state_root')}
    root, state, package = paths['task_root'], paths['state_root'], paths['package']
    if paths['config'] != root / '.memorynexus.json': fail('scope_mismatch')
    if any(inside(paths[key], root) for key in ('ledger', 'state_root', 'package')): fail('scope_mismatch')
    if inside(path, root) or inside(path, package) or inside(path, state): fail('scope_mismatch')
    if inside(state, package) or inside(package, state) or inside(paths['ledger'], state): fail('scope_mismatch')
    if inside(state, paths['ledger'].parent): fail('scope_mismatch')
    for key in ('task_root', 'state_root', 'package'):
        if not paths[key].is_dir(): fail('io_error')
    safe_path(state, True); safe_path(paths['ledger'], True)
    safe_path(paths['ledger'].parent, True)
    if not paths['ledger'].is_file(): fail('io_error')
    if paths['python'] != Path(sys.executable).resolve(): fail('stale_snapshot')
    if Path(__file__).resolve() != package / 'scripts/current_knowledge_task.py': fail('stale_snapshot')
    if not os.access(paths['binary'], os.X_OK) or not os.access(paths['python'], os.X_OK): fail()
    raw_config = read_regular(paths['config'], 32768, True)
    config = strict_loads(raw_config)
    fields(config, ('schema_version', 'task_id', 'ledger_env'))
    if type(config['schema_version']) is not int or config['schema_version'] != 1: fail('unsupported_version')
    if config['task_id'] != b['task_id'] or config['ledger_env'] != 'MEMORYNEXUS_TASK_LEDGER': fail('scope_mismatch')
    if sha(read_regular(paths['binary'], 128 * 1024 * 1024)) != b['binary_sha256']: fail('stale_snapshot')
    if digest(package_inventory(package)) != b['package_sha256']: fail('stale_snapshot')
    for c in array(b['copies'], 20):
        fields(c, ('id', 'path', 'role'), ('consumer_id',))
        name(c['id']); string(c['path'], 1, 4096)
        copy_path = Path(c['path'])
        if not copy_path.is_absolute() or '..' in copy_path.parts or '\x00' in c['path']: fail()
        if c['role'] == 'managed':
            fields(c, ('id', 'path', 'role', 'consumer_id')); name(c['consumer_id'])
        elif c['role'] == 'external': fields(c, ('id', 'path', 'role'))
        else: fail()
    unique([c['id'] for c in b['copies']]); unique([c['path'] for c in b['copies']])
    if 'repository_binding' in b:
        safe_path(b['repository_binding'], True)
        # G2 cannot attest the profile/kernel agreement owned by G3.
        fail('unsupported_version')
    if 'bridge' in b:
        fields(b['bridge'], ('executable', 'sha256'))
        safe_path(b['bridge']['executable']); hash_value(b['bridge']['sha256'])
    return b, {'binding_sha256': digest(b), 'config_sha256': sha(raw_config),
               'binary_sha256': b['binary_sha256'], 'package_sha256': b['package_sha256']}


def request(value):
    if not isinstance(value, dict): fail()
    if type(value.get('schema_version')) is not int or value['schema_version'] != 1: fail('unsupported_version')
    if value.get('mode') == 'repository':
        fields(value, ('schema_version', 'mode', 'purpose', 'consumer_ids'))
        if value['purpose'] != 'current': fail()
        for x in array(value['consumer_ids'], 2, 1): name(x)
        unique(value['consumer_ids']); fail('unsupported_version')
    if value.get('mode') != 'task' or value.get('purpose') not in ('current', 'history'): fail()
    fields(value, ('schema_version', 'mode', 'purpose', 'limit') + (('record_ids',) if value['purpose'] == 'history' else ()))
    number(value['limit'], 1, 20)
    if value['purpose'] == 'history':
        for x in array(value['record_ids'], 20, 1): uuid(x)
        unique(value['record_ids'])
    return value


def transport(argv, body, env, cwd, timeout):
    """Bound both pipes during execution, including descendants holding pipes open."""
    raw = canonical(body)
    if len(raw) > 32768: fail()
    try:
        child = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, cwd=cwd, env=env, start_new_session=True)
    except OSError: fail('io_error')
    output = bytearray(); selector = selectors.DefaultSelector()
    deadline = time.monotonic() + timeout; remaining = memoryview(raw); stderr_size = 0
    try:
        for stream, event in ((child.stdin, selectors.EVENT_WRITE),
                              (child.stdout, selectors.EVENT_READ), (child.stderr, selectors.EVENT_READ)):
            os.set_blocking(stream.fileno(), False); selector.register(stream, event)
        while selector.get_map():
            left = deadline - time.monotonic()
            if left <= 0: fail('io_error')
            for key, _ in selector.select(min(left, 0.1)):
                stream = key.fileobj
                if stream is child.stdin:
                    try: sent = os.write(stream.fileno(), remaining)
                    except BrokenPipeError: sent = len(remaining)
                    remaining = remaining[sent:]
                    if not remaining: selector.unregister(stream); stream.close()
                else:
                    data = os.read(stream.fileno(), 8192)
                    if not data: selector.unregister(stream); stream.close()
                    elif stream is child.stdout:
                        output.extend(data)
                        if len(output) > JSON_LIMIT: fail('io_error')
                    else:
                        stderr_size += len(data)
                        if stderr_size > 4096: fail('io_error')
        try: code = child.wait(timeout=max(0.001, deadline - time.monotonic()))
        except subprocess.TimeoutExpired: fail('io_error')
        return code, bytes(output)
    finally:
        selector.close()
        try: os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError: pass
        child.wait()
        for stream in (child.stdin, child.stdout, child.stderr): stream.close()


def call(b, action, body):
    if action not in ('context', 'workflow-status'): fail()
    env = {'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'MEMORYNEXUS_TASK_LEDGER': b['ledger']}
    code, raw = transport([b['binary'], 'task', '--config', b['config'], action],
                          body, env, b['task_root'], b['timeout_seconds'])
    try: value = strict_loads(raw)
    except WorkflowError: fail('io_error')
    if not isinstance(value, dict) or type(value.get('schema_version')) is not int or value['schema_version'] != 1: fail('io_error')
    if code:
        if code != 2 or value.get('status') != 'error': fail('io_error')
        error = value.get('code')
        if error in ('conflict', 'recovery_required'): fail('stale_snapshot')
        if error in ('scope_mismatch', 'unsupported_version'): fail(error)
        fail('io_error')
    if value.get('status') != 'ok': fail('io_error')
    if value.get('task_id') != b['task_id']: fail('scope_mismatch')
    try:
        keys = ('schema_version', 'status', 'task_id', 'revision', 'result')
        fields(value, keys + (('workflow_contract', 'incarnation') if action == 'workflow-status' else ()))
        number(value['revision'])
        if action == 'workflow-status':
            if type(value['workflow_contract']) is not int or value['workflow_contract'] != 1: fail()
            uuid(value['incarnation']); fields(value['result'], ('items', 'complete'), ('next_after',))
            array(value['result']['items'], 20)
            if type(value['result']['complete']) is not bool: fail()
    except WorkflowError: fail('io_error')
    return value


def context_result(value, req):
    """Validate context before copying any entry body."""
    fields(value, ('display_name', 'entries', 'candidates', 'candidate_count', 'omitted_count', 'truncated'))
    string(value['display_name'], 1, 120); number(value['candidate_count']); number(value['omitted_count'])
    if type(value['truncated']) is not bool or value['truncated'] != (value['omitted_count'] > 0): fail()
    entries, candidates = array(value['entries'], 20), array(value['candidates'], 20)
    if len(entries) + len(candidates) > req['limit']: fail()
    seen = []
    for candidate, rows in ((False, entries), (True, candidates)):
        for row in rows:
            fields(row, ('id', 'kind', 'trust', 'state', 'revision', 'occurred_at', 'run_id', 'client', 'source_ids'), ('text', 'evidence_refs', 'supersedes_id'))
            uuid(row['id']); uuid(row['run_id']); number(row['revision'], 1)
            string(row['client'], 1, 64); string(row['occurred_at'], 1, 64)
            for x in array(row['source_ids'], 20): uuid(x)
            unique(row['source_ids']); seen.append(row['id'])
            if 'supersedes_id' in row: uuid(row['supersedes_id'])
            if row['kind'] not in ('instruction', 'checkpoint', 'execution', 'lesson', 'draft'): fail()
            if row['state'] not in ('current', 'superseded', 'stale', 'retracted'): fail()
            if row['trust'] not in ('owner_confirmed', 'execution_reported', 'review_candidate', 'source_reported'): fail()
            if candidate != (row['trust'] == 'review_candidate'): fail()
            if candidate and row['kind'] != 'lesson': fail()
            if row['state'] in ('stale', 'retracted'):
                if 'text' in row or 'evidence_refs' in row: fail()
            else:
                string(row.get('text'), 1, 1000)
                for ref in array(row.get('evidence_refs'), 5):
                    fields(ref, ('path',), ('sha256',)); string(ref['path'], 1, 32768)
                    if 'sha256' in ref: hash_value(ref['sha256'])
            if req['purpose'] == 'current':
                if row['state'] != 'current' or row['kind'] == 'draft': fail()
                if row['kind'] == 'lesson' and not candidate and row['trust'] != 'owner_confirmed': fail()
            elif row['id'] not in req['record_ids']: fail()
    unique(seen)
    if req['purpose'] == 'history' and len(seen) + value['omitted_count'] != len(req['record_ids']): fail()
    return entries, [{k: row[k] for k in ('id', 'kind', 'trust', 'state')} for row in candidates]
