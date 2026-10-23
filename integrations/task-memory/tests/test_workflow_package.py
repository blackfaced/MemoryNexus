import json, os, pathlib, subprocess, sys, unittest
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'scripts'))
from workflow_fixture import Fixture
from workflow_cli import package_inventory
from workflow_types import digest
class Package(unittest.TestCase):
    def test_plain_python_imports_do_not_change_pinned_package(self):
        f=Fixture()
        try:
            before=digest(package_inventory(f.package))
            for label in ('first','second'):
                driver="import sys,runpy;sys.pycache_prefix=None;sys.path.insert(0,sys.argv[1]);sys.argv=sys.argv[2:];runpy.run_path(sys.argv[0],run_name='__main__')"
                env=dict(os.environ);env.pop('PYTHONDONTWRITEBYTECODE',None)
                r=subprocess.run([sys.executable,'-c',driver,str(f.package),str(f.package/'workflow.py'),'prepare','--binding',str(f.binding_path),'--workflow','capture','--client','synthetic','--run-dir',str(f.state/label)],capture_output=True,env=env)
                self.assertEqual(r.returncode,3,r.stdout+r.stderr)
                self.assertEqual(json.loads(r.stdout)['status'],'needs_review')
            self.assertEqual(digest(package_inventory(f.package)),before)
            self.assertFalse((f.package/'__pycache__').exists())
        finally:f.close()
