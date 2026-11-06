#!/usr/bin/env bash
# Build the field-read tracer (32-bit, needs mingw-w64) into tools/tracer/build/.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p build
CC=${CC:-i686-w64-mingw32-gcc}
"$CC" -O2 -Wall -Wextra -shared -s -o build/yo3trace.dll yo3trace.c
"$CC" -O2 -Wall -Wextra -s -o build/yo3trace-launch.exe launch.c
echo "built $(pwd)/build/yo3trace.dll and yo3trace-launch.exe"
