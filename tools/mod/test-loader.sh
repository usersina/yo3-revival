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
