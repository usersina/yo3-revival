#!/usr/bin/env python3
"""The repository's checks, by group: the single list CI and `task check` both run.

    tools/check.py                  every group CI's release job runs, in its order
    tools/check.py py docs          some groups
    tools/check.py --list           the groups and their steps
    tools/check.py --fail-fast ...  stop at the first failure (default: run all, then report)
    tools/check.py mod              local only: the connection DLL's Wine tests (not in CI; see LOCAL_GROUPS)

A Python test under tools/ is either in PY (hermetic: CI runs it) or in OTHER with what it needs; test_check.py fails
on one that is in neither, so a new test can't be forgotten.
"""
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Hermetic Python tests: no images, game files, display or network. .github/workflows/release.yml runs them through
# `tools/check.py py`.
PY = [
    'tools/mod/test_fixture_layout.py',
    'tools/mod/test_winmm_exports.py',
    'tools/client/test_vclient.py',
    'tools/client/test_ui_scenario.py',
    'tools/monkey/test_duel_report.py',
    'tools/monkey/test_duel_replay.py',
    'tools/release/test_package_editions.py',
    'tools/release/test_release_notes.py',
    'tools/release/test_release_lifecycle.py',
    'tools/host/test_host_scripts.py',
    'tools/containers/test_config.py',
    'tools/containers/test_deployment.py',
    'tools/containers/test_dashboard_delivery.py',
    'tools/containers/test_dashboard_workflow.py',
    'tools/containers/test_local_cargo_cache.py',
    'tools/docs/test_docs_check.py',
    'tools/engine/test_coverage.py',
    'tools/vm/test_vm.py',
    'tools/dev/test_scratch_server.py',
    'tools/test_check.py',
]

# Tests that need something CI's release job doesn't have at that point, and where they run instead.
OTHER = {
    'tools/dev/test_scratch_server_live.py': 'built server and original game files; run explicitly on a development machine',
    'tools/containers/test_build_cache.py': 'Docker Buildx; containers.yml runs it before release image builds',
    'tools/containers/test_provisioning.py': 'Ansible; containers.yml runs it',
    'tools/containers/test_images.py': 'built images (--server, --dashboard); containers.yml and dashboard.yml',
    'tools/containers/test_dashboard_browser.py': 'a released server and a candidate dashboard image; dashboard.yml',
    'tools/containers/test_dashboard_deploy.py': 'images and a built dashboard/dist',
    'tools/containers/test_dashboard_runtime.py': 'a built dashboard image (YO3_TEST_DASHBOARD); containers.yml',
    'tools/containers/test_certificates.py': 'a built dashboard image',
    'tools/containers/test_stack.py': 'built images and the game files',
}

ACTIONLINT_IGNORE = r'^unexpected key "queue" for "concurrency" section\.'


def steps(group):
    """The (name, argv, cwd) steps of a group."""
    py = [sys.executable]
    if group == 'lint':
        return [('actionlint', ['actionlint', '-ignore', ACTIONLINT_IGNORE, *sorted(p.relative_to(ROOT).as_posix() for p in ROOT.glob('.github/workflows/*.yml'))], ROOT),
                ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'], ROOT)]
    if group == 'py':
        return [(Path(t).name, [*py, t], ROOT) for t in PY]
    if group == 'docs':
        return [('docs links', [*py, 'tools/docs/docs-check.py'], ROOT)]
    if group == 'rust':
        return [('nextest', ['cargo', 'nextest', 'run', '--workspace'], ROOT)]
    if group == 'dashboard':
        d = ROOT / 'dashboard'
        install = [] if (d / 'node_modules').is_dir() else [('npm ci', ['npm', 'ci'], d)]
        return install + [(n, ['npm', 'run', *a], d) for n, a in [
            ('api drift', ['api:check']), ('format', ['format:check']), ('lint', ['lint']),
            ('typecheck', ['typecheck']), ('unit tests', ['test']), ('build', ['build'])]]
    if group == 'e2e':
        d = ROOT / 'dashboard'
        return [('browser', ['npx', 'playwright', 'install', 'chromium', '--only-shell'], d),
                ('e2e', ['npm', 'run', 'test:e2e', '--', '--workers', '3'], d)]
    if group == 'mod':
        # Wine on a private display (:78 unless YO3_DISPLAY) and prefix; test-direct.sh needs YO3_WINEPREFIX set to a
        # fresh private prefix, YO3_GAME_DIR when that prefix has no game, and the release builds it names.
        return [('build', ['tools/mod/build.sh'], ROOT),
                ('dll', ['tools/mod/test-dll.sh'], ROOT),
                ('loader', ['tools/mod/test-loader.sh'], ROOT),
                ('direct start', ['tools/mod/test-direct.sh'], ROOT)]
    raise KeyError(group)


GROUPS = ['lint', 'py', 'docs', 'rust', 'dashboard', 'e2e']
# Groups run only when named: they need Wine, a private display or the game, which CI's release job doesn't have.
LOCAL_GROUPS = ['mod']
# Groups that compile the engine crate, which needs EDOPro's sources first (as CI fetches them before Clippy).
NEEDS_ENGINE = {'lint', 'rust'}
ENGINE = ('engine sources', ['engine/fetch.sh'], ROOT)


def plan(groups):
    """(group, step) in order, with the engine sources fetched once before the first group that compiles them."""
    out, fetched = [], False
    for group in groups:
        if group in NEEDS_ENGINE and not fetched:
            out.append((group, ENGINE))
            fetched = True
        out += [(group, step) for step in steps(group)]
    return out


def run(groups, fail_fast):
    results = []
    for group, (name, argv, cwd) in plan(groups):
        label = f'{group}: {name}'
        if shutil.which(argv[0]) is None and not Path(cwd, argv[0]).exists():
            # actionlint is provisioned by CI; elsewhere a missing tool is reported, not hidden.
            if argv[0] == 'actionlint' and not os.environ.get('CI'):
                results.append((label, 'skipped', 0.0, 'actionlint is not installed'))
                continue
            results.append((label, 'FAILED', 0.0, f'{argv[0]} is not installed'))
            if fail_fast:
                return results
            continue
        print(f'\n==> {label}', flush=True)
        start = time.monotonic()
        code = subprocess.run(argv, cwd=cwd).returncode
        results.append((label, 'ok' if code == 0 else 'FAILED', time.monotonic() - start, '' if code == 0 else f'exit {code}'))
        if code and fail_fast:
            return results
    return results


def main(argv):
    args = [a for a in argv if not a.startswith('--')]
    if '--help' in argv or '-h' in argv:
        print(__doc__)
        return 0
    if '--list' in argv:
        last = None
        for group, (name, command, cwd) in plan(GROUPS + LOCAL_GROUPS):
            if group != last:
                print(group + ('  (local only: run by name)' if group in LOCAL_GROUPS else ''))
                last = group
            where = '' if cwd == ROOT else f'  (in {cwd.relative_to(ROOT)})'
            print(f'  {name}: {" ".join(command[:6])}{" …" if len(command) > 6 else ""}{where}')
        return 0
    unknown = [a for a in args if a not in GROUPS + LOCAL_GROUPS]
    if unknown:
        print(f'unknown group {", ".join(unknown)}; the groups are {", ".join(GROUPS + LOCAL_GROUPS)}', file=sys.stderr)
        return 2
    results = run(args or GROUPS, '--fail-fast' in argv)
    print('\nSummary')
    for label, status, seconds, note in results:
        print(f'  {status:8} {label}{f" ({seconds:.0f} s)" if seconds >= 1 else ""}{f": {note}" if note else ""}')
    return 1 if any(status == 'FAILED' for _, status, _, _ in results) else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
