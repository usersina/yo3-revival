#!/usr/bin/env bash
# Drive the game client on a private virtual X display, so it never touches the real
# screen or mouse.
#
#   tools/client/vclient.sh xvfb            start the virtual display (:77) if it is not running
#   tools/client/vclient.sh desktop         graphical manager in the foreground on this instance's private display
#   tools/client/vclient.sh start [--trace] launch yo3.exe on it (trace -> logs/client-stdout.log);
#                                           --trace also logs every field lookup -> logs/fields.tsv
#   tools/client/vclient.sh start-stock    open the original executable using its installed automatic loader
#   tools/client/vclient.sh login [SLOT]    title -> ONLINE -> player SLOT (0-3, default 0); `login select` stops at Select Player
#   tools/client/vclient.sh shot NAME       screenshot the game window -> logs/shots/NAME.png
#   tools/client/vclient.sh click X Y       left click at game coordinates (1024x768, mapped into a larger window)
#   tools/client/vclient.sh key KEY...      xdotool key names, e.g. Return Escape
#   tools/client/vclient.sh type TEXT       type text
#   tools/client/vclient.sh pixel X Y       print the window's colour at X Y as "R G B"
#   tools/client/vclient.sh ui [--raw]      what the client shows, from the connection DLL's UI probe: open components,
#                                           recent UI texts and, in a duel, each field slot (face up / defense)
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
mkdir -p "$LOGS/shots" "$LOGS/ui"
# The connection DLL's UI probe answers `vclient.sh ui` through this folder (tools/mod/ui-probe.h); players never set it.
ui_probe() { YO3_UI_PROBE="$(winepath -w "$LOGS/ui" 2>/dev/null || true)"; export YO3_UI_PROBE; }

win() { xdotool search --name '^Yu-Gi-Oh! ONLINE 3' 2>/dev/null | head -1; }

# The window, or fail (an empty id makes `import` wait for a click and hang).
need_win() {
  local w; w="$(win)"
  [ -n "$w" ] || { echo "no game window on $DISPLAY" >&2; exit 1; }
  echo "$w"
}

# Game coordinates (the game's 1024x768 frame) -> window coordinates. With the connection DLL's higher resolution
# (YO3_RESOLUTION) the window is larger and the frame sits in it scaled to fit and centred (tools/mod/resolution-fit.h).
window_xy() {
  local w=$1 x=$2 y=$3 gw gh
  eval "$(xdotool getwindowgeometry --shell "$w" | sed -n 's/^WIDTH=/gw=/p; s/^HEIGHT=/gh=/p')"
  if (( gw == 1024 && (gh == 768 || gh == 600) )); then echo "$x $y"; return; fi
  local vh=768 cw ch
  if (( gw * vh <= gh * 1024 )); then cw=$gw; ch=$(( (gw * vh * 2 + 1024) / 2048 )); else ch=$gh; cw=$(( (gh * 1024 * 2 + vh) / (2 * vh) )); fi
  echo "$(( (gw - cw) / 2 + (2 * x + 1) * cw / 2048 )) $(( (gh - ch) / 2 + (2 * y + 1) * ch / (2 * vh) ))"
}

pixel() {
  local w; w="$(win)"
  [ -n "$w" ] || return 1
  local xy; xy="$(window_xy "$w" "$1" "$2")"
  import -window "$w" -crop "1x1+${xy% *}+${xy#* }" -depth 8 txt:- 2>/dev/null | tail -1 |
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
  desktop)
    # The graphical manager can start its game here too, always on this instance's private display/prefix.
    echo "$$" >"$LOGS/desktop.pid"
    echo "desktop pid $$ on $DISPLAY"
    WINEDEBUG=fixme-all exec "${YO3_DESKTOP_BIN:-$ROOT/target/release/yo3-revival}" >"$LOGS/desktop.log" 2>&1 ;;
  xvfb)
    # Keep the display alive without holding a caller's captured input/output pipes open.
    xdpyinfo >/dev/null 2>&1 || {
      nohup Xvfb "$DISPLAY" -screen 0 "${YO3_XVFB_SCREEN:-1280x960x24}" -nolisten tcp </dev/null >"$LOGS/xvfb.log" 2>&1 &
      sleep 1
    }
    xdpyinfo | grep dimensions ;;
  start-stock)
    # Installed automatic loader path: no injector or account credentials on this command line.
    if [ ! -f "$GAME/winmm.dll" ] || [ ! -f "$GAME/yo3-revival-loader.json" ]; then
      echo 'Enable direct start in the GUI and wait for setup to complete before start-stock.' >&2
      exit 1
    fi
    ui_probe
    cd "$GAME"
    WINEDLLOVERRIDES=winmm=n,b WINEDEBUG=fixme-all nohup wine "$GAME/yo3.exe" >"$LOGS/client-stdout.log" 2>"$LOGS/client-wine.log" &
    for _ in $(seq 60); do [ -n "$(win)" ] && break; sleep 0.5; done
    echo "window $(win)" ;;
  start)
    PROFILE="${YO3_AUTH_PROFILE:?set YO3_AUTH_PROFILE to the private client.json for this account}"
    ui_probe
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
    WINEDEBUG="${WINEDEBUG:-fixme-all}" nohup "${cmd[@]}" >"$LOGS/client-stdout.log" 2>"$LOGS/client-wine.log" &
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
    wait_for players 30 600 231 38 60 116                     # first player slot
    [ "${2:-}" = select ] && { "$0" shot select; exit 0; }       # `login select` stops at Select Player
    "$0" click 656 $((231 + ${2:-0} * 46))                     # slot rows are 46 px apart
    sleep "${YO3_MAIN_WAIT:-10}"
    "$0" shot login ;;
  shot)
    W="$(need_win)"; import -window "$W" "$LOGS/shots/${2:-shot}.png" && echo "$LOGS/shots/${2:-shot}.png" ;;
  click)
    W="$(need_win)"; xy="$(window_xy "$W" "$2" "$3")"
    xdotool mousemove --window "$W" "${xy% *}" "${xy#* }"; sleep 0.15; xdotool click --delay 120 1 ;;
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
  ui)
    # Ask the probe (a request file it polls every 200 ms) and print its answer.
    need_win >/dev/null
    rm -f "$LOGS/ui/ui.json"; : > "$LOGS/ui/request"
    for _ in $(seq 25); do [ -s "$LOGS/ui/ui.json" ] && break; sleep 0.2; done
    [ -s "$LOGS/ui/ui.json" ] || { echo "no answer from the UI probe (client started before it existed? restart it)" >&2; exit 1; }
    if [ "${2:-}" = "--raw" ]; then cat "$LOGS/ui/ui.json"; else python3 "$ROOT/tools/client/ui_view.py" "$LOGS/ui/ui.json"; fi ;;
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
