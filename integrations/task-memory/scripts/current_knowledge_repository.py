"""Read-only repository snapshot pinned across CLI pages, sources and publications."""
from pathlib import Path
import re

from current_knowledge_task import call, inside, sha, JSON_LIMIT, MATERIAL_LIMIT
from workflow_cli import package_inventory
from workflow_io import WorkflowError, read_regular, safe_path, _region
from workflow_scan import selected, row_text
from workflow_types import canonical, digest, fail, fields, array, hash_value, number, uuid, strict_loads, validate_binding, validate_profile


def repository_binding(b):
    if 'repository_binding' not in b: fail('scope_mismatch')
    # Common installation/config pins have already been verified by load_binding.
    # Reuse the workflow schemas and path rules, without its unbounded help probe;
    # all CLI calls from this reader use the bounded read-only transport.
    rb = validate_binding(strict_loads(read_regular(b['repository_binding'], 32768, True)))
    for key in ('task_id', 'task_root', 'config', 'ledger', 'binary', 'binary_sha256',
                'python', 'package', 'package_sha256'):
        if rb[key] != b[key]: fail('scope_mismatch')
    state = Path(b['state_root']); workflow_state = Path(rb['state_root'])
    if inside(state, workflow_state) or inside(workflow_state, state): fail('scope_mismatch')
    if any(inside(Path(p), state) for p in [b['repository_binding'], rb['profile'], *[t['path'] for t in rb['targets']]]):
        fail('scope_mismatch')
    safe_path(workflow_state, True)
    if not workflow_state.is_dir() or inside(workflow_state, Path(b['task_root'])): fail('scope_mismatch')
    profile = validate_profile(strict_loads(read_regular(rb['profile'], 32768, True)))
    if profile['workflow_id'] != rb['workflow_id']: fail('scope_mismatch')
    if {t['target_id'] for t in rb['targets']} != {c['target_id'] for c in profile['consumers']}: fail('scope_mismatch')
    for target in rb['targets']:
        path = safe_path(target['path'])
        if not path.parent.is_dir(): fail('io_error')
        if any(inside(path, Path(rb[k])) for k in ('config', 'ledger', 'binary', 'python', 'package', 'state_root', 'profile')):
            fail('scope_mismatch')
        for collection in profile['collections']:
            source = safe_path(Path(b['task_root']) / collection['path'])
            if source == path and collection['selector']['kind'] != 'marked': fail('scope_mismatch')
    rules = set()
    for item in package_inventory(b['package']):
        try:
            raw = read_regular(Path(b['package']) / item['path'], 32768)
            raw.decode('utf-8'); rules.add(sha(raw))
        except (WorkflowError, UnicodeError): pass
    if any(c['rules_sha256'] not in rules for c in profile['consumers']): fail('stale_snapshot')
    return dict(binding=rb, profile=profile, binding_sha256=digest(rb), profile_sha256=digest(profile))


class Snapshot:
    def __init__(self, b, ctx, header):
        self.b, self.ctx, self.header = b, ctx, header
        self.budget = [0]; self.files = {}; self.pages = {}

    def status(self, view, **extra):
        self.pages[view] = self.pages.get(view, 0) + 1
        if self.pages[view] > 128: fail('incomplete')
        response = call(self.b, 'workflow-status', dict(workflow_id=self.ctx['binding']['workflow_id'],
                        view=view, limit=20, at_revision=self.header['revision'], **extra), self.budget)
        if any(response[k] != self.header[k] for k in ('revision', 'incarnation')): fail('stale_snapshot')
        return response['result']

    def all(self, view):
        rows = []; after = 0
        while True:
            result = self.status(view, **({'after': after} if after else {}))
            rows.extend(result['items'])
            if result['complete']:
                if 'next_after' in result: fail('io_error')
                return rows
            cursor = result.get('next_after'); number(cursor, after + 1)
            if not result['items']: fail('io_error')
            after = cursor

    def read(self, path, limit, private=False, code='source_unavailable'):
        try: raw = read_regular(path, limit, private)
        except (OSError, WorkflowError): fail(code)
        path = Path(path)
        observed = (sha(raw), limit, private, code)
        if path in self.files and self.files[path][0] != observed[0]: fail('stale_snapshot')
        self.files[path] = observed
        return raw

    def recheck(self):
        for path, (value, limit, private, code) in list(self.files.items()):
            if sha(self.read(path, limit, private, code)) != value: fail('stale_snapshot')


def keyed(rows, key):
    result = {}
    for row in rows:
        if not isinstance(row, dict) or key not in row or row[key] in result: fail('io_error')
        result[row[key]] = row
    return result


def source_bytes(snap, source, collection, observed):
    if (not observed or not observed['active'] or observed['availability'] != 'available'
            or observed['format'] != collection['format'] or source['availability'] != 'available' or source['blocked']):
        fail('source_unavailable')
    loc = source['locator']
    if source['extractor_version'] != collection['extractor_version'] or loc['path'] != collection['path']:
        fail('source_drift')
    raw = snap.read(Path(snap.b['task_root']) / collection['path'],
                    (2 if collection['format'] == 'text' else 64) * 1024 * 1024)
    try:
        if collection['format'] == 'text':
            if loc != {'path': collection['path'], 'selector': collection['selector']}: fail('source_drift')
            actual = sha(selected(raw, collection['selector']))
        else:
            cursor = observed.get('cursor')
            if not cursor: fail('incomplete')
            offset = cursor['offset']
            if len(raw) < offset or sha(raw[:offset]) != cursor['prefix_sha256']: fail('source_drift')
            tail = raw[offset:]
            if b'\n' in tail: fail('source_drift')
            if tail: fail('incomplete')
            start, end = loc['start'], loc['end']
            if not 0 <= start < end <= offset or (start and raw[start-1:start] != b'\n') or raw[end-1:end] != b'\n':
                fail('source_drift')
            chunk = raw[start:end]
            if len(chunk) > 32768: fail('incomplete')
            row_text(chunk, collection['selector']); actual = sha(chunk)
        if actual != source['sha256']: fail('source_drift')
    except (ValueError, UnicodeError): fail('source_unavailable')
    except WorkflowError as exc:
        if exc.code == 'blocked_unit': fail('source_unavailable')
        raise


def dependencies(snap, decision, records, sources, collections, observed):
    deps = array(decision['dependencies'], 20, 1)
    if digest(deps) != decision['dependency_sha256']: fail('stale_snapshot')
    by_record = {}; by_source = {}
    for dep in deps:
        kind = dep.get('type')
        fields(dep, ('type', 'id', 'sha256') + (('revision',) if kind == 'source' else ()))
        uuid(dep['id']); hash_value(dep['sha256'])
        if kind not in ('record', 'source'): fail('io_error')
        index = by_record if kind == 'record' else by_source
        if dep['id'] in index: fail('io_error')
        index[dep['id']] = dep
    if decision['input_record_id'] not in by_record: fail('stale_snapshot')
    for rid, dep in by_record.items():
        if rid not in records:
            # inputs excludes lessons; explicit context history can inspect their
            # dependency bytes without adding their body to delivered material.
            response = call(snap.b, 'context', {'limit': 1, 'record_ids': [rid]}, snap.budget)
            if response['revision'] != snap.header['revision']: fail('stale_snapshot')
            value = response['result']; rows = value['entries'] + value['candidates']
            if value['truncated'] or len(rows) != 1 or rows[0]['id'] != rid: fail('incomplete')
            records[rid] = rows[0]
        record = records[rid]
        if record['state'] != 'current' or sha(record.get('text', '').encode()) != dep['sha256']: fail('stale_snapshot')
        if any(x not in by_record for x in record['source_ids']): fail('stale_snapshot')
        origin = record.get('origin')
        if origin and (origin['source_id'] not in by_source or by_source[origin['source_id']]['revision'] != origin['source_revision']):
            fail('stale_snapshot')
    for sid, dep in by_source.items():
        source = sources.get(sid)
        if source and any(r.get('origin', {}).get('source_id') == sid and r['origin']['extractor_version'] != source['extractor_version'] for r in (records[k] for k in by_record)):
            fail('source_drift')
        if not source: fail('source_unavailable')
        if source['source_revision'] != dep['revision'] or source['sha256'] != dep['sha256']: fail('source_drift')
        collection = collections.get(source['collection_id'])
        if not collection: fail('source_unavailable')
        source_bytes(snap, source, collection, observed.get(source['collection_id']))
    return [records[k] for k in sorted(by_record)], [sources[k] for k in sorted(by_source)]


def snapshot(binding_path, b, pins, req, read_id, created_at):
    from current_knowledge_artifacts import now
    from current_knowledge_task import load_binding
    from current_knowledge_publication import retained_output
    ctx = repository_binding(b); header = call(b, 'workflow-status', {})
    snap = Snapshot(b, ctx, header)
    views = {v: snap.all(v) for v in ('summary', 'collections', 'sources', 'inputs', 'decisions', 'publications')}
    workflow = {'workflow_id': ctx['binding']['workflow_id'], 'profile_revision': ctx['profile']['revision'],
                'profile_sha256': ctx['profile_sha256'], 'binding_sha256': ctx['binding_sha256']}
    if len(views['summary']) != 1: fail('scope_mismatch')
    summary = views['summary'][0]
    if summary['state'] != 'active' or any(summary.get(k) != v for k, v in workflow.items()): fail('stale_snapshot')
    consumers = keyed(ctx['profile']['consumers'], 'consumer_id')
    if any(cid not in consumers for cid in req['consumer_ids']): fail('scope_mismatch')
    targets = keyed(ctx['binding']['targets'], 'target_id')
    collections = keyed(ctx['profile']['collections'], 'collection_id')
    observed = keyed(views['collections'], 'collection_id'); sources = keyed(views['sources'], 'source_id')
    records = keyed(views['inputs'], 'id'); keyed(views['decisions'], 'decision_id'); keyed(views['publications'], 'approval_id')
    outputs = []; chosen_records = {}; chosen_sources = {}; output_evidence = []; bodies = {}
    for decision in sorted(views['decisions'], key=lambda x: (x['consumer_id'], x['decision_id'])):
        cid = decision['consumer_id']
        if cid not in req['consumer_ids'] or decision['validity'] != 'current': continue
        if decision['decision'] != 'approved': continue
        if any(p['decision_id'] == decision['decision_id'] and p.get('maintenance') == 'remove'
               and p['publication'] == 'applied' for p in views['publications']): continue
        if decision['profile_revision'] != workflow['profile_revision']: fail('stale_snapshot')
        pubs = [p for p in views['publications'] if p['decision_id'] == decision['decision_id'] and
                p['validity'] == 'current' and p['publication'] == 'applied' and not p.get('maintenance')]
        if not pubs: continue
        if len(pubs) != 1: fail('stale_snapshot')
        pub = pubs[0]; target_id = consumers[cid]['target_id']
        if pub['incarnation'] != header['incarnation'] or pub['profile_revision'] != workflow['profile_revision']: fail('stale_snapshot')
        inputs, used_sources = dependencies(snap, decision, records, sources, collections, observed)
        text = retained_output(snap, decision, pub, target_id)
        key = (target_id, decision['block_id'])
        if key in bodies: fail('stale_snapshot')
        bodies[key] = text.encode()
        outputs.append(dict(consumer_id=cid, decision_id=decision['decision_id'], approval_id=pub['approval_id'],
                       input_record_id=decision['input_record_id'], block_id=decision['block_id'], text=text,
                       classification='consumer_output', dependencies=decision['dependencies'],
                       inputs=[{k: row[k] for k in ('id', 'kind', 'trust', 'state')} for row in inputs]))
        chosen_records.update((r['id'], r) for r in inputs); chosen_sources.update((s['source_id'], s) for s in used_sources)
        output_evidence.append({**{k: outputs[-1][k] for k in ('consumer_id', 'decision_id', 'approval_id', 'block_id')},
                                'output_sha256': decision['output_sha256']})
    if len(outputs) > 20 or len(chosen_records) > 400 or len(chosen_sources) > 400: fail('incomplete')
    checked_at = now(); copies = []; gaps = []
    declared = {c['id']: c for c in b['copies']}
    for cid in req['consumer_ids']:
        tid = consumers[cid]['target_id']
        automatic = dict(id=tid, path=targets[tid]['path'], role='managed', consumer_id=cid)
        for item in declared.values():
            if (item['id'] == tid or item['path'] == automatic['path']) and item != automatic: fail('scope_mismatch')
        declared[tid] = automatic
    if len(declared) > 20: fail('incomplete')
    for item in sorted(declared.values(), key=lambda x: x['id']):
        row = dict(id=item['id'], status='unknown', checked_at=checked_at)
        if item['role'] == 'external':
            try: row['sha256'] = sha(snap.read(item['path'], 8*1024*1024))
            except WorkflowError: pass
            gaps.append('injection_unknown')
        else:
            cid = item['consumer_id']
            if cid not in req['consumer_ids'] or item['path'] != targets[consumers[cid]['target_id']]['path']: fail('scope_mismatch')
            raw = snap.read(item['path'], 64*1024*1024, code='target_drift'); tid = consumers[cid]['target_id']
            row['sha256'] = sha(raw); row['status'] = 'verified'
            expected = {block: body for (target, block), body in bodies.items() if target == tid}
            try:
                matches = list(re.finditer(rb'<!-- memorynexus:([a-z0-9][a-z0-9._-]{0,127}):(begin|end) -->', raw))
                if raw.count(b'<!-- memorynexus:') != len(matches) or len(matches) != 2 * len(expected): fail('target_drift')
                if {m[1].decode() for m in matches} != set(expected): fail('target_drift')
                for block, body in expected.items():
                    region = _region(raw, block)
                    if not region or region[2] != body: fail('target_drift')
            except WorkflowError:
                row['status'] = 'stale'; gaps.append('target_drift')
        copies.append(row)
    if any(not any(o['consumer_id'] == cid for o in outputs) for cid in req['consumer_ids']): gaps.append('empty')
    gaps = list(dict.fromkeys(gaps))
    material = dict(schema_version=1, read_id=read_id, task_id=b['task_id'], revision=header['revision'],
                    incarnation=header['incarnation'], mode='repository', purpose='current', current_use=True,
                    entries=[], candidates=[], outputs=outputs, gaps=gaps)
    evidence = dict(schema_version=1, read_id=read_id, created_at=created_at, checked_at=checked_at,
                    task_id=b['task_id'], revision=header['revision'], incarnation=header['incarnation'], **pins,
                    request=req, workflow=workflow,
                    selected=[{k: r[k] for k in ('id', 'kind', 'trust', 'state', 'revision')} for r in chosen_records.values()],
                    sources=[{**{k: s[k] for k in ('source_id', 'source_revision', 'sha256', 'availability', 'observed_at')},
                              'checked_at': checked_at} for _, s in sorted(chosen_sources.items())],
                    outputs=output_evidence, copies=copies, coverage='unknown' if 'injection_unknown' in gaps else 'declared_files',
                    omitted_count=0, candidate_count=0, truncated=False, gaps=gaps)
    if load_binding(binding_path)[1] != pins: fail('stale_snapshot')
    fresh = repository_binding(b)
    if fresh != ctx: fail('stale_snapshot')
    snap.recheck()
    final = call(b, 'workflow-status', {'at_revision': header['revision']})
    if any(final[k] != header[k] for k in ('revision', 'incarnation')): fail('stale_snapshot')
    raw = canonical(material) + b'\n'
    if len(raw) > MATERIAL_LIMIT: gaps.append('incomplete')
    if not gaps: evidence['material'] = dict(path='material.json', sha256=sha(raw), bytes=len(raw))
    if len(canonical(evidence))+1 > JSON_LIMIT: fail('incomplete')
    return raw, evidence
