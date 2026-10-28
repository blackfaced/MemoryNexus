"""Strict immutable local artifact validation before any effect."""
from workflow_types import fields, uuid, hash_value, number, string as _string, name, fail

def string(v, maximum):
    return _string(v, 1, maximum)

def arr(v, limit):
    if not isinstance(v, list) or len(v) > limit:
        fail()
    return v

def schema(v):
    if type(v) is not int or v != 1:
        fail('unsupported_version')

def dep(v):
    if not isinstance(v, dict):
        fail()
    fields(v, ('type', 'id', 'sha256'), ('revision',))
    uuid(v['id'])
    hash_value(v['sha256'])
    if v['type'] == 'source':
        number(v.get('revision'), 1)
    elif v['type'] != 'record' or 'revision' in v:
        fail()

def ref(v):
    from workflow_types import relative
    fields(v, ('path', 'sha256', 'bytes'))
    relative(v['path'])
    hash_value(v['sha256'])
    number(v['bytes'])

def prior(v):
    fields(v, ('run_dir', 'artifact'))
    string(v['run_dir'], 4096)
    ref(v['artifact'])

def scan(v):
    fields(v, ('collection_id', 'expected_scan', 'observed_at', 'availability', 'units'), ('cursor', 'reason'))
    uuid(v['collection_id'])
    number(v['expected_scan'])
    string(v['observed_at'], 64)
    if v['availability'] not in ('available', 'unavailable'):
        fail()
    for u in arr(v['units'], 20):
        fields(u, ('sha256',), ('start', 'end'))
        hash_value(u['sha256'])
        if 'start' in u:
            number(u['start'])
            number(u.get('end'), 1)
    if 'cursor' in v:
        fields(v['cursor'], ('offset', 'prefix_sha256'))
        number(v['cursor']['offset'])
        hash_value(v['cursor']['prefix_sha256'])
    if 'reason' in v and v['reason'] not in ('missing', 'unreadable', 'identity_uncertain'):
        fail()

def snapshot(v):
    fields(v, ('task_id', 'workflow_id', 'incarnation', 'profile_revision', 'profile_sha256', 'binding_sha256', 'ledger_revision', 'scans', 'dependencies', 'targets'))
    for k in ('task_id', 'workflow_id', 'incarnation'):
        uuid(v[k])
    for k in ('profile_sha256', 'binding_sha256'):
        hash_value(v[k])
    number(v['profile_revision'], 1)
    number(v['ledger_revision'])
    for x in arr(v['scans'], 30):
        scan(x)
    for x in arr(v['dependencies'], 600):
        dep(x)
    seen = set()
    for t in arr(v['targets'], 2):
        fields(t, ('target_id', 'exists', 'sha256'), ('prior_block_sha256',))
        name(t['target_id'])
        hash_value(t['sha256'])
        if type(t['exists']) is not bool or t['target_id'] in seen:
            fail()
        seen.add(t['target_id'])
        if 'prior_block_sha256' in t:
            hash_value(t['prior_block_sha256'])

def recovery(w):
    fields(w, ('id', 'kind', 'consumer_id', 'approval_id', 'original_plan', 'item_id', 'classification'), ('pending_request',))
    name(w['consumer_id'])
    uuid(w['approval_id'])
    prior(w['original_plan'])
    name(w['item_id'])
    if w['classification'] not in ('before', 'after', 'conflict', 'uncertain', 'restored'):
        fail()
    if 'pending_request' in w:
        prior(w['pending_request'])

def validate_plan(v):
    fields(v, ('schema_version', 'plan_id', 'created_at', 'workflow', 'client', 'prepared_sha256', 'snapshot', 'items', 'authorization'))
    schema(v['schema_version'])
    uuid(v['plan_id'])
    string(v['created_at'], 64)
    string(v['client'], 64)
    hash_value(v['prepared_sha256'])
    snapshot(v['snapshot'])
    if v['workflow'] not in ('capture', 'review', 'publish', 'recovery'):
        fail()
    auth = v['authorization']
    fields(auth, ('mode',), ('profile_revision',))
    if auth['mode'] == 'profile':
        number(auth.get('profile_revision'), 1)
    elif auth['mode'] != 'owner' or 'profile_revision' in auth:
        fail()
    seen = set()
    targets = set()
    for w in arr(v['items'], 20):
        name(w.get('id'))
        kind = w.get('kind')
        if w['id'] in seen or (kind != v['workflow'] and (not (kind == 'publish' and v['workflow'] == 'publish'))):
            fail()
        seen.add(w['id'])
        if kind == 'capture':
            fields(w, ('id', 'kind', 'source_id', 'source_revision', 'extractor_version', 'text', 'occurred_at'))
            uuid(w['source_id'])
            number(w['source_revision'], 1)
            name(w['extractor_version'])
            string(w['text'], 1000)
            string(w['occurred_at'], 64)
        elif kind == 'review':
            fields(w, ('id', 'kind', 'consumer_id', 'input_record_id', 'decision', 'reason', 'dependencies'), ('previous_decision_id', 'output_text'))
            name(w['consumer_id'])
            uuid(w['input_record_id'])
            string(w['reason'], 240)
            for d in arr(w['dependencies'], 20):
                dep(d)
            if w['decision'] == 'proposed':
                string(w.get('output_text'), 1001)
            elif w['decision'] not in ('skipped', 'deferred') or 'output_text' in w:
                fail()
            if 'previous_decision_id' in w:
                uuid(w['previous_decision_id'])
        elif kind == 'publish':
            fields(w, ('id', 'kind', 'consumer_id', 'decision_id', 'decision_sha256', 'target_id', 'block_id', 'base_exists', 'base_sha256', 'after_sha256', 'output_text', 'output_sha256', 'rendered', 'purpose'), ('previous_approval_id', 'reason', 'prior_block_sha256'))
            name(w['consumer_id'])
            uuid(w['decision_id'])
            name(w['target_id'])
            if w['purpose'] == 'remove':
                if w['output_text'] != '':
                    fail()
            else:
                string(w['output_text'], 1001)
            ref(w['rendered'])
            for k in ('decision_sha256', 'block_id', 'base_sha256', 'after_sha256', 'output_sha256'):
                hash_value(w[k])
            if type(w['base_exists']) is not bool or w['target_id'] in targets or w['purpose'] not in ('publish', 'neutralize', 'remove'):
                fail()
            targets.add(w['target_id'])
            if w['purpose'] == 'remove':
                uuid(w.get('previous_approval_id'))
                string(w.get('reason'), 240)
                hash_value(w.get('prior_block_sha256'))
            elif 'reason' in w or 'prior_block_sha256' in w:
                fail()
            if 'previous_approval_id' in w:
                uuid(w['previous_approval_id'])
        else:
            recovery(w)
    return v

def validate_prepared(v):
    fields(v, ('schema_version', 'prepared_id', 'created_at', 'workflow', 'client', 'snapshot', 'items', 'remaining', 'evidence'), ('remaining_count',))
    schema(v['schema_version'])
    uuid(v['prepared_id'])
    string(v['created_at'], 64)
    string(v['client'], 64)
    snapshot(v['snapshot'])
    if v['workflow'] not in ('capture', 'review', 'publish', 'recovery'):
        fail()
    for e in arr(v['evidence'], 600):
        fields(e, ('source_id', 'source_revision', 'sha256', 'artifact'))
        uuid(e['source_id'])
        number(e['source_revision'])
        hash_value(e['sha256'])
        ref(e['artifact'])
    seen = set()
    for w in arr(v['items'], 20):
        name(w.get('id'))
        if w['id'] in seen:
            fail()
        seen.add(w['id'])
        if w.get('kind') != v['workflow']:
            fail()
        if w['kind'] == 'capture':
            fields(w, ('id', 'kind', 'collection_id', 'source_id', 'source_revision', 'extractor_version', 'source_sha256', 'evidence_index'))
            uuid(w['collection_id'])
            uuid(w['source_id'])
            number(w['source_revision'], 1)
            name(w['extractor_version'])
            hash_value(w['source_sha256'])
            number(w['evidence_index'])
            if w['evidence_index'] >= len(v['evidence']):
                fail()
        elif w['kind'] == 'review':
            fields(w, ('id', 'kind', 'consumer_id', 'input_record_id', 'root_record_id', 'input_text', 'dependencies'), ('previous_decision_id', 'previous_validity'))
            name(w['consumer_id'])
            uuid(w['input_record_id'])
            uuid(w['root_record_id'])
            string(w['input_text'], 1000)
            for d in arr(w['dependencies'], 20):
                dep(d)
            if 'previous_decision_id' in w:
                uuid(w['previous_decision_id'])
            if 'previous_validity' in w and w['previous_validity'] != 'stale':
                fail()
        elif w['kind'] == 'publish':
            fields(w, ('id', 'kind', 'consumer_id', 'decision_id', 'decision_sha256', 'target_id', 'block_id', 'output', 'output_sha256', 'purpose'), ('previous_approval_id', 'reason', 'prior_block_sha256'))
            name(w['consumer_id'])
            uuid(w['decision_id'])
            name(w['target_id'])
            prior(w['output'])
            for k in ('decision_sha256', 'block_id', 'output_sha256'):
                hash_value(w[k])
            if w['purpose'] not in ('publish', 'neutralize', 'remove'):
                fail()
            if w['purpose'] == 'remove':
                uuid(w.get('previous_approval_id'))
                string(w.get('reason'), 240)
                hash_value(w.get('prior_block_sha256'))
            elif 'reason' in w or 'prior_block_sha256' in w:
                fail()
            if 'previous_approval_id' in w:
                uuid(w['previous_approval_id'])
        else:
            recovery(w)
    reasons = ('awaiting_extraction', 'awaiting_review', 'awaiting_approval', 'target_queued', 'partial_tail', 'source_unavailable', 'blocked_unit', 'plan_missing', 'restore_gate', 'unselected')
    for r in arr(v['remaining'], 20):
        fields(r, ('id', 'reason'))
        string(r['id'], 128)
        if r['reason'] not in reasons:
            fail()
    if 'remaining_count' in v:
        number(v['remaining_count'])
    return v
