import hashlib
import os
import pathlib
import sys
import tempfile
import unittest
from unittest import mock
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / 'scripts'))
import workflow_io as io


class SafeIOTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.tmp.name).resolve()
        self.root.chmod(0o700)

    def tearDown(self):
        self.tmp.cleanup()

    def test_render_preserves_human_and_rejects_broken_markers(self):
        base = b'human  \r\n'
        once = io.render(base, 'abc', 'body\n\n')
        self.assertEqual(once, base + b'<!-- memorynexus:abc:begin -->\nbody\n<!-- memorynexus:abc:end -->\n')
        self.assertEqual(io.render(once + b'tail', 'abc', 'new'), io.render(base, 'abc', 'new') + b'tail')
        for broken in (b'<!-- memorynexus:abc:begin -->\n', once + once):
            with self.assertRaises(io.WorkflowError):
                io.render(broken, 'abc', 'new')
        with self.assertRaises(io.WorkflowError):
            io.render(base, 'abc', '<!-- memorynexus:evil:begin -->')

    def test_artifact_immutable_symlink_and_digest(self):
        run = io.new_run(self.root / 'run')
        io.write_new(run / 'evidence', b'one')
        ref = io.artifact_ref(run, 'evidence')
        self.assertEqual(io.resolve_ref(run, ref, self.root), b'one')
        with self.assertRaises(io.WorkflowError):
            io.write_new(run / 'evidence', b'two')
        (run / 'link').symlink_to(run / 'evidence')
        with self.assertRaises(io.WorkflowError):
            io.read_regular(run / 'link', 100)
        with self.assertRaises(io.WorkflowError):
            io.resolve_ref(run, dict(ref, path='../evidence'), self.root)
        (run / 'evidence').write_bytes(b'bad')
        with self.assertRaises(io.WorkflowError):
            io.resolve_ref(run, ref, self.root)

    def test_publish_absent_empty_and_compare(self):
        target = self.root / 'doc.md'
        empty = hashlib.sha256(b'').hexdigest()
        rendered = io.render(b'', 'abc', 'hello')
        digest = hashlib.sha256(rendered).hexdigest()
        self.assertEqual(io.target_snapshot(target), {'exists': False, 'sha256': empty})
        self.assertEqual(io.publish(target, False, empty, rendered, digest), 'after')
        self.assertEqual(target.stat().st_mode & 0o777, 0o600)
        with self.assertRaises(io.WorkflowError):
            io.publish(target, False, empty, rendered, digest)
        self.assertEqual(target.read_bytes(), rendered)
        self.assertEqual(io.target_snapshot(target, 'abc')['prior_block_sha256'], hashlib.sha256(b'hello\n').hexdigest())
        updated = io.render(rendered, 'abc', 'next')
        self.assertEqual(io.publish(target, True, digest, updated, hashlib.sha256(updated).hexdigest()), 'after')

    def test_lock_contention_and_release(self):
        with io.task_lock(self.root, 'task', 'workflow'):
            with self.assertRaises(io.WorkflowError) as error:
                with io.task_lock(self.root, 'task', 'workflow'):
                    pass
            self.assertEqual(error.exception.code, 'busy')
        with io.task_lock(self.root, 'task', 'workflow'):
            pass

    def test_short_write_failure_removes_only_new_partial_file(self):
        target = self.root / 'partial'
        with mock.patch.object(io.os, 'write', return_value=0):
            with self.assertRaises(io.WorkflowError):
                io.write_new(target, b'content')
        self.assertFalse(target.exists())
        io.write_new(target, b'retry')
        self.assertEqual(target.read_bytes(), b'retry')

    def test_empty_existing_is_not_absent(self):
        target = self.root / 'empty'
        target.write_bytes(b'')
        digest = hashlib.sha256(b'').hexdigest()
        with self.assertRaises(io.WorkflowError) as error:
            io.publish(target, False, digest, b'new', hashlib.sha256(b'new').hexdigest())
        self.assertEqual(error.exception.code, 'stale_snapshot')
        self.assertEqual(target.read_bytes(), b'')

    def test_non_regular_source_rejected_without_blocking(self):
        fifo = self.root / 'fifo'
        os.mkfifo(fifo)
        with self.assertRaises(io.WorkflowError):
            io.read_regular(fifo, 32)

    def test_missing_source_is_distinct_from_io_failure(self):
        with self.assertRaises(FileNotFoundError):
            io.read_regular(self.root / 'missing', 32)

    def test_private_file_requires_private_parent_and_oversize_code(self):
        parent = self.root / 'public'
        parent.mkdir(mode=0o755)
        path = parent / 'binding.json'
        path.write_bytes(b'1234')
        path.chmod(0o600)
        with self.assertRaises(io.WorkflowError) as error:
            io.read_regular(path, 32, private=True)
        self.assertEqual(error.exception.code, 'unsafe_path')
        parent.chmod(0o700)
        self.assertEqual(io.read_regular(path, 32, private=True), b'1234')
        with self.assertRaises(io.WorkflowError) as error:
            io.read_regular(path, 3, private=True)
        self.assertEqual(error.exception.code, 'oversize')
