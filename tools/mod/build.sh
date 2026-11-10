#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p build
CC=${CC:-i686-w64-mingw32-gcc}
python3 - > build/version.h <<'PY'
import json,os,subprocess,re
version=os.environ.get('YO3_BUILD_VERSION','0.1.0-dev')
commit=os.environ.get('YO3_BUILD_COMMIT') or subprocess.check_output(['git','rev-parse','--short=12','HEAD'],text=True).strip()
assert len(version)<=64 and version.isascii() and not any(ord(c)<32 for c in version)
assert len(commit)<=40 and commit.isascii()
print('#define YO3_BUILD_VERSION '+json.dumps(version))
print('#define YO3_BUILD_COMMIT '+json.dumps(commit))
source=open('../../crates/yo3-tunnel/src/lib.rs').read()
print('#define YO3_MOD_ABI '+re.search(r'pub const MOD_ABI: u16 = (\d+);',source)[1])
PY
"$CC" -O2 -Wall -Wextra -Werror -shared -Wl,--kill-at -o build/yo3-auth.dll auth.c -lws2_32 -lwininet
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-test.exe test.c -lws2_32 -lwininet
i686-w64-mingw32-dlltool -d wsock32-ordinals.def -l build/wsock32-ordinals.a
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-ordinal-test.exe test.c build/wsock32-ordinals.a -lws2_32 -lwininet
"$CC" -O2 -Wall -Wextra -Werror -municode -o build/yo3-auth-launch.exe launch.c
"$CC" -O2 -Wall -Wextra -Werror -shared -Wl,--kill-at -o build/winmm.dll loader.c
