#!/usr/bin/env bash
# Contract tests of named/ordinal imports, local socket guarding, and fail-closed injection. No game is needed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export DISPLAY="${YO3_DISPLAY:-:78}"
unset WAYLAND_DISPLAY
export WINEPREFIX="${YO3_WINEPREFIX:-$ROOT/.wine-b}"
export WINEDEBUG=-all
"$ROOT/tools/mod/build.sh"
MOD="$ROOT/tools/mod/build"
wine "$MOD/yo3-login-test.exe"
DLL="$(winepath -w "$MOD/yo3-auth.dll" 2>/dev/null)"
wine "$MOD/yo3-auth-test.exe" "$DLL"
wine "$MOD/yo3-auth-ordinal-test.exe" "$DLL"
wine "$MOD/yo3-auth-ordinal-test.exe" "$DLL" missing-env
wine "$MOD/yo3-auth-guard-test.exe" "$DLL" mismatched-login
wine "$MOD/yo3-auth-guard-test.exe" "$DLL" mismatched-game
LOG="$(mktemp)"
UNICODE="$(mktemp -d)/日本語 game"
mkdir -p "$UNICODE"
trap 'rm -f "$LOG"; rm -rf "${UNICODE%/*}"' EXIT
if wine "$MOD/yo3-auth-launch.exe" "$(winepath -w "$MOD/yo3-auth-test.exe" 2>/dev/null)" 'Z:\missing-auth.dll' >"$LOG" 2>&1; then
  echo "FAIL: injector resumed with a missing connection mod" >&2
  exit 1
fi
grep -q 'game terminated before startup' "$LOG"
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
info=json.load(open(sys.argv[1])); assert info['abi']==2 and info['release']
PY
echo 'PASS: injector inspects the required mod interface before starting a game'
