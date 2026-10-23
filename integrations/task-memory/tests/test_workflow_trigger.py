import json, os, pathlib, subprocess, sys, unittest
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]/'scripts'))
from workflow_fixture import Fixture
class Trigger(unittest.TestCase):
    def test_unverified_trigger_is_manual_with_gap_and_private_evidence(self):
        f=Fixture()
        try:
            proof=f.private/'proof.json';f.write(proof,{'source':'synthetic-unsupported'})
            path=f.private/'trigger.json';f.write(path,{'kind':'scheduler','provider':'unconfigured','run_id':'synthetic','evidence_path':str(proof)})
            env=dict(os.environ, MEMORYNEXUS_TRIGGER_FILE=str(path),PYTHONDONTWRITEBYTECODE='1')
            run=f.state/'trigger'
            r=subprocess.run([f.binding['python'],str(f.package/'workflow.py'),'prepare','--binding',str(f.binding_path),'--workflow','capture','--client','synthetic','--run-dir',str(run)],capture_output=True,env=env)
            report=json.loads(r.stdout);self.assertEqual(r.returncode,3,report);self.assertEqual(report['trigger'],{'kind':'manual'});self.assertIn('trigger',report['pending']);self.assertEqual(report['items'][-1]['code'],'unverified_trigger')
            inventory=json.loads((run/'artifacts.json').read_text());self.assertIn('trigger-proof.json',{x['path'] for x in inventory['files']});self.assertEqual((run/'trigger-proof.json').read_bytes(),proof.read_bytes())
        finally:f.close()
    def test_malformed_unsafe_oversized_trigger_never_claims_scheduler(self):
        f=Fixture()
        try:
            for i,raw in enumerate((b'{bad',b'x'*65537,b'{"kind":"scheduler","command":"bad"}')):
                path=f.private/'trigger.json';path.write_bytes(raw);os.chmod(path,0o600)
                env=dict(os.environ,MEMORYNEXUS_TRIGGER_FILE=str(path),PYTHONDONTWRITEBYTECODE='1')
                r=subprocess.run([f.binding['python'],str(f.package/'workflow.py'),'status','--binding',str(f.binding_path)],capture_output=True,env=env)
                value=json.loads(r.stdout);self.assertEqual(r.returncode,3);self.assertEqual(value['trigger'],{'kind':'manual'});self.assertIn('trigger',value['pending'])
        finally:f.close()
