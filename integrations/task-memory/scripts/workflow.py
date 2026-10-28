#!/usr/bin/env python3
"""Explicit repository-memory adapter. Standard library, pinned local CLI only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
sys.dont_write_bytecode = True
import uuid
from datetime import datetime, timezone
from workflow_io import WorkflowError, read_regular, write_new, new_run, artifact_ref, safe_path

def now():
    return datetime.now(timezone.utc).isoformat()

def sha(b):
    return hashlib.sha256(b).hexdigest()

def encode(v):
    return (json.dumps(v, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()

def canonical(v):
    return json.dumps(v, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()

def load(p):

    def obj(pairs):
        d = {}
        for (k, v) in pairs:
            if k in d:
                raise WorkflowError('invalid_input')
            d[k] = v
        return d
    v = json.loads(read_regular(p, 8 * 1024 * 1024), object_pairs_hook=obj)

    def valid(v):
        if v is None or isinstance(v, float):
            raise WorkflowError('invalid_input')
        if isinstance(v, dict):
            for x in v.values():
                valid(x)
        if isinstance(v, list):
            for x in v:
                valid(x)
    valid(v)
    return v

def exact(v, required, optional=()):
    if not isinstance(v, dict) or not set(required) <= set(v) or set(v) - set(required) - set(optional):
        raise WorkflowError('invalid_input')

def report(workflow, run_id, items, status=None):
    from workflow_trigger import metadata
    gap,_ = metadata()
    if gap:
        items=list(items)+[dict(id='trigger',status='needs_review',code='unverified_trigger',receipt_ids=[])]
        if status in ('ok','noop'):
            status=None
    good = any((i['status'] == 'ok' for i in items))
    bad = any((i['status'] in ('failed', 'pending', 'needs_review') for i in items))
    failed = any(i['status']=='failed' for i in items)
    status = status or ('partial' if good and bad else 'failed' if failed else 'needs_review' if bad else 'ok' if good else 'noop')
    return dict(schema_version=1, status=status, run_id=run_id, workflow=workflow, trigger={'kind': 'manual'}, items=items, pending=[i['id'] for i in items if i['status'] in ('pending', 'needs_review', 'failed')], report_errors=[])

def save_report(run, r, initial=True):
    try:
        from workflow_trigger import retain
        retain(run)
        write_new(run / 'summary.json', encode(r))
        files = []
        for p in sorted(run.rglob('*')):
            if p.is_file() and p.name != 'artifacts.json':
                a = artifact_ref(run, str(p.relative_to(run)))
                a['mode'] = '0600'
                files.append(a)
        write_new(run / 'artifacts.json', encode({'schema_version': 1, 'files': files}))
    except (OSError, WorkflowError):
        r['report_errors'].append({'artifact': 'summary', 'code': 'io_error'})
        r['status'] = 'partial' if any((i['status'] == 'ok' for i in r['items'])) else 'failed'
    return r

def inspect(a):
    root = safe_path(a.root)
    selection = load(a.selection)
    exact(selection, ['schema_version', 'paths'])
    if type(selection['schema_version']) is not int or selection['schema_version'] != 1 or (not isinstance(selection['paths'], list)) or (not 1 <= len(selection['paths']) <= 30) or (len(set(selection['paths'])) != len(selection['paths'])):
        raise WorkflowError('invalid_input')
    run = new_run(a.run_dir, task_root=root)
    files = []
    for name in selection['paths']:
        item = {'path': name}
        try:
            if not isinstance(name, str) or Path(name).is_absolute() or '..' in Path(name).parts or (':' in name):
                raise WorkflowError('unsafe_path')
            raw = read_regular(root / name, 2 * 1024 * 1024)
            text = raw.decode('utf-8')
            if '\x00' in text:
                raise WorkflowError('binary')
            item.update(status='read', sha256=sha(raw), bytes=len(raw))
        except FileNotFoundError:
            item.update(status='missing', reason='not_found')
        except UnicodeError:
            item.update(status='unreadable', reason='invalid_text')
        except (OSError, WorkflowError) as e:
            item.update(status='unreadable', reason=getattr(e, 'code', 'io_error') if getattr(e, 'code', 'io_error') in ('unsafe_path', 'oversize', 'binary', 'invalid_text') else 'io_error')
        files.append(item)
    write_new(run / 'inventory.json', encode(dict(schema_version=1, root=str(root), observed_at=now(), files=files)))
    return save_report(run, report('inspect', str(uuid.uuid4()), [dict(id='inventory', status='ok', receipt_ids=[])]))

def main():
    os.umask(63)

    class Parser(argparse.ArgumentParser):

        def error(self, message):
            raise WorkflowError('invalid_input')
    parser = Parser(description=__doc__)
    subs = parser.add_subparsers(dest='command', required=True)
    p = subs.add_parser('inspect')
    p.add_argument('--root', required=True)
    p.add_argument('--selection', required=True)
    p.add_argument('--run-dir', required=True)
    for command in ('prepare', 'preview', 'apply', 'status'):
        p = subs.add_parser(command)
        p.add_argument('--binding', required=True)
        if command == 'prepare':
            p.add_argument('--workflow', choices=('capture', 'review', 'publish', 'recovery'), required=True)
            p.add_argument('--client', required=True)
            p.add_argument('--removals')
        if command == 'preview':
            p.add_argument('--prepared', required=True)
            p.add_argument('--submission', required=True)
        if command in ('prepare', 'preview'):
            p.add_argument('--run-dir', required=True)
        if command == 'apply':
            p.add_argument('--plan', required=True)
            p.add_argument('--approve')
        if command == 'status':
            p.add_argument('--plan')
    try:
        a = parser.parse_args()
        if a.command == 'inspect':
            r = inspect(a)
        else:
            from workflow_engine import execute
            r = execute(a)
    except (WorkflowError, OSError, ValueError, TypeError, KeyError) as e:
        code = getattr(e, 'code', 'io_error' if isinstance(e, OSError) else 'invalid_input')
        status = 'busy' if code == 'busy' else 'failed' if code in ('io_error', 'invalid_input', 'invalid_response', 'unsupported_version', 'unsafe_path') else 'needs_review'
        r = report('unknown', str(uuid.uuid4()), [{'id': 'operation', 'status': 'failed' if status in ('failed', 'busy') else 'needs_review', 'code': code, 'receipt_ids': []}], status=status)
    print(json.dumps(r, ensure_ascii=False))
    return {'ok': 0, 'noop': 0, 'failed': 2, 'needs_review': 3, 'partial': 4, 'busy': 5}[r['status']]
if __name__ == '__main__':
    sys.exit(main())
