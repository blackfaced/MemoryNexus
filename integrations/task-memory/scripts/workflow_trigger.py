"""Bounded trigger provenance. No scheduler provider proof adapter is installed."""
import os
from pathlib import Path
from workflow_io import WorkflowError, read_regular, safe_path, write_new
from workflow_types import strict_loads, fields, string, name

def metadata():
    path = os.environ.get('MEMORYNEXUS_TRIGGER_FILE')
    if not path:
        return False, []
    evidence = []
    try:
        raw = read_regular(safe_path(path, private=True), 65536, private=True)
        value = strict_loads(raw)
        if value.get('kind') == 'manual':
            fields(value, ('kind',))
            return False, [('trigger-metadata.json', raw)]
        fields(value, ('kind', 'provider', 'run_id', 'evidence_path'))
        if value['kind'] != 'scheduler':
            raise WorkflowError('invalid_input')
        name(value['provider'])
        string(value['run_id'], 1, 240)
        proof = read_regular(safe_path(value['evidence_path'], private=True), 65536, private=True)
        evidence = [('trigger-metadata.json', raw), ('trigger-proof.json', proof)]
    except (WorkflowError, OSError, ValueError, TypeError, AttributeError):
        pass
    # A label/file alone never establishes a supported scheduler execution.
    return True, evidence

def retain(run):
    gap, files = metadata()
    for filename, raw in files:
        path = Path(run) / filename
        if path.exists():
            if read_regular(path,65536,private=True) != raw:
                continue
        else:
            write_new(path, raw)
