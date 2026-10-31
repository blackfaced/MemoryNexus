"""Evidence proofs for exactly the substantive publications in a plan."""
from workflow_cli import status_all
from workflow_io import WorkflowError
from workflow_types import digest


def bounded_snapshot(ctx, snapshot, items, fresh=False):
    from workflow import sha, now
    targets = {w['target_id'] for w in items}
    bounded = dict(snapshot, targets=[t for t in snapshot['targets'] if t['target_id'] in targets],
                   dependencies=[], scans=[])
    substantive = [w for w in items if w['purpose'] == 'publish']
    if not substantive:
        return bounded
    decisions = {d['decision_id']: d for d in status_all(ctx, 'decisions')}
    dependencies = {}
    for item in substantive:
        d = decisions.get(item['decision_id'])
        if (d is None or d['validity'] != 'current' or d['proposed_decision'] != 'proposed'
                or d['decision_sha256'] != item['decision_sha256']):
            raise WorkflowError('stale_snapshot')
        for dep in d['dependencies']:
            key = (dep['type'], dep['id'])
            if key in dependencies and dependencies[key] != dep:
                raise WorkflowError('stale_snapshot')
            dependencies[key] = dep
    bounded['dependencies'] = list(dependencies.values())
    if not fresh and not {digest(d) for d in bounded['dependencies']}.issubset(
            {digest(d) for d in snapshot['dependencies']}):
        # Legacy material without complete proofs needs a fresh owner preview.
        raise WorkflowError('stale_snapshot')
    records = {r['id']: r for r in status_all(ctx, 'inputs')}
    sources = {s['source_id']: s for s in status_all(ctx, 'sources')}
    collections = {c['collection_id']: c for c in ctx['profile']['collections']}
    registered = {c['collection_id']: c for c in status_all(ctx, 'collections')}
    old_scans = {s['collection_id']: s for s in snapshot['scans']}
    scans = {}
    for dep in bounded['dependencies']:
        if dep['type'] == 'record':
            r = records.get(dep['id'])
            if r is None:
                # workflow-status inputs excludes lessons; their evidence may
                # still be a valid transitive dependency. Reuse the public,
                # bounded read-only context transport and validate its shape.
                from current_knowledge_task import call, context_result
                from workflow_cli import call as workflow_call
                revision = workflow_call(ctx, 'workflow-status', {})['revision']
                request = dict(limit=1, record_ids=[dep['id']])
                response = call(ctx['binding'], 'context', request)
                value = response['result']
                context_result(value, dict(request, purpose='history'), allow_historical=True)
                rows = value['entries'] + value['candidates']
                if response['revision'] != revision or value['truncated'] or len(rows) != 1 or rows[0]['id'] != dep['id']:
                    raise WorkflowError('stale_snapshot')
                r = rows[0]
                records[dep['id']] = r
            if r is None or r['state'] != 'current' or sha(r['text'].encode()) != dep['sha256']:
                raise WorkflowError('stale_snapshot')
            continue
        source = sources.get(dep['id'])
        if (source is None or source['source_revision'] != dep['revision']
                or source['sha256'] != dep['sha256'] or source['availability'] != 'available'
                or source['blocked']):
            raise WorkflowError('stale_snapshot')
        cid = source['collection_id']
        c = collections[cid]
        if cid not in scans:
            if fresh:
                scan = dict(collection_id=cid, expected_scan=registered[cid]['scan'],
                            observed_at=now(), availability='available', units=[])
                if c['format'] == 'append_jsonl':
                    scan['cursor'] = registered[cid]['cursor']
            else:
                old = old_scans.get(cid)
                if old is None or old['availability'] != 'available':
                    raise WorkflowError('stale_snapshot')
                scan = dict(old, units=[])
            scans[cid] = scan
        unit = dict(sha256=source['sha256'])
        if c['format'] == 'append_jsonl':
            unit.update(start=source['locator']['start'], end=source['locator']['end'])
        if not fresh and unit not in old_scans[cid]['units']:
            raise WorkflowError('stale_snapshot')
        if unit not in scans[cid]['units']:
            scans[cid]['units'].append(unit)
    bounded['scans'] = list(scans.values())
    return bounded
