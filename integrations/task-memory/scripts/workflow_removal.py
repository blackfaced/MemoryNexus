"""Explicit, owner-approved removal; never inferred from source invalidity."""
from workflow import exact, sha
from workflow_io import WorkflowError, target_snapshot, write_new, artifact_ref
from workflow_types import uuid, string


def validate_owner(approval_id, decisions, publications):
    prior = next((p for p in publications if p['approval_id'] == approval_id), None)
    if not prior or prior.get('maintenance') or prior['publication'] != 'applied':
        raise WorkflowError('invalid_input')
    d = next((d for d in decisions if d['decision_id'] == prior['decision_id']), None)
    if d is None:
        raise WorkflowError('scope_mismatch')
    applied = {p['decision_id'] for p in publications if p['publication'] == 'applied'}
    if any(x['block_id'] == d['block_id'] and x['generation'] > d['generation']
           and x['decision_id'] in applied for x in decisions):
        raise WorkflowError('stale_snapshot')
    if any(p['decision_id'] == d['decision_id'] and p.get('maintenance') == 'remove'
           and p['publication'] == 'applied' for p in publications):
        raise WorkflowError('stale_snapshot')
    return prior, d


def prepare_removals(ctx, run, snapshot, request, decisions, publications):
    exact(request, ['schema_version', 'items'])
    if type(request['schema_version']) is not int or request['schema_version'] != 1:
        raise WorkflowError('invalid_input')
    if not isinstance(request['items'], list) or not 1 <= len(request['items']) <= 2:
        raise WorkflowError('invalid_input')
    items, targets = [], set()
    for value in request['items']:
        exact(value, ['approval_id', 'reason'])
        uuid(value['approval_id'])
        string(value['reason'], 1, 240)
        prior, d = validate_owner(value['approval_id'], decisions, publications)
        target = prior['proposal']['target_id']
        consumer = next((c for c in ctx['profile']['consumers'] if c['consumer_id'] == d['consumer_id']), None)
        if consumer is None or consumer['target_id'] != target:
            raise WorkflowError('stale_snapshot')
        if target in targets:
            raise WorkflowError('invalid_input')
        targets.add(target)
        path = next((t['path'] for t in ctx['binding']['targets'] if t['target_id'] == target), None)
        if path is None:
            raise WorkflowError('stale_snapshot')
        snap = target_snapshot(path, d['block_id'])
        if not snap['exists'] or snap.get('prior_block_sha256') != prior['proposal']['output_sha256']:
            raise WorkflowError('target_conflict')
        snapshot['targets'].append(dict(target_id=target, **snap))
        name = 'evidence/remove-' + d['decision_id'] + '.txt'
        write_new(run / name, b'')
        items.append(dict(kind='publish', consumer_id=d['consumer_id'],
                          decision_id=d['decision_id'], decision_sha256=d['decision_sha256'],
                          target_id=target, block_id=d['block_id'], purpose='remove',
                          previous_approval_id=prior['approval_id'], reason=value['reason'],
                          prior_block_sha256=snap['prior_block_sha256'], output_sha256=sha(b''),
                          output=dict(run_dir=str(run), artifact=artifact_ref(run, name))))
    return items
