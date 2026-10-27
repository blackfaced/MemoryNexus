#!/usr/bin/env python3
"""Verify one immutable installation, then invoke its existing workflow runner."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys

sys.dont_write_bytecode = True
COMMANDS = {'inspect', 'prepare', 'preview', 'apply', 'status'}
CAPABILITIES = {'workflow-bind', 'source-sync', 'source-capture', 'consumer-review',
                'consumer-approve', 'publication-result', 'workflow-status',
                'context', 'export', 'backup', 'restore', 'migrate'}


class InstallError(Exception):
    pass


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def checked_path(value, private=False):
    """Bootstrap checks before importing any installed package code."""
    path = Path(value)
    if not path.is_absolute() or '..' in path.parts:
        raise InstallError('unsafe_path')
    for current in [*reversed(path.parents), path]:
        info = current.lstat()
        if stat.S_ISLNK(info.st_mode) or info.st_uid not in (0, os.getuid()):
            raise InstallError('unsafe_path')
        if info.st_mode & 0o022 and not (stat.S_ISDIR(info.st_mode) and info.st_mode & stat.S_ISVTX):
            raise InstallError('unsafe_path')
    if private and (info.st_uid != os.getuid() or info.st_mode & 0o077):
        raise InstallError('unsafe_path')
    return path


def read(path, limit, private=False):
    path = checked_path(path, private)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise InstallError('unsafe_path')
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise InstallError('oversize')
    return raw


def parse(raw):
    def unique(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise InstallError('invalid_manifest')
            value[key] = item
        return value
    return json.loads(raw, object_pairs_hook=unique)


def inventory(root):
    files = []
    for path in sorted(root.rglob('*')):
        checked_path(path, private=True)
        if path.is_dir():
            if stat.S_IMODE(path.stat().st_mode) != 0o700:
                raise InstallError('unsafe_path')
            continue
        relative = path.relative_to(root).as_posix()
        if relative == 'manifest.json':
            continue
        raw = read(path, 128 * 1024 * 1024, private=True)
        files.append({'path': relative, 'sha256': sha(raw), 'bytes': len(raw),
                      'mode': format(stat.S_IMODE(path.stat().st_mode), '04o')})
    return files


def capability(binary):
    result = subprocess.run([str(binary), 'task', '--help'], capture_output=True,
                            timeout=30, env={'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8'})
    if result.returncode or len(result.stdout) > 131072:
        raise InstallError('unsupported_version')
    try:
        commands = parse(result.stdout)['result']['commands']
        if not CAPABILITIES.issubset(commands):
            raise InstallError('unsupported_version')
    except (KeyError, TypeError, ValueError):
        raise InstallError('unsupported_version')


def verify(root):
    checked_path(root, private=True)
    manifest = parse(read(root / 'manifest.json', 1024 * 1024, private=True))
    fields = {'schema_version', 'workflow_contract', 'source_revision', 'python',
              'python_sha256', 'binary_sha256', 'package_sha256', 'files'}
    if (not isinstance(manifest, dict) or set(manifest) != fields
            or type(manifest['schema_version']) is not int or manifest['schema_version'] != 1
            or type(manifest['workflow_contract']) is not int or manifest['workflow_contract'] != 1):
        raise InstallError('unsupported_version')
    if inventory(root) != manifest['files']:
        raise InstallError('installation_changed')
    python = checked_path(manifest['python'])
    if (python != Path(sys.executable).resolve()
            or sha(read(python, 128 * 1024 * 1024)) != manifest['python_sha256']):
        raise InstallError('python_mismatch')
    binary = root / 'bin/memorynexus-feedback-cli'
    if sha(read(binary, 128 * 1024 * 1024, True)) != manifest['binary_sha256']:
        raise InstallError('installation_changed')
    package_files = [{'path': f['path'][8:], 'sha256': f['sha256']}
                     for f in manifest['files'] if f['path'].startswith('package/')]
    if sha(canonical(package_files)) != manifest['package_sha256']:
        raise InstallError('installation_changed')
    if not os.access(binary, os.X_OK) or not os.access(python, os.X_OK):
        raise InstallError('unsafe_path')
    capability(binary)
    return manifest


def binding_argument(argv):
    # Do not permit argparse abbreviations or repeated bindings to evade the
    # installation match. The runner still validates the complete command.
    values = []
    for i, arg in enumerate(argv):
        if arg == '--binding' and i + 1 < len(argv):
            values.append(argv[i + 1])
        elif arg.startswith('--binding='):
            values.append(arg.partition('=')[2])
    if len(values) != 1:
        raise InstallError('explicit_binding_required')
    return values[0]


def main():
    os.umask(0o077)
    try:
        # -I prevents PYTHONPATH / cwd injection before this file even starts.
        if not sys.flags.isolated:
            raise InstallError('isolated_python_required')
        argv = sys.argv[1:]
        if not argv or argv[0] not in COMMANDS | {'verify'}:
            raise InstallError('invalid_command')
        flags = {'--binding', '--root', '--selection', '--run-dir', '--workflow', '--client',
                 '--prepared', '--submission', '--plan', '--approve'}
        if any(arg.startswith('--') and arg.partition('=')[0] not in flags for arg in argv[1:]):
            raise InstallError('invalid_command')
        root = Path(__file__).absolute().parent
        manifest = verify(root)
        sys.path.insert(0, str(root / 'package'))
        from workflow_cli import load_context
        from workflow_io import WorkflowError
        try:
            ctx = None
            if argv[0] not in ('inspect', 'verify') or (argv[0] == 'verify' and len(argv) > 1):
                ctx = load_context(binding_argument(argv))
                b = ctx['binding']
                if any(b[key] != value for key, value in {
                    'binary': str(root / 'bin/memorynexus-feedback-cli'),
                    'binary_sha256': manifest['binary_sha256'],
                    'package': str(root / 'package'),
                    'package_sha256': manifest['package_sha256'],
                    'python': manifest['python'],
                }.items()):
                    raise InstallError('installation_binding_mismatch')
            if argv[0] == 'verify':
                parser = argparse.ArgumentParser(allow_abbrev=False)
                parser.add_argument('--binding')
                parser.parse_args(argv[1:])
                result = {'schema_version': 1, 'status': 'ok', 'workflow_contract': 1,
                          'source_revision': manifest['source_revision'],
                          'binary': str(root / 'bin/memorynexus-feedback-cli'),
                          'package': str(root / 'package'),
                          'python': manifest['python'],
                          'binary_sha256': manifest['binary_sha256'],
                          'package_sha256': manifest['package_sha256']}
                if ctx:
                    result.update(task_id=ctx['binding']['task_id'], ledger=ctx['binding']['ledger'],
                                  binding_sha256=ctx['binding_sha256'], profile_sha256=ctx['profile_sha256'])
                print(json.dumps(result))
                return 0
            # Trigger evidence stays visible to the runner, never to the CLI.
            env = {k: os.environ[k] for k in ('HOME', 'TMPDIR', 'MEMORYNEXUS_TRIGGER_FILE') if k in os.environ}
            env.update(PATH=str(root / 'package/bin') + ':/usr/bin:/bin', LANG='C.UTF-8')
            os.environ.clear()
            os.environ.update(env)
            import workflow
            return workflow.main()
        except WorkflowError as exc:
            raise InstallError(exc.code) from exc
    except (InstallError, OSError, ValueError, TypeError, subprocess.SubprocessError) as exc:
        code = str(exc) if isinstance(exc, InstallError) else 'installation_invalid'
        print(json.dumps({'schema_version': 1, 'status': 'failed', 'code': code}))
        return 2


if __name__ == '__main__':
    sys.exit(main())
