#!/usr/bin/env bash
# Live direct-start contract, using our own executable fixture and an explicit private profile.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export DISPLAY="${YO3_DISPLAY:-:78}" WINEPREFIX="${YO3_WINEPREFIX:?Choose a private test Wine prefix}" WINEDEBUG=-all
unset WAYLAND_DISPLAY
PROFILE="${1:?A private, pinned profile for a running scratch server is required}"
if wine reg.exe query 'HKCU\Software\YO3 Revival' /v LoaderConfig >/dev/null 2>&1; then
  echo 'Use a separate private test prefix without an installed LoaderConfig; the fixture must use its own profile/config.' >&2
  exit 1
fi
DIRECT_ROOT="$(mktemp -d /tmp/yo3-direct-test.XXXXXXXX)"
SCRATCH="$DIRECT_ROOT/日本語 game"
mkdir "$SCRATCH"
trap 'if [[ $? == 0 ]]; then rm -rf "$DIRECT_ROOT"; else echo "Direct-start failure artifacts: $SCRATCH" >&2; fi' EXIT
cp "$ROOT/tools/mod/build/yo3-auth-test.exe" "$SCRATCH/yo3.exe"
cp "$ROOT/tools/mod/build/"{winmm.dll,yo3-auth.dll,yo3-auth-launch.exe} "$SCRATCH/"
python3 - "$PROFILE" "$SCRATCH" "$(winepath -w "$SCRATCH" 2>/dev/null)" "$(winepath -w "$ROOT/target/x86_64-pc-windows-gnu/release/yo3-launcher.exe" 2>/dev/null)" <<'PY'
from pathlib import Path
import sys,json,hashlib
source=Path(sys.argv[1]); p=Path(sys.argv[2]); w=sys.argv[3]
profile=json.loads(source.read_text())
if not profile.get('identity'):
    (p/'server.crt').write_bytes((source.parent/profile['certificate']).read_bytes()); profile['certificate']='server.crt'
(p/'profile.json').write_text(json.dumps(profile)); (p/'profile.json').chmod(0o600)
text='[revival]\nlauncher='+sys.argv[4]+'\n'+''.join(k+'='+w+'\\'+v+'\n' for k,v in [('injector','yo3-auth-launch.exe'),('dll','yo3-auth.dll'),('profile','profile.json')])
text+='state='+w+'\nlogs='+w+'\\diagnostics\nsha256='+hashlib.sha256((p/'yo3-auth.dll').read_bytes()).hexdigest()+'\n'
(p/'yo3-revival-loader.ini').write_text(text,encoding='utf-16')
PY
WINEDLLOVERRIDES=winmm=n,b timeout 70 wine "$SCRATCH/yo3.exe" direct
# The helper observes game exit, logs out, and removes its private readiness exchange.
for _ in {1..40}; do
  if ! compgen -G "$SCRATCH/direct-*.json" >/dev/null; then break; fi
  sleep .25
done
if compgen -G "$SCRATCH/direct-*.json" >/dev/null; then echo 'FAIL: direct session readiness was not cleaned up' >&2; exit 1; fi
