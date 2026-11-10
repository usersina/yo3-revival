#!/usr/bin/env bash
# Drive the game client on a private virtual X display, so it never touches the real
# screen or mouse.
#
#   tools/client/vclient.sh xvfb            start the virtual display (:77) if it is not running
#   tools/client/vclient.sh start [--trace] launch yo3.exe on it (trace -> logs/client-stdout.log);
#                                           --trace also logs every field lookup -> logs/fields.tsv
#   tools/client/vclient.sh login [SLOT]    title -> ONLINE -> OK (saved credentials) -> player SLOT (0-3, default 0); `login select` stops at Select Player
#   tools/client/vclient.sh shot NAME       screenshot the game window -> logs/shots/NAME.png
#   tools/client/vclient.sh click X Y       left click at game-window coordinates (1024x768)
#   tools/client/vclient.sh key KEY...      xdotool key names, e.g. Return Escape
#   tools/client/vclient.sh type TEXT       type text
#   tools/client/vclient.sh pixel X Y       print the window's colour at X Y as "R G B"
#   tools/client/vclient.sh check           process state, client trace, crash, last packets
#   tools/client/vclient.sh stop            close this instance's client (wineserver -k on its prefix)
#   tools/client/vclient.sh prefix          create this instance's Wine prefix (reflinked copy of ~/.wine)
#
# More clients: YO3_INSTANCE=b (or c, d) uses display :78 (:79, :80), logs/b/ and its own Wine
# prefix .wine-b/ (git-ignored). Wine assumes one X display per prefix: two clients in one
# prefix on two displays die with X BadWindow errors. Run `xvfb` and `prefix` once per instance.
set -euo pipefail

INSTANCE="${YO3_INSTANCE:-}"
case "$INSTANCE" in "") DEFAULT_DISPLAY=:77 ;; b) DEFAULT_DISPLAY=:78 ;; c) DEFAULT_DISPLAY=:79 ;; *) DEFAULT_DISPLAY=:80 ;; esac
export DISPLAY="${YO3_DISPLAY:-$DEFAULT_DISPLAY}"
unset WAYLAND_DISPLAY
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if [ -n "$INSTANCE" ]; then export WINEPREFIX="${YO3_WINEPREFIX:-$ROOT/.wine-$INSTANCE}"; fi
GAME="${YO3_GAME_DIR:-${WINEPREFIX:-$HOME/.wine}/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3}"
LOGS="$ROOT/logs${INSTANCE:+/$INSTANCE}"
mkdir -p "$LOGS/shots"

win() { xdotool search --name '^Yu-Gi-Oh! ONLINE 3' 2>/dev/null | head -1; }

# The window, or fail (an empty id makes `import` wait for a click and hang).
need_win() {
  local w; w="$(win)"
  [ -n "$w" ] || { echo "no game window on $DISPLAY" >&2; exit 1; }
  echo "$w"
}

pixel() {
  local w; w="$(win)"
  [ -n "$w" ] || return 1
  import -window "$w" -crop "1x1+$1+$2" -depth 8 txt:- 2>/dev/null | tail -1 |
    sed -E 's/.*\(([0-9]+),([0-9]+),([0-9]+).*/\1 \2 \3/'
}

# near X Y R G B: is the pixel within 14 of the given colour on every channel?
near() {
  local r g b
  read -r r g b < <(pixel "$1" "$2") || return 1
  (( ${r:-999} - $3 < 15 && $3 - ${r:-999} < 15 && g - $4 < 15 && $4 - g < 15 && b - $5 < 15 && $5 - b < 15 ))
}

# wait_for NAME SECONDS X Y R G B
wait_for() {
  local name=$1 secs=$2; shift 2
  for _ in $(seq $((secs * 2))); do
    near "$@" && return 0
    sleep 0.5
  done
  echo "timed out waiting for the $name screen" >&2
  "$0" shot "timeout-$name" >&2
  return 1
}

case "${1:-}" in
  xvfb)
    xdpyinfo >/dev/null 2>&1 || { Xvfb "$DISPLAY" -screen 0 1280x960x24 -nolisten tcp & sleep 1; }
    xdpyinfo | grep dimensions ;;
  start)
    PROFILE="${YO3_AUTH_PROFILE:?set YO3_AUTH_PROFILE to the private client.json for this account}"
    MOD="$ROOT/tools/mod/build"
    [ -f "$MOD/yo3-auth.dll" ] || "$ROOT/tools/mod/build.sh" >/dev/null
    cmd=("$ROOT/target/release/yo3-launcher" --profile "$PROFILE" --game-dir "$GAME"
         --injector "$MOD/yo3-auth-launch.exe" --dll "$MOD/yo3-auth.dll"
         --wine-prefix "${WINEPREFIX:-$HOME/.wine}")
    if [ "${2:-}" = "--trace" ]; then
      TRACER="$ROOT/tools/tracer/build"
      [ -f "$TRACER/yo3trace.dll" ] || "$ROOT/tools/tracer/build.sh" >/dev/null
      export YO3_TRACE_FILE; YO3_TRACE_FILE="$(winepath -w "$LOGS/fields.tsv" 2>/dev/null)"
      cmd+=(--trace-dll "$TRACER/yo3trace.dll")
    fi
    cd "$GAME"
    WINEDEBUG=fixme-all nohup "${cmd[@]}" >"$LOGS/client-stdout.log" 2>"$LOGS/client-wine.log" &
    for _ in $(seq 60); do [ -n "$(win)" ] && break; sleep 0.5; done
    echo "window $(win)" ;;
  login)
    # Fingerprints sampled from the real screens (software rendering under Xvfb).
    # Boot: patch check -> KONAMI splash -> "Click the screen" -> title. Click a corner
    # (inert on the title itself) every 3 s until the title's ONLINE button shows.
    # The menu is ONLINE *and* OFFICIAL WEBSITE buttons both blue; the logo screen shares
    # the first colour for a frame or two, never the second.
    menu() { near 297 301 39 61 117 && near 297 434 38 60 116; }
    for i in $(seq 240); do
      menu && break
      (( i % 6 == 0 )) && [ -n "$(win)" ] && "$0" click 900 700
      sleep 0.5
    done
    menu || { echo "timed out waiting for the title menu" >&2; "$0" shot timeout-title >&2; exit 1; }
    "$0" click 297 301
    wait_for login 30 500 210 12 93 117                       # login dialog header bar
    "$0" click 402 459                                        # OK (saved credentials)
    wait_for players 30 600 231 38 60 116                     # first player slot
    [ "${2:-}" = select ] && { "$0" shot select; exit 0; }       # `login select` stops at Select Player
    "$0" click 656 $((231 + ${2:-0} * 46))                     # slot rows are 46 px apart
    sleep "${YO3_MAIN_WAIT:-10}"
    "$0" shot login ;;
  shot)
    W="$(need_win)"; import -window "$W" "$LOGS/shots/${2:-shot}.png" && echo "$LOGS/shots/${2:-shot}.png" ;;
  click)
    W="$(need_win)"; xdotool mousemove --window "$W" "$2" "$3"; sleep 0.15; xdotool click --delay 120 1 ;;
  prefix)
    [ -n "${WINEPREFIX:-}" ] || { echo "the default instance uses ~/.wine itself" >&2; exit 2; }
    if [ ! -d "$WINEPREFIX" ]; then
      cp -a --reflink=auto "$HOME/.wine" "$WINEPREFIX"   # btrfs: near-instant, shares blocks
    fi
    echo "$WINEPREFIX" ;;
  key)
    shift; xdotool key --delay 80 "$@" ;;
  type)
    xdotool type --delay 60 "$2" ;;
  pixel)
    pixel "$2" "$3" ;;
  check)
    if [ -n "$(win)" ]; then echo "client: running (window $(win))"; else echo "client: not running"; fi
    echo "--- client trace (last 8)"
    grep -E '\.cpp L[0-9]+ :' "$LOGS/client-stdout.log" 2>/dev/null | tail -8 || true
    echo "--- crash"
    grep -iE 'page fault|unhandled exception' "$LOGS/client-wine.log" 2>/dev/null | tail -2 || echo "none"
    latest=$(find "${YO3_PACKET_LOG_DIR:-$ROOT/logs}" -maxdepth 1 -name 'packets-*.jsonl' -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2-)
    echo "--- packets ($latest, last 10)"
    [ -n "$latest" ] && python3 -c '
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1])][-10:]
for d in rows:
    m = d.get("msg") or {}
    rid = (m.get("fields") or {}).get("request")
    what = hex(rid) if isinstance(rid, int) else ("version" if "version" in m else "?")
    print(d["server"], d["dir"], what)
' "$latest" ;;
  stop)
    wineserver -k || true ;;
  *)
    sed -n '2,14p' "$0"; exit 2 ;;
esac
