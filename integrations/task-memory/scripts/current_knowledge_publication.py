"""Prove original publication bytes using retained plans and kernel receipts."""
from pathlib import Path

from current_knowledge_task import sha
from workflow_io import WorkflowError, safe_path
from workflow_material import validate_plan
from workflow_types import canonical, digest, fail, fields, strict_loads


def envelope(plan, action, index, attempt=0):
    s = plan['snapshot']
    return dict(idempotency_key=f"wf:{plan['plan_id']}:{action}:{index}:{attempt}", run_id=plan['plan_id'],
                client=plan['client'], workflow_id=s['workflow_id'], profile_revision=s['profile_revision'], incarnation=s['incarnation'])


def receipt(snap, run, action, index, attempt, expected):
    basename = f'{action}-{index}-{attempt}.json'
    raw = snap.read(run / 'requests' / basename, 32768, True)
    body = strict_loads(raw)
    if body != expected or raw != canonical(body): fail('stale_snapshot')
    local = strict_loads(snap.read(run / 'receipts' / basename, 131072, True))
    fields(local, ('request_sha256', 'action', 'task_id', 'incarnation', 'response'))
    if (local['request_sha256'] != sha(raw) or local['action'] != action or
            local['task_id'] != snap.b['task_id'] or local['incarnation'] != snap.header['incarnation']): fail('stale_snapshot')
    response = local['response']
    fields(response, ('schema_version', 'workflow_contract', 'status', 'task_id', 'revision', 'incarnation', 'result'))
    if (response['schema_version'] != 1 or response['workflow_contract'] != 1 or
            response['status'] not in ('accepted', 'replayed', 'noop') or response['task_id'] != snap.b['task_id'] or
            response['incarnation'] != snap.header['incarnation'] or response['revision'] > snap.header['revision']): fail('stale_snapshot')
    # Local retained files are not authority. Match the exact request to its live,
    # read-only kernel receipt at the same R/I as every other view.
    live = snap.status('receipts', keys=[body['idempotency_key']])
    if not live['complete'] or len(live['items']) != 1: fail('incomplete')
    row = live['items'][0]
    if row['idempotency_key'] != body['idempotency_key'] or row['action'] != action or row['digest'] != digest({'action': action, 'request': body}): fail('stale_snapshot')
    return response['result'], row['result']


def retained_output(snap, decision, pub, target_id):
    root = safe_path(snap.ctx['binding']['state_root'], True)
    # Bounded direct run lookup, never recursively search private task/source data.
    children = []
    for path in root.iterdir():
        children.append(path)
        if len(children) > 1000: fail('incomplete')
    retained = None
    for child in sorted(children):
        if child.is_symlink() or not child.is_dir(): continue
        path = child / 'plan.json'
        try:
            from workflow_io import read_regular
            raw = read_regular(path, 8*1024*1024, True)
            plan = strict_loads(raw)
            if digest(plan) != pub['proposal']['plan_sha256']: continue
            validate_plan(plan)
        except (WorkflowError, OSError, ValueError, KeyError, TypeError): continue
        if retained is not None: fail('stale_snapshot')
        retained = (child, plan)
        if snap.read(path, 8*1024*1024, True) != raw: fail('stale_snapshot')
    if retained is None: fail('source_unavailable')
    run, plan = retained; s = plan['snapshot']
    for key, value in {'task_id': snap.b['task_id'], 'workflow_id': snap.ctx['binding']['workflow_id'],
                       'incarnation': snap.header['incarnation'], 'profile_revision': pub['profile_revision'],
                       'profile_sha256': snap.ctx['profile_sha256'], 'binding_sha256': snap.ctx['binding_sha256']}.items():
        if s[key] != value: fail('stale_snapshot')
    if plan['workflow'] != 'publish' or plan['authorization']['mode'] != 'owner': fail('stale_snapshot')
    matches = [(i, item) for i, item in enumerate(plan['items']) if item.get('decision_id') == decision['decision_id']]
    if len(matches) != 1: fail('stale_snapshot')
    index, item = matches[0]
    if item['kind'] != 'publish' or item['purpose'] != 'publish': fail('stale_snapshot')
    for key in ('decision_id', 'decision_sha256', 'consumer_id', 'block_id', 'output_sha256'):
        if item[key] != decision[key]: fail('stale_snapshot')
    proposal = {k: item[k] for k in ('target_id', 'block_id', 'base_exists', 'base_sha256', 'after_sha256', 'output_sha256')}
    proposal['plan_sha256'] = digest(plan)
    if proposal != pub['proposal'] or item['target_id'] != target_id: fail('stale_snapshot')
    original = item['output_text'].encode()
    if sha(original) != decision['output_sha256']: fail('stale_snapshot')
    ref = item['rendered']; path = safe_path(run / ref['path'], True)
    if root not in path.parents or run not in path.parents: fail('scope_mismatch')
    rendered = snap.read(path, 64*1024*1024, True)
    if len(rendered) != ref['bytes'] or sha(rendered) != ref['sha256'] or sha(rendered) != item['after_sha256']: fail('stale_snapshot')
    approved = dict(envelope(plan, 'consumer-approve', index), decision_id=decision['decision_id'],
                    decision_sha256=decision['decision_sha256'], proposal=proposal, confirmation='confirmed')
    local, live = receipt(snap, run, 'consumer-approve', index, 0, approved)
    for value in (local, live):
        if (value.get('approval_id') != pub['approval_id'] or value.get('decision_id') != decision['decision_id'] or
                value.get('proposal') != proposal or value.get('validity') != 'current'): fail('stale_snapshot')
    attempts = pub['attempts']
    if len(attempts) != 1 or attempts[0]['outcome'] != 'applied' or attempts[0].get('observed_sha256') != item['after_sha256']:
        fail('stale_snapshot')
    found = False
    paths = []
    for path in (run / 'receipts').iterdir():
        paths.append(path)
        if len(paths) > 1000: fail('incomplete')
    for path in sorted(paths):
        prefix = f'publication-result-{index}-'
        if not path.name.startswith(prefix) or path.suffix != '.json': continue
        suffix = path.stem[len(prefix):]
        if not suffix.isdecimal(): continue
        local = strict_loads(snap.read(path, 131072, True))
        if local.get('response', {}).get('result', {}).get('attempt_id') != attempts[0]['attempt_id']: continue
        expected = dict(envelope(plan, 'publication-result', index, int(suffix)), approval_id=pub['approval_id'],
                        outcome='applied', observed_at=plan['created_at'], observed_sha256=item['after_sha256'])
        local, live = receipt(snap, run, 'publication-result', index, int(suffix), expected)
        for value in (local, live):
            if any(value.get(k) != v for k, v in dict(approval_id=pub['approval_id'], attempt_id=attempts[0]['attempt_id'],
                            outcome='applied', validity='current', publication='applied').items()): fail('stale_snapshot')
        if attempts[0]['observed_at'] != expected['observed_at']: fail('stale_snapshot')
        found = True
    if not found: fail('source_unavailable')
    return item['output_text'].rstrip('\n') + '\n'
