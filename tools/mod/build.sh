#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p build
CC=${CC:-i686-w64-mingw32-gcc}
python3 - > build/version.h <<'PY'
import json,os,subprocess,re,tomllib
# The default matches yo3-tunnel's build.rs ("<workspace version>-dev"), so dev DLLs pass the launcher's release check.
workspace=tomllib.load(open('../../Cargo.toml','rb'))['workspace']['package']['version']
version=os.environ.get('YO3_BUILD_VERSION',workspace+'-dev')
commit=os.environ.get('YO3_BUILD_COMMIT') or subprocess.check_output(['git','rev-parse','--short=12','HEAD'],text=True).strip()
assert len(version)<=64 and version.isascii() and not any(ord(c)<32 for c in version)
assert len(commit)<=40 and commit.isascii()
print('#define YO3_BUILD_VERSION '+json.dumps(version))
print('#define YO3_BUILD_COMMIT '+json.dumps(commit))
source=open('../../crates/yo3-tunnel/src/lib.rs').read()
print('#define YO3_MOD_ABI '+re.search(r'pub const MOD_ABI: u16 = (\d+);',source)[1])
PY
"$CC" -O2 -Wall -Wextra -Werror -shared -Wl,--kill-at -o build/yo3-auth.dll auth.c -lws2_32 -lwininet
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-login-test.exe login-test.c
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-duel-snapshot-test.exe duel-snapshot-test.c
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-ui-state-test.exe ui-state-test.c
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-resolution-test.exe resolution-test.c
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-test.exe test.c -lws2_32 -lwininet -lwinmm
# Link synthetic sites so the PE headers and callback relocation are valid on native Windows too.
python3 - <<'PY'
from pathlib import Path
Path('build/runtime-test.manifest').write_text('''<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<dependency><dependentAssembly><assemblyIdentity type="win32" name="Microsoft.VC90.CRT" version="9.0.21022.8"
processorArchitecture="x86" publicKeyToken="1fc8b3b9a1e18e3b" /></dependentAssembly></dependency></assembly>''')
Path('build/runtime-test.rc').write_text('1 24 "runtime-test.manifest"\n')
PY
"$CC" -O2 -Wall -Wextra -Werror -Wl,-T,fixture-hooks.ld -o build/yo3-auth-direct-test.exe \
  test.c fixture-hooks.s -lws2_32 -lwininet -lwinmm
"$CC" -O2 -Wall -Wextra -Werror -Wl,--image-base,0x600000,-T,fixture-hooks.ld \
  -o build/yo3-auth-relocated-test.exe test.c fixture-hooks.s -lws2_32 -lwininet -lwinmm
i686-w64-mingw32-windres -I build build/runtime-test.rc -o build/runtime-test.o
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-runtime-test.exe test.c build/runtime-test.o -lws2_32 -lwininet -lwinmm
"$CC" -O2 -Wall -Wextra -Werror -DYO3_TEST_BAD_GAME_HOOK -o build/yo3-auth-guard-test.exe test.c -lws2_32 -lwininet -lwinmm
i686-w64-mingw32-dlltool -d wsock32-ordinals.def -l build/wsock32-ordinals.a
"$CC" -O2 -Wall -Wextra -Werror -o build/yo3-auth-ordinal-test.exe test.c build/wsock32-ordinals.a -lws2_32 -lwininet -lwinmm
"$CC" -O2 -Wall -Wextra -Werror -municode -o build/yo3-auth-launch.exe launch.c -lshell32
"$CC" -O2 -Wall -Wextra -Werror -shared -Wl,--kill-at -o build/winmm.dll loader.c -ladvapi32 -lws2_32 -lwininet -lcomdlg32
