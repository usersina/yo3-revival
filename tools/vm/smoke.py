#!/usr/bin/env python3
"""The standard Windows check of a release build on a native VM (`task vm:smoke`): the Desktop bundle's own launcher and
connection DLL, against a scratch server on this PC, through the real game.

    smoke.py [--vm win11] [--bundle ZIP] [--game-dir GUEST_DIR] [--port 1214] [--out DIR] [--keep]

What it does, in order (each step's screenshot goes to --out):
  1. A scratch server the VM can reach: 192.168.122.1:PORT (PORT must be one the host's `libvirt` firewall zone lets
     through: 1214 by default; `firewall-cmd --zone=libvirt --list-ports`), one key account, the packet log on.
  2. The newest dist/yo3-revival-desktop-*-windows-x86_64.zip (`task release`; run `git fetch --tags` first so the
     bundle carries the right version), the account's profile and the server's certificate, copied into the guest and
     unpacked in C:\\YO3-smoke. Its yo3-launcher.exe starts the game with the bundle's DLL, as Scripts does.
  3. The title screen, ONLINE, the player: then, read from the server's packet log, the checks:
       login       the player was chosen (0x1F4C answered 0x2266) and the map opened
       auto-join   the chat entered the Lobby by itself (0x2136 room 1 answered 0x245A), no click
       chat        a typed line went out as room chat and was delivered (0x2134 with the line, 0x2456)
       command     `/p Server bug ...` was answered by the server's chat commands ([bug] in the log)
  4. The game closed, the profile deleted from the guest and the shared folder, the server stopped (--keep leaves them).

It drives the VM's virtual mouse and keyboard (never the host's): run it only on a VM you may drive. It assumes the VM
is signed in at its desktop and the game draws 1024x600 (the VM's 1280x800 screen); clicks are the game's own
coordinates placed in the centred window. Exit status 1 when a check fails.
"""
import argparse
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VM = [sys.executable, str(ROOT / 'tools/vm/vm.py')]
SCRATCH = [sys.executable, str(ROOT / 'tools/dev/scratch-server.py')]
HOST = '192.168.122.1'
GUEST = r'C:\YO3-smoke'
GAME = r'C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3'
# The game's 1024x600 layout: where to click, in its own coordinates.
TITLE, ONLINE, PLAYER, CHAT_INPUT = (512, 389), (297, 228), (657, 231), (642, 554)
WINDOW, TITLE_BAR = (1024, 600), 25


class Fail(Exception):
    pass


def newest_bundle(dist):
    """The newest Windows Desktop bundle in `dist`, or None."""
    bundles = sorted(Path(dist).glob('yo3-revival-desktop-*-windows-x86_64.zip'), key=lambda p: p.stat().st_mtime)
    return bundles[-1] if bundles else None


def client_origin(screen):
    """Where the game's 1024x600 client area sits on a screen of `screen` (width, height): Windows centres the window
    (the client plus its title bar)."""
    w, h = screen
    return (w - WINDOW[0]) // 2, (h - WINDOW[1] - TITLE_BAR) // 2 + TITLE_BAR


def at(origin, point):
    return origin[0] + point[0], origin[1] + point[1]


def play_bat(guest=GUEST, game=GAME):
    """The guest's batch file: the bundle's launcher with the profile, its injector and DLL, output to launcher.log."""
    b = guest + r'\yo3-revival-desktop'
    return '\r\n'.join(['@echo off', f'cd /d {guest}',
                        f'"{b}\\yo3-launcher.exe" --profile {guest}\\client-1.json --game-dir "{game}" '
                        f'--injector "{b}\\yo3-auth-launch.exe" --dll "{b}\\yo3-auth.dll" > {guest}\\launcher.log 2>&1', ''])


def checks(log, nonce):
    """Each check's result, from the server's log since the game started (its packet log and its own lines)."""
    return {
        'login': '"summary":"<- 0x1f4c {player_id=' in log and '"summary":"0x2266 {}"' in log,
        'auto-join': '"summary":"<- 0x2136 {room_id=1}"' in log and '"summary":"0x245a {room_id=1}"' in log,
        'chat': f'smoke {nonce}' in log and '"summary":"0x2456 {}"' in log,
        # `[bug] from NAME ... report=N` since the server's lines carry fields (docs/logs.md); `[bug] #N from NAME` before.
        'command': re.search(r'\[bug\] (#\d+ )?from \S+', log) is not None,
    }


def run(command, **kw):
    return subprocess.run(command, capture_output=True, text=True, **kw)


def vm(*args, timeout=120):
    r = run([*VM, *args], timeout=timeout)
    if r.returncode != 0:
        raise Fail(f'vm.py {" ".join(args)}: {r.stderr.strip() or r.stdout.strip()}')
    return r.stdout


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--vm', default='win11')
    p.add_argument('--bundle', type=Path, help='a Windows Desktop zip (default: the newest in dist/)')
    p.add_argument('--game-dir', default=GAME, help='the game folder in the guest')
    p.add_argument('--port', type=int, default=1214, help="the server's TLS port, one the libvirt zone allows")
    p.add_argument('--http', type=int, default=18080, help="the server's loopback HTTP port")
    p.add_argument('--out', type=Path, help='where the screenshots and the server log go (default: a temporary folder)')
    p.add_argument('--keep', action='store_true', help='leave the game, the server and the guest files when done')
    args = p.parse_args(argv)
    try:
        return smoke(args)
    except Fail as e:
        print(f'FAIL: {e}', file=sys.stderr)
        return 1


def smoke(args):
    bundle = args.bundle or newest_bundle(ROOT / 'dist')
    if not bundle or not bundle.is_file():
        raise Fail('no Windows Desktop bundle: run `git fetch --tags && task release` (or give --bundle)')
    head = run(['git', '-C', str(ROOT), 'rev-parse', '--short', 'HEAD']).stdout.strip()
    if head and head not in bundle.name:
        print(f'note: {bundle.name} was not built from HEAD ({head})')
    out = args.out or Path(tempfile.mkdtemp(prefix='yo3-vm-smoke.'))
    out.mkdir(parents=True, exist_ok=True)
    nonce = secrets.randbelow(10**6)
    shot = lambda name: vm('shot', args.vm, str(out / f'{name}.png'))

    state = vm('list')
    line = next((l for l in state.splitlines() if l.split()[:1] == [args.vm]), '')
    if 'running' not in line:
        print(f'starting {args.vm}…')
        vm('start', args.vm)
        time.sleep(90)
    screen = tuple(int(n) for n in re.findall(r'\d+', vm('size', args.vm))[:2])
    origin = client_origin(screen)

    print(f'scratch server on {HOST}:{args.port}…')
    started = run([*SCRATCH, 'start', '--accounts', '1', '--port', str(args.port), '--http', str(args.http),
                   '--host', HOST, '--', '--bind', HOST], timeout=120)
    found = re.search(r'Scratch server (\S+) \(pid (\d+)\)', started.stdout)
    if not found:
        raise Fail(f'the scratch server did not start: {started.stdout.strip()} {started.stderr.strip()}')
    server_dir, pid = Path(found.group(1)), int(found.group(2))
    share = Path(tempfile.mkdtemp(prefix='yo3-vm-smoke-files.'))
    results = {}
    try:
        for name, source in (('yo3-smoke-client-1.json', server_dir / 'client-1.json'), ('yo3-smoke-server.crt', server_dir / 'server.crt')):
            shutil.copy2(source, share / name)
        (share / 'yo3-smoke-play.bat').write_text(play_bat(game=args.game_dir), newline='')
        print(f'copying {bundle.name} into {args.vm}…')
        vm('put', args.vm, str(bundle), *(str(share / n) for n in ('yo3-smoke-client-1.json', 'yo3-smoke-server.crt', 'yo3-smoke-play.bat')), timeout=600)
        z = 'Z:'
        vm('run', args.vm, '--timeout', '300', '--',
           f'taskkill /f /im yo3.exe >nul 2>&1 & rmdir /s /q {GUEST} 2>nul & '
           # Windows' own tar unpacks the bundle's 20000-odd files in seconds; Expand-Archive took over five minutes.
           f'mkdir {GUEST} & tar -xf {z}\\{bundle.name} -C {GUEST} & '
           f'copy /y {z}\\yo3-smoke-client-1.json {GUEST}\\client-1.json >nul & copy /y {z}\\yo3-smoke-server.crt {GUEST}\\server.crt >nul & '
           f'copy /y {z}\\yo3-smoke-play.bat {GUEST}\\play.bat >nul & echo unpacked', timeout=360)
        log_start = (server_dir / 'server.log').stat().st_size
        vm('run', args.vm, '--', f'start "" /min cmd /c {GUEST}\\play.bat & echo started')
        time.sleep(40)
        shot('1-title')
        for name, point, wait in (('2-menu', TITLE, 5), ('3-select-player', ONLINE, 10), ('4-map', PLAYER, 18)):
            vm('click', args.vm, *map(str, at(origin, point)))
            time.sleep(wait)
            shot(name)
        vm('click', args.vm, *map(str, at(origin, CHAT_INPUT)))
        vm('type', args.vm, f'vm smoke {nonce}')
        vm('key', args.vm, 'ret')
        time.sleep(3)
        vm('type', args.vm, f'/p Server bug vm smoke {nonce}, ignore')
        vm('key', args.vm, 'ret')
        time.sleep(4)
        shot('5-chat')
        with open(server_dir / 'server.log', errors='replace') as f:
            f.seek(log_start)
            log = f.read()
        results = checks(log, nonce)
    finally:
        shutil.copy2(server_dir / 'server.log', out / 'server.log')
        if not args.keep:
            try:
                vm('run', args.vm, '--', f'taskkill /f /im yo3-launcher.exe >nul 2>&1 & taskkill /f /im yo3.exe >nul 2>&1 & '
                   f'del /q {GUEST}\\client-1.json 2>nul & echo closed')
            except Fail as e:
                print(f'note: the guest was not cleaned: {e}')
            for name in ('yo3-smoke-client-1.json', 'yo3-smoke-server.crt', 'yo3-smoke-play.bat', bundle.name):
                (guest_share() / name).unlink(missing_ok=True)
            run([*SCRATCH, 'stop', str(server_dir)], timeout=60)
            subprocess.run(['kill', str(pid)], capture_output=True)
        shutil.rmtree(share, ignore_errors=True)
    for name, ok in results.items():
        print(f'{"PASS" if ok else "FAIL"}  {name}')
    print(f'screenshots and server log: {out}')
    return 0 if results and all(results.values()) else 1


def guest_share():
    """The host folder the guests mount (tools/vm/vms.toml `share`)."""
    text = (ROOT / 'tools/vm/vms.toml').read_text()
    override = Path.home() / '.config/yo3-revival/vms.toml'
    if override.is_file():
        text = override.read_text() + '\n' + text
    found = re.search(r'^share\s*=\s*"([^"]+)"', text, re.M)
    return Path(found.group(1) if found else '/data/shared')


if __name__ == '__main__':
    sys.exit(main())
