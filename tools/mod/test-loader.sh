#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export DISPLAY="${YO3_DISPLAY:-:78}"
unset WAYLAND_DISPLAY
export WINEPREFIX="${YO3_WINEPREFIX:-$ROOT/.wine-b}"
export WINEDEBUG=-all
mkdir -p "$ROOT/tools/mod/build"
i686-w64-mingw32-gcc -Wall -Wextra -Werror -O2 -o "$ROOT/tools/mod/build/loader-test.exe" "$ROOT/tools/mod/loader-test.c" -lwinmm
WINEDLLOVERRIDES=winmm=n,b wine "$ROOT/tools/mod/build/loader-test.exe"
SCRATCH="$(mktemp -d /tmp/yo3-loader-test.XXXXXXXX)"
trap 'rm -rf "$SCRATCH"' EXIT
mkdir "$SCRATCH/配置 game"
TEST="$SCRATCH/配置 game"
cp "$ROOT/tools/mod/build/loader-test.exe" "$ROOT/tools/mod/build/winmm.dll" "$ROOT/tools/mod/build/yo3-auth.dll" "$ROOT/tools/mod/build/yo3-auth-launch.exe" "$TEST/"
python3 - "$TEST" "$(winepath -w "$TEST" 2>/dev/null)" <<'PY'
import sys,hashlib
from pathlib import Path
p=Path(sys.argv[1]); w=sys.argv[2]
(p/'profile.json').write_text('{}')
text='[revival]\n'+''.join(k+'='+w+'\\'+v+'\n' for k,v in [('launcher','yo3-auth-launch.exe'),('injector','yo3-auth-launch.exe'),('dll','yo3-auth.dll'),('profile','profile.json')])
text+='state='+w+'\nsha256='+hashlib.sha256((p/'yo3-auth.dll').read_bytes()).hexdigest()+'\n'
(p/'yo3-revival-loader.ini').write_text(text,encoding='utf-16')
PY
WINEDLLOVERRIDES=winmm=n,b wine "$TEST/loader-test.exe" 0
printf corrupt > "$TEST/yo3-auth.dll"
WINEDLLOVERRIDES=winmm=n,b wine "$TEST/loader-test.exe" 23
cp "$ROOT/tools/mod/build/yo3-auth.dll" "$TEST/yo3-auth.dll"
python3 - "$TEST/yo3-revival-loader.ini" "$(winepath -w "$TEST" 2>/dev/null)" <<'PY'
from pathlib import Path
import sys
p=Path(sys.argv[1]); text=p.read_text(encoding='utf-16'); text=text.replace('state='+sys.argv[2]+'\n','state='+sys.argv[2]+'\\missing-state\n'); p.write_text(text,encoding='utf-16')
PY
WINEDLLOVERRIDES=winmm=n,b wine "$TEST/loader-test.exe" 3
echo 'PASS: Unicode loader setup validates the exact mod hash before loading it'
