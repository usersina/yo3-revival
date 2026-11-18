#!/usr/bin/env python3
"""Drive the Windows test VMs (libvirt): screen, input, guest commands and files. Standard library only.

    vm.py list                                  VMs from tools/vm/vms.toml: state, address, description
    vm.py start|shutdown|kill VM                power (shutdown asks the guest; kill pulls the plug)
    vm.py snapshots VM | snapshot VM NAME | revert VM NAME
    vm.py shot VM FILE.png                      screenshot (the host's pointer is never touched)
    vm.py size VM                               the guest screen's size
    vm.py click VM X Y [--right] [--double]     screen coordinates as in `shot`
    vm.py move VM X Y
    vm.py type VM TEXT                          US keyboard layout
    vm.py key VM KEY...                         one combination: `key win7 ctrl shift esc`, `key win7 meta_l r`
    vm.py run VM [--timeout S] -- COMMAND       a cmd.exe command in the guest: prints its output, exits with its code
    vm.py put VM FILE...                        copy into the shared folder; prints each guest path
    vm.py wait VM --changed | --pixel X Y RRGGBB [--timeout S]

`run` uses OpenSSH when the VM has `ssh` in vms.toml. Otherwise it writes a batch file to the shared folder and starts
it from the Run box (Win+R) in the signed-in session, so it needs an unlocked desktop and runs as that user, not
elevated (an elevated prompt can't see mapped drives: copy files to C: first). Input goes through QEMU's virtual
tablet and keyboard; only use it on VMs you were allowed to drive.
"""
import argparse
import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
USER_CONFIG = Path.home() / '.config/yo3-revival/vms.toml'
RUN_DIR = '.yo3-vm'


class Fail(Exception):
    pass


def load_config(paths=(HERE / 'vms.toml', USER_CONFIG)):
    defaults, vms = {}, {}
    for path in paths:
        if not path.is_file():
            continue
        data = tomllib.loads(path.read_text())
        defaults |= data.pop('defaults', {})
        for name, values in data.items():
            vms[name] = vms.get(name, {}) | values
    return {name: defaults | values | {'name': name} for name, values in vms.items()}


def vm_config(name, config=None):
    config = load_config() if config is None else config
    if name not in config:
        raise Fail(f'no VM {name!r} in tools/vm/vms.toml; known: {", ".join(sorted(config))}')
    return config[name]


def virsh(vm, *args, capture=True, check=True):
    result = subprocess.run(['virsh', '-c', vm['connect'], *args], capture_output=capture, text=True)
    if check and result.returncode:
        raise Fail(f'virsh {" ".join(args)}: {(result.stderr or result.stdout).strip()}')
    return result.stdout


def qmp(vm, command, arguments=None):
    message = {'execute': command} | ({'arguments': arguments} if arguments is not None else {})
    reply = json.loads(virsh(vm, 'qemu-monitor-command', vm['name'], json.dumps(message)))
    if 'error' in reply:
        raise Fail(f'{command}: {reply["error"].get("desc", reply["error"])}')
    return reply.get('return')


# --- PNG (what virsh writes): size and pixels without Pillow -------------------------------------------------------

def png_size(data):
    if data[:8] != b'\x89PNG\r\n\x1a\n' or data[12:16] != b'IHDR':
        raise Fail('not a PNG screenshot')
    return struct.unpack('>II', data[16:24])


def png_pixels(data):
    """(width, height, rows): rows of RGB bytes, for 8-bit RGB/RGBA, non-interlaced (what QEMU produces)."""
    width, height = png_size(data)
    depth, colour, interlace = data[24], data[25], data[28]
    if depth != 8 or colour not in (2, 6) or interlace:
        raise Fail('unsupported PNG layout')
    channels = 3 if colour == 2 else 4
    at, idat = 8, b''
    while at < len(data):
        length, kind = struct.unpack('>I4s', data[at:at + 8])
        if kind == b'IDAT':
            idat += data[at + 8:at + 8 + length]
        at += 12 + length
    raw, stride, rows, previous = zlib.decompress(idat), width * channels, [], bytearray(width * channels)
    for y in range(height):
        kind, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b, c = previous[x], previous[x - channels] if x >= channels else 0
            if kind == 1: line[x] = (line[x] + a) & 255
            elif kind == 2: line[x] = (line[x] + b) & 255
            elif kind == 3: line[x] = (line[x] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line) if channels == 3 else bytes(v for i, v in enumerate(line) if i % 4 != 3))
        previous = line
    return width, height, rows


def screenshot(vm, path=None):
    """The screen as PNG bytes; also written to `path` when given."""
    with tempfile.TemporaryDirectory() as tmp:
        target = Path(tmp) / 'shot.png'
        virsh(vm, 'screenshot', vm['name'], str(target))
        data = target.read_bytes()
    if path:
        Path(path).write_bytes(data)
    return data


def screen_size(vm):
    if 'screen' in vm:
        return tuple(vm['screen'])
    return png_size(screenshot(vm))


# --- input -----------------------------------------------------------------------------------------------------------

def tablet(x, y, size):
    width, height = size
    if not (0 <= x < width and 0 <= y < height):
        raise Fail(f'({x}, {y}) is outside the {width}x{height} screen')
    scale = lambda v, n: round(v * 32767 / max(n - 1, 1))
    return [{'type': 'abs', 'data': {'axis': 'x', 'value': scale(x, width)}},
            {'type': 'abs', 'data': {'axis': 'y', 'value': scale(y, height)}}]


def move(vm, x, y, size=None):
    qmp(vm, 'input-send-event', {'events': tablet(x, y, size or screen_size(vm))})


def click(vm, x, y, button='left', count=1, size=None):
    move(vm, x, y, size)
    time.sleep(0.15)
    for _ in range(count):
        for down in (True, False):
            qmp(vm, 'input-send-event', {'events': [{'type': 'btn', 'data': {'down': down, 'button': button}}]})
            time.sleep(0.06)


SHIFTED = {'!': '1', '@': '2', '#': '3', '$': '4', '%': '5', '^': '6', '&': '7', '*': '8', '(': '9', ')': '0',
           '_': 'minus', '+': 'equal', ':': 'semicolon', '"': 'apostrophe', '<': 'comma', '>': 'dot', '?': 'slash',
           '|': 'backslash', '{': 'bracket_left', '}': 'bracket_right', '~': 'grave_accent'}
PLAIN = {' ': 'spc', '-': 'minus', '=': 'equal', ';': 'semicolon', "'": 'apostrophe', ',': 'comma', '.': 'dot',
         '/': 'slash', '\\': 'backslash', '[': 'bracket_left', ']': 'bracket_right', '`': 'grave_accent',
         '\n': 'ret', '\t': 'tab'}
ALIASES = {'enter': 'ret', 'return': 'ret', 'win': 'meta_l', 'windows': 'meta_l', 'escape': 'esc', 'del': 'delete',
           'space': 'spc', 'backspace': 'backspace', 'pgup': 'pgup', 'pgdn': 'pgdn'}


def qcodes_for(char):
    """The key combination that types one character on a US layout."""
    if char.isascii() and char.isalpha():
        return ['shift', char.lower()] if char.isupper() else [char]
    if char.isascii() and char.isdigit():
        return [char]
    if char in SHIFTED:
        return ['shift', SHIFTED[char]]
    if char in PLAIN:
        return [PLAIN[char]]
    raise Fail(f'{char!r} has no key on a US layout; put the text in a file and `put` it instead')


def key_events(names):
    codes = [ALIASES.get(n.lower(), n.lower()) for n in names]
    press = lambda down, k: {'type': 'key', 'data': {'down': down, 'key': {'type': 'qcode', 'data': k}}}
    return [press(True, k) for k in codes] + [press(False, k) for k in reversed(codes)]


def keys(vm, *names):
    qmp(vm, 'input-send-event', {'events': key_events(names)})
    time.sleep(0.05)


def type_text(vm, text):
    combos = [qcodes_for(c) for c in text]  # refuse before typing anything
    for combo in combos:
        keys(vm, *combo)


# --- guest commands and files ----------------------------------------------------------------------------------------

def guest_path(vm, name):
    return vm['guest_share'].rstrip('\\') + '\\' + name.replace('/', '\\')


def batch(vm, ident, command):
    """The batch file `run` starts: the command's output and exit code land beside it in the shared folder. The command
    runs as a subroutine, so its own `exit /b N` returns N instead of ending the file before the code is written."""
    base = guest_path(vm, f'{RUN_DIR}/{ident}')
    return '\r\n'.join(['@echo off', 'cd /d "%USERPROFILE%"', f'call :run > "{base}.out" 2>&1',
                        f'echo %errorlevel% > "{base}.tmp"', f'move /y "{base}.tmp" "{base}.rc" > nul', 'exit',
                        ':run', command, 'exit /b %errorlevel%', ''])


def run(vm, command, timeout=120):
    if vm.get('ssh'):
        result = subprocess.run(['ssh', '-o', 'BatchMode=yes', vm['ssh'], command], timeout=timeout)
        return result.returncode
    share = Path(vm['share']) / RUN_DIR
    share.mkdir(parents=True, exist_ok=True)
    ident = uuid.uuid4().hex[:12]
    script, out, rc = share / f'{ident}.cmd', share / f'{ident}.out', share / f'{ident}.rc'
    # cmd.exe reads a batch file in the OEM code page; %NAME% expands variables, so a literal % is written %%.
    script.write_bytes(batch(vm, ident, command).encode('cp437'))
    # Through cmd /c: started directly from a network share, a script first asks "publisher could not be verified".
    launcher = f'cmd /c {guest_path(vm, f"{RUN_DIR}/{ident}.cmd")}'
    if len(launcher) > 250:
        raise Fail('the shared folder path is too long for the Run box')
    keys(vm, 'meta_l', 'r')
    time.sleep(1.5)
    type_text(vm, launcher)
    keys(vm, 'ret')
    deadline = time.monotonic() + timeout
    while not rc.exists():
        if time.monotonic() > deadline:
            raise Fail(f'no result after {timeout} s: is the desktop unlocked, and {launcher} reachable? '
                       f'(left in {share} for a look)')
        time.sleep(0.5)
    time.sleep(0.2)
    sys.stdout.write(out.read_bytes().decode('cp437', errors='replace').replace('\r\n', '\n'))
    code = int(rc.read_text(encoding='cp437').strip() or 1)
    for path in (script, out, rc):
        path.unlink(missing_ok=True)
    return code


def put(vm, files):
    share = Path(vm['share'])
    paths = []
    for file in map(Path, files):
        target = share / file.name
        (shutil.copytree if file.is_dir() else shutil.copy2)(file, target, **({'dirs_exist_ok': True} if file.is_dir() else {}))
        paths.append(guest_path(vm, file.name))
    return paths


def wait(vm, timeout, changed=False, pixel=None):
    first = hashlib.sha256(screenshot(vm)).digest() if changed else None
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        data = screenshot(vm)
        if changed and hashlib.sha256(data).digest() != first:
            return True
        if pixel:
            x, y, want = pixel
            _, _, rows = png_pixels(data)
            if rows[y][x * 3:x * 3 + 3].hex() == want.lower():
                return True
        time.sleep(1)
    return False


# --- command line ----------------------------------------------------------------------------------------------------

def main(argv):
    if '--' in argv:
        split = argv.index('--')
        argv, tail = argv[:split], argv[split + 1:]
    else:
        tail = []
    parser = argparse.ArgumentParser(prog='vm.py', description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('list')
    for name in ('start', 'shutdown', 'kill', 'snapshots', 'size'):
        sub.add_parser(name).add_argument('vm')
    for name in ('snapshot', 'revert'):
        p = sub.add_parser(name); p.add_argument('vm'); p.add_argument('snapshot')
    p = sub.add_parser('shot'); p.add_argument('vm'); p.add_argument('file')
    p = sub.add_parser('click'); p.add_argument('vm'); p.add_argument('x', type=int); p.add_argument('y', type=int)
    p.add_argument('--right', action='store_true'); p.add_argument('--double', action='store_true')
    p = sub.add_parser('move'); p.add_argument('vm'); p.add_argument('x', type=int); p.add_argument('y', type=int)
    p = sub.add_parser('type'); p.add_argument('vm'); p.add_argument('text')
    p = sub.add_parser('key'); p.add_argument('vm'); p.add_argument('keys', nargs='+')
    p = sub.add_parser('run'); p.add_argument('vm'); p.add_argument('--timeout', type=float, default=120)
    p = sub.add_parser('put'); p.add_argument('vm'); p.add_argument('files', nargs='+')
    p = sub.add_parser('wait'); p.add_argument('vm'); p.add_argument('--timeout', type=float, default=60)
    p.add_argument('--changed', action='store_true'); p.add_argument('--pixel', nargs=3, metavar=('X', 'Y', 'RRGGBB'))
    args = parser.parse_args(argv)
    try:
        if args.command == 'list':
            config = load_config()
            for name, vm in sorted(config.items()):
                state = virsh(vm, 'domstate', name, check=False).strip() or 'not defined'
                address = ''
                if state == 'running':
                    for line in virsh(vm, 'domifaddr', name, check=False).splitlines():
                        if 'ipv4' in line:
                            address = line.split()[-1].split('/')[0]
                print(f'{name:8} {state:12} {address:16} {vm.get("description", "")}')
            return 0
        vm = vm_config(args.vm)
        if args.command == 'start': virsh(vm, 'start', vm['name'])
        elif args.command == 'shutdown': virsh(vm, 'shutdown', vm['name'])
        elif args.command == 'kill': virsh(vm, 'destroy', vm['name'])
        elif args.command == 'snapshots': print(virsh(vm, 'snapshot-list', vm['name'], '--name').strip())
        elif args.command == 'snapshot': virsh(vm, 'snapshot-create-as', vm['name'], args.snapshot)
        elif args.command == 'revert': virsh(vm, 'snapshot-revert', vm['name'], args.snapshot)
        elif args.command == 'size': print('x'.join(map(str, screen_size(vm))))
        elif args.command == 'shot': screenshot(vm, args.file); print(args.file)
        elif args.command == 'click':
            click(vm, args.x, args.y, 'right' if args.right else 'left', 2 if args.double else 1)
        elif args.command == 'move': move(vm, args.x, args.y)
        elif args.command == 'type': type_text(vm, args.text)
        elif args.command == 'key': keys(vm, *args.keys)
        elif args.command == 'run':
            if not tail:
                raise Fail('give the command after --: vm.py run win7 -- ver')
            return run(vm, ' '.join(tail), args.timeout)
        elif args.command == 'put': print('\n'.join(put(vm, args.files)))
        elif args.command == 'wait':
            if not (args.changed or args.pixel):
                raise Fail('wait for --changed or --pixel X Y RRGGBB')
            pixel = (int(args.pixel[0]), int(args.pixel[1]), args.pixel[2]) if args.pixel else None
            return 0 if wait(vm, args.timeout, args.changed, pixel) else 1
        return 0
    except Fail as error:
        print(f'vm.py: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
