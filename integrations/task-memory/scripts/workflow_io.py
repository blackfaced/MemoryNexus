"""Private immutable artifacts and bounded managed Markdown publication."""
import contextlib
import fcntl
import hashlib
import os
from pathlib import Path
import re
import stat
import tempfile


class WorkflowError(Exception):
    def __init__(self, code):
        self.code = code
        super().__init__(code)


def _hash(raw):
    return hashlib.sha256(raw).hexdigest()


def safe_path(value, private=False):
    path = Path(value)
    if not path.is_absolute() or '..' in path.parts or '\x00' in str(path):
        raise WorkflowError('unsafe_path')
    current = Path(path.anchor)
    for part in path.parts[1:]:
        current /= part
        try:
            info = current.lstat()
        except FileNotFoundError:
            if current == path:
                if private:
                    _private_parent(path)
                return path
            raise WorkflowError('unsafe_path')
        if stat.S_ISLNK(info.st_mode):
            raise WorkflowError('unsafe_path')
        if current != path and not stat.S_ISDIR(info.st_mode):
            raise WorkflowError('unsafe_path')
        # Root-owned ancestors (/Users, /private/tmp) are expected. Reject
        # other users and writable non-sticky ancestors.
        if info.st_uid not in (0, os.getuid()):
            raise WorkflowError('unsafe_path')
        if current != path and info.st_mode & 0o022 and not info.st_mode & stat.S_ISVTX:
            raise WorkflowError('unsafe_path')
    if private and (info.st_uid != os.getuid() or info.st_mode & 0o077):
        raise WorkflowError('unsafe_path')
    if private and not stat.S_ISDIR(info.st_mode):
        _private_parent(path)
    return path


def _private_parent(path):
    info = path.parent.lstat()
    if info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise WorkflowError('unsafe_path')


def read_regular(path, limit, private=False):
    path = safe_path(path, private)
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, 'rb') as stream:
            info = os.fstat(stream.fileno())
            if (not stat.S_ISREG(info.st_mode) or info.st_uid not in (0, os.getuid())
                    or (private and (info.st_uid != os.getuid() or info.st_mode & 0o077))):
                raise WorkflowError('unsafe_path')
            raw = stream.read(limit + 1)
            if len(raw) > limit:
                raise WorkflowError('oversize')
            return raw
    except FileNotFoundError:
        raise
    except OSError as exc:
        raise WorkflowError('io_error') from exc


def _sync_parent(path):
    fd = os.open(path.parent, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def write_new(path, raw):
    path = safe_path(path)
    fd = None
    made = False
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        made = True
        view = memoryview(raw)
        while view:
            n = os.write(fd, view)
            if n <= 0:
                raise OSError('short write')
            view = view[n:]
        os.fsync(fd)
        os.close(fd)
        fd = None
        _sync_parent(path)
    except OSError as exc:
        if fd is not None:
            os.close(fd)
        if made:
            path.unlink(missing_ok=True)
        raise WorkflowError('io_error') from exc


def new_run(path, state_root=None, task_root=None):
    path = safe_path(path)
    if task_root is not None and _inside(path, safe_path(task_root)):
        raise WorkflowError('unsafe_path')
    if state_root is not None and not _inside(path, safe_path(state_root, True)):
        raise WorkflowError('unsafe_path')
    try:
        path.mkdir(mode=0o700)
        _sync_parent(path)
    except OSError as exc:
        raise WorkflowError('io_error') from exc
    return path


def _inside(path, root):
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def _relative(run_dir, relative):
    if not isinstance(relative, str):
        raise WorkflowError('invalid_input')
    part = Path(relative)
    if not relative or part.is_absolute() or '..' in part.parts or str(part) != relative:
        raise WorkflowError('unsafe_path')
    return safe_path(safe_path(run_dir, True) / part, True)


def artifact_ref(run_dir, relative):
    raw = read_regular(_relative(run_dir, relative), 64 * 1024 * 1024, True)
    return {'path': relative, 'sha256': _hash(raw), 'bytes': len(raw)}


def resolve_ref(run_dir, ref, state_root):
    if not isinstance(ref, dict) or set(ref) != {'path', 'sha256', 'bytes'}:
        raise WorkflowError('invalid_input')
    if not isinstance(ref['bytes'], int) or isinstance(ref['bytes'], bool) or not 0 <= ref['bytes'] <= 64 * 1024 * 1024:
        raise WorkflowError('invalid_input')
    if not isinstance(ref['sha256'], str) or not re.fullmatch('[0-9a-f]{64}', ref['sha256']):
        raise WorkflowError('invalid_input')
    run = safe_path(run_dir, True)
    if not _inside(run, safe_path(state_root, True)):
        raise WorkflowError('unsafe_path')
    raw = read_regular(_relative(run, ref['path']), ref['bytes'], True)
    if len(raw) != ref['bytes'] or _hash(raw) != ref['sha256']:
        raise WorkflowError('stale_snapshot')
    return raw


def _region(base, block_id):
    if not isinstance(block_id, str) or not re.fullmatch('[a-z0-9][a-z0-9._-]{0,127}', block_id):
        raise WorkflowError('invalid_input')
    begin = ('<!-- memorynexus:' + block_id + ':begin -->').encode()
    end = ('<!-- memorynexus:' + block_id + ':end -->').encode()
    nb, ne = base.count(begin), base.count(end)
    if nb == ne == 0:
        return None
    if nb != 1 or ne != 1:
        raise WorkflowError('target_conflict')
    start = base.index(begin)
    stop = base.index(end)
    body_start = start + len(begin)
    if stop < body_start or base[body_start:body_start + 1] != b'\n':
        raise WorkflowError('target_conflict')
    region_end = stop + len(end)
    if base[region_end:region_end + 1] == b'\n':
        region_end += 1
    return start, region_end, base[body_start + 1:stop]


def target_snapshot(path, block_id=None):
    path = safe_path(path)
    exists = path.exists()
    raw = read_regular(path, 64 * 1024 * 1024) if exists else b''
    result = {'exists': exists, 'sha256': _hash(raw)}
    if block_id is not None:
        region = _region(raw, block_id)
        if region:
            result['prior_block_sha256'] = _hash(region[2])
    return result


def render(base, block_id, output_text):
    if not isinstance(output_text, str) or '\x00' in output_text or '<!-- memorynexus:' in output_text:
        raise WorkflowError('invalid_input')
    try:
        base.decode('utf-8')
        body = (output_text.rstrip('\n') + '\n').encode('utf-8')
    except UnicodeError as exc:
        raise WorkflowError('invalid_input') from exc
    region = _region(base, block_id)
    block = ('<!-- memorynexus:' + block_id + ':begin -->\n').encode() + body + ('<!-- memorynexus:' + block_id + ':end -->\n').encode()
    if region:
        return base[:region[0]] + block + base[region[1]:]
    return base + (b'\n' if base and not base.endswith(b'\n') else b'') + block


def publish(path, expected_exists, expected_sha, rendered, after_sha):
    path = safe_path(path)
    if _hash(rendered) != after_sha:
        raise WorkflowError('stale_snapshot')
    def check():
        snap = target_snapshot(path)
        if snap['exists'] != expected_exists or snap['sha256'] != expected_sha:
            raise WorkflowError('stale_snapshot')
    check()
    temporary = None
    try:
        fd, tmp = tempfile.mkstemp(prefix='.memorynexus-', dir=path.parent)
        os.close(fd)
        temporary = Path(tmp)
        temporary.unlink()
        write_new(temporary, rendered)
        check()
        if expected_exists:
            os.replace(temporary, path)
        else:
            os.link(temporary, path, follow_symlinks=False)
            temporary.unlink()
        temporary = None
        _sync_parent(path)
        if read_regular(path, len(rendered)) != rendered:
            raise WorkflowError('target_conflict')
        return 'after'
    except FileExistsError as exc:
        raise WorkflowError('stale_snapshot') from exc
    except OSError as exc:
        raise WorkflowError('io_error') from exc
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


@contextlib.contextmanager
def task_lock(state_root, task_id, workflow_id):
    root = safe_path(state_root, True)
    lock = root / ('lock-' + _hash((str(task_id) + ':' + str(workflow_id)).encode()))
    safe_path(lock)
    fd = os.open(lock, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc:
            raise WorkflowError('busy') from exc
        yield
    finally:
        os.close(fd)
