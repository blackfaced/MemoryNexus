"""Reconcile retained publication plans without reconstructing operation bytes."""
import json
from pathlib import Path
from workflow_io import WorkflowError, read_regular, artifact_ref, resolve_ref, target_snapshot
from workflow_types import canonical, digest


def _load(path):
    from workflow import load
    return load(path)


def _retained(ctx, pub):
    root = Path(ctx['binding']['state_root'])
    for path in sorted(root.glob('*/plan.json')):
        try:
            plan = _load(path)
            if digest(plan) != pub['proposal']['plan_sha256']:
                continue
            snap = plan['snapshot']
            if (snap['task_id'] != ctx['binding']['task_id'] or snap['workflow_id'] != ctx['binding']['workflow_id']
                    or snap['incarnation'] != pub['incarnation']):
                continue
            for index, item in enumerate(plan['items']):
                if item.get('kind') != 'publish' or item.get('decision_id') != pub['decision_id']:
                    continue
                proposal = {k: item[k] for k in ('target_id', 'block_id', 'base_exists', 'base_sha256', 'after_sha256', 'output_sha256')}
                proposal['plan_sha256'] = digest(plan)
                if proposal != pub['proposal']:
                    continue
                rendered = resolve_ref(path.parent, item['rendered'], root)
                if digest_bytes(rendered) != item['after_sha256']:
                    continue
                # Retained authorization may be replayed, never reconstructed.
                request = path.parent / 'requests' / ('consumer-approve-%s-0.json' % index)
                body = _load(request)
                if body.get('proposal') != proposal or body.get('decision_id') != pub['decision_id']:
                    continue
                if body.get('incarnation') != snap['incarnation']:
                    continue
                return path, plan, item, index
        except (WorkflowError, OSError, ValueError, KeyError, TypeError):
            continue
    return None


def digest_bytes(raw):
    import hashlib
    return hashlib.sha256(raw).hexdigest()


def _classify(ctx, plan, item):
    if plan['snapshot']['incarnation'] != ctx['incarnation']:
        return 'restored'
    paths = [t['path'] for t in ctx['binding']['targets'] if t['target_id'] == item['target_id']]
    if len(paths) != 1:
        return 'uncertain'
    try:
        current = target_snapshot(paths[0], item['block_id'])
    except (WorkflowError, OSError):
        return 'uncertain'
    if current['exists'] and current['sha256'] == item['after_sha256']:
        return 'after'
    if current['exists'] == item['base_exists'] and current['sha256'] == item['base_sha256']:
        return 'before'
    return 'conflict'


def prepare_recovery(ctx, publications):
    items, remaining = [], []
    for pub in publications:
        if pub['publication'] == 'applied' or pub.get('superseded_by'):
            continue
        retained = _retained(ctx, pub)
        if retained is None:
            remaining.append({'id': pub['approval_id'], 'reason': 'plan_missing'})
            continue
        path, plan, original, index = retained
        item = {'kind': 'recovery', 'consumer_id': original['consumer_id'], 'approval_id': pub['approval_id'],
                'original_plan': {'run_dir': str(path.parent), 'artifact': artifact_ref(path.parent, 'plan.json')},
                'item_id': original['id'], 'classification': _classify(ctx, plan, original)}
        pending = path.parent / 'requests' / ('publication-result-%s-0.json' % index)
        receipt = path.parent / 'receipts' / pending.name
        if pending.exists() and not receipt.exists():
            item['pending_request'] = {'run_dir': str(path.parent), 'artifact': artifact_ref(path.parent, str(pending.relative_to(path.parent)))}
        items.append(item)
    return items, remaining


def apply_recovery(ctx, run, plan, item, index):
    from workflow_cli import status_all
    from workflow_engine import publication
    result = {'id': item['id'], 'status': 'needs_review', 'receipt_ids': []}
    pubs = [p for p in status_all(ctx, 'publications') if p['approval_id'] == item['approval_id']]
    if len(pubs) != 1:
        raise WorkflowError('plan_missing')
    retained = _retained(ctx, pubs[0])
    if retained is None:
        raise WorkflowError('plan_missing')
    path, original_plan, original, original_index = retained
    reference = item['original_plan']
    raw = resolve_ref(reference['run_dir'], reference['artifact'], ctx['binding']['state_root'])
    if Path(reference['run_dir']) != path.parent or raw != read_regular(path, 8 * 1024 * 1024, True) or original['id'] != item['item_id']:
        raise WorkflowError('stale_snapshot')
    state = _classify(ctx, original_plan, original)
    if state in ('conflict', 'uncertain', 'restored'):
        result['code'] = 'recovery_required' if state == 'restored' else 'stale_snapshot'
        return result
    if state == 'before' and pubs[0]['validity'] != 'current' and original.get('purpose') != 'remove':
        raise WorkflowError('stale_snapshot')
    # publication() replays original approval, then verifies current authority
    # before write, or reports already-observed after bytes without replacement.
    publication(ctx, path.parent, original_plan, original, original_index, result)
    return result
