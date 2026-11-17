#!/usr/bin/env python3
"""Plan a dashboard update against deployed state. No Docker build, SSH mutation or server compilation.

Unknown dashboard inputs and runtime changes defer delivery. Other components may have unreleased changes.
The caller reads state from the VPS; filenames from the last push are never the deployment baseline.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

COMMIT = re.compile(r'[0-9a-f]{40}')
VERSION = re.compile(r'v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)')
IMAGE = 'ghcr.io/usersina/yo3-revival-'
CONTRACT = ['dashboard/api/openapi.json', 'dashboard/src/api/generated/']
ALLOWED = {'dashboard/index.html', 'dashboard/vite.config.ts',
           'dashboard/package.json', 'dashboard/package-lock.json'}
RUNTIME = {'.dockerignore', 'compose.yaml', 'compose.yml', 'docker-compose.yml', 'docker-compose.yaml'}


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args], timeout=30)


def changed(repo, before, after, *paths):
    return [p.decode('utf-8', errors='surrogateescape') for p in
            git(repo, 'diff', '--name-only', '--no-renames', '-z', before, after, '--', *paths).split(b'\0') if p]


def image(value, service):
    if not isinstance(value, str) or not re.fullmatch(re.escape(IMAGE+service)+r'@sha256:[0-9a-f]{64}', value):
        raise ValueError(f'Expected this project\'s digest-pinned {service} image')
    return value


def deployed(state):
    if not isinstance(state, dict) or not VERSION.fullmatch(str(state.get('version', ''))):
        raise ValueError('Expected a successfully deployed release state')
    result = {key: state.get(key) for key in ['version', 'commit', 'server', 'dashboard']}
    result['dashboard_commit'] = state.get('dashboard_commit', result['commit'])
    result['released_dashboard'] = state.get('released_dashboard', result['dashboard'])
    for key in ['commit', 'dashboard_commit']:
        if not isinstance(result[key], str) or not COMMIT.fullmatch(result[key]):
            raise ValueError(f'Expected the full deployed {key}')
    image(result['server'], 'server')
    image(result['dashboard'], 'dashboard')
    image(result['released_dashboard'], 'dashboard')
    return result


def plan(repo, state, candidate):
    state = deployed(state)
    if not isinstance(candidate, str) or not COMMIT.fullmatch(candidate):
        raise ValueError('Expected the full candidate commit')
    release = git(repo, 'rev-parse', f'refs/tags/{state["version"]}^{{commit}}').decode().strip()
    if release != state['commit']:
        raise ValueError('The deployed release tag does not resolve to the recorded server commit')
    for baseline in [release, state['dashboard_commit']]:
        if subprocess.run(['git', '-C', str(repo), 'merge-base', '--is-ancestor', baseline, candidate],
                          timeout=30, check=False).returncode:
            raise ValueError('The deployed release and dashboard must be ancestors of the candidate')
    changes = changed(repo, state['dashboard_commit'], candidate)
    contract = changed(repo, release, candidate, *CONTRACT)
    blocked, frontend = [], []
    for path in changes:
        if path.startswith('dashboard/'):
            if path == 'dashboard/README.md':
                continue
            if path in ALLOWED or path.startswith(('dashboard/src/', 'dashboard/public/')):
                frontend.append(path)
                # Build inputs cannot redirect the frontend build outside its source tree.
                entries = git(repo, 'ls-tree', candidate, '--', path).decode().splitlines()
                if any(line.startswith(('120000 ', '160000 ')) for line in entries):
                    blocked.append(path)
            else:
                blocked.append(path)
        elif (path.startswith('deploy/') or path in RUNTIME
              or '/' not in path and (path.startswith('Dockerfile')
                  or path.startswith(('compose.', 'docker-compose.')) and path.endswith(('.yaml', '.yml')))):
            blocked.append(path)
    reasons = []
    if contract:
        reasons.append('API specification or generated client differs from the deployed release')
    if blocked:
        reasons.append('Unknown dashboard inputs or deployment changes require a full release')
    if not frontend:
        reasons.append('No frontend changes since the deployed dashboard')
    return {'eligible': not reasons, 'reasons': reasons, 'changes': frontend, 'blocked': sorted(set(blocked)),
            'contract_changes': contract, 'version': state['version'], 'commit': release,
            'server': state['server'], 'released_dashboard': state['released_dashboard'],
            'expected_dashboard': state['dashboard'], 'expected_dashboard_commit': state['dashboard_commit'],
            'dashboard_commit': candidate,
            'contract_sha256': hashlib.sha256(git(repo, 'show', f'{release}:dashboard/api/openapi.json')).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path.cwd())
    parser.add_argument('--state', type=Path, required=True)
    parser.add_argument('--candidate', required=True)
    args = parser.parse_args()
    print(json.dumps(plan(args.repo, json.loads(args.state.read_text()), args.candidate), indent=2))


if __name__ == '__main__':
    main()
