import json, os, pathlib, subprocess, sys, tempfile, unittest
SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'scripts/workflow.py'
class Inspect(unittest.TestCase):
    def test_inspect_records_missing_and_read_without_task_or_cli(self):
        with tempfile.TemporaryDirectory(prefix='mn-runner-') as d:
            p=pathlib.Path(d).resolve(); root=p/'root';root.mkdir();(root/'note.md').write_text('Observed mismatch\n')
            selection=p/'selection.json';selection.write_text(json.dumps({'schema_version':1,'paths':['note.md','absent.md']}));os.chmod(selection,0o600)
            r=subprocess.run([sys.executable,str(SCRIPT),'inspect','--root',str(root),'--selection',str(selection),'--run-dir',str(p/'run')],capture_output=True)
            self.assertEqual(r.returncode,0,r.stderr)
            self.assertEqual(json.loads(r.stdout)['status'],'ok')
            inventory=json.loads((p/'run/inventory.json').read_text());self.assertEqual([f['status'] for f in inventory['files']],['read','missing'])
            self.assertFalse((root/'.memorynexus.json').exists())

class PublicErrors(unittest.TestCase):
    def test_unknown_argv_has_structured_content_free_failure(self):
        r=subprocess.run([sys.executable,str(SCRIPT),'inspect','--unexpected','PRIVATE_CONTENT'],capture_output=True)
        self.assertEqual(r.returncode,2)
        self.assertEqual(json.loads(r.stdout)['status'],'failed')
        self.assertNotIn(b'PRIVATE_CONTENT',r.stdout+r.stderr)
