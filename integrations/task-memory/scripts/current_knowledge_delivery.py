"""One claimed bridge attempt using rechecked bytes; no retry or client discovery."""
import os

from current_knowledge_artifacts import now
from current_knowledge_task import sha, transport_bytes
from workflow_io import WorkflowError, read_regular, safe_path
from workflow_types import canonical, fields, strict_loads, string, fail


def claim(origin, read_id, material_sha256, run, started_at):
    path = origin / 'delivery-started.json'
    # The validated original read directory is private. O_EXCL also rejects any
    # pre-existing symlink/directory; do not remove even an incomplete crash claim.
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    except FileExistsError: fail('delivery_already_attempted')
    raw = canonical(dict(read_id=read_id, material_sha256=material_sha256, run_dir=str(run), started_at=started_at)) + b'\n'
    try:
        remaining = memoryview(raw)
        while remaining:
            count = os.write(fd, remaining)
            if count <= 0: raise OSError('short write')
            remaining = remaining[count:]
        os.fsync(fd)
    finally: os.close(fd)
    directory = os.open(origin, os.O_RDONLY | os.O_NOFOLLOW)
    try: os.fsync(directory)
    finally: os.close(directory)


def deliver(b, origin, run, read_id, raw):
    bridge = b.get('bridge')
    if not bridge: fail('bridge_unavailable')
    try:
        path = safe_path(bridge['executable'])
        if not os.access(path, os.X_OK) or sha(read_regular(path, 128*1024*1024)) != bridge['sha256']:
            fail('bridge_unavailable')
    except (OSError, WorkflowError): fail('bridge_unavailable')
    started_at = now(); material_sha256 = sha(raw)
    claim(origin, read_id, material_sha256, run, started_at)
    delivery = dict(schema_version=1, read_id=read_id, material_sha256=material_sha256, bytes=len(raw),
                    bridge_sha256=bridge['sha256'], started_at=started_at, status='unknown')
    try:
        env = dict(os.environ, MEMORYNEXUS_TASK_LEDGER=b['ledger'])
        code, output = transport_bytes([str(path)], raw, env, b['task_root'], b['timeout_seconds'], 4096)
        ack = strict_loads(output)
        fields(ack, ('schema_version', 'read_id', 'material_sha256', 'session_ref'))
        if (code or type(ack['schema_version']) is not int or ack['schema_version'] != 1 or
                ack['read_id'] != read_id or ack['material_sha256'] != material_sha256): fail('delivery_unknown')
        string(ack['session_ref'], 1, 128)
        delivery.update(status='acknowledged', session_ref=ack['session_ref'])
    except (OSError, WorkflowError, ValueError, TypeError, KeyError, RecursionError):
        # A spawn/pipe/ack failure cannot prove that the client did not receive
        # stdin. Keep the claim permanently; never retry under another run-dir.
        pass
    delivery['finished_at'] = now()
    return delivery
