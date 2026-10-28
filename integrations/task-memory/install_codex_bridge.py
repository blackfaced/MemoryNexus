#!/usr/bin/env python3
"""Generate one immutable bridge from a verified installed version and explicit pins."""
import argparse
import os
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from launch import InstallError, canonical, checked_path, read, parse, sha, verify


def generate(spec_path, destination):
    root = Path(__file__).resolve().parent
    manifest = verify(root)
    # Only import the template after the installation inventory has matched.
    import codex_bridge
    spec = parse(read(spec_path, 32768, private=True))
    codex_bridge.validate(spec)
    codex_bridge.version(spec)
    codex_bridge.validate(spec)
    target = Path(destination)
    if not target.is_absolute() or '..' in target.parts or target.exists() or target.is_symlink():
        raise InstallError('new_directory_required')
    checked_path(target.parent, private=True)
    if root == target or root in target.parents:
        raise InstallError('unsafe_path')
    python = manifest['python']
    if any(c.isspace() for c in python) or len(python.encode()) > 200:
        raise InstallError('unsafe_path')
    pin = {'path': python, 'sha256': manifest['python_sha256']}
    raw = ('#!' + python + ' -I\n').encode() + read(root/'codex_bridge.py', 65536)
    raw += ('\nif __name__ == "__main__":\n    raise SystemExit(main(' + repr(spec) + ', ' + repr(pin) + '))\n').encode()
    target.mkdir(mode=0o700)
    executable = target / 'codex-bridge'
    fd = os.open(executable, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o700)
    with os.fdopen(fd, 'wb') as f:
        f.write(raw); f.flush(); os.fsync(f.fileno())
    fd = os.open(target, os.O_RDONLY | os.O_NOFOLLOW)
    try: os.fsync(fd)
    finally: os.close(fd)
    return {'schema_version': 1, 'bridge': {'executable': str(executable), 'sha256': sha(raw)},
            'source_revision': manifest['source_revision'], 'client_version': spec['client_version'],
            'coverage': 'declared_pins_only'}


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument('--spec', required=True)
    parser.add_argument('--destination', required=True)
    a = parser.parse_args()
    try:
        print(canonical(generate(a.spec, a.destination)).decode())
        return 0
    except Exception:
        # No spec contents, credentials or external client diagnostics in output.
        print(canonical({'schema_version': 1, 'status': 'failed', 'code': 'bridge_installation_failed'}).decode())
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
