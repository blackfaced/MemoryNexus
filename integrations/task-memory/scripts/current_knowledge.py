#!/usr/bin/env python3
"""Frozen current-knowledge v1 adapter. G2 implements task read/check only."""
import argparse
import copy
from pathlib import Path
import sys
import uuid as uuid_module

sys.dont_write_bytecode = True
from workflow_io import WorkflowError, new_run, read_regular, safe_path, write_new
from workflow_types import canonical, fail, strict_loads
from current_knowledge_task import (JSON_LIMIT, MATERIAL_LIMIT, call, context_result,
                                    load_binding, request, sha)
from current_knowledge_artifacts import (cleanup, expired, now, purge, purge_failed_check, read_origin, verify_material)

ERRORS = {'invalid_input', 'unsupported_version', 'io_error', 'scope_mismatch'}
CODES = ERRORS | {'stale_snapshot', 'source_drift', 'source_unavailable', 'target_drift',
                 'injection_unknown', 'incomplete', 'empty', 'history_only', 'bridge_unavailable',
                 'delivery_unknown', 'delivery_already_attempted', 'ok'}


def code_for(error):
    if isinstance(error, WorkflowError):
        if error.code in CODES: return error.code
        if error.code in ('unsafe_path', 'oversize'): return 'invalid_input'
    if isinstance(error, (OSError,)): return 'io_error'
    return 'invalid_input'


def snapshot(binding_path, b, pins, req, read_id, created_at):
    header = call(b, 'workflow-status', {})
    body = {'limit': req['limit']}
    if req['purpose'] == 'history': body['record_ids'] = req['record_ids']
    response = call(b, 'context', body)
    if response['revision'] != header['revision']: fail('stale_snapshot')
    try: entries, candidates = context_result(response['result'], req)
    except WorkflowError: fail('io_error')
    checked_at = now()
    copies = []
    for item in sorted(b['copies'], key=lambda c: c['id']):
        row = {'id': item['id'], 'status': 'unknown', 'checked_at': checked_at}
        try: row['sha256'] = sha(read_regular(item['path'], 8 * 1024 * 1024))
        except (WorkflowError, OSError): pass
        copies.append(row)
    result = response['result']; gaps = []
    if copies: gaps.append('injection_unknown')
    if result['truncated'] or result['omitted_count']: gaps.append('incomplete')
    if req['purpose'] == 'current' and not any(row.get('text') for row in entries): gaps.append('empty')
    material = {'schema_version': 1, 'read_id': read_id, 'task_id': b['task_id'],
                'revision': header['revision'], 'incarnation': header['incarnation'],
                'mode': 'task', 'purpose': req['purpose'], 'current_use': req['purpose'] == 'current',
                'entries': entries, 'candidates': candidates, 'outputs': [], 'gaps': gaps}
    evidence = {'schema_version': 1, 'read_id': read_id, 'created_at': created_at, 'checked_at': checked_at,
                'task_id': b['task_id'], 'revision': header['revision'], 'incarnation': header['incarnation'],
                **pins, 'request': req, 'selected': [{k: row[k] for k in ('id', 'kind', 'trust', 'state', 'revision')}
                                                   for row in result['entries'] + result['candidates']],
                'sources': [], 'outputs': [], 'copies': copies,
                'coverage': 'unknown' if copies else 'declared_files',
                'omitted_count': result['omitted_count'], 'candidate_count': result['candidate_count'],
                'truncated': result['truncated'], 'gaps': gaps}
    if load_binding(binding_path)[1] != pins: fail('stale_snapshot')
    for item, observed in zip(sorted(b['copies'], key=lambda c: c['id']), copies):
        try: current_hash = sha(read_regular(item['path'], 8 * 1024 * 1024))
        except (WorkflowError, OSError): current_hash = None
        if current_hash != observed.get('sha256'): fail('stale_snapshot')
    final = call(b, 'workflow-status', {'at_revision': header['revision']})
    if final['incarnation'] != header['incarnation']: fail('stale_snapshot')
    raw = canonical(material) + b'\n'
    if len(raw) > MATERIAL_LIMIT:
        if 'incomplete' not in gaps: gaps.append('incomplete')
    if not gaps:
        evidence['material'] = {'path': 'material.json', 'sha256': sha(raw), 'bytes': len(raw)}
    if len(canonical(evidence)) + 1 > JSON_LIMIT: fail('incomplete')
    return raw, evidence


def stable(evidence):
    value = copy.deepcopy(evidence)
    value.pop('checked_at', None)
    for rows in ('sources', 'copies'):
        for row in value[rows]: row.pop('checked_at', None)
    return value


class Parser(argparse.ArgumentParser):
    def error(self, message): fail()


def arguments(argv):
    parser = Parser(add_help=False, allow_abbrev=False)
    parser.add_argument('command', choices=('read', 'check', 'launch'))
    for flag in ('binding', 'request', 'evidence', 'run-dir'): parser.add_argument('--' + flag)
    args = parser.parse_args(argv)
    if not args.binding or not args.run_dir: fail()
    if args.command == 'read':
        if not args.request or args.evidence: fail()
    elif not args.evidence or args.request: fail()
    return args


def execute(argv):
    read_id = str(uuid_module.uuid4()); run = None; origin = None; evidence = None; material = None
    code = 'ok'; previous_digest = None; args = None
    try:
        args = arguments(argv)
        b, pins = load_binding(args.binding)
        if Path(sys.argv[0]).resolve() != Path(b['package']) / 'scripts/current_knowledge.py': fail('stale_snapshot')
        run_path = safe_path(args.run_dir)
        if run_path.parent != Path(b['state_root']): fail()
        run = new_run(run_path, b['state_root'], b['task_root'])
        cleanup(b['state_root'])
        if args.command == 'launch': fail('unsupported_version')
        created_at = now()
        if args.command == 'read':
            req = request(strict_loads(read_regular(args.request, 32768, True)))
        else:
            old, previous_digest, origin = read_origin(args.evidence, b['state_root'])
            read_id = old['read_id']; created_at = old['created_at']; req = old['request']
            if old['task_id'] != b['task_id']: fail('scope_mismatch')
            if req['purpose'] != 'current': fail('history_only')
            if expired(old): fail('stale_snapshot')
            verify_material(origin, old['material'])
            if any(old[key] != value for key, value in pins.items()): fail('stale_snapshot')
        for attempt in range(2 if args.command == 'read' else 1):
            try:
                material, evidence = snapshot(args.binding, b, pins, req, read_id, created_at)
                break
            except WorkflowError as exc:
                if exc.code != 'stale_snapshot' or args.command != 'read' or attempt == 1: raise
        if args.command == 'check':
            if stable(evidence) != stable(old): fail('stale_snapshot')
            evidence.pop('material', None)
            evidence['previous_evidence_sha256'] = previous_digest
            material = None
        if evidence['gaps']: fail(evidence['gaps'][0])
    except (WorkflowError, OSError, ValueError, TypeError, RecursionError) as exc:
        code = code_for(exc); material = None
        if evidence:
            evidence.pop('material', None)
            evidence['gaps'] = list(dict.fromkeys([code, *evidence['gaps']]))
            if previous_digest: evidence['previous_evidence_sha256'] = previous_digest
        if args and args.command == 'check': purge_failed_check(args.binding, args.evidence)
        if origin:
            try: purge(origin)
            except (WorkflowError, OSError): pass
    result = {'schema_version': 1, 'status': 'ok' if code == 'ok' else ('error' if code in ERRORS else 'needs_review'),
              'read_id': read_id, 'code': code, 'artifacts': []}
    try:
        if run:
            if material is not None:
                write_new(run / 'material.json', material)
                result['artifacts'].append({'path': 'material.json', 'sha256': sha(material), 'bytes': len(material)})
            if evidence:
                raw = canonical(evidence) + b'\n'
                if len(raw) > JSON_LIMIT: fail('incomplete')
                write_new(run / 'evidence.json', raw)
                result['artifacts'].append({'path': 'evidence.json', 'sha256': sha(raw), 'bytes': len(raw)})
            write_new(run / 'result.json', canonical(result) + b'\n')
    except (WorkflowError, OSError) as exc:
        if run:
            try: purge(run)
            except (WorkflowError, OSError): pass
        code = code_for(exc)
        result.update(status='error' if code in ERRORS else 'needs_review', code=code, artifacts=[])
    return result


def main():
    result = execute(sys.argv[1:])
    sys.stdout.buffer.write(canonical(result) + b'\n')
    return {'ok': 0, 'needs_review': 3, 'error': 2}[result['status']]


if __name__ == '__main__':
    raise SystemExit(main())
