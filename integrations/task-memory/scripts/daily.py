#!/usr/bin/env python3
"""Existing-task daily transport; the Rust CLI remains the sole state authority."""
import argparse
import os
from pathlib import Path
import sys
import uuid as uuid_module

sys.dont_write_bytecode = True
# -I deliberately excludes cwd/PYTHONPATH; import only the pinned sibling package.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from current_knowledge_task import load_binding, transport_bytes, context_result, inside, sha
from workflow_io import WorkflowError, safe_path, read_regular, write_new, new_run, task_lock
from workflow_types import (array, canonical, digest, fail, fields, number, string,
                            strict_loads, unique, uuid, hash_value, relative)

ACTIONS = {'record', 'review', 'correct', 'retract'}
STATES = {'current', 'historical', 'superseded', 'stale', 'retracted'}
CODES = {'invalid_input', 'scope_mismatch', 'unsupported_version', 'stale_snapshot',
         'conflict', 'not_found', 'storage_unavailable', 'unsafe_path', 'oversize',
         'io_error', 'busy', 'invalid_response', 'readback_failed'}


def invoke(b, action, raw):
    argv = [b['binary'], 'task', '--help'] if action == 'help' else [
        b['binary'], 'task', '--config', b['config'], action]
    code, output = transport_bytes(argv, raw, {
        'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8',
        'MEMORYNEXUS_TASK_LEDGER': b['ledger'],
    }, b['task_root'], b['timeout_seconds'])
    try:
        response = strict_loads(output)
        if not isinstance(response, dict) or type(response.get('schema_version')) is not int or response['schema_version'] != 1:
            fail('invalid_response')
    except WorkflowError:
        fail('invalid_response')
    if code:
        error = response.get('code')
        if code == 2 and response.get('status') == 'error' and isinstance(error, str) and error in CODES:
            fail(error)
        fail('invalid_response')
    try:
        keys = ('schema_version', 'status', 'revision', 'result')
        fields(response, keys if action == 'help' else keys + ('task_id',))
        number(response['revision'])
        if action != 'help' and response['task_id'] != b['task_id']:
            fail('scope_mismatch')
        if response['status'] not in (('accepted', 'replayed') if action in ACTIONS else ('ok',)):
            fail('invalid_response')
        if not isinstance(response['result'], dict): fail('invalid_response')
        if action in ACTIONS:
            fields(response['result'], ('record_id', 'state'))
            uuid(response['result']['record_id'])
            if response['result']['state'] not in STATES: fail('invalid_response')
    except WorkflowError as exc:
        if exc.code == 'scope_mismatch': raise
        fail('invalid_response')
    return response


def bind(path):
    b, pins = load_binding(path)
    if Path(__file__).resolve() != Path(b['package']) / 'daily.py': fail('stale_snapshot')
    state = Path(b['state_root'])
    if any((parent / '.git').exists() for parent in (state, *state.parents)):
        fail('unsafe_path')
    # The existing reader binding does not imply a repository workflow or v2.
    help_result = invoke(b, 'help', b'')['result']
    if (help_result.get('command') != 'task' or
            not isinstance(help_result.get('commands'), list) or
            not (ACTIONS | {'context'}).issubset(help_result['commands'])):
        fail('unsupported_version')
    return b, pins


def identity(b, pins):
    # Cross-client retry may change only the caller label, never original bytes.
    return {'binding': {k: v for k, v in b.items() if k != 'client'},
            'config_sha256': pins['config_sha256']}


def read_context(b, request):
    fields(request, (), ('limit', 'record_ids'))
    limit = number(request.get('limit', 20), 1, 20)
    req = {'limit': limit, 'purpose': 'current'}
    if 'record_ids' in request:
        for item in array(request['record_ids'], 20): uuid(item)
        unique(request['record_ids'])
        req.update(purpose='history', record_ids=request['record_ids'])
    response = invoke(b, 'context', canonical(request))
    try:
        context_result(response['result'], req, allow_historical=True)
        rows = response['result']['entries'] + response['result']['candidates']
        if any(row['revision'] > response['revision'] for row in rows): fail()
        if len(response['result']['display_name']) + sum(len(row.get('text', '')) for row in rows) > 12000: fail()
    except WorkflowError:
        fail('invalid_response')
    return response


def envelope(action, raw):
    body = strict_loads(raw)
    common = ('idempotency_key', 'run_id', 'client')
    required = {
        'record': ('kind', 'text', 'occurred_at'),
        'review': ('text', 'occurred_at', 'source_ids'),
        'correct': ('target_id', 'text', 'occurred_at', 'confirmation'),
        'retract': ('target_id', 'reason', 'confirmation'),
    }[action]
    optional = {
        'record': ('confirmation', 'evidence_refs'),
        'review': ('confirmation',),
        'correct': ('source_ids', 'evidence_refs'), 'retract': (),
    }[action]
    fields(body, common + required, optional)
    string(body['idempotency_key'], 1, 128); uuid(body['run_id']); string(body['client'], 1, 64)
    if 'confirmation' in body and body['confirmation'] != 'confirmed': fail()
    if action == 'record':
        if body['kind'] not in ('checkpoint', 'execution', 'instruction'): fail()
        if body['kind'] == 'instruction' and body.get('confirmation') != 'confirmed': fail()
    if 'target_id' in body: uuid(body['target_id'])
    if 'text' in body: string(body['text'], 1, 1000)
    if 'reason' in body: string(body['reason'], 1, 240)
    if 'occurred_at' in body: string(body['occurred_at'], 1, 64)
    if 'source_ids' in body:
        for item in array(body['source_ids'], 20, 1 if action == 'review' else 0): uuid(item)
        unique(body['source_ids'])
    for ref in array(body.get('evidence_refs', []), 5):
        fields(ref, ('path',), ('sha256',)); relative(ref['path'])
        if 'sha256' in ref: hash_value(ref['sha256'])
    return body


def run_path(b, value):
    run = safe_path(value, True)
    if run == Path(b['state_root']) or not inside(run, Path(b['state_root'])):
        fail('unsafe_path')
    return run


def stage(b, pins, args):
    request_path = safe_path(args.request, True)
    if inside(request_path, Path(b['task_root'])): fail('unsafe_path')
    raw = read_regular(request_path, 32768, True)
    body = envelope(args.action, raw)
    if body['client'] != b['client']: fail('scope_mismatch')
    request = {'record_ids': [body['target_id']]} if 'target_id' in body else {'limit': 1}
    context = read_context(b, request)
    if 'target_id' in body:
        rows = context['result']['entries'] + context['result']['candidates']
        if len(rows) != 1 or (args.action == 'correct' and rows[0]['state'] != 'current'):
            fail('conflict')
    run = new_run(run_path(b, args.run_dir), state_root=b['state_root'], task_root=b['task_root'])
    filename = str(uuid_module.uuid4()) + '.json'
    note = {'schema_version': 1, 'identity': identity(b, pins), 'action': args.action,
            'request': filename, 'request_sha256': sha(raw), 'idempotency_key': body['idempotency_key']}
    write_new(run / filename, raw)
    write_new(run / 'pending.json', canonical(note))
    return {'status': 'staged', 'task_id': b['task_id'], 'run_dir': str(run),
            'request_sha256': sha(raw), 'action': args.action}


def readback(b, receipt, body=None):
    result = read_context(b, {'record_ids': [receipt['record_id']]})
    rows = result['result']['entries'] + result['result']['candidates']
    if (result['result']['truncated'] or len(rows) != 1 or
            rows[0]['id'] != receipt['record_id'] or result['revision'] < receipt['revision']):
        fail('readback_failed')
    row = rows[0]
    if receipt['action'] == 'retract' and row['state'] != 'retracted': fail('readback_failed')
    if body and row['state'] == 'current':
        for key in ('text', 'client', 'run_id', 'occurred_at', 'source_ids', 'evidence_refs'):
            if key in body and row.get(key) != body[key]: fail('readback_failed')
        if receipt['action'] == 'record':
            trust = 'owner_confirmed' if body.get('confirmation') == 'confirmed' else 'execution_reported'
            if row['kind'] != body['kind'] or row['trust'] != trust: fail('readback_failed')
        if receipt['action'] == 'review' and (row['kind'] != 'lesson' or row['trust'] != 'review_candidate'):
            fail('readback_failed')
        if receipt['action'] == 'correct' and (row['trust'] != 'owner_confirmed' or row.get('supersedes_id') != body['target_id']):
            fail('readback_failed')
    return result


def validate_receipt(receipt, b, pins):
    fields(receipt, ('schema_version', 'identity_sha256', 'action', 'idempotency_key',
                     'request_sha256', 'task_id', 'record_id', 'revision', 'status', 'state'))
    if (type(receipt['schema_version']) is not int or receipt['schema_version'] != 1 or
            receipt['identity_sha256'] != digest(identity(b, pins)) or receipt['task_id'] != b['task_id']):
        fail('scope_mismatch')
    if receipt['action'] not in ACTIONS or receipt['status'] not in ('accepted', 'replayed') or receipt['state'] not in STATES: fail('invalid_response')
    uuid(receipt['record_id']); number(receipt['revision']); hash_value(receipt['request_sha256'])
    string(receipt['idempotency_key'], 1, 128)


def apply(b, pins, args):
    run = run_path(b, args.run_dir)
    with task_lock(b['state_root'], b['task_id'], 'daily'):
        note_path, receipt_path = run / 'pending.json', run / 'receipt.json'
        note = None; body = None
        if note_path.exists():
            note = strict_loads(read_regular(note_path, 32768, True))
            fields(note, ('schema_version', 'identity', 'action', 'request', 'request_sha256', 'idempotency_key'))
            if (type(note['schema_version']) is not int or note['schema_version'] != 1 or
                    note['identity'] != identity(b, pins)): fail('scope_mismatch')
            if note['action'] not in ACTIONS: fail()
            filename = note['request']
            if not isinstance(filename, str) or not filename.endswith('.json'): fail()
            uuid(filename[:-5]); hash_value(note['request_sha256'])
            raw = read_regular(run / filename, 32768, True) if (run / filename).exists() else None
            # A receipt plus missing payload is the valid cleanup-crash boundary.
            if raw is not None:
                if sha(raw) != note['request_sha256']: fail('stale_snapshot')
                body = envelope(note['action'], raw)
                if body['idempotency_key'] != note['idempotency_key']: fail('stale_snapshot')
        if receipt_path.exists():
            receipt = strict_loads(read_regular(receipt_path, 32768, True))
            validate_receipt(receipt, b, pins)
            if note and any(receipt[k] != note[k] for k in ('action', 'request_sha256', 'idempotency_key')):
                fail('stale_snapshot')
        else:
            if note is None or body is None: fail('not_found')
            # Verify registration again before invoking a write; no fallback DB.
            read_context(b, {'limit': 1})
            response = invoke(b, note['action'], raw)
            receipt = {'schema_version': 1, 'identity_sha256': digest(identity(b, pins)),
                       'action': note['action'], 'idempotency_key': note['idempotency_key'],
                       'request_sha256': note['request_sha256'], 'task_id': b['task_id'],
                       'record_id': response['result']['record_id'], 'revision': response['revision'],
                       'status': response['status'], 'state': response['result']['state']}
        context = readback(b, receipt, body)
        if not receipt_path.exists(): write_new(receipt_path, canonical(receipt))
        status = 'synchronized'
        if note:
            try:
                (run / note['request']).unlink(missing_ok=True)
                note_path.unlink()
            except OSError:
                status = 'saved_cleanup_pending'
        return {'status': status, 'receipt': receipt, 'readback': context, 'run_dir': str(run)}


class Parser(argparse.ArgumentParser):
    def error(self, message): fail()


def main():
    os.umask(0o077)
    action = None
    try:
        parser = Parser(description=__doc__, allow_abbrev=False)
        sub = parser.add_subparsers(dest='command', required=True, parser_class=Parser)
        for name in ('verify', 'read', 'stage', 'apply'):
            p = sub.add_parser(name, allow_abbrev=False)
            p.add_argument('--binding', required=True)
            if name in ('read', 'stage'): p.add_argument('--request', required=name == 'stage')
            if name in ('stage', 'apply'): p.add_argument('--run-dir', required=True)
            if name == 'stage': p.add_argument('--action', required=True, choices=sorted(ACTIONS))
        args = parser.parse_args(); action = args.command
        if not sys.flags.isolated: fail('unsafe_path')
        b, pins = bind(args.binding)
        if action == 'verify':
            context = read_context(b, {'limit': 1})
            result = {'status': 'ok', 'task_id': b['task_id'], 'revision': context['revision'],
                      'protocol_version': 1, 'binary_sha256': b['binary_sha256'],
                      'package_sha256': b['package_sha256']}
        elif action == 'read':
            request = strict_loads(read_regular(safe_path(args.request, True), 32768, True)) if args.request else {'limit': 20}
            context = read_context(b, request)
            result = {'status': 'incomplete' if context['result']['truncated'] else 'ok', 'context': context}
        elif action == 'stage': result = stage(b, pins, args)
        else: result = apply(b, pins, args)
        print(canonical({'schema_version': 1, **result}).decode())
        return 3 if result['status'] in ('incomplete', 'saved_cleanup_pending') else 0
    except (WorkflowError, OSError, ValueError, TypeError, KeyError):
        exc = sys.exc_info()[1]
        code = exc.code if isinstance(exc, WorkflowError) and exc.code in CODES else 'io_error'
        status = 'pending' if action == 'apply' else ('not_staged' if action == 'stage' else 'history_not_loaded')
        print(canonical({'schema_version': 1, 'status': status, 'code': code}).decode())
        return 2


if __name__ == '__main__':
    sys.exit(main())
