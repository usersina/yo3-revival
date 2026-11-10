#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p build
CC=${CC:-i686-w64-mingw32-gcc}
"$CC" -O2 -Wall -Wextra -Werror -shared -o build/yo3-auth.dll auth.c -lws2_32 -lwininet
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-test.exe test.c -lws2_32 -lwininet
i686-w64-mingw32-dlltool -d wsock32-ordinals.def -l build/wsock32-ordinals.a
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-ordinal-test.exe test.c build/wsock32-ordinals.a -lws2_32 -lwininet
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-launch.exe launch.c
