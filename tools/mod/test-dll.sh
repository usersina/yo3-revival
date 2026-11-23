#!/usr/bin/env bash
# Contract tests of named/ordinal imports, local socket guarding, and fail-closed injection. No game is needed.
case "${1:-}" in -h|--help) awk 'NR>1 && /^#/{sub(/^# ?/,""); print; next} NR>1{exit}' "$0"; exit 0 ;; esac
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export DISPLAY="${YO3_DISPLAY:-:78}"
unset WAYLAND_DISPLAY
export WINEPREFIX="${YO3_WINEPREFIX:-$ROOT/.wine-b}"
export WINEDEBUG=-all
"$ROOT/tools/mod/build.sh"
MOD="$ROOT/tools/mod/build"
wine "$MOD/yo3-login-test.exe"
wine "$MOD/yo3-duel-snapshot-test.exe"
wine "$MOD/yo3-ui-state-test.exe"
DLL="$(winepath -w "$MOD/yo3-auth.dll" 2>/dev/null)"
wine "$MOD/yo3-auth-test.exe" "$DLL"
wine "$MOD/yo3-auth-ordinal-test.exe" "$DLL"
wine "$MOD/yo3-auth-direct-test.exe" "$DLL"
wine "$MOD/yo3-auth-relocated-test.exe" "$DLL"
LOG="$(mktemp)"
UNICODE="$(mktemp -d)/日本語 game"
mkdir -p "$UNICODE"
trap 'rm -f "$LOG"; rm -rf "${UNICODE%/*}"' EXIT
wine "$MOD/yo3-auth-ordinal-test.exe" "$DLL" missing-env >"$LOG" 2>&1
grep -q 'stage=mod-environment' "$LOG"
for pair in 'mismatched-login:mod-login-hook' 'mismatched-game:mod-starter-hook' 'mismatched-duel:mod-duel-hook' 'mismatched-update:mod-duel-hook'; do
  wine "$MOD/yo3-auth-guard-test.exe" "$DLL" "${pair%%:*}" >"$LOG" 2>&1
  grep -q "stage=${pair#*:}" "$LOG" || { command cat "$LOG" >&2; exit 1; }
done
echo 'PASS: incompatible game hooks and missing launch state identify their failure stage'
wine "$MOD/yo3-resolution-test.exe"
wine "$MOD/yo3-teardown-test.exe"
wine "$MOD/yo3-rental-return-test.exe"
wine "$MOD/yo3-web-links-test.exe"
wine "$MOD/yo3-card-search-test.exe"
wine "$MOD/yo3-card-profile-test.exe"
wine "$MOD/yo3-card-profile-hook-test.exe"
wine "$MOD/yo3-signout-test.exe"
wine "$MOD/yo3-chat-lobby-test.exe"
wine "$MOD/yo3-added-cards-test.exe"
wine "$MOD/yo3-survey-test.exe"
# The higher resolution (resolution.h) is opt in: an invalid value, or a game whose drawing code isn't v1.153's (this
# fixture has none), refuses to start; off loads as usual.
for pair in 'banana:YO3_RESOLUTION must be' '1440x1080:drawing code is unsupported' 'auto:drawing code is unsupported'; do
  YO3_RESOLUTION="${pair%%:*}" wine "$MOD/yo3-auth-test.exe" "$DLL" resolution >"$LOG" 2>&1 || { command cat "$LOG" >&2; exit 1; }
  grep -q "stage=mod-resolution: .*${pair#*:}" "$LOG" || { command cat "$LOG" >&2; exit 1; }
done
YO3_RESOLUTION=off wine "$MOD/yo3-auth-test.exe" "$DLL" >"$LOG" 2>&1 || { command cat "$LOG" >&2; exit 1; }
echo 'PASS: the higher resolution is off unless asked for, and refuses a bad value or an unsupported game'
# What else runs in the game (#68): the survey lists another program's DLL (with its SHA-256, in the log's grouped form),
# an .asi plugin beside the game, a redirected import and a detour, and finds nothing wrong with a game left alone.
SURVEY="$(mktemp -d)/game folder"
mkdir -p "$SURVEY"
trap 'rm -f "$LOG"; rm -rf "${UNICODE%/*}" "${SURVEY%/*}"' EXIT
cp "$MOD/yo3-auth-test.exe" "$MOD/yo3-auth.dll" "$MOD/yo3-survey-fixture.dll" "$SURVEY/"
printf 'plugin' > "$SURVEY/plugin.asi"
for mode in survey survey-hooked survey-detour; do
  rm -f "$SURVEY/survey.json"
  YO3_SURVEY_FILE="$(winepath -w "$SURVEY/survey.json" 2>/dev/null)" wine "$SURVEY/yo3-auth-test.exe" "$(winepath -w "$SURVEY/yo3-auth.dll" 2>/dev/null)" "$mode" >"$LOG" 2>&1 \
    || { command cat "$LOG" >&2; exit 1; }
  python3 - "$mode" "$SURVEY" "$LOG" <<'PY' || { command cat "$LOG" >&2; exit 1; }
import hashlib,json,sys
mode,folder,log=sys.argv[1:4]
report=json.load(open(folder+'/survey.json')); text=open(log).read()
digest=hashlib.sha256(open(folder+'/yo3-survey-fixture.dll','rb').read()).hexdigest()
grouped=':'.join(digest[i:i+16] for i in range(0,64,16))
fixture=[m for m in report['modules'] if m['name']=='yo3-survey-fixture.dll']
assert len(fixture)==1 and fixture[0]['sha256']==digest and fixture[0]['loaded'] and fixture[0]['game_folder'], report['modules']
assert fixture[0]['size']==len(open(folder+'/yo3-survey-fixture.dll','rb').read())
assert 'name=yo3-survey-fixture.dll' in text and 'sha256='+grouped in text, text
plugin=[m for m in report['modules'] if m['name']=='plugin.asi']
assert len(plugin)==1 and not plugin[0]['loaded'] and plugin[0]['game_folder'], report['modules']
names={m['name'].lower() for m in report['modules']}
assert 'yo3-auth.dll' not in names and 'yo3-auth-test.exe' not in names and not any(n in names for n in ('kernel32.dll','ws2_32.dll')), names
assert report['providers_known'] in (True,False) and 'providers' in report
hooked={(h['function'],h['how'],h['module']) for h in report['hooked']}
want={'survey':set(),'survey-hooked':{('send','import','yo3-survey-fixture.dll')},'survey-detour':{('closesocket','detour','yo3-survey-fixture.dll')}}[mode]
assert hooked==want, (mode,hooked)
assert all(p['microsoft'] for p in report['providers']), report['providers']  # Wine's own providers are no finding
assert '[YO3 MOD] Survey: done modules=2 hooked=%d'%len(want) in text, text
PY
done
echo 'PASS: the survey lists foreign modules with their SHA-256, a redirected import and a detour, and finds nothing in a clean game'
wine "$MOD/yo3-auth-launch.exe" --check-runtime "$(winepath -w "$MOD/yo3-auth-test.exe" 2>/dev/null)" >"$LOG" 2>&1 || { command cat "$LOG" >&2; exit 1; }
grep -q 'Game runtime prerequisites available' "$LOG"
wine "$MOD/yo3-auth-launch.exe" --check-runtime "$(winepath -w "$MOD/yo3-auth-runtime-test.exe" 2>/dev/null)" >"$LOG" 2>&1 || { command cat "$LOG" >&2; exit 1; }
grep -q 'Game runtime prerequisites available' "$LOG"
for dependency in d3dx9_40 msvcp90; do
  if WINEDLLOVERRIDES="$dependency=" wine "$MOD/yo3-auth-launch.exe" --check-runtime \
    "$(winepath -w "$MOD/yo3-auth-test.exe" 2>/dev/null)" >"$LOG" 2>&1; then
    echo "FAIL: missing $dependency runtime passed validation" >&2; exit 1
  fi
  grep -q 'stage=runtime-dependency' "$LOG"
  grep -qi "$dependency.dll" "$LOG"
  if grep -q 'FAIL line' "$LOG"; then echo 'FAIL: runtime validation started the game' >&2; exit 1; fi
done
echo 'PASS: runtime validation identifies missing DirectX and VC90 without starting the game'
# Every import of the game is looked up (#41): a function missing from its DLL is named before the game starts.
if wine "$MOD/yo3-auth-launch.exe" --check-runtime "$(winepath -w "$MOD/yo3-missing-import-test.exe" 2>/dev/null)" >"$LOG" 2>&1; then
  echo 'FAIL: a missing imported function passed validation' >&2; exit 1
fi
grep -q 'stage=runtime-import windows_error=127: .*Yo3MissingFunction from KERNEL32.dll' "$LOG" || { command cat "$LOG" >&2; exit 1; }
if grep -q 'FAIL line' "$LOG"; then echo 'FAIL: runtime validation started the game' >&2; exit 1; fi
echo 'PASS: runtime validation names a function missing from an imported DLL'
if wine "$MOD/yo3-auth-launch.exe" "$(winepath -w "$MOD/yo3-auth-test.exe" 2>/dev/null)" 'Z:\missing-auth.dll' >"$LOG" 2>&1; then
  echo "FAIL: injector resumed with a missing connection mod" >&2
  exit 1
fi
grep -q 'game terminated before startup' "$LOG"
grep -q 'stage=mod-metadata' "$LOG"
grep -q 'windows_error=' "$LOG"
if grep -q 'FAIL line' "$LOG"; then echo "FAIL: the suspended test executable ran" >&2; exit 1; fi
echo 'PASS: injector refuses startup when the required DLL cannot load'
cp "$MOD/yo3-auth-test.exe" "$UNICODE/game.exe"
if wine "$MOD/yo3-auth-launch.exe" "$(winepath -w "$UNICODE/game.exe" 2>/dev/null)" "$DLL" >"$LOG" 2>&1; then
  echo 'FAIL: Unicode-path game resumed without its required DLL' >&2; exit 1
fi
grep -q 'game terminated before startup' "$LOG" || { command cat "$LOG" >&2; exit 1; }
echo 'PASS: injector starts a suspended game from a Unicode folder'
cp "$MOD/yo3-auth.dll" "$UNICODE/auth.dll"
# The fixture deliberately exits 1 without its usual argument; the injector must load the Unicode-path DLL first.
YO3_LOCAL_GATE=1 YO3_LOCAL_FRONT=1 YO3_LOCAL_DUEL=1 YO3_LOCAL_HTTP=1 \
YO3_LOCAL_COOKIE=0707070707070707070707070707070707070707070707070707070707070707 \
wine "$MOD/yo3-auth-launch.exe" "$(winepath -w "$MOD/yo3-auth-test.exe" 2>/dev/null)" \
  "$(winepath -w "$UNICODE/auth.dll" 2>/dev/null)" >"$LOG" 2>&1 && { echo 'FAIL: fixture unexpectedly succeeded' >&2; exit 1; }
grep -q 'connection mod loaded' "$LOG" || { command cat "$LOG" >&2; exit 1; }
echo 'PASS: injector loads its required DLL from a Unicode folder'
wine "$MOD/yo3-auth-launch.exe" --inspect "$DLL" >"$LOG" 2>&1
python3 - "$LOG" <<'PY'
import json,sys
info=json.load(open(sys.argv[1])); assert info['abi']==3 and info['release']
PY
echo 'PASS: injector inspects the required mod interface before starting a game'
