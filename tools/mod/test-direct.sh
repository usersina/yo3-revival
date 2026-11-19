#!/usr/bin/env bash
# Direct start's contract under Wine (#33), one block per row of the issue's table: our own executable fixture plays the
# game's part (the gate's redirect, then the front's opener and login 0x1F47), through the real loader, helper and
# connection DLL, against scratch servers on private ports. Needs a private Wine prefix without an installed
# LoaderConfig, the Windows helper (cargo build --release --target x86_64-pc-windows-gnu -p yo3-launcher) and the
# Linux server (cargo build --release -p yo3-server).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export DISPLAY="${YO3_DISPLAY:-:78}" WINEPREFIX="${YO3_WINEPREFIX:?Choose a private test Wine prefix}" WINEDEBUG=-all
unset WAYLAND_DISPLAY
PORT="${YO3_TEST_PORT:-17443}"
SERVER="$ROOT/target/release/yo3-server"
HELPER="$ROOT/target/x86_64-pc-windows-gnu/release/yo3-launcher.exe"
for file in "$SERVER" "$HELPER" "$ROOT/tools/mod/build/yo3-auth-direct-test.exe"; do
  [[ -f "$file" ]] || { echo "Build $file first" >&2; exit 1; }
done
if wine reg.exe query 'HKCU\Software\YO3 Revival' /v LoaderConfig >/dev/null 2>&1; then
  echo 'Use a separate private test prefix without an installed LoaderConfig; the fixture must use its own profile/config.' >&2
  exit 1
fi
if ss -ltn | grep -q ":$PORT "; then echo "Port $PORT is in use; choose another with YO3_TEST_PORT" >&2; exit 1; fi
DIRECT_ROOT="$(mktemp -d /tmp/yo3-direct-test.XXXXXXXX)"
GAME="$DIRECT_ROOT/日本語 game"
DATA="$DIRECT_ROOT/data"
mkdir "$GAME"
cleanup() {
  local status=$?
  stop_server
  if [[ $status == 0 ]]; then rm -rf "$DIRECT_ROOT"; else echo "Direct-start failure artifacts: $DIRECT_ROOT" >&2; fi
}
trap cleanup EXIT
cp "$ROOT/tools/mod/build/yo3-auth-direct-test.exe" "$GAME/yo3.exe"
cp "$ROOT/tools/mod/build/"{winmm.dll,yo3-auth.dll,yo3-auth-launch.exe} "$GAME/"
cp "$HELPER" "$GAME/yo3-launcher.exe"
read -r OPENER LOGIN < <(python3 "$ROOT/tools/mod/direct_frames.py" frames)
export YO3_TEST_OPENER="$OPENER" YO3_TEST_LOGIN="$LOGIN"

"$SERVER" --data-dir "$DATA" init-auth localhost 127.0.0.1 >/dev/null
"$SERVER" --data-dir "$DATA" add-account Direct >/dev/null
"$SERVER" --data-dir "$DATA" --auth-port "$PORT" profile Direct "$DIRECT_ROOT/key.json" --advertise localhost >/dev/null
"$SERVER" --data-dir "$DATA" set registration open >/dev/null
SERVER_ARGS=(--bind 127.0.0.1 --auth-port "$PORT" --http-port 0 --data-dir "$DATA" --log-dir "$DIRECT_ROOT/logs"
  --no-packet-log --game-dir "${YO3_GAME_DIR:-$WINEPREFIX/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3}")
SERVER_PID=
start_server() {
  "$SERVER" "${SERVER_ARGS[@]}" >"$DIRECT_ROOT/server.out" 2>&1 & SERVER_PID=$!
  for _ in {1..100}; do ss -ltn | grep -q ":$PORT " && return; sleep .1; done
  echo 'FAIL: the scratch server did not start' >&2; exit 1
}
stop_server() {
  if [[ -n "$SERVER_PID" ]]; then kill "$SERVER_PID" 2>/dev/null || true; wait "$SERVER_PID" 2>/dev/null || true; SERVER_PID=; fi
  for _ in {1..100}; do ss -ltn | grep -q ":$PORT " || return 0; sleep .1; done
}

# profile NAME JSON-EXTRA: the direct profile (pinned identity) with a key, a start or a vault entry.
profile() {
  python3 - "$DIRECT_ROOT/key.json" "$GAME/profile.json" "$1" "$DATA/server.crt" <<'PY'
import json,sys
from pathlib import Path
key=json.loads(Path(sys.argv[1]).read_text())
pem=Path(sys.argv[4]).read_text()
extra=json.loads(sys.argv[3])
body={'server':key['server'],'server_name':key['server_name'],'certificate':'server.crt'}
Path(sys.argv[2]).with_name('server.crt').write_text(pem)
if extra.pop('with_key',False): body['key']=key['key']
body.update(extra)
Path(sys.argv[2]).write_text(json.dumps(body))
PY
}
ini() { # ini LAUNCHER: the loader configuration, Unicode, beside the loader
  python3 - "$GAME" "$(winepath -w "$GAME" 2>/dev/null)" "$1" <<'PY'
from pathlib import Path
import sys,hashlib
p=Path(sys.argv[1]); w=sys.argv[2]
text='[revival]\n'+''.join(k+'='+w+'\\'+v+'\n' for k,v in [('launcher',sys.argv[3]),('injector','yo3-auth-launch.exe'),('dll','yo3-auth.dll'),('profile','profile.json')])
text+='state='+w+'\nlogs='+w+'\\diagnostics\nsha256='+hashlib.sha256((p/'yo3-auth.dll').read_bytes()).hexdigest()+'\n'
(p/'yo3-revival-loader.ini').write_text(text,encoding='utf-16')
PY
}
# play EXPECTED...: start the fixture as yo3.exe directly; its replies must be the expected lines.
play() {
  local out="$DIRECT_ROOT/replies.txt" notice="$DIRECT_ROOT/notice.txt"; rm -f "$out" "$notice"
  YO3_LOADER_NOTICE_FILE="$(winepath -w "$notice" 2>/dev/null)" WINEDLLOVERRIDES=winmm=n,b \
    timeout 120 wine "$GAME/yo3.exe" direct "$(winepath -w "$out" 2>/dev/null)" >/dev/null
  local got; got="$(python3 "$ROOT/tools/mod/direct_frames.py" replies "$out")"
  if [[ -f "$notice" ]]; then got+=$'\n'"notice: $(iconv -f UTF-16LE -t UTF-8 "$notice")"; fi
  local want; want="$(printf '%s\n' "$@")"
  if [[ "$got" != "$want" ]]; then printf 'FAIL: %s\n--- got\n%s\n--- want\n%s\n' "$ROW" "$got" "$want" >&2; exit 1; fi
  # The helper observes the game's exit and removes its private readiness exchange.
  for _ in {1..40}; do compgen -G "$GAME/direct-*.json" >/dev/null || break; sleep .25; done
  if compgen -G "$GAME/direct-*.json" >/dev/null; then echo "FAIL: $ROW: readiness was not cleaned up" >&2; exit 1; fi
  echo "PASS: $ROW"
}
account() { # account JSON: an account request (TLS operation 6) to the scratch server; prints the reply. It names
  # the helper's version as a launcher does: servers refuse launchers older than themselves (update_required).
  python3 - "$PORT" "$DATA/server.crt" "$1" "$VERSION" <<'PY'
import json,socket,ssl,struct,sys
ctx=ssl.create_default_context(cafile=sys.argv[2])
request=json.loads(sys.argv[3]); request.setdefault('launcher',sys.argv[4])
with socket.create_connection(('127.0.0.1',int(sys.argv[1]))) as raw, ctx.wrap_socket(raw,server_hostname='localhost') as s:
    body=json.dumps(request).encode(); s.sendall(b'YO3TLS02'+bytes([6])+struct.pack('>H',len(body))+body)
    def take(n):
        out=b''
        while len(out)<n: out+=s.recv(n-len(out))
        return out
    print(take(struct.unpack('>H',take(2))[0]).decode())
PY
}
VERSION="$(wine "$HELPER" --version </dev/null 2>/dev/null | awk '{print $2}' | tr -d '\r' | sed 's/^v//')"
OK_FRONT=('mod loaded' 'gate 0x21fe' 'front 0x219f' 'front 0x219a')
REFUSED=('mod loaded' 'gate 0x21fe' 'front 0x219f' 'front 0x21a0')
ini yo3-launcher.exe

ROW='Singleplayer: the helper starts your server, which waits on the title screen and stops about 30 s after the game'
# As the Linux launcher writes it (yo3-desktop direct.rs): Wine starts a Linux program detached, so a wrapper keeps the
# server's input open while the helper touches its heartbeat.
WRAPPER="$(sed -n 's/^pub const WRAPPER: &str = r#"\(.*\)"#;$/\1/p' "$ROOT/crates/yo3-launcher/src/direct.rs")"
[[ -n "$WRAPPER" ]] || { echo 'FAIL: no wrapper in direct.rs' >&2; exit 1; }
profile "$(python3 -c 'import json,sys; print(json.dumps({"with_key":True,"start":{"program":sys.argv[1],"heartbeat":sys.argv[2],
  "args":["-c",sys.argv[3],"sh",sys.argv[4],"/usr/bin/env","NO_COLOR=1"]+sys.argv[5:]}}))' \
  "$(winepath -w /bin/sh 2>/dev/null)" "$(winepath -w "$DIRECT_ROOT/server-owner" 2>/dev/null)" "$WRAPPER" "$DIRECT_ROOT/server-owner" \
  "$SERVER" "${SERVER_ARGS[@]}")"
YO3_TEST_IDLE_MS=40000 play "${OK_FRONT[@]}"
ROW='Singleplayer: a game opened again while the last one'"'"'s server counts down starts it again'
YO3_TEST_IDLE_MS=60000 play "${OK_FRONT[@]}"
grep -rq 'Your server had stopped: started it again' "$GAME/diagnostics" || { echo "FAIL: $ROW: it was not restarted" >&2; exit 1; }
ss -ltn | grep -q ":$PORT " || { echo "FAIL: $ROW: the server stopped with the game" >&2; exit 1; }
for _ in {1..90}; do ss -ltn | grep -q ":$PORT " || break; sleep 1; done
if ss -ltn | grep -q ":$PORT "; then echo "FAIL: $ROW: the server still runs 90 s after the game" >&2; exit 1; fi
echo "PASS: $ROW (stopped by itself)"

ROW='Singleplayer: your server fails to start'
profile '{"with_key":true,"start":{"program":"C:\\no\\such\\yo3-server.exe"}}'
play "${REFUSED[@]}"

start_server
ROW='Singleplayer: the key in the Wine prefix'"'"'s credential store (key_vault), not in the profile'
# As the Linux launcher sets it up (#23): the key copied into the prefix's store with --vault-set, the profile naming it.
KEY="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["key"])' "$DIRECT_ROOT/key.json")"
printf %s "$KEY" | wine "$HELPER" --vault-set key:direct-test 2>/dev/null
profile '{"key_vault":"key:direct-test"}'
if grep -q "$KEY" "$GAME/profile.json"; then echo "FAIL: $ROW: the key is in the profile" >&2; exit 1; fi
play "${OK_FRONT[@]}"
wine "$HELPER" --vault-delete key:direct-test </dev/null 2>/dev/null

REGISTER="$(account '{"op":"register","user":"directuser","password":"LongPassphrase1","rules":true}')"
[[ "$REGISTER" == *'"ok":true'* ]] || { echo "FAIL: register: $REGISTER" >&2; exit 1; }
TOKEN="$(account '{"op":"login","user":"directuser","password":"LongPassphrase1","stay":true,"device":"test"}' | python3 -c 'import json,sys; print(json.load(sys.stdin)["device_token"])')"
VAULT=server:direct-test
profile "{\"vault\":\"$VAULT\"}"

ROW='Multiplayer: a valid saved sign-in goes straight to Select Player'
printf %s "$TOKEN" | wine "$HELPER" --vault-set "$VAULT" 2>/dev/null
play "${OK_FRONT[@]}"

ROW='Multiplayer: Delete Login Data revokes the saved sign-in on the server and forgets it here'
YO3_TEST_SIGN_OUT=1 play "${OK_FRONT[@]}" 'signout HTTP/1.1 204'
RESUMED="$(account "{\"op\":\"resume\",\"device_token\":\"$TOKEN\"}")"
[[ "$RESUMED" == *'"signed_out"'* ]] || { echo "FAIL: $ROW: the server still accepts the token: $RESUMED" >&2; exit 1; }
echo "PASS: $ROW (revoked on the server)"

ROW='Multiplayer: no saved sign-in, or a revoked one'
play "${REFUSED[@]}"
printf %s "$TOKEN" | wine "$HELPER" --vault-set "$VAULT" 2>/dev/null
play "${REFUSED[@]}"

ROW='#34: no saved sign-in, the game'"'"'s own form: a wrong password, then a good one with Save Password'
wine "$HELPER" --vault-delete "$VAULT" </dev/null 2>/dev/null
FORM=('mod loaded' 'gate 0x21fe' 'state form' 'typed HTTP/1.1 204')
YO3_TEST_TYPED='{"user":"directuser","password":"not the passphrase","stay":true}' play "${FORM[@]}" 'front 0x219f' 'front 0x219b'
YO3_TEST_TYPED='{"user":"directuser","password":"LongPassphrase1","stay":true}' play "${FORM[@]}" 'front 0x219f' 'front 0x219a'
play "${OK_FRONT[@]}"
echo "PASS: $ROW (Save Password kept the device signed in: the next start skipped the form)"

ROW='#34: a ban gets "This User ID is currently restricted."'
"$SERVER" --data-dir "$DATA" ban directuser >/dev/null
wine "$HELPER" --vault-delete "$VAULT" </dev/null 2>/dev/null
YO3_TEST_TYPED='{"user":"directuser","password":"LongPassphrase1","stay":false}' play "${FORM[@]}" 'front 0x219f' 'front 0x219e'
"$SERVER" --data-dir "$DATA" lift directuser >/dev/null

ROW='Multiplayer: a server that needs a newer launcher (the game fails to connect; one line says to update)'
TOKEN="$(account '{"op":"login","user":"directuser","password":"LongPassphrase1","stay":true,"device":"test"}' | python3 -c 'import json,sys; print(json.load(sys.stdin)["device_token"])')"
printf %s "$TOKEN" | wine "$HELPER" --vault-set "$VAULT" 2>/dev/null
"$SERVER" --data-dir "$DATA" set min-launcher 99.0.0 >/dev/null
play "${REFUSED[@]}"
grep -rq 'Told the player: This server needs a newer YO3 Revival' "$GAME/diagnostics" || { echo "FAIL: $ROW: no line" >&2; exit 1; }
"$SERVER" --data-dir "$DATA" reset min-launcher >/dev/null

ROW='Multiplayer: the server is down'
stop_server
play "${REFUSED[@]}"

ROW='Either: YO3 Revival moved or uninstalled (the helper is missing)'
ini moved/yo3-launcher.exe
play 'mod absent' 'gate refused 10061' "notice: Opening the game directly needs repair: open YO3 Revival once, then start the game again."
wine "$HELPER" --vault-delete "$VAULT" </dev/null 2>/dev/null
echo "PASS: every row of direct start's table (helper ${VERSION:-unknown})"
