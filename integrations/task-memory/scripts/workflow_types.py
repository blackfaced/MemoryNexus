"""Strict JSON and values used by repository workflow operations."""
import hashlib
import json
import re
import uuid as uuid_module
from workflow_io import WorkflowError


def fail(code='invalid_input'):
    raise WorkflowError(code)


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'), allow_nan=False).encode('utf-8')


def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def number(v, minimum=0, maximum=2**53-1):
    if type(v) is not int or not minimum <= v <= maximum: fail()
    return v


def string(v, minimum=1, maximum=240):
    if not isinstance(v, str) or not minimum <= len(v) <= maximum or any((ord(c)<32 and c not in '\t\r\n') or 0xd800<=ord(c)<=0xdfff for c in v): fail()
    return v


def fields(v, required, optional=()):
    if not isinstance(v, dict) or set(v)-set(required)-set(optional) or set(required)-set(v): fail()
    return v


def strict_loads(data):
    def pairs(items):
        result={}
        for k,v in items:
            if k in result: fail()
            result[k]=v
        return result
    def bad(_): fail()
    try: v=json.loads(data, object_pairs_hook=pairs, parse_float=bad, parse_constant=bad)
    except (ValueError, UnicodeError, TypeError): fail()
    def check(x):
        if x is None: fail()
        if type(x) is int: number(x)
        if isinstance(x, str): string(x,0,2**53-1)
        if isinstance(x, dict):
            for k,y in x.items(): check(k); check(y)
        elif isinstance(x,list):
            for y in x: check(y)
    check(v)
    return v


def name(v):
    if not isinstance(v,str) or re.fullmatch('[a-z][a-z0-9_-]{0,63}',v) is None: fail()
    return v


def uuid(v):
    try:
        if not isinstance(v,str) or str(uuid_module.UUID(v))!=v: fail()
    except (ValueError, AttributeError): fail()
    return v


def hash_value(v):
    if not isinstance(v,str) or re.fullmatch('[0-9a-f]{64}',v) is None: fail()
    return v


def boolean(v):
    if type(v) is not bool: fail()
    return v


def array(v, maximum, minimum=0):
    if not isinstance(v,list) or not minimum<=len(v)<=maximum: fail()
    return v


def unique(v):
    if len(v)!=len(set(v)): fail()


def relative(v):
    string(v,1,4096)
    if v.startswith('/') or any(x in ('','.','..') for x in v.split('/')) or any(c in v for c in ':\\*?$`~'): fail()
    return v


def validate_profile(p):
    fields(p,('schema_version','workflow_id','revision','purpose','mechanisms','collections','capture','consumers','operation'))
    if type(p['schema_version']) is not int or p['schema_version']!=1: fail('unsupported_version')
    uuid(p['workflow_id']); number(p['revision'],1); string(p['purpose'])
    for m in array(p['mechanisms'],8):
        fields(m,('name','purpose','trigger','read_paths','write_paths','evidence')); name(m['name'])
        for k in ('purpose','trigger','evidence'): string(m[k])
        for k in ('read_paths','write_paths'):
            for x in array(m[k],20): relative(x)
    for c in array(p['collections'],30):
        fields(c,('collection_id','format','path','selector','extractor_version','authority'))
        uuid(c['collection_id']); relative(c['path']); name(c['extractor_version']); string(c['authority']); s=c['selector']
        if not isinstance(s,dict): fail()
        if c['format']=='append_jsonl':
            fields(s,('kind','keys'))
            if s['kind']!='fields': fail()
            for k in array(s['keys'],8,1): string(k,1,120)
            unique(s['keys'])
        elif c['format']=='text':
            if s.get('kind')=='full': fields(s,('kind',))
            elif s.get('kind')=='marked':
                fields(s,('kind','begin','end'))
                for k in ('begin','end'):
                    string(s[k],1,120)
                    if '\n' in s[k] or '\r' in s[k]: fail()
                if s['begin']==s['end']: fail()
            else: fail()
        else: fail()
    unique([x['collection_id'] for x in p['collections']])
    fields(p['capture'],('automatic_drafts','automatic_reviews'))
    for v in p['capture'].values(): boolean(v)
    for c in array(p['consumers'],2,2):
        fields(c,('consumer_id','lens_version','rules_sha256','target_id','neutralize_stale'))
        for k in ('consumer_id','lens_version','target_id'): name(c[k])
        hash_value(c['rules_sha256']); boolean(c['neutralize_stale'])
    for k in ('consumer_id','target_id'): unique([c[k] for c in p['consumers']])
    fields(p['operation'],('workflow_name','trigger')); name(p['operation']['workflow_name']); string(p['operation']['trigger'])
    return p


def validate_binding(b):
    fields(b,('schema_version','workflow_id','task_id','task_root','config','ledger','binary','binary_sha256','python','package','package_sha256','profile','state_root','targets','timeout_seconds'))
    if type(b['schema_version']) is not int or b['schema_version']!=1: fail('unsupported_version')
    uuid(b['workflow_id']); uuid(b['task_id']); hash_value(b['binary_sha256']); hash_value(b['package_sha256']); number(b['timeout_seconds'],1,300)
    for t in array(b['targets'],2,2): fields(t,('target_id','path')); name(t['target_id'])
    for k in ('target_id','path'): unique([t[k] for t in b['targets']])
    return b
