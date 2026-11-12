#!/usr/bin/env python3
"""Refresh one explicitly paired task using two owner-configured relay commands."""
import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import fcntl
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
from threading import Event
import time


def call(endpoint, action, body):
    child = subprocess.run([endpoint, action], input=json.dumps(body).encode(),
                           capture_output=True, timeout=30)
    if child.returncode:
        try:
            error = json.loads(child.stdout)
        except (ValueError, UnicodeDecodeError):
            raise ConnectionError(f'{action}: endpoint exited {child.returncode}') from None
        raise ValueError(f'{action}: {error.get("code", "invalid relay response")}')
    reply = json.loads(child.stdout)
    if reply['status'] != 'ok' or reply['schema_version'] != 1:
        raise ValueError(f'invalid {action} response')
    return reply['result']


def transfer(sender, receiver, offer):
    round_id = offer['body']['round']
    before = call(sender, 'status', {})
    if before.get('round') != round_id:
        raise InterruptedError('refresh round changed')
    after = before['outgoing']['through']
    page = {'offer': offer}
    previous = None
    while True:
        receipt = call(receiver, 'receive-page', page)['receipt']
        state = call(sender, 'ack', receipt)
        if state['round'] != round_id: raise InterruptedError('refresh round changed')
        if state['outgoing']['covered']:
            return {'after': after, 'through': offer['body']['upper']}
        through = state['outgoing']['through']
        if previous is not None and through <= previous: raise ValueError('no contiguous progress')
        previous = through
        page = call(sender, 'page', {'round': round_id})
        if not page.get('batch'): raise ValueError('no batch available for remaining range')


def run_refresh(local, peer, reason, observed=None):
    # Serialize a round with a handoff launched by another process on this coordinator.
    state = observed or call(local, 'status', {})
    pair = json.dumps([state['task_id'], state['epoch']]).encode()
    name = f'memorynexus-relay-{os.getuid()}-{hashlib.sha256(pair).hexdigest()}.lock'
    path = Path(tempfile.gettempdir()) / name
    fd = os.open(path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
            raise ValueError('unsafe relay lock')
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | (fcntl.LOCK_NB if observed else 0))
        except BlockingIOError:
            time.sleep(0.2)
            return
        if observed:
            source = local if reason['side'] == 'local' else peer
            current = call(source, 'status', {})
            if (current['task_id'], current['epoch'], current['outgoing']['through']) != (
                    state['task_id'], state['epoch'], state['outgoing']['through']):
                return
        refresh(local, peer, reason)
    finally:
        os.close(fd)


def refresh(local_endpoint, peer_endpoint, reason):
    # Freeze and send the stream that reached its threshold first.
    source = reason.get('side', 'local')
    other = 'peer' if source == 'local' else 'local'
    endpoints = {'local': local_endpoint, 'peer': peer_endpoint}
    sender, receiver = endpoints[source], endpoints[other]
    # Probe a previously unreachable endpoint before opening another round.
    route = call(receiver, 'status', {}) if reason['reason'] == 'reconnect' else None
    offer = call(sender, 'refresh', {})
    target = offer['body']
    round_id = target['round']
    if route is None:
        route = call(receiver, 'status', {})
    if (route['task_id'], route['epoch'], route['device'], route['peer']) != (
            target['task_id'], target['epoch'], target['destination'], target['origin']):
        raise ValueError('peer belongs to a different paired task')
    try:
        reply = call(receiver, 'refresh', {'round': round_id})
        ranges = {source: transfer(sender, receiver, offer),
                  other: transfer(receiver, sender, reply)}
        states = [call(endpoint, 'status', {}) for endpoint in (local_endpoint, peer_endpoint)]
        if any(s.get('round') != round_id for s in states):
            raise InterruptedError('refresh round changed')
        if not all(s['outgoing']['covered'] and s['incoming']['covered'] for s in states):
            raise ValueError('refresh incomplete')
    except ValueError:
        # A manual refresh can supersede an automatic round between any two calls.
        if any(call(endpoint, 'status', {}).get('round') != round_id
               for endpoint in (local_endpoint, peer_endpoint)):
            raise InterruptedError('refresh round changed') from None
        raise
    print(json.dumps({'round': round_id, 'local': states[0], 'peer': states[1],
                      'trigger': reason, 'targets': ranges}), flush=True)


def wait_for_batch(endpoint, side, stop, reconnect):
    while not stop.is_set():
        state = call(endpoint, 'status', {'include_batch_age': True})
        outgoing = state['outgoing']
        count, age = outgoing['pending_count'], outgoing['oldest_age_ms']
        delay = 0.2
        if count:
            if reconnect:
                reason = 'reconnect'
            elif count >= 8:
                reason = 'count_and_age' if age is not None and age >= 2000 else 'count'
            elif age is None:
                reason = 'legacy_backlog'
            elif age >= 2000:
                reason = 'age'
            else:
                stop.wait(min(delay, (2000 - age) / 1000))
                continue
            return {'reason': reason, 'side': side, 'pending_count': count, 'oldest_age_ms': age}, state
        reconnect = False
        stop.wait(delay)


def watch(local, peer):
    retry_delay = 0.25
    disconnected = False
    with ThreadPoolExecutor(max_workers=2) as pool:
        while True:
            stop = Event()
            probes = [pool.submit(wait_for_batch, endpoint, side, stop, disconnected)
                      for side, endpoint in (('local', local), ('peer', peer))]
            try:
                ready, _ = wait(probes, return_when=FIRST_COMPLETED)
                trigger, state = next(iter(ready)).result()
                stop.set()
                run_refresh(local, peer, trigger, state)
                retry_delay = 0.25
                disconnected = False
            except (ConnectionError, subprocess.TimeoutExpired) as error:
                stop.set()
                if not disconnected:
                    print(f'Peer unavailable: {error}', file=sys.stderr, flush=True)
                disconnected = True
                time.sleep(retry_delay)
                retry_delay = min(2, 2 * retry_delay)
            except InterruptedError:
                time.sleep(0.2)
            finally:
                stop.set()
                wait(probes)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for arg in ('--local', '--peer'):
        p.add_argument(arg, type=Path, required=True, help='absolute executable bound to one task relay')
    p.add_argument('--auto', action='store_true', help='watch this pair in the foreground and refresh due batches')
    args = p.parse_args()
    if not args.local.is_absolute() or not args.peer.is_absolute(): p.error('absolute paths required')
    if args.auto:
        watch(args.local, args.peer)
    else:
        run_refresh(args.local, args.peer, {'reason': 'manual'})
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        sys.exit(str(error))
