#!/usr/bin/env python3
"""Refresh one explicitly paired task using two owner-configured relay commands."""
import argparse
import json
from pathlib import Path
import subprocess
import sys


def call(endpoint, action, body):
    child = subprocess.run([endpoint, action], input=json.dumps(body).encode(),
                           capture_output=True, timeout=30)
    if child.returncode:
        raise ValueError(f'{action} failed (exit {child.returncode})')
    reply = json.loads(child.stdout)
    if reply['status'] != 'ok' or reply['schema_version'] != 1:
        raise ValueError(f'invalid {action} response')
    return reply['result']


def transfer(sender, receiver, offer):
    round_id = offer['body']['round']
    page = {'offer': offer}
    previous = None
    while True:
        receipt = call(receiver, 'receive-page', page)['receipt']
        state = call(sender, 'ack', receipt)
        if state['round'] != round_id: raise ValueError('refresh round changed')
        if state['outgoing']['covered']: return
        through = state['outgoing']['through']
        if previous is not None and through <= previous: raise ValueError('no contiguous progress')
        previous = through
        page = call(sender, 'page', {'round': round_id})
        if not page.get('batch'): raise ValueError('no batch available for remaining range')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for arg in ('--local', '--peer'):
        p.add_argument(arg, type=Path, required=True, help='absolute executable bound to one task relay')
    args = p.parse_args()
    if not args.local.is_absolute() or not args.peer.is_absolute(): p.error('absolute paths required')
    local = call(args.local, 'refresh', {})
    target = local['body']
    round_id = target['round']
    route = call(args.peer, 'status', {})
    if (route['task_id'], route['epoch'], route['device'], route['peer']) != (
            target['task_id'], target['epoch'], target['destination'], target['origin']):
        raise ValueError('peer belongs to a different paired task')
    peer = call(args.peer, 'refresh', {'round': round_id})
    for sender, receiver, offer in ((args.local, args.peer, local), (args.peer, args.local, peer)):
        transfer(sender, receiver, offer)
    states = [call(endpoint, 'status', {}) for endpoint in (args.local, args.peer)]
    if not all(s['round'] == round_id and s['outgoing']['covered'] and s['incoming']['covered'] for s in states):
        raise ValueError('refresh incomplete')
    print(json.dumps({'round': round_id, 'local': states[0], 'peer': states[1]}))
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        sys.exit(str(error))
