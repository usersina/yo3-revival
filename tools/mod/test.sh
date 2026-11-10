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
DLL="$(winepath -w "$MOD/yo3-auth.dll" 2>/dev/null)"
wine "$MOD/yo3-auth-test.exe" "$DLL"
wine "$MOD/yo3-auth-ordinal-test.exe" "$DLL"
wine "$MOD/yo3-auth-ordinal-test.exe" "$DLL" missing-env
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT
if wine "$MOD/yo3-auth-launch.exe" "$(winepath -w "$MOD/yo3-auth-test.exe" 2>/dev/null)" 'Z:\missing-auth.dll' >"$LOG" 2>&1; then
  echo "FAIL: injector resumed with a missing connection mod" >&2
  exit 1
fi
grep -q 'game terminated before startup' "$LOG"
if grep -q 'FAIL line' "$LOG"; then echo "FAIL: the suspended test executable ran" >&2; exit 1; fi
echo 'PASS: injector refuses startup when the required DLL cannot load'
