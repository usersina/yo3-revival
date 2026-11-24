#!/usr/bin/env python3
"""A throwaway server beside a live one: its own data, logs, ports and launcher profiles, in one command.

    scratch-server.py start [--accounts N] [--from-db FILE --profile-for WHO ...] [--port P] [--http P]
                            [--game-dir DIR] [--server BIN] [--host HOST] [-- EXTRA SERVER ARGS]
    scratch-server.py status [DIR]
    scratch-server.py env [DIR] [--account N | --profile-for WHO]   the export lines for a client terminal
    scratch-server.py stop [DIR]
    scratch-server.py dir [DIR]          the folder alone, for scripts: "$(scratch-server.py dir)"/server.log
    scratch-server.py admin [--dir DIR] ARGS...   a yo3-server admin command on this scratch server's data:
                                         `scratch-server.py admin set anime-cards on`, `... admin give-cards TestPlayer1 "Card"`

`start` makes /tmp/yo3-dev.XXXX (printed), creates the identity and TestPlayer1..N key accounts, writes each one's
profile (client-N.json, beside server.crt), starts the server in the background (server.log) and prints the exports
for `tools/client/vclient.sh`. With --from-db it starts from a copy of another server's server.db instead (WAL
included, read consistently) and writes a profile for each --profile-for player or User ID. It refuses ports that
are taken, so a live server on 7443/8080 is never touched. DIR defaults to the newest scratch server.
The server binary: --server, else $YO3_SERVER, else target/release/yo3-server (build it with `task build:dev`).
"""
import argparse
import json
import os
import shutil
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CURRENT = Path(tempfile.gettempdir()) / 'yo3-dev.current'


class Fail(Exception):
    pass


def port_free(port, host='127.0.0.1'):
    with socket.socket() as s:
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            s.bind((host, port))
        except OSError:
            return False
    return True


def bind_address(extra):
    """The address the server listens on: its `--bind` among the extra server arguments, else loopback."""
    for i, arg in enumerate(extra):
        if arg == '--bind' and i + 1 < len(extra):
            return extra[i + 1]
        if arg.startswith('--bind='):
            return arg.split('=', 1)[1]
    return '127.0.0.1'


def listening(port, host):
    """Whether something accepts connections on host:port (0.0.0.0 is checked on loopback)."""
    with socket.socket() as s:
        s.settimeout(0.5)
        return s.connect_ex(('127.0.0.1' if host in ('0.0.0.0', '') else host, port)) == 0


def profile_from_key_output(text):
    """The JSON profile in `yo3-server key` output, which prints a heading line before it."""
    start = text.find('{')
    if start < 0:
        raise Fail(f'no profile in the server output: {text.strip()[:200]}')
    profile = json.loads(text[start:text.rindex('}') + 1])
    if not {'certificate', 'key', 'server'} <= profile.keys():
        raise Fail('the server printed an incomplete profile')
    return profile


def copy_database(source, destination):
    """A consistent copy of a live or copied server.db (its WAL included), as one file."""
    source = Path(source)
    if not source.is_file():
        raise Fail(f'{source} does not exist')
    src = sqlite3.connect(f'file:{source}?mode=ro', uri=True)
    try:
        dst = sqlite3.connect(destination)
        with dst:
            src.backup(dst)
        dst.close()
    finally:
        src.close()


def server_binary(explicit=None):
    path = Path(explicit or os.environ.get('YO3_SERVER') or ROOT / 'target/release/yo3-server')
    if not path.is_file():
        raise Fail(f'no server at {path}: build it with `task build:dev`, or pass --server / set YO3_SERVER')
    return path


def admin(binary, data, *args, port=None):
    command = [str(binary), '--data-dir', str(data), *(['--auth-port', str(port)] if port else []), *args]
    result = subprocess.run(command, capture_output=True, text=True, stdin=subprocess.DEVNULL)
    if result.returncode:
        raise Fail(f'{" ".join(args[:2])}: {(result.stderr or result.stdout).strip()[-400:]}')
    return result.stdout


def state_of(directory):
    directory = Path(directory) if directory else (Path(CURRENT.read_text().strip()) if CURRENT.is_file() else None)
    if not directory or not (directory / 'state.json').is_file():
        raise Fail('no scratch server here; start one with `scratch-server.py start`')
    return directory, json.loads((directory / 'state.json').read_text())


def alive(pid, directory):
    """Whether `pid` is still this scratch server: its command line names this directory's data folder. Not its program
    file: a rebuild replaces target/release/yo3-server, and the running server's /proc/PID/exe then no longer matches
    (stop used to leave it running)."""
    try:
        args = Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
    except OSError:
        return False
    data = str(Path(directory) / 'data').encode()
    return any(a == b'--data-dir' and b == data for a, b in zip(args, args[1:]))


def admin_command(argv):
    """`admin [--dir DIR] ARGS...`: the server's own admin command with this scratch server's data folder, so no caller
    puts a data path together (a running server gets it through its admin API, a stopped one's files are changed)."""
    directory = None
    if argv[:1] == ['--dir'] and len(argv) > 1:
        directory, argv = argv[1], argv[2:]
    if not argv:
        print('scratch-server.py: admin [--dir DIR] ARGS... (a yo3-server admin command)', file=sys.stderr)
        return 2
    try:
        directory, state = state_of(directory)
    except Fail as error:
        print(f'scratch-server.py: {error}', file=sys.stderr)
        return 2
    return subprocess.run([state['binary'], *argv, '--data-dir', str(directory / 'data')]).returncode


def exports(directory, profile):
    return [f'export YO3_AUTH_PROFILE={directory / profile}', f'export YO3_SCRATCH={directory}']


def start(args, extra):
    binary = server_binary(args.server)
    bind = bind_address(extra)
    for port, host in ((args.port, bind), (args.http, '127.0.0.1')):
        if not port_free(port, host):
            raise Fail(f'port {port} is taken: pick others with --port/--http (never the live 7443/8080)')
    directory = Path(tempfile.mkdtemp(prefix='yo3-dev.'))
    data = directory / 'data'
    data.mkdir()
    if args.from_db:
        copy_database(args.from_db, data / 'server.db')
    host = args.host
    admin(binary, data, 'init-auth', 'localhost', '127.0.0.1', *([host] if host not in ('localhost', '127.0.0.1') else []))
    shutil.copy2(data / 'server.crt', directory / 'server.crt')
    profiles = []
    if args.from_db:
        for who in args.profile_for:
            name = f'client-{"".join(c if c.isalnum() else "_" for c in who)}.json'
            profile = profile_from_key_output(admin(binary, data, 'key', who, '--advertise', host, port=args.port))
            (directory / name).write_text(json.dumps(profile))
            profiles.append(name)
    else:
        for n in range(1, args.accounts + 1):
            admin(binary, data, 'add-account', f'TestPlayer{n}')
            admin(binary, data, 'profile', f'TestPlayer{n}', str(directory / f'client-{n}.json'), '--advertise', host,
                  port=args.port)
            profiles.append(f'client-{n}.json')
    command = [str(binary), '--auth-port', str(args.port), '--http-port', str(args.http), '--data-dir', str(data),
               '--log-dir', str(directory / 'logs'), *(['--game-dir', args.game_dir] if args.game_dir else []), *extra]
    with open(directory / 'server.log', 'ab') as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL,
                                   start_new_session=True)
    deadline = time.monotonic() + 30
    # Ready for the admin commands too: the server writes data/admin.json and opens its HTTP port (where they go)
    # after the TLS port; until then a command finds the data folder locked.
    while not (listening(args.port, bind) and (data / 'admin.json').is_file() and listening(args.http, '127.0.0.1')):
        if process.poll() is not None:
            raise Fail(f'the server exited ({process.returncode}); see {directory / "server.log"}')
        if time.monotonic() > deadline:
            process.terminate()
            raise Fail(f'the server was not ready on {args.port} and {args.http} within 30 s; see {directory / "server.log"}')
        time.sleep(0.2)
    state = {'pid': process.pid, 'binary': str(binary), 'port': args.port, 'http': args.http, 'profiles': profiles}
    (directory / 'state.json').write_text(json.dumps(state, indent=2))
    CURRENT.write_text(str(directory))
    print(f'Scratch server {directory} (pid {process.pid}): TLS {bind}:{args.port}, HTTP {args.http}')
    print(f'Profiles: {", ".join(profiles) or "none"}. In a client terminal:')
    print('\n'.join(exports(directory, profiles[0]) if profiles else [f'export YO3_SCRATCH={directory}']))
    return 0


def main(argv):
    if argv[:1] == ['admin']:
        return admin_command(argv[1:])
    extra = []
    if '--' in argv:
        split = argv.index('--')
        argv, extra = argv[:split], argv[split + 1:]
    parser = argparse.ArgumentParser(prog='scratch-server.py', description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest='command', required=True)
    p = sub.add_parser('start')
    p.add_argument('--accounts', type=int, default=2)
    p.add_argument('--from-db')
    p.add_argument('--profile-for', action='append', default=[])
    p.add_argument('--port', type=int, default=17443)
    p.add_argument('--http', type=int, default=18080)
    p.add_argument('--game-dir')
    p.add_argument('--server')
    p.add_argument('--host', default='localhost', help='the name or address a client dials (a VM: 192.168.122.1); also bound by --bind')
    for name in ('status', 'stop', 'dir'):
        sub.add_parser(name).add_argument('dir', nargs='?')
    p = sub.add_parser('env')
    p.add_argument('dir', nargs='?')
    p.add_argument('--account', type=int)
    p.add_argument('--profile-for')
    args = parser.parse_args(argv)
    try:
        if args.command == 'start':
            if args.profile_for and not args.from_db:
                raise Fail('--profile-for names players of a --from-db copy')
            if args.port in (7443, 8080) or args.http in (7443, 8080):
                raise Fail('7443 and 8080 are the live server\'s ports')
            return start(args, extra)
        directory, state = state_of(args.dir)
        running = alive(state['pid'], directory)
        if args.command == 'dir':
            print(directory)
            return 0
        if args.command == 'status':
            print(f'{directory}: {"running" if running else "stopped"} (pid {state["pid"]}), TLS {state["port"]}, '
                  f'HTTP {state["http"]}, profiles {", ".join(state["profiles"]) or "none"}')
            return 0 if running else 1
        if args.command == 'env':
            if args.profile_for:
                name = f'client-{"".join(c if c.isalnum() else "_" for c in args.profile_for)}.json'
            else:
                name = f'client-{args.account}.json' if args.account else (state['profiles'] or [None])[0]
            if not name or name not in state['profiles']:
                raise Fail(f'no such profile; this server has {", ".join(state["profiles"]) or "none"}')
            print('\n'.join(exports(directory, name)))
            return 0
        if args.command == 'stop':
            if running:
                os.kill(state['pid'], 15)  # the server's graceful stop
                for _ in range(100):
                    if not alive(state['pid'], directory):
                        break
                    time.sleep(0.1)
                else:
                    raise Fail(f'pid {state["pid"]} still runs 10 s after SIGTERM; see {directory / "server.log"}')
            print(f'stopped; its files stay in {directory}')
            return 0
    except Fail as error:
        print(f'scratch-server.py: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
