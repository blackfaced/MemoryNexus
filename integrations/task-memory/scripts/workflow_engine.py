"""Phase selection and exact immutable operation material."""
from pathlib import Path
import uuid
from workflow import load, exact, sha, now, encode, canonical, report, save_report
from workflow_io import WorkflowError, read_regular, write_new, new_run, artifact_ref, resolve_ref, target_snapshot, render, remove_block, publish, task_lock
from workflow_cli import load_context, call, status_all
from workflow_scan import scans

def fail(code):
    raise WorkflowError(code)

def header(ctx, allow_recovery=False):
    h = call(ctx, 'workflow-status', {})
    ctx['incarnation'] = h['incarnation']
    ctx['ledger_revision'] = h['revision']
    rows = status_all(ctx, 'summary')
    if not rows or (rows[0]['state'] != 'active' and (not allow_recovery)):
        fail('recovery_required')
    s = rows[0]
    if s['profile_sha256'] != ctx['profile_sha256'] or s['binding_sha256'] != ctx['binding_sha256']:
        fail('stale_snapshot')
    return h

def snapshot(ctx):
    return dict(task_id=ctx['binding']['task_id'], workflow_id=ctx['binding']['workflow_id'], incarnation=ctx['incarnation'], profile_revision=ctx['profile']['revision'], profile_sha256=ctx['profile_sha256'], binding_sha256=ctx['binding_sha256'], ledger_revision=ctx['ledger_revision'], scans=[], dependencies=[], targets=[])

def check_snap(ctx, s):
    header(ctx)
    for key in ('task_id', 'workflow_id', 'incarnation', 'profile_revision', 'profile_sha256', 'binding_sha256'):
        if snapshot(ctx)[key] != s[key]:
            fail('stale_snapshot')
    if s['scans']:
        from workflow_scan import selected
        for scan in s['scans']:
            collection = next((c for c in ctx['profile']['collections'] if c['collection_id'] == scan['collection_id']))
            path = Path(ctx['binding']['task_root']) / collection['path']
            try:
                raw = read_regular(path, (2 if collection['format'] == 'text' else 64) * 1024 * 1024)
            except (OSError, WorkflowError):
                if scan['availability'] == 'unavailable':
                    continue
                fail('stale_snapshot')
            if scan['availability'] != 'available':
                fail('stale_snapshot')
            if collection['format'] == 'text':
                if sha(selected(raw, collection['selector'])) != scan['units'][0]['sha256']:
                    fail('stale_snapshot')
            else:
                cursor = scan['cursor']
                if len(raw) < cursor['offset'] or sha(raw[:cursor['offset']]) != cursor['prefix_sha256']:
                    fail('stale_snapshot')
                for unit in scan['units']:
                    if sha(raw[unit['start']:unit['end']]) != unit['sha256']:
                        fail('stale_snapshot')
    for t in s['targets']:
        path = next((x['path'] for x in ctx['binding']['targets'] if x['target_id'] == t['target_id']))
        actual = target_snapshot(path)
        if actual['exists'] != t['exists'] or actual['sha256'] != t['sha256']:
            fail('stale_snapshot')

def ref_bytes(ctx, ref):
    return resolve_ref(ref['run_dir'], ref['artifact'], ctx['binding']['state_root'])

def retained_output(ctx, decision):
    root = Path(ctx['binding']['state_root'])
    for output in sorted(root.glob('*/outputs/' + decision['decision_id'] + '.txt')):
        try:
            raw = read_regular(output, 8192, private=True)
        except (WorkflowError,OSError):
            continue
        if sha(raw) == decision.get('output_sha256'):
            return {'run_dir': str(output.parent.parent), 'artifact': artifact_ref(output.parent.parent, str(output.relative_to(output.parent.parent)))}
    return None

def block_generations(decisions, publications):
    """Current proposals or later published generations retire older block owners."""
    published = {p['decision_id'] for p in publications
                 if p['publication'] == 'applied' and not p.get('maintenance')}
    generations = {}
    for d in decisions:
        if (d['validity'] == 'current' and d['proposed_decision'] == 'proposed') or d['decision_id'] in published:
            generations[d['block_id']] = max(generations.get(d['block_id'], 0), d['generation'])
    return generations

def prepare(a, ctx):
    if getattr(a, 'removals', None) and a.workflow != 'publish':
        fail('invalid_input')
    selection = None
    if getattr(a, 'publish_selection', None):
        if a.workflow != 'publish' or getattr(a, 'removals', None):
            fail('invalid_input')
        from workflow_types import uuid as valid_uuid
        request = load(a.publish_selection)
        exact(request, ['schema_version', 'items'])
        if type(request['schema_version']) is not int or request['schema_version'] != 1 or not isinstance(request['items'], list) or not 1 <= len(request['items']) <= 2:
            fail('invalid_input')
        selection = {}
        for x in request['items']:
            exact(x, ['decision_id', 'purpose'])
            valid_uuid(x['decision_id'])
            if x['decision_id'] in selection or x['purpose'] not in ('publish', 'neutralize'):
                fail('invalid_input')
            selection[x['decision_id']] = x['purpose']
    header(ctx, allow_recovery=a.workflow == 'recovery')
    b = ctx['binding']
    run = new_run(a.run_dir, state_root=b['state_root'], task_root=b['task_root'])
    s = snapshot(ctx)
    items = []
    evidence = []
    remaining = []
    sources = status_all(ctx, 'sources')
    inputs = status_all(ctx, 'inputs')
    decisions = status_all(ctx, 'decisions')
    publications = status_all(ctx, 'publications')
    (ss, units, pending) = scans(ctx, status_all(ctx, 'collections'), sources)
    s['scans'] = ss
    remaining.extend(pending)
    (run / 'evidence').mkdir(mode=448)
    unavailable_collections={scan['collection_id'] for scan in ss if scan['availability']=='unavailable'}
    changed_sources={source['source_id'] for source in sources if source['collection_id'] in unavailable_collections}
    for (c, sid, rev, digest, data, old) in units:
        if a.workflow=='review' and (old is None or old['source_revision']!=rev or old['sha256']!=digest or old['availability']!='available'):
            changed_sources.add(sid)
            remaining.append(dict(id=sid,reason='awaiting_extraction'))
            continue
        if a.workflow not in ('capture','review'):
            continue
        captured = next((x for x in inputs if x.get('origin', {}).get('source_id') == sid and x['origin']['source_revision'] == rev and (x['origin']['extractor_version'] == c['extractor_version']) and (x['state'] == 'current')), None)
        if a.workflow == 'capture' and (captured or (old and old['blocked'])):
            continue
        # Work submissions remain capped at 20. Review may reference the full
        # bounded evidence cache, including registered sync-only auxiliaries.
        if len(evidence) >= (600 if a.workflow == 'review' else 20):
            remaining.append(dict(id=sid, reason='awaiting_extraction' if a.workflow == 'capture' else 'awaiting_review'))
            continue
        name = 'evidence/' + sid + '.txt'
        write_new(run / name, data)
        evidence.append({'source_id': sid, 'source_revision': old['source_revision'] if old else 0, 'sha256': digest, 'artifact': artifact_ref(run, name)})
        captured = next((x for x in inputs if x.get('origin', {}).get('source_id') == sid and x['origin']['source_revision'] == rev and (x['origin']['extractor_version'] == c['extractor_version']) and (x['state'] == 'current')), None)
        if a.workflow == 'capture' and (not captured) and (not (old and old['blocked'])):
            items.append(dict(kind='capture', collection_id=c['collection_id'], source_id=sid, source_revision=rev, extractor_version=c['extractor_version'], source_sha256=digest, evidence_index=len(evidence) - 1))
    if a.workflow == 'review':
        for source in sources:
            if source['availability'] == 'available' and (not source['blocked']) and (source['source_id'] in {e['source_id'] for e in evidence}):
                s['dependencies'].append(dict(type='source', id=source['source_id'], revision=source['source_revision'], sha256=source['sha256']))
        for rec in inputs:
            if rec.get('origin',{}).get('source_id') in changed_sources:
                continue
            if rec['state'] != 'current':
                continue
            for consumer in ctx['profile']['consumers']:
                slot = [d for d in decisions if d['consumer_id'] == consumer['consumer_id'] and d['root_record_id'] == rec['root_record_id']]
                previous = max(slot, key=lambda d: d['generation']) if slot else None
                if previous and previous['validity'] == 'current':
                    continue
                deps = [dict(type='record', id=rec['id'], sha256=sha(rec['text'].encode()))]
                w = dict(kind='review', consumer_id=consumer['consumer_id'], input_record_id=rec['id'], root_record_id=rec['root_record_id'], input_text=rec['text'], dependencies=deps)
                if previous:
                    w.update(previous_decision_id=previous['decision_id'], previous_validity='stale')
                items.append(w)
    if a.workflow == 'publish' and getattr(a, 'removals', None):
        from workflow_removal import prepare_removals
        items = prepare_removals(ctx, run, s, load(a.removals), decisions, publications)
    elif a.workflow == 'publish':
        queued = set()
        generations = block_generations(decisions, publications)
        for d in decisions:
            if any(p.get('maintenance') == 'remove' and p['publication'] == 'applied'
                   and any(x['decision_id'] == p['decision_id'] and x['block_id'] == d['block_id']
                           and x['generation'] >= d['generation'] for x in decisions) for p in publications):
                continue
            pubs = [p for p in publications if p['decision_id'] == d['decision_id']]
            if any((p['publication'] == 'applied' and p['validity'] == 'current' for p in pubs)):
                continue
            purpose = 'publish' if d['validity'] == 'current' and d['proposed_decision'] == 'proposed' else 'neutralize' if d['validity'] != 'current' and any((p['publication'] == 'applied' and (not p.get('maintenance')) for p in pubs)) else None
            if not purpose:
                continue
            c = next((c for c in ctx['profile']['consumers'] if c['consumer_id'] == d['consumer_id']), None)
            if not c or (purpose == 'neutralize' and (not c['neutralize_stale'])):
                continue
            if purpose == 'neutralize':
                if generations.get(d['block_id'], 0) > d['generation']:
                    continue
            if selection is not None and selection.get(d['decision_id']) != purpose:
                remaining.append(dict(id=d['decision_id'], reason='unselected'))
                continue
            target = c['target_id']
            output = retained_output(ctx, d) if purpose == 'publish' else None
            if purpose == 'publish' and (not output):
                remaining.append(dict(id=d['decision_id'], reason='plan_missing'))
                continue
            path = next((t['path'] for t in b['targets'] if t['target_id'] == target))
            t = target_snapshot(path, d['block_id'])
            t['target_id'] = target
            if purpose == 'neutralize':
                # An applied maintenance publication is historical (stale), but
                # its exact block body proves the file already needs no write.
                if any(p.get('maintenance') == 'neutralize' and p['publication'] == 'applied'
                       and t.get('prior_block_sha256') == p['proposal']['output_sha256'] for p in pubs):
                    continue
                prior = next((p for p in pubs if p['publication'] == 'applied' and (not p.get('maintenance'))))
                # The block region must still hold exactly the body that was
                # published.  Comparing the whole file digest instead would refuse
                # every block but the last one in a file that holds more than one
                # block, because publishing a later block changes the file digest
                # without touching this block.
                if not t['exists'] or t.get('prior_block_sha256') != prior['proposal']['output_sha256']:
                    remaining.append(dict(id=d['decision_id'], reason='awaiting_approval'))
                    continue
                text = b'MemoryNexus: source evidence changed; review required.\n'
                n = 'evidence/neutral-' + d['decision_id'] + '.txt'
                write_new(run / n, text)
                output = {'run_dir': str(run), 'artifact': artifact_ref(run, n)}
            # The slot is claimed only by a candidate that can actually run, so a
            # rejected neutralisation never blocks the target for later rounds.
            if target in queued:
                remaining.append(dict(id=d['decision_id'], reason='target_queued'))
                continue
            queued.add(target)
            s['targets'].append(t)
            w = dict(kind='publish', consumer_id=d['consumer_id'], decision_id=d['decision_id'], decision_sha256=d['decision_sha256'], target_id=target, block_id=d['block_id'], output=output, output_sha256=sha(ref_bytes(ctx, output)), purpose=purpose)
            if purpose == 'neutralize':
                w['previous_approval_id'] = next((p['approval_id'] for p in pubs if p['publication'] == 'applied' and (not p.get('maintenance'))))
            items.append(w)
    if selection is not None and {w['decision_id']: w['purpose'] for w in items} != selection:
        fail('invalid_input')
    if a.workflow == 'recovery':
        from workflow_recovery import prepare_recovery
        (items, recovery_pending) = prepare_recovery(ctx, publications)
        remaining.extend(recovery_pending)
    allitems = items
    items = items[:20]
    remaining.extend((dict(id=x.get('source_id', x.get('decision_id', x.get('input_record_id', 'pending'))), reason='awaiting_extraction' if a.workflow == 'capture' else 'awaiting_review') for x in allitems[20:]))
    for (i, item) in enumerate(items):
        item['id'] = 'u' + str(i)
    prepared = dict(schema_version=1, prepared_id=str(uuid.uuid4()), created_at=now(), workflow=a.workflow, client=a.client, snapshot=s, items=items, remaining=remaining[:20], evidence=evidence)
    if len(remaining) > 20:
        prepared['remaining_count'] = len(remaining)
    write_new(run / 'prepared.json', encode(prepared))
    r = report(a.workflow, prepared['prepared_id'], [dict(id=w['id'], status='needs_review', receipt_ids=[]) for w in items] + [dict(id='remaining' + str(i), status='needs_review', code=x['reason'], receipt_ids=[]) for (i, x) in enumerate(remaining[:20])])
    return save_report(run, r)

def preview(a, ctx):
    if Path(ctx['binding']['state_root']) not in Path(a.prepared).parents:
        fail('scope_mismatch')
    prepared = load(a.prepared)
    from workflow_material import validate_prepared
    validate_prepared(prepared)
    for evidence in prepared['evidence']:
        resolve_ref(Path(a.prepared).parent, evidence['artifact'], ctx['binding']['state_root'])
    exact(prepared, ['schema_version', 'prepared_id', 'created_at', 'workflow', 'client', 'snapshot', 'items', 'remaining', 'evidence'], ['remaining_count'])
    submission = load(a.submission)
    exact(submission, ['schema_version', 'prepared_sha256', 'items'])
    if submission['prepared_sha256'] != sha(read_regular(a.prepared, 8 * 1024 * 1024)) or len(submission['items']) > 20:
        fail('invalid_input')
    check_snap(ctx, prepared['snapshot'])
    run = new_run(a.run_dir, state_root=ctx['binding']['state_root'], task_root=ctx['binding']['task_root'])
    items = []
    used = set()
    workflow = prepared['workflow']
    if workflow in ('publish', 'recovery') and submission['items']:
        fail('invalid_input')
    for value in submission['items']:
        key = value.get('unit_id') if workflow == 'capture' else (value.get('input_record_id'), value.get('consumer_id'))
        if str(key) in used:
            fail('invalid_input')
        used.add(str(key))
        matches = [w for w in prepared['items'] if (w['id'] if workflow == 'capture' else (w['input_record_id'], w['consumer_id'])) == key]
        if len(matches) != 1:
            fail('invalid_input')
        w = matches[0]
        if workflow == 'capture':
            exact(value, ['unit_id', 'text', 'occurred_at'])
            item = {k: w[k] for k in ('id', 'kind', 'source_id', 'source_revision', 'extractor_version')}
            item.update(text=value['text'], occurred_at=value['occurred_at'])
        else:
            exact(value, ['input_record_id', 'consumer_id', 'decision', 'reason', 'dependencies'], ['output_text', 'previous_decision_id'])
            if value['decision'] not in ('proposed', 'deferred', 'skipped'):
                fail('invalid_input')
            if value.get('previous_decision_id') != w.get('previous_decision_id'):
                fail('invalid_input')
            allowed = w['dependencies'] + prepared['snapshot']['dependencies']
            if any((d not in allowed for d in value['dependencies'])) or not all((d in value['dependencies'] for d in w['dependencies'])):
                fail('invalid_input')
            item = {'id': w['id'], 'kind': 'review', **value}
            if value['decision'] == 'proposed':
                text = value.get('output_text')
                if not isinstance(text, str) or not text.strip() or len(text) > 1000 or ('<!-- memorynexus:' in text):
                    fail('invalid_input')
                item['output_text'] = text.rstrip('\n') + '\n'
            elif 'output_text' in value:
                fail('invalid_input')
        if workflow == 'capture' and (not isinstance(item['text'], str) or not item['text'].strip() or len(item['text']) > 1000):
            fail('invalid_input')
        items.append(item)
    if workflow == 'publish':
        (run / 'rendered').mkdir(mode=448)
        for w in prepared['items']:
            text = ref_bytes(ctx, w['output']).decode()
            path = next((t['path'] for t in ctx['binding']['targets'] if t['target_id'] == w['target_id']))
            target = next((t for t in prepared['snapshot']['targets'] if t['target_id'] == w['target_id']))
            base = read_regular(path, 2 * 1024 * 1024) if target['exists'] else b''
            after = remove_block(base, w['block_id'], w['prior_block_sha256']) if w['purpose'] == 'remove' else render(base, w['block_id'], text)
            n = 'rendered/' + w['id'] + '.md'
            write_new(run / n, after)
            item = {k: w[k] for k in ('id', 'kind', 'consumer_id', 'decision_id', 'decision_sha256', 'target_id', 'block_id', 'purpose')}
            item.update(base_exists=target['exists'], base_sha256=target['sha256'], after_sha256=sha(after), output_text=text, output_sha256=sha(text.encode()), rendered=artifact_ref(run, n))
            if 'previous_approval_id' in w:
                item['previous_approval_id'] = w['previous_approval_id']
            if w['purpose'] == 'remove':
                item.update(reason=w['reason'], prior_block_sha256=w['prior_block_sha256'])
            import difflib
            write_new(run / ('rendered/' + w['id'] + '.diff'), ''.join(difflib.unified_diff(base.decode().splitlines(True), after.decode().splitlines(True), fromfile=path, tofile=path)).encode())
            items.append(item)
    if workflow == 'recovery':
        items = prepared['items']
    automatic = ctx['profile']['capture'].get('automatic_drafts' if workflow == 'capture' else 'automatic_reviews', False) if workflow in ('capture', 'review') else bool(items) and all((x.get('purpose') == 'neutralize' for x in items))
    auth = {'mode': 'profile', 'profile_revision': ctx['profile']['revision']} if automatic or (workflow == 'capture' and (not items)) else {'mode': 'owner'}
    plan = dict(schema_version=1, plan_id=str(uuid.uuid4()), created_at=now(), workflow=workflow, client=prepared['client'], prepared_sha256=submission['prepared_sha256'], snapshot=prepared['snapshot'], items=items, authorization=auth)
    from workflow_material import validate_plan
    validate_plan(plan)
    write_new(run / 'plan.json', encode(plan))
    text = '# Repository memory preview\n\nPlan SHA-256: `' + sha(canonical(plan)) + '`\n\nRetention preserves reported trust; it does not establish external truth. Fresh owner-mode digest requires fresh human confirmation.\n\n```json\n' + encode(plan).decode() + '```\n'
    write_new(run / 'preview.md', text.encode())
    return save_report(run, report(workflow, plan['plan_id'], [dict(id=x['id'], status='needs_review', receipt_ids=[]) for x in items]))

def staged(ctx, run, plan, action, index, body, attempt=0):
    path = run / 'requests' / f'{action}-{index}-{attempt}.json'
    rp = run / 'receipts' / path.name
    raw = canonical(body)
    if path.exists():
        if read_regular(path, 32768, private=True) != raw:
            fail('conflict')
    else:
        write_new(path, raw)
    if rp.exists():
        receipt = load(rp)
        if receipt['request_sha256'] != sha(raw):
            fail('conflict')
    response = call(ctx, action, body)
    receipt = dict(request_sha256=sha(raw), action=action, task_id=ctx['binding']['task_id'], incarnation=response['incarnation'], response=response)
    if not rp.exists():
        write_new(rp, encode(receipt))
    return response

def envelope(plan, action, index, attempt=0):
    s = plan['snapshot']
    return dict(idempotency_key=f"wf:{plan['plan_id']}:{action}:{index}:{attempt}", run_id=plan['plan_id'], client=plan['client'], workflow_id=s['workflow_id'], profile_revision=s['profile_revision'], incarnation=s['incarnation'])

def apply(a, ctx):
    plan = load(a.plan)
    from workflow_material import validate_plan
    validate_plan(plan)
    exact(plan, ['schema_version', 'plan_id', 'created_at', 'workflow', 'client', 'prepared_sha256', 'snapshot', 'items', 'authorization'])
    run = Path(a.plan).parent
    if Path(ctx['binding']['state_root']) not in run.parents:
        fail('scope_mismatch')
    if plan['authorization']['mode'] == 'owner' and a.approve != sha(canonical(plan)):
        fail('approval_required')
    if plan['authorization']['mode'] == 'profile':
        if plan['authorization'].get('profile_revision') != ctx['profile']['revision']:
            fail('approval_required')
        if plan['workflow'] in ('capture', 'review') and plan['items'] and (not ctx['profile']['capture']['automatic_drafts' if plan['workflow'] == 'capture' else 'automatic_reviews']):
            fail('approval_required')
        if plan['workflow'] == 'publish' and any((x['purpose'] != 'neutralize' for x in plan['items'])):
            fail('approval_required')
    for n in ('requests', 'receipts', 'outputs'):
        if not (run / n).exists():
            (run / n).mkdir(mode=448)
    results = []
    report_errors = []
    with task_lock(ctx['binding']['state_root'], ctx['binding']['task_id'], ctx['binding']['workflow_id']):
        header(ctx)
        if ctx['incarnation'] != plan['snapshot']['incarnation']:
            fail('recovery_required')
        if plan['workflow'] != 'publish':
            check_snap(ctx, plan['snapshot'])
        if plan['workflow'] == 'capture':
            for (i, scan) in enumerate(plan['snapshot']['scans']):
                try:
                    body = envelope(plan, 'source-sync', i)
                    body.update(scan)
                    r = staged(ctx, run, plan, 'source-sync', i, body)
                    results.append(dict(id='scan' + str(i), status='ok', receipt_ids=[r['result']['collection_id']]))
                except (WorkflowError, OSError) as e:
                    results.append(dict(id='scan' + str(i), status='failed', code=getattr(e, 'code', 'io_error'), receipt_ids=[]))
        for (i, item) in enumerate(plan['items']):
            result = dict(id=item['id'], status='pending', receipt_ids=[])
            try:
                if item['kind'] == 'capture':
                    actual = next((s for s in status_all(ctx, 'sources') if s['source_id'] == item['source_id']), None)
                    if not actual or actual['source_revision'] != item['source_revision']:
                        fail('stale_snapshot')
                    body = envelope(plan, 'source-capture', i)
                    body.update({k: item[k] for k in ('source_id', 'source_revision', 'extractor_version', 'text', 'occurred_at')})
                    if plan['authorization']['mode'] == 'owner':
                        body['confirmation'] = 'confirmed'
                    r = staged(ctx, run, plan, 'source-capture', i, body)
                    result.update(status='ok', receipt_ids=[r['result']['record_id']])
                elif item['kind'] == 'review':
                    body = envelope(plan, 'consumer-review', i)
                    body.update({k: v for (k, v) in item.items() if k not in ('id', 'kind', 'output_text')})
                    if 'output_text' in item:
                        body['output_sha256'] = sha(item['output_text'].encode())
                    if plan['authorization']['mode'] == 'owner':
                        body['confirmation'] = 'confirmed'
                    r = staged(ctx, run, plan, 'consumer-review', i, body)
                    decision = r['result']['decision_id']
                    result.update(status='ok', receipt_ids=[decision])
                    if 'output_text' in item:
                        op = run / 'outputs' / (decision + '.txt')
                        raw = item['output_text'].encode()
                        if op.exists():
                            if read_regular(op, 8192, private=True) != raw:
                                fail('conflict')
                        else:
                            write_new(op, raw)
                elif item['kind'] == 'publish':
                    publication(ctx, run, plan, item, i, result)
                elif item['kind'] == 'recovery':
                    from workflow_recovery import apply_recovery
                    result = apply_recovery(ctx, run, plan, item, i)
                else:
                    fail('plan_missing')
            except (WorkflowError, OSError, ValueError) as e:
                result.update(status='needs_review' if getattr(e, 'code', '') in ('stale_snapshot', 'approval_required', 'recovery_required', 'plan_missing') else 'failed', code=getattr(e, 'code', 'io_error'))
            results.append(result)
            try:
                attempts=[int(path.stem.rsplit('-',1)[1]) for path in (run/'receipts').glob(f'publication-result-{i}-*.json')] if item['kind']=='publish' else []
                result_attempt=max(attempts,default=0)
                rp=run/f"result-{item['id']}-{result_attempt}.json"
                if not rp.exists():
                    write_new(rp, encode(result))
            except (WorkflowError, OSError):
                report_errors.append({'artifact': 'result-' + item['id'], 'code': 'io_error'})
        found_prepared=False
        for prepared_path in Path(ctx['binding']['state_root']).glob('*/prepared.json'):
            try:
                raw = read_regular(prepared_path, 8 * 1024 * 1024, private=True)
            except (WorkflowError,OSError):
                continue
            if sha(raw) == plan['prepared_sha256']:
                found_prepared=True
                material = load(prepared_path)
                chosen = {x['id'] for x in plan['items']}
                results.extend((dict(id=x['id'], status='pending', receipt_ids=[]) for x in material['items'] if x['id'] not in chosen))
                results.extend((dict(id='remaining' + str(j), status='pending', code=x['reason'], receipt_ids=[]) for (j, x) in enumerate(material['remaining'])))
                break
        if not found_prepared:
            results.append(dict(id='prepared',status='needs_review',code='plan_missing',receipt_ids=[]))
        summary = report(plan['workflow'], plan['plan_id'], results)
        if report_errors:
            summary['report_errors']=list(report_errors)
            summary['status']='partial' if any(r['status']=='ok' for r in results) else 'failed'
        attempt = 0
        while (run / f'summary-{attempt}.json').exists():
            attempt += 1
        errors = list(report_errors)
        try:
            from workflow_trigger import retain
            retain(run)
        except (WorkflowError,OSError):
            errors.append(dict(artifact='trigger',code='io_error'))
        for (name, raw) in [(f'summary-{attempt}.json', encode(summary))]:
            try:
                write_new(run / name, raw)
            except (WorkflowError, OSError):
                errors.append({'artifact': 'summary', 'code': 'io_error'})
        try:
            files = []
            for p in sorted(run.rglob('*')):
                if p.is_file() and (not p.name.startswith('inventory-')):
                    v = artifact_ref(run, str(p.relative_to(run)))
                    v['mode'] = '0600'
                    files.append(v)
            write_new(run / f'inventory-{attempt}.json', encode(dict(schema_version=1, files=files)))
        except (WorkflowError, OSError):
            errors.append({'artifact': 'inventory', 'code': 'io_error'})
        if errors:
            summary['report_errors'] = errors
            summary['status'] = 'partial' if any((r['status'] == 'ok' for r in results)) else 'failed'
        return summary

def publication(ctx, run, plan, item, i, result):
    for prior_receipt in sorted((run / 'receipts').glob(f'publication-result-{i}-*.json')):
        if load(prior_receipt)['response']['result']['outcome'] == 'conflict':
            fail('stale_snapshot')
    for pending_path in sorted((run / 'requests').glob(f'publication-result-{i}-*.json')):
        if not (run / 'receipts' / pending_path.name).exists():
            old = load(pending_path)
            old_attempt = int(pending_path.stem.rsplit('-', 1)[1])
            replay = staged(ctx, run, plan, 'publication-result', i, old, old_attempt)['result']
            if replay['outcome'] == 'conflict':
                fail('stale_snapshot')
            if replay['outcome'] == 'applied':
                result.update(status='ok', receipt_ids=[replay['approval_id'], replay['attempt_id']])
                return
    target = next((t['path'] for t in ctx['binding']['targets'] if t['target_id'] == item['target_id']))
    rendered = resolve_ref(run, item['rendered'], ctx['binding']['state_root'])
    ph = sha(canonical(plan))
    proposal = {k: item[k] for k in ('target_id', 'block_id', 'base_exists', 'base_sha256', 'after_sha256', 'output_sha256')}
    proposal['plan_sha256'] = ph
    body = envelope(plan, 'consumer-approve', i)
    body.update(decision_id=item['decision_id'], decision_sha256=item['decision_sha256'], proposal=proposal)
    if item['purpose'] == 'remove':
        body.update(maintenance='remove', confirmation='confirmed', previous_approval_id=item['previous_approval_id'], reason=item['reason'], prior_block_sha256=item['prior_block_sha256'])
    elif item['purpose'] == 'neutralize':
        body['maintenance'] = 'neutralize'
    else:
        body['confirmation'] = 'confirmed'
    approval = staged(ctx, run, plan, 'consumer-approve', i, body)['result']
    aid = approval['approval_id']
    result['receipt_ids'] = [aid]
    # Replayed approval responses preserve their original validity. Consult the
    # ledger before any new file effect, including when local receipts survived.
    publications = status_all(ctx, 'publications')
    live = next((p for p in publications if p['approval_id'] == aid), None)
    if live is None:
        fail('stale_snapshot')
    if live['publication'] == 'applied':
        result.update(status='ok', receipt_ids=[aid] + [a['attempt_id'] for a in live['attempts']])
        return
    if live['publication'] == 'conflict':
        fail('stale_snapshot')
    current = target_snapshot(target, item['block_id'])
    after = current['exists'] and current['sha256'] == item['after_sha256']
    outcome = 'applied'
    error = None
    if not after and (current['exists'] != item['base_exists'] or current['sha256'] != item['base_sha256']):
        outcome = 'conflict'
        error = 'baseline_changed'
    elif not after:
        bounded = {**plan['snapshot'], 'targets': [t for t in plan['snapshot']['targets'] if t['target_id'] == item['target_id']]}
        check_snap(ctx, bounded)
        # A maintenance approval records the validity of the decision it
        # neutralises, so it is stale by construction; only a substantive
        # publication requires a current approval.
        allowed = ('current', 'stale', 'retracted') if item['purpose'] == 'remove' else ('stale', 'retracted') if item['purpose'] == 'neutralize' else ('current',)
        decisions = status_all(ctx, 'decisions')
        decision = next((d for d in decisions if d['decision_id'] == item['decision_id']), None)
        if (live['validity'] not in allowed or decision is None
                or decision['validity'] not in allowed
                or decision['decision_sha256'] != item['decision_sha256']):
            fail('stale_snapshot')
        if item['purpose'] == 'remove':
            from workflow_removal import validate_owner
            validate_owner(item['previous_approval_id'], decisions, publications)
            base = read_regular(target, 2 * 1024 * 1024)
            if remove_block(base, item['block_id'], item['prior_block_sha256']) != rendered:
                fail('stale_snapshot')
        elif item['purpose'] == 'neutralize':
            if block_generations(decisions, publications).get(item['block_id'], 0) > decision['generation']:
                fail('stale_snapshot')
        elif decision['proposed_decision'] != 'proposed':
            fail('stale_snapshot')
        try:
            publish(target, item['base_exists'], item['base_sha256'], rendered, item['after_sha256'])
        except WorkflowError as e:
            outcome = 'conflict' if e.code in ('stale_snapshot', 'target_conflict') else 'failed'
            error = 'baseline_changed' if outcome == 'conflict' else 'io_error'
    observed = target_snapshot(target)
    attempt = 0
    while (run / 'receipts' / f'publication-result-{i}-{attempt}.json').exists():
        previous = load(run / 'receipts' / f'publication-result-{i}-{attempt}.json')['response']['result']
        if previous['outcome'] == 'applied':
            result.update(status='ok')
            return
        attempt += 1
    request = envelope(plan, 'publication-result', i, attempt)
    request.update(approval_id=aid, outcome=outcome, observed_at=plan['created_at'])
    if observed['exists']:
        request['observed_sha256'] = observed['sha256']
    if error:
        request['error_code'] = error
    pending = run / 'requests' / f'publication-result-{i}-{attempt}.json'
    if pending.exists():
        request = load(pending)
    r = staged(ctx, run, plan, 'publication-result', i, request, attempt)
    result.update(status='ok' if r['result']['outcome'] == 'applied' else 'failed', receipt_ids=[aid, r['result']['attempt_id']])
    if error:
        result['code'] = error

def execute(a):
    ctx = load_context(a.binding)
    if Path(__file__).parent != Path(ctx['binding']['package']):
        fail('stale_snapshot')
    if a.command == 'prepare':
        return prepare(a, ctx)
    if a.command == 'preview':
        return preview(a, ctx)
    if a.command == 'apply':
        return apply(a, ctx)
    header(ctx)
    if a.plan:
        plan = load(a.plan)
        try:
            check_snap(ctx, plan['snapshot'])
            state = 'noop'
            code = None
        except WorkflowError as e:
            state = 'needs_review'
            code = e.code
        item = dict(id='plan', status=state, receipt_ids=[])
        if code:
            item['code'] = code
        return report(plan['workflow'], plan['plan_id'], [item])
    pending = status_all(ctx, 'publications')
    return report('status', str(uuid.uuid4()), [dict(id=x['approval_id'], status='ok' if x['publication'] == 'applied' and x['validity'] == 'current' else 'needs_review', receipt_ids=[]) for x in pending])
