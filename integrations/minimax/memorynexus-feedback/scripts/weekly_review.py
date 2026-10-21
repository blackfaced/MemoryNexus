#!/usr/bin/env python3
"""Read-only ledger context and disposable weekly interpretation files."""

import argparse
from datetime import date, datetime, timedelta
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid
from zoneinfo import ZoneInfo

CLI = Path('/Users/mac/.local/bin/memorynexus-feedback-cli')
LEDGER = Path('/Users/mac/.local/share/memorynexus/dogfood-2026-08-30.sqlite')
REPORTS = LEDGER.parent / 'reflections'
ZONE = ZoneInfo('Asia/Shanghai')
KIND = 'memorynexus-weekly-interpretation-v1'


def digest(value):
    data = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))
    return hashlib.sha256(data.encode()).hexdigest()


def read_ledger():
    if not LEDGER.is_file():
        raise ValueError('Formal ledger is missing; no record was created.')
    replies = []
    for command in ('observation-history', 'review'):
        result = subprocess.run(
            [str(CLI), '--ledger', str(LEDGER), command],
            stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30,
        )
        if result.returncode:
            raise ValueError(f'Formal ledger {command} read failed.')
        reply = json.loads(result.stdout)
        if not isinstance(reply, dict) or reply.get('status') != 'ok':
            raise ValueError(f'Formal ledger {command} did not return ok.')
        replies.append(reply)
    history, review = replies
    for container, key in ((history, 'observations'), (review, 'experiments'),
                           (review, 'outcomes'), (review, 'evidence_gaps')):
        if not isinstance(container.get(key), list):
            raise ValueError(f'Formal ledger response is missing {key}.')
    return history, review


def local_date(value):
    stamp = datetime.fromisoformat(value.replace('Z', '+00:00'))
    if stamp.tzinfo is None:
        raise ValueError('Ledger timestamps must include an offset.')
    return stamp.astimezone(ZONE).date()


def period_for(start=None, end=None):
    today = datetime.now(ZONE).date()
    if bool(start) != bool(end):
        raise ValueError('Supply both start and end, or neither.')
    end_day = date.fromisoformat(end) if end else today - timedelta(days=1)
    start_day = date.fromisoformat(start) if start else end_day - timedelta(days=6)
    if start_day > end_day or end_day >= today:
        raise ValueError('Use an ordered range of complete days ending before today.')
    return {'start': start_day.isoformat(), 'end': end_day.isoformat(),
            'timezone': ZONE.key}


def in_period(record, start, end):
    return start <= local_date(record['occurred_at']) <= end


def evidence(history, review, period):
    start, end = (date.fromisoformat(period[key]) for key in ('start', 'end'))
    current = [r for r in history['observations']
               if r.get('is_current') is True and r.get('state') == 'current']
    current.sort(key=lambda r: (r['occurred_at'], r['id']))
    prior_start = start - timedelta(days=7)
    observations = [r for r in current if in_period(r, start, end)]
    previous = [r for r in current if in_period(r, prior_start, start - timedelta(days=1))]
    replaced = {r['supersedes_outcome_id'] for r in review['outcomes']
                if r.get('supersedes_outcome_id')}
    outcomes = [r for r in review['outcomes'] if r['id'] not in replaced]
    state = {'experiments': review['experiments'], 'outcomes': outcomes,
             'evidence_gaps': review['evidence_gaps']}
    all_sources = {r['id']: r for r in current + state['experiments'] + outcomes}
    data = {'period': period, 'observations': observations,
            'previous_observations': previous, 'review_state': state}
    data['evidence_fingerprint'] = digest(data)
    data['record_days'] = sorted({local_date(r['occurred_at']).isoformat()
                                  for r in observations})
    return data, all_sources


def observation_fingerprint(history, period):
    start, end = (date.fromisoformat(period[key]) for key in ('start', 'end'))
    # Include the comparison week: later backfills can change a prior interpretation.
    start -= timedelta(days=7)
    records = [r for r in history['observations']
               if r.get('is_current') is True and r.get('state') == 'current'
               and in_period(r, start, end)]
    return digest(sorted(records, key=lambda r: r['id']))


def previous_reading(period, all_sources, history):
    candidates = []
    if not REPORTS.exists():
        return {'status': 'none'}
    for path in REPORTS.glob('*.json'):
        try:
            report = json.loads(path.read_text())
            if (report.get('kind') == KIND and report.get('ledger') == str(LEDGER)
                    and report['period']['timezone'] == ZONE.key
                    and date.fromisoformat(report['period']['end'])
                    < date.fromisoformat(period['start'])):
                datetime.fromisoformat(report['generated_at'])
                candidates.append(report)
        except (ValueError, KeyError, TypeError, AttributeError, OSError):
            continue
    if not candidates:
        return {'status': 'none'}
    report = max(candidates, key=lambda r: (r['period']['end'], r['generated_at']))
    try:
        sources = report['sources']
        valid = bool(sources) and all(
            key in all_sources and digest(all_sources[key]) == value
            for key, value in sources.items())
        valid = valid and report['observation_fingerprint'] == observation_fingerprint(
            history, report['period'])
        if valid:
            if not all(isinstance(report.get(key), str) for key in ('markdown', 'question')):
                raise ValueError('Incomplete reading')
            return {'status': 'valid', 'authority': 'agent_interpretation_only',
                    'period': report['period'], 'markdown': report['markdown'],
                    'question': report['question'], 'source_ids': list(sources)}
    except (ValueError, KeyError, TypeError, AttributeError):
        pass
    return {'status': 'stale', 'period': report['period'],
            'reason': 'Source records changed or the saved reading is incomplete; reread the ledger.'}


def context(start=None, end=None):
    period = period_for(start, end)
    history, review = read_ledger()
    data, sources = evidence(history, review, period)
    data['previous_reading'] = previous_reading(period, sources, history)
    # Existing MiniMax conversations may cache a previously read Skill reference.
    # Return the installed rules with each fresh evidence read, from one source file.
    rules = Path(__file__).resolve().parent.parent / 'references' / 'weekly-perspectives.md'
    data['reading_rules'] = rules.read_text()
    return {'status': 'ok', 'authority': 'confirmed_ledger_evidence', **data}


def save_reading(draft):
    supplied = draft['period']
    if supplied.get('timezone') != ZONE.key:
        raise ValueError('Use the context period and Asia/Shanghai timezone.')
    period = period_for(supplied['start'], supplied['end'])
    history, review = read_ledger()
    data, sources = evidence(history, review, period)
    if not data['observations']:
        raise ValueError('No observations in this period; do not save an invented reading.')
    if draft.get('evidence_fingerprint') != data['evidence_fingerprint']:
        raise ValueError('Evidence changed; regenerate the reading from fresh context.')
    ids = draft['source_ids']
    eligible = {r['id'] for r in data['observations'] + data['previous_observations']
                + data['review_state']['experiments'] + data['review_state']['outcomes']}
    if (not isinstance(ids, list) or not ids or not all(isinstance(i, str) for i in ids)
            or len(ids) != len(set(ids)) or not set(ids) <= eligible):
        raise ValueError('Citations must be unique record IDs from this context.')
    if not set(ids) & {r['id'] for r in data['observations']}:
        raise ValueError('Cite at least one observation from the actual review period.')
    for field in ('markdown', 'question'):
        if not isinstance(draft.get(field), str) or not draft[field].strip():
            raise ValueError(f'A nonempty {field} is required.')
    if draft['question'].strip() not in draft['markdown']:
        raise ValueError('Include the same shared question in the reading.')
    now = datetime.now(ZONE)
    report = {'kind': KIND, 'authority': 'agent_interpretation_only',
              'ledger': str(LEDGER), 'period': period, 'generated_at': now.isoformat(),
              'strategies': ['curator-v1', 'life-editor-v1'],
              'evidence_fingerprint': data['evidence_fingerprint'],
              'observation_fingerprint': observation_fingerprint(history, period),
              'sources': {key: digest(sources[key]) for key in ids},
              'markdown': draft['markdown'].strip(), 'question': draft['question'].strip()}
    REPORTS.mkdir(mode=0o700, parents=True, exist_ok=True)
    filename = f"{period['start']}_{period['end']}_{now.strftime('%Y%m%dT%H%M%S%f')}_{uuid.uuid4().hex[:8]}.json"
    path = REPORTS / filename
    # Exclusive creation preserves old readings; data stays outside the repository.
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w') as stream:
        json.dump(report, stream, ensure_ascii=False, indent=2)
        stream.write('\n')
    markdown_path = path.with_suffix('.md')
    fd = os.open(markdown_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w') as stream:
        stream.write('> Agent 派生解读；事实以正式账本的当前有效记录为准。\n\n')
        stream.write(report['markdown'] + '\n\n')
        stream.write('来源记录 ID：\n\n')
        stream.write('\n'.join(f'- `{key}`' for key in ids) + '\n')
    return {'status': 'saved', 'authority': 'agent_interpretation_only',
            'path': str(path), 'markdown_path': str(markdown_path), 'ledger_written': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    read = commands.add_parser('context', help='Read recent confirmed diary evidence')
    read.add_argument('--start', help='Inclusive local date')
    read.add_argument('--end', help='Inclusive local date')
    commands.add_parser('save', help='Save a derived interpretation JSON from stdin')
    args = parser.parse_args()
    try:
        result = (context(args.start, args.end) if args.command == 'context'
                  else save_reading(json.load(sys.stdin)))
    except (ValueError, KeyError, TypeError, AttributeError, OSError, subprocess.TimeoutExpired) as error:
        print(json.dumps({'status': 'error', 'message': str(error)}, ensure_ascii=False))
        return 2
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == '__main__':
    sys.exit(main())
