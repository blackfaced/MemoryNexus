"""Bounded source scans. Offsets and full-prefix proofs, never event timestamps."""
import json
import re
import uuid
from pathlib import Path
from workflow_io import read_regular, WorkflowError
from workflow import sha, now, canonical

def selected(raw, selector):
    text = raw.decode('utf-8')
    if any((ord(c) < 32 and c not in '\t\r\n' for c in text)) or re.search(r'data:[a-z0-9.+-]+/[a-z0-9.+-]+[;,]',text,re.I):
        raise WorkflowError('blocked_unit')
    if selector['kind'] == 'marked':
        lines = raw.splitlines(keepends=True)
        begins = [i for (i, l) in enumerate(lines) if l.rstrip(b'\r\n').decode() == selector['begin']]
        ends = [i for (i, l) in enumerate(lines) if l.rstrip(b'\r\n').decode() == selector['end']]
        if len(begins) != 1 or len(ends) != 1 or begins[0] >= ends[0]:
            raise WorkflowError('source_unavailable')
        raw = b''.join(lines[begins[0] + 1:ends[0]])
        text = raw.decode()
    if '<!-- memorynexus:' in text:
        raise WorkflowError('blocked_unit')
    return raw

def row_text(raw, selector):

    def obj(pairs):
        d = {}
        for (k, v) in pairs:
            if k in d:
                raise WorkflowError('blocked_unit')
            d[k] = v
        return d
    row = json.loads(raw, object_pairs_hook=obj)
    if not isinstance(row, dict):
        raise WorkflowError('blocked_unit')
    values = []
    for key in selector['keys']:
        if key in row:
            if not isinstance(row[key], str):
                raise WorkflowError('blocked_unit')
            values.append(row[key])
    if not values:
        raise WorkflowError('blocked_unit')
    data = '\n'.join(values).encode()
    selected(data, {'kind': 'full'})
    if len(data) > 8192:
        raise WorkflowError('blocked_unit')
    return data

def scans(ctx, collections, sources):
    b = ctx['binding']
    p = ctx['profile']
    byc = {x['collection_id']: x for x in collections}
    bys = {x['source_id']: x for x in sources}
    out = []
    units = []
    remaining = []
    for c in p['collections']:
        cid = c['collection_id']
        old = byc.get(cid, {})
        scan = dict(collection_id=cid, expected_scan=old.get('scan', 0), observed_at=now(), availability='available', units=[])
        known = [s for s in sources if s['collection_id'] == cid]
        cursor = old.get('cursor', {'offset': 0, 'prefix_sha256': sha(b'')})
        try:
            raw = read_regular(Path(b['task_root']) / c['path'], (2 if c['format'] == 'text' else 64) * 1024 * 1024)
            if c['format'] == 'text':
                data = selected(raw, c['selector'])
                digest = sha(data)
                scan['units'] = [{'sha256': digest}]
                oldsrc = bys.get(cid)
                rev = 1 if oldsrc is None else oldsrc['source_revision'] + (oldsrc['sha256'] != digest or oldsrc['availability'] != 'available' or oldsrc['extractor_version'] != c['extractor_version'] or (oldsrc['locator'] != {'path': c['path'], 'selector': c['selector']}))
                units.append((c, cid, rev, digest, data, oldsrc))
            else:
                offset = cursor['offset']
                if len(raw) < offset or sha(raw[:offset]) != cursor['prefix_sha256']:
                    raise WorkflowError('stale_snapshot')
                for s in known:
                    loc = s['locator']
                    chunk = raw[loc['start']:loc['end']]
                    if sha(chunk) != s['sha256']:
                        raise WorkflowError('stale_snapshot')
                    data = row_text(chunk, c['selector'])
                    rev = s['source_revision'] + (s['availability'] != 'available' or s['extractor_version'] != c['extractor_version'] or loc['path'] != c['path'])
                    units.append((c, s['source_id'], rev, sha(chunk), data, s))
                pos = offset
                count = 0
                while pos < len(raw):
                    end = raw.find(b'\n', pos)
                    if end < 0:
                        remaining.append({'id': cid, 'reason': 'partial_tail'})
                        break
                    end += 1
                    chunk = raw[pos:end]
                    if count >= 20:
                        remaining.append({'id': cid, 'reason': 'awaiting_extraction'})
                        break
                    try:
                        if len(chunk) > 32768:
                            raise WorkflowError('blocked_unit')
                        data = row_text(chunk, c['selector'])
                    except (WorkflowError, ValueError, UnicodeError):
                        remaining.append({'id': cid, 'reason': 'blocked_unit'})
                        break
                    sid = str(uuid.uuid5(uuid.UUID(cid), str(pos)))
                    scan['units'].append({'start': pos, 'end': end, 'sha256': sha(chunk)})
                    units.append((c, sid, 1, sha(chunk), data, None))
                    pos = end
                    count += 1
                scan['cursor'] = {'offset': pos, 'prefix_sha256': sha(raw[:pos])}
        except FileNotFoundError:
            scan.update(availability='unavailable', units=[], reason='missing')
            remaining.append({'id': cid, 'reason': 'source_unavailable'})
        except (OSError, UnicodeError, ValueError) as e:
            scan.update(availability='unavailable', units=[], reason='unreadable')
            remaining.append({'id': cid, 'reason': 'source_unavailable'})
        except WorkflowError as e:
            if e.code == 'stale_snapshot':
                raise
            scan.update(availability='unavailable', units=[], reason='unreadable')
            scan.pop('cursor', None)
            remaining.append({'id': cid, 'reason': 'blocked_unit'})
        out.append(scan)
    return (out, units, remaining)
