#!/usr/bin/env python3
"""Explicit, local task-memory bootstrap adapter. Python standard library only."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import uuid


class Rejected(Exception):
    pass


class LaunchFailed(Rejected):
    """The OS did not start the executable; no child command ran."""


def require(condition, message):
    if not condition:
        raise Rejected(message)


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode('utf-8')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def read_bytes(path, limit=128 * 1024):
    safe_path(str(path))
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as handle:
        require(stat.S_ISREG(os.fstat(handle.fileno()).st_mode), 'regular file required')
        raw = handle.read(limit + 1)
    require(len(raw) <= limit, 'file exceeds size limit')
    return raw


def read_json(path):
    return json.loads(read_bytes(path), object_pairs_hook=unique_object)


def fields(value, expected):
    require(isinstance(value, dict) and set(value) == set(expected.split()), 'unexpected or missing fields')


def bounded(value, limit):
    require(isinstance(value, str) and value.strip() and len(value) <= limit, 'invalid bounded text')


def safe_path(value, private=False, *, cli_executable=False):
    require(isinstance(value, str) and Path(value).is_absolute(), 'absolute path required')
    path = Path(value)
    require('..' not in path.parts and str(path) == value, 'canonical absolute path required')
    for part in [*reversed(path.parents), path]:
        if not part.exists() and not part.is_symlink():
            continue
        info = part.lstat()
        require(not stat.S_ISLNK(info.st_mode), 'symlink path refused')
        # Cargo can hardlink the read-only CLI artifact into target/debug/deps.
        require(not stat.S_ISREG(info.st_mode) or info.st_nlink == 1 or
                (cli_executable and part == path), 'hardlinked file refused')
        require(info.st_uid in (0, os.getuid()), 'path owner mismatch')
        # System temporary roots are safe only with their sticky bit.
        require(not info.st_mode & 0o022 or (stat.S_ISDIR(info.st_mode) and info.st_uid == 0 and info.st_mode & stat.S_ISVTX), 'writable parent refused')
    if private and path.exists():
        info = path.stat()
        require(info.st_uid == os.getuid() and not info.st_mode & 0o077, 'owner-only state required')
    return path


def outside(path, root):
    require(path != root and root not in path.parents, 'state and ledger must be outside task directory')


def relative_source(value):
    bounded(value, 240)
    path = Path(value)
    require(not path.is_absolute() and '..' not in path.parts and str(path) == value and ':' not in value, 'invalid relative source')
    return value


def validate_draft(data):
    fields(data, 'schema_version task_root binding sources entries conflicts gaps')
    require(type(data['schema_version']) is int and data['schema_version'] == 1, 'unsupported schema')
    root = safe_path(data['task_root'])
    require(root.is_dir(), 'task directory missing')
    binding = data['binding']
    fields(binding, 'config ledger display_name automatic_capture')
    config = safe_path(binding['config'])
    require(config == root / '.memorynexus.json', 'config must be task-root .memorynexus.json')
    ledger = safe_path(binding['ledger'])
    outside(ledger, root)
    bounded(binding['display_name'], 120)
    require(binding['automatic_capture'] == ['execution', 'review'], 'bootstrap requires explicit execution/review capture policy')
    require(isinstance(data['sources'], list) and 1 <= len(data['sources']) <= 30, 'select 1 to 30 source files')
    require(len(set(data['sources'])) == len(data['sources']), 'duplicate source')
    for source in data['sources']:
        relative_source(source)
    for key in ('conflicts', 'gaps'):
        require(isinstance(data[key], list) and len(data[key]) <= 30, 'invalid gaps or conflicts')
        for item in data[key]:
            bounded(item, 1000)
    entries = data['entries']
    require(isinstance(entries, list) and 1 <= len(entries) <= 12, 'seed requires 1 to 12 records')
    seen = {}
    for entry in entries:
        require(isinstance(entry, dict), 'invalid entry')
        kind = entry.get('kind')
        require(kind in ('checkpoint', 'execution', 'instruction', 'lesson'), 'invalid entry kind')
        fields(entry, 'id kind text ' + ('source_entries' if kind == 'lesson' else 'sources'))
        require(isinstance(entry['id'], str) and re.fullmatch(r'[a-z][a-z0-9_-]{0,39}', entry['id']), 'invalid entry id')
        require(entry['id'] != 'init' and entry['id'] not in seen, 'reserved or duplicate entry id')
        bounded(entry['text'], 1000)
        refs = entry['source_entries'] if kind == 'lesson' else entry['sources']
        require(isinstance(refs, list) and 1 <= len(refs) <= (12 if kind == 'lesson' else 5), 'invalid source references')
        require(all(isinstance(ref, str) for ref in refs) and len(set(refs)) == len(refs), 'invalid source references')
        for ref in refs:
            require((ref in seen and seen[ref] != 'lesson') if kind == 'lesson' else ref in data['sources'], 'source must exist and lessons must cite earlier base records')
        seen[entry['id']] = kind
    require(sum(e['kind'] == 'checkpoint' for e in entries) <= 1, 'at most one checkpoint')
    require(sum(len(e['text']) for e in entries) + len(binding['display_name']) <= 12000, 'context text budget exceeded')
    return root


def snapshot(data):
    root = Path(data['task_root'])
    sources = []
    for name in data['sources']:
        path = safe_path(str(root / name))
        require(path.is_file() and path.stat().st_size <= 2 * 1024 * 1024, 'source missing or exceeds 2 MiB')
        sources.append({'path': name, 'sha256': digest(read_bytes(path, 2 * 1024 * 1024))})
    git = None
    if (root / '.git').exists():
        def git_read(*args):
            result = subprocess.run(['git', '-C', str(root), *args], capture_output=True, timeout=10, env={**os.environ, 'GIT_OPTIONAL_LOCKS': '0'})
            require(result.returncode == 0 and len(result.stdout) <= 65536, 'Git snapshot unavailable or exceeds 64 KiB')
            return result.stdout.decode('utf-8')
        git = {'head': git_read('rev-parse', 'HEAD').strip(),
               'status': git_read('status', '--porcelain=v1', '--untracked-files=normal', '--', '.', ':(exclude).memorynexus.json')}
    return {'sources': sources, 'git': git}


def write_new(path, data):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as handle:
        handle.write(data)
        handle.flush()
        os.fsync(handle.fileno())
    parent = os.open(path.parent, os.O_RDONLY)
    try:
        os.fsync(parent)
    finally:
        os.close(parent)


def render(manifest, sha):
    data = manifest['draft']
    lines = ['# Task-memory bootstrap preview', '', '**UNCONFIRMED — no memory has been written.**', '',
             f'Manifest SHA-256: `{sha}`', '', 'Approval applies to every exact entry and this binding, including the capture policy.',
             'Instruction entries become owner_confirmed only after this exact-content approval. Reports remain execution_reported; lessons remain review_candidate.', '',
             '## Binding', '', '```json', json.dumps(data['binding'], ensure_ascii=False, indent=2), '```',
             '', f'Task root: `{data["task_root"]}`', f'Import ID: `{manifest["import_id"]}`', f'Occurrence time: `{manifest["created_at"]}`',
             'Task ID: generated by confirmed init and retained in validated receipts.',
             'Provenance: client `task-bootstrap`; run ID is the import ID; keys are `bootstrap:<import-id>:<entry-id>`.', '', '## Source snapshot', '', '```json',
             json.dumps(manifest['snapshot'], ensure_ascii=False, indent=2), '```', '', '## Proposed writes', '',
             'First: confirmed init of a new task. Existing tasks are refused. Then, in this order:']
    for entry in data['entries']:
        trust = 'review_candidate' if entry['kind'] == 'lesson' else ('owner_confirmed after exact approval' if entry['kind'] == 'instruction' else 'execution_reported')
        lines += ['', f'### {entry["id"]}: {entry["kind"]} ({trust})', '', '```json', json.dumps(entry, ensure_ascii=False, indent=2), '```']
    for key in ('conflicts', 'gaps'):
        lines += ['', f'## {key.title()}', ''] + ['- ' + item for item in data[key]] if data[key] else ['', f'## {key.title()}', '', 'None recorded.']
    lines += ['', 'Conflicts block apply. Gaps remain visible and do not become evidence.',
              'Application is serial, not batch atomic. A partial result retains pending envelopes for explicit resume.',
              'This snapshot does not automatically refresh. Later source or binding changes require a new preview and approval.',
              'Synthetic execution does not establish #301 real-client acceptance or the #283 personal-experiment gate.', '']
    return '\n'.join(lines).encode('utf-8')


def preview(args):
    draft = read_json(Path(args.draft))
    root = validate_draft(draft)
    state = safe_path(args.state_dir)
    outside(state, root)
    require(not state.exists() and state.parent.is_dir(), 'state must be a new directory with an existing safe parent')
    manifest = {'schema_version': 1, 'import_id': str(uuid.uuid4()),
                'created_at': datetime.now(timezone.utc).isoformat(), 'draft': draft, 'snapshot': snapshot(draft)}
    raw = encode(manifest)
    state.mkdir(mode=0o700)
    write_new(state / 'manifest.json', raw)
    write_new(state / 'preview.md', render(manifest, digest(raw)))
    return {'status': 'preview', 'manifest_sha256': digest(raw), 'manifest': str(state / 'manifest.json'), 'preview': str(state / 'preview.md')}


def load_approved(args):
    state = safe_path(args.state_dir, private=True)
    require(state.is_dir(), 'state directory missing')
    manifest_path = safe_path(str(state / 'manifest.json'), private=True)
    raw = read_bytes(manifest_path)
    require(args.approve is not None and args.approve == digest(raw), 'exact manifest approval required')
    manifest = read_json(manifest_path)
    fields(manifest, 'schema_version import_id created_at draft snapshot')
    require(type(manifest['schema_version']) is int and manifest['schema_version'] == 1, 'unsupported manifest schema')
    require(str(uuid.UUID(manifest['import_id'])) == manifest['import_id'], 'invalid import identity')
    require(datetime.fromisoformat(manifest['created_at']).utcoffset() is not None, 'timestamp needs offset')
    root = validate_draft(manifest['draft'])
    outside(state, root)
    binding = manifest['draft']['binding']
    require(args.config == binding['config'] and args.ledger == binding['ledger'], 'approved binding mismatch')
    require(not manifest['draft']['conflicts'], 'unresolved conflicts block application')
    require(snapshot(manifest['draft']) == manifest['snapshot'], 'sources or Git snapshot changed; new preview required')
    return state, manifest


def checked_uuid(value):
    require(isinstance(value, str) and str(uuid.UUID(value)) == value, 'invalid CLI identity')
    return value


def call_cli(args, action, body, task_id=None):
    try:
        completed = subprocess.run([args.binary, 'task', '--config', args.config, action],
                                   input=body, capture_output=True, timeout=args.timeout, umask=0o077,
                                   env={**os.environ, 'MEMORYNEXUS_TASK_LEDGER': args.ledger})
    except OSError:
        raise LaunchFailed('CLI executable could not be launched')
    require(completed.returncode == 0 and len(completed.stdout) <= 128 * 1024, 'CLI failed; request remains pending')
    response = json.loads(completed.stdout, object_pairs_hook=unique_object)
    require(isinstance(response, dict) and type(response.get('schema_version')) is int and response['schema_version'] == 1, 'unexpected CLI schema')
    require(response.get('status') in (['ok'] if action == 'context' else ['accepted', 'replayed']), 'unexpected CLI status')
    require(type(response.get('revision')) is int and response['revision'] >= 0, 'unexpected CLI revision')
    checked_uuid(response.get('task_id'))
    require(task_id is None or response['task_id'] == task_id, 'CLI task identity mismatch')
    require(isinstance(response.get('result'), dict), 'unexpected CLI result')
    return response


def config_identity(path):
    safe_path(str(path))
    data = read_json(path)
    fields(data, 'schema_version task_id ledger_env')
    require(type(data['schema_version']) is int and data['schema_version'] == 1 and data['ledger_env'] == 'MEMORYNEXUS_TASK_LEDGER', 'config identity mismatch')
    return checked_uuid(data['task_id'])


def receipt_path(state, entry_id):
    return state / ('receipt-' + entry_id + '.json')


def receipt(state, entry_id, action, key):
    path = safe_path(str(receipt_path(state, entry_id)), private=True)
    if not path.exists():
        return None
    item = read_json(path)
    fields(item, 'action idempotency_key task_id revision status' + (' record_id state' if action != 'init' else ''))
    require(item['action'] == action and item['idempotency_key'] == key, 'receipt request mismatch')
    checked_uuid(item['task_id'])
    require(type(item['revision']) is int and item['revision'] >= 0 and item['status'] in ('accepted', 'replayed'), 'invalid receipt')
    if action != 'init':
        checked_uuid(item['record_id'])
        require(item['state'] == 'current', 'seed record is no longer current')
    return item


def ledger_identity(state, ledger):
    path = safe_path(str(state / 'ledger-identity.json'), private=True)
    actual = None
    if ledger.exists():
        info = ledger.stat()
        require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_uid == os.getuid(), 'unsafe ledger file')
        actual = {'device': info.st_dev, 'inode': info.st_ino}
    if path.exists():
        require(actual == read_json(path), 'ledger missing or replaced; refusing replacement scope')
    elif actual:
        write_new(path, encode(actual))


def current_context(args, task_id, manifest):
    response = call_cli(args, 'context', b'{}', task_id)
    result = response['result']
    require(result.get('display_name') == manifest['draft']['binding']['display_name'], 'task display name mismatch')
    require(result.get('omitted_count') == 0 and result.get('truncated') is False, 'task context is incomplete')
    require(isinstance(result.get('entries'), list) and isinstance(result.get('candidates'), list), 'invalid task context')
    records = result['entries'] + result['candidates']
    require(result.get('candidate_count') == len(result['candidates']), 'invalid candidate count')
    # Never seed an existing unrelated task or supersede another writer's checkpoint.
    require(all(isinstance(r, dict) and r.get('run_id') == manifest['import_id'] and
                r.get('client') == 'task-bootstrap' and r.get('state') == 'current' for r in records),
            'nonempty task contains records outside this import')
    for record in records:
        checked_uuid(record.get('id'))
        require(record.get('kind') in ('checkpoint', 'execution', 'instruction', 'lesson'), 'invalid context kind')
        require(record.get('trust') in ('execution_reported', 'owner_confirmed', 'review_candidate'), 'invalid context trust')
        bounded(record.get('text'), 1000)
        require(isinstance(record.get('source_ids'), list), 'invalid context sources')
        for source_id in record['source_ids']:
            checked_uuid(source_id)
    require(len({r['id'] for r in records}) == len(records), 'duplicate context identities')
    require(response['revision'] == len(records) + 1, 'task changed outside this import')
    return records


def apply_locked(args, state, manifest):
    draft = manifest['draft']
    binding = draft['binding']
    ledger = Path(args.ledger)
    config = Path(args.config)
    entries = [('init', 'init', None)] + [(e['id'], 'review' if e['kind'] == 'lesson' else 'record', e) for e in draft['entries']]
    receipts = {}
    attempted = []
    pending_id = None
    task_id = None
    def key(entry_id):
        return 'bootstrap:' + manifest['import_id'] + ':' + entry_id
    for entry_id, action, _ in entries:
        saved = receipt(state, entry_id, action, key(entry_id))
        if saved:
            receipts[entry_id] = saved
    init = receipts.get('init')
    init_pending = state / 'pending-init.json'
    if not init and not init_pending.exists():
        require(not config.exists(), 'existing task config refused; bootstrap is new-task-only')
    if init or init_pending.exists():
        not_started = safe_path(str(state / 'not-started-init.json'), private=True)
        known_not_started = (not_started.exists() and read_json(not_started) == {'idempotency_key': key('init')}
                             and not config.exists() and not (state / 'ledger-identity.json').exists())
        require(ledger.exists() or known_not_started, 'uncertain or missing ledger; do not create a replacement')
        if config.exists():
            task_id = config_identity(config)
            require(not init or task_id == init['task_id'], 'config task identity changed')
            current_context(args, task_id, manifest)
        else:
            require(not init and ((state / 'ledger-identity.json').exists() or known_not_started), 'missing task config; scope uncertain')
    ledger_identity(state, ledger)
    require(not receipts or init is not None, 'records without an init receipt')
    require(all(r['task_id'] == init['task_id'] for r in receipts.values()) if init else True, 'receipt scope mismatch')
    try:
        for entry_id, action, entry in entries:
            pending_id = entry_id
            pending = safe_path(str(state / ('pending-' + entry_id + '.json')), private=True)
            if entry_id in receipts:
                if pending.exists():
                    pending.unlink()
                continue
            if action == 'init':
                body = {'idempotency_key': key(entry_id), 'display_name': binding['display_name'],
                        'confirmation': 'confirmed', 'automatic_capture': binding['automatic_capture']}
            else:
                body = {'idempotency_key': key(entry_id), 'run_id': manifest['import_id'],
                        'client': 'task-bootstrap', 'occurred_at': manifest['created_at'], 'text': entry['text']}
                if action == 'review':
                    body['source_ids'] = [receipts[source]['record_id'] for source in entry['source_entries']]
                else:
                    body['kind'] = entry['kind']
                    source_hashes = {source['path']: source['sha256'] for source in manifest['snapshot']['sources']}
                    body['evidence_refs'] = [{'path': path, 'sha256': source_hashes[path]} for path in entry['sources']]
                    if entry['kind'] == 'instruction':
                        body['confirmation'] = 'confirmed'
            encoded = encode(body)
            require(len(encoded) <= 32768, 'request exceeds CLI budget')
            if pending.exists():
                require(read_bytes(pending) == encoded, 'pending envelope changed')
            else:
                write_new(pending, encoded)
            not_started = safe_path(str(state / 'not-started-init.json'), private=True)
            if action == 'init' and not_started.exists():
                not_started.unlink()
            try:
                response = call_cli(args, action, read_bytes(pending), task_id)
            except LaunchFailed:
                if action == 'init':
                    write_new(not_started, encode({'idempotency_key': key('init')}))
                raise
            finally:
                ledger_identity(state, ledger)
            result = response['result']
            if action == 'init':
                require(result.get('display_name') == binding['display_name'] and
                        result.get('automatic_capture') == binding['automatic_capture'], 'init policy mismatch')
                task_id = response['task_id']
                require(config_identity(config) == task_id, 'init config mismatch')
                require(not current_context(args, task_id, manifest), 'new task is not empty')
            else:
                checked_uuid(result.get('record_id'))
                require(result.get('state') == 'current', 'seed record is not current')
            saved = {k: response[k] for k in ('task_id', 'revision', 'status')}
            saved.update(action=action, idempotency_key=key(entry_id))
            if action != 'init':
                saved.update(record_id=result['record_id'], state=result['state'])
            write_new(receipt_path(state, entry_id), encode(saved))
            receipts[entry_id] = saved
            attempted.append({'entry_id': entry_id, 'status': response['status']})
            pending.unlink()
        actual = current_context(args, task_id or receipts['init']['task_id'], manifest)
        actual = {r['id']: r for r in actual}
        require(len(actual) == len(draft['entries']), 'readback count mismatch')
        for entry in draft['entries']:
            record = actual.get(receipts[entry['id']]['record_id'], {})
            expected_trust = 'review_candidate' if entry['kind'] == 'lesson' else ('owner_confirmed' if entry['kind'] == 'instruction' else 'execution_reported')
            require(record.get('kind') == entry['kind'] and record.get('text') == entry['text'] and
                    record.get('trust') == expected_trust, 'readback content/trust mismatch')
            expected_sources = [receipts[source]['record_id'] for source in entry.get('source_entries', [])]
            require(record.get('source_ids') == expected_sources, 'readback source mismatch')
        return {'status': 'complete', 'task_id': receipts['init']['task_id'], 'receipts': receipts,
                'attempted': attempted, 'pending': []}
    except (Rejected, OSError, ValueError, TypeError, subprocess.SubprocessError):
        return {'status': 'pending', 'task_id': task_id, 'receipts': receipts, 'attempted': attempted,
                'pending': [e[0] for e in entries if e[0] not in receipts],
                'attention': pending_id, 'message': 'Not complete: pending writes, cleanup or readback need attention. Keep staged requests and explicitly resume with the same approval.'}


def apply(args):
    import fcntl
    state, manifest = load_approved(args)
    require(0 < args.timeout <= 300, 'timeout must be between 0 and 300 seconds')
    binary = safe_path(args.binary, cli_executable=True)
    require(binary.is_file() and os.access(binary, os.X_OK), 'compiled CLI missing or not executable')
    ledger = safe_path(args.ledger, private=True)
    for suffix in ('-wal', '-shm', '-journal'):
        sidecar = safe_path(str(ledger) + suffix, private=True)
        require(not sidecar.exists() or (ledger.exists() and sidecar.is_file()), 'orphan or unsafe ledger sidecar')
    require(ledger.parent.is_dir(), 'create the approved owner-only ledger parent explicitly before apply')
    safe_path(str(ledger.parent), private=True)
    lock_path = safe_path(str(state / 'apply.lock'), private=True)
    fd = os.open(lock_path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'r+') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise Rejected('another apply is active for this manifest')
        return apply_locked(args, state, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    command = commands.add_parser('preview')
    command.add_argument('--draft', required=True)
    command.add_argument('--state-dir', required=True)
    command = commands.add_parser('apply')
    command.add_argument('--state-dir', required=True)
    command.add_argument('--approve')
    command.add_argument('--config', required=True)
    command.add_argument('--ledger', required=True)
    command.add_argument('--binary', required=True)
    command.add_argument('--timeout', type=float, default=30)
    args = parser.parse_args()
    try:
        result = preview(args) if args.command == 'preview' else apply(args)
        print(json.dumps(result, ensure_ascii=False))
        if result['status'] == 'pending':
            return 2
    except Rejected as error:
        print(json.dumps({'status': 'error', 'message': str(error)}))
        return 2
    except (OSError, ValueError, TypeError, subprocess.SubprocessError):
        print(json.dumps({'status': 'error', 'message': 'Bootstrap rejected; inspect manifest, sources, binding and owner-only paths. No success is implied.'}))
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())
