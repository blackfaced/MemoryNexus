"""Private original-read evidence and bounded material expiry; no authority state."""
from datetime import datetime, timezone
from pathlib import Path
import re

from workflow_io import WorkflowError, read_regular, safe_path
from workflow_types import array, fields, hash_value, number, strict_loads, uuid, digest, name, string
from current_knowledge_task import JSON_LIMIT, MATERIAL_LIMIT, request, sha


def now():
    return datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')


def instant(value):
    if not isinstance(value, str) or not re.fullmatch(r'\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ', value):
        raise WorkflowError('invalid_input')
    try: return datetime.strptime(value, '%Y-%m-%dT%H:%M:%SZ').replace(tzinfo=timezone.utc)
    except ValueError: raise WorkflowError('invalid_input') from None


def expired(evidence):
    age = (datetime.now(timezone.utc) - instant(evidence['created_at'])).total_seconds()
    return age < 0 or age >= 86400


def read_origin(path, state):
    path = safe_path(path, True)
    root = safe_path(state, True)
    if path.name != 'evidence.json' or path.parent.parent != root:
        raise WorkflowError('invalid_input')
    safe_path(path.parent, True)
    raw = read_regular(path, JSON_LIMIT, True)
    e = strict_loads(raw)
    fields(e, ('schema_version', 'read_id', 'created_at', 'checked_at', 'task_id', 'revision', 'incarnation',
               'binding_sha256', 'config_sha256', 'binary_sha256', 'package_sha256', 'request',
               'selected', 'sources', 'outputs', 'copies', 'coverage', 'omitted_count', 'candidate_count',
               'truncated', 'gaps', 'material'), ('workflow',))
    if type(e['schema_version']) is not int or e['schema_version'] != 1: raise WorkflowError('unsupported_version')
    for key in ('read_id', 'task_id', 'incarnation'): uuid(e[key])
    for key in ('binding_sha256', 'config_sha256', 'binary_sha256', 'package_sha256'): hash_value(e[key])
    for key in ('revision', 'omitted_count', 'candidate_count'): number(e[key])
    instant(e['created_at']); instant(e['checked_at']); request(e['request'])
    for row in array(e['selected'], 400):
        fields(row, ('id', 'kind', 'trust', 'state', 'revision'))
        uuid(row['id']); number(row['revision'], 1)
        if row['kind'] not in ('instruction', 'execution', 'checkpoint', 'lesson', 'draft'): raise WorkflowError('invalid_input')
        if row['trust'] not in ('owner_confirmed', 'execution_reported', 'review_candidate', 'source_reported'): raise WorkflowError('invalid_input')
        if row['state'] not in ('current', 'superseded', 'stale', 'retracted'): raise WorkflowError('invalid_input')
    if e['request']['mode'] == 'task':
        if 'workflow' in e or any(e[k] != [] for k in ('sources', 'outputs', 'copies')):
            raise WorkflowError('invalid_input')
    else:
        workflow = e.get('workflow')
        fields(workflow, ('workflow_id', 'profile_revision', 'profile_sha256', 'binding_sha256'))
        uuid(workflow['workflow_id']); number(workflow['profile_revision'], 1)
        hash_value(workflow['profile_sha256']); hash_value(workflow['binding_sha256'])
        for row in array(e['sources'], 400):
            fields(row, ('source_id', 'source_revision', 'sha256', 'availability', 'observed_at', 'checked_at'))
            uuid(row['source_id']); number(row['source_revision'], 1); hash_value(row['sha256'])
            string(row['observed_at'], 1, 64); instant(row['checked_at'])
            if row['availability'] != 'available': raise WorkflowError('invalid_input')
        for row in array(e['outputs'], 400):
            fields(row, ('consumer_id', 'decision_id', 'approval_id', 'block_id', 'output_sha256'))
            name(row['consumer_id']); uuid(row['decision_id']); uuid(row['approval_id'])
            hash_value(row['block_id']); hash_value(row['output_sha256'])
        for row in array(e['copies'], 20):
            fields(row, ('id', 'status', 'sha256', 'checked_at'))
            name(row['id']); hash_value(row['sha256']); instant(row['checked_at'])
            if row['status'] != 'verified': raise WorkflowError('invalid_input')
    if e['gaps'] != [] or e['coverage'] != 'declared_files': raise WorkflowError('invalid_input')
    if e['truncated'] is not False or e['omitted_count'] != 0: raise WorkflowError('invalid_input')
    ref = e['material']
    fields(ref, ('path', 'sha256', 'bytes')); hash_value(ref['sha256']); number(ref['bytes'], 1, MATERIAL_LIMIT)
    if ref['path'] != 'material.json': raise WorkflowError('invalid_input')
    result = strict_loads(read_regular(path.parent / 'result.json', JSON_LIMIT, True))
    fields(result, ('schema_version', 'status', 'read_id', 'code', 'artifacts'))
    if type(result['schema_version']) is not int or result['schema_version'] != 1 or result['status'] != 'ok' or result['code'] != 'ok' or result['read_id'] != e['read_id']:
        raise WorkflowError('invalid_input')
    refs = array(result['artifacts'], 2, 2)
    if refs != [ref, {'path': 'evidence.json', 'sha256': sha(raw), 'bytes': len(raw)}]:
        raise WorkflowError('stale_snapshot')
    return e, sha(raw), path.parent


def verify_material(run, ref):
    path = safe_path(run / 'material.json', True)
    try: raw = read_regular(path, MATERIAL_LIMIT, True)
    except FileNotFoundError: return
    if sha(raw) != ref['sha256'] or len(raw) != ref['bytes']:
        raise WorkflowError('stale_snapshot')


def purge(run):
    # The caller has validated the original read directory and fixed basename.
    path = safe_path(run / 'material.json', True)
    path.unlink(missing_ok=True)


def cleanup(state):
    root = safe_path(state, True)
    # Bound traversal, including non-directories; never recursively scan the ledger.
    children = []
    for path in root.iterdir():
        children.append(path)
        if len(children) > 1000: raise WorkflowError('incomplete')
    for child in children:
        try:
            if child.is_symlink() or not child.is_dir(): continue
            evidence, _, run = read_origin(child / 'evidence.json', root)
            if expired(evidence):
                verify_material(run, evidence['material']); purge(run)
        except (WorkflowError, OSError, ValueError, TypeError, RecursionError):
            # Unknown/damaged artifacts belong to the owner, not our cleanup.
            continue


def purge_failed_check(binding_path, evidence_path):
    """Installation drift still invalidates an identifiable original material copy."""
    try:
        binding = strict_loads(read_regular(binding_path, 32768, True))
        if not isinstance(binding, dict) or not isinstance(binding.get('state_root'), str): return
        old, _, run = read_origin(evidence_path, binding['state_root'])
        if old['binding_sha256'] == digest(binding): purge(run)
    except (WorkflowError, OSError, ValueError, TypeError, RecursionError):
        pass
