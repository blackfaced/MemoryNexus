#!/usr/bin/env python3
"""Install a pinned Git tree and supplied CLI into a NEW owner-only directory.

Never selects a ledger, binds a workflow, modifies client settings or schedules.
"""
import argparse
import os
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True
from launch import InstallError, canonical, sha, checked_path, read, parse, inventory, capability

INTEGRATION = 'integrations/task-memory/'


def destination(path):
    if not path.is_absolute() or '..' in path.parts or path.exists() or path.is_symlink():
        raise InstallError('new_directory_required')
    checked_path(path.parent, private=True)


def selected(path):
    if path.startswith(INTEGRATION + 'scripts/'):
        return 'package/' + path[len(INTEGRATION + 'scripts/'):]
    if path in (INTEGRATION + name for name in ('launch.py', 'codex_bridge.py', 'install_codex_bridge.py')):
        return path[len(INTEGRATION):]
    if (path == INTEGRATION + 'PROTOCOL.md'
            or path.startswith(INTEGRATION + 'memorynexus-')
            or path.startswith('integrations/minimax/memorynexus-task-memory/')
            or path == 'integrations/codex/task-memory.md'
            or path.startswith('docs/current-knowledge/')
            or path.startswith('docs/current-knowledge-')
            or path == 'docs/acceptance/current-knowledge.md'
            or path.startswith('docs/repository-memory/')
            or path.startswith('docs/repository-memory-')
            or path in ('docs/task-memory-contract.md', 'docs/task-memory-onboarding.md',
                        'docs/acceptance/task-memory-cross-client.md',
                        'docs/acceptance/repository-memory-installation.md')
            or path.startswith(('decisions/ADR-027-', 'decisions/ADR-028-', 'decisions/ADR-029-', 'decisions/ADR-030-'))):
        return 'package/' + path
    return None


def git(a, *args):
    result = subprocess.run([a.git, '-C', a.source, *args], capture_output=True, timeout=60,
                            env={'PATH': '/usr/bin:/bin', 'LANG': 'C.UTF-8', 'GIT_CONFIG_NOSYSTEM': '1'})
    if result.returncode:
        raise InstallError('source_revision_unavailable')
    return result.stdout


def collect(a):
    if not re.fullmatch('[0-9a-f]{40}', a.revision):
        raise InstallError('full_source_revision_required')
    if git(a, 'rev-parse', '--verify', a.revision + '^{commit}').decode().strip() != a.revision:
        raise InstallError('source_revision_unavailable')
    files = {}
    for line in git(a, 'ls-tree', '--full-tree', '-rz', a.revision).split(b'\x00'):
        if not line:
            continue
        meta, raw_path = line.split(b'\t', 1)
        path = raw_path.decode('utf-8')
        target = selected(path)
        if target is None:
            continue
        mode, kind, oid = meta.split()
        if mode not in (b'100644', b'100755') or kind != b'blob' or '..' in Path(target).parts:
            raise InstallError('unsafe_source')
        if target in files:
            raise InstallError('duplicate_install_path')
        raw = git(a, 'cat-file', 'blob', oid.decode())
        if len(raw) > 8 * 1024 * 1024:
            raise InstallError('oversize')
        files[target] = raw
    for required in ('launch.py', 'package/workflow.py', 'package/workflow_cli.py',
                     'package/workflow_io.py', 'package/workflow_engine.py',
                     'package/integrations/task-memory/PROTOCOL.md',
                     'package/integrations/task-memory/memorynexus-task-bootstrap/scripts/bootstrap.py',
                     'package/integrations/task-memory/memorynexus-repository-review/SKILL.md'):
        if required not in files:
            raise InstallError('incomplete_source_package')
    for i, path in enumerate(a.rule_file):
        raw = read(path, 32768)
        if not raw or '\x00' in raw.decode('utf-8'):
            raise InstallError('invalid_rule_file')
        files['package/rules/lens-%s.txt' % (i + 1)] = raw
    return files


def write(path, raw, mode=0o600):
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    with os.fdopen(fd, 'wb') as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


def install(a):
    root = Path(a.destination)
    destination(root)
    checked_path(a.source)
    checked_path(a.git)
    python = checked_path(a.python)
    # CLT shims and symlinked interpreters must not silently change the binding.
    probe = subprocess.run([str(python), '-I', '-S', '-c', 'import sys; print(sys.executable)'],
                           capture_output=True, timeout=30, env={'PATH': '/usr/bin:/bin'})
    if probe.returncode or probe.stdout.decode().strip() != str(python):
        raise InstallError('python_mismatch')
    python_hash = sha(read(python, 128 * 1024 * 1024))
    binary = read(a.binary, 128 * 1024 * 1024)
    if not re.fullmatch('[0-9a-f]{64}', a.binary_sha256) or sha(binary) != a.binary_sha256:
        raise InstallError('binary_hash_mismatch')
    capability(a.binary)
    files = collect(a)
    files['bin/memorynexus-feedback-cli'] = binary
    # All preflight is read-only. O_EXCL/no-overwrite applies to every new file.
    root.mkdir(mode=0o700)
    for path, raw in sorted(files.items()):
        write(root / path, raw, 0o700 if path.startswith('bin/') else 0o600)
    listed = inventory(root)
    package = [{'path': f['path'][8:], 'sha256': f['sha256']}
               for f in listed if f['path'].startswith('package/')]
    manifest = {'schema_version': 1, 'workflow_contract': 1, 'source_revision': a.revision,
                'python': str(python), 'python_sha256': python_hash,
                'binary_sha256': a.binary_sha256, 'package_sha256': sha(canonical(package)),
                'files': listed}
    write(root / 'manifest.json', canonical(manifest))
    result = subprocess.run([str(python), '-I', '-S', '-B', str(root / 'launch.py'), 'verify'],
                            capture_output=True, timeout=60, env={'PATH': '/usr/bin:/bin'})
    if result.returncode:
        raise InstallError('installed_verification_failed')
    return parse(result.stdout)


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    for name in ('source', 'revision', 'binary', 'binary-sha256', 'python', 'destination'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--git', default='/usr/bin/git')
    parser.add_argument('--rule-file', action='append', required=True)
    a = parser.parse_args()
    try:
        if not 1 <= len(a.rule_file) <= 2:
            raise InstallError('one_or_two_rule_files_required')
        print(canonical(install(a)).decode())
        return 0
    except (InstallError, OSError, ValueError, TypeError, subprocess.SubprocessError) as exc:
        code = str(exc) if isinstance(exc, InstallError) else 'installation_failed'
        print(canonical({'schema_version': 1, 'status': 'failed', 'code': code}).decode())
        return 2


if __name__ == '__main__':
    sys.exit(main())
