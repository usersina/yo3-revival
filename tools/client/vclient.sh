#!/usr/bin/env bash
# Drive the game client on a private virtual X display, so it never touches the real
# screen or mouse.
#
#   tools/vclient.sh xvfb            start the virtual display (:77) if it is not running
#   tools/vclient.sh start           launch yo3.exe on it (trace -> logs/client-stdout.log)
#   tools/vclient.sh shot NAME       screenshot the game window -> logs/shots/NAME.png
#   tools/vclient.sh click X Y       left click at game-window coordinates (1024x768)
#   tools/vclient.sh key KEY...      xdotool key names, e.g. Return Escape
#   tools/vclient.sh type TEXT       type text
#   tools/vclient.sh login           splash -> ONLINE -> OK (saved credentials) -> first player
#   tools/vclient.sh stop            close the game (wineserver -k)
set -euo pipefail

export DISPLAY="${YO3_DISPLAY:-:77}"
unset WAYLAND_DISPLAY
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GAME="${YO3_GAME_DIR:-$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3}"
LOGS="$ROOT/logs"
mkdir -p "$LOGS/shots"

win() { xdotool search --name '^Yu-Gi-Oh! ONLINE 3' | head -1; }

case "${1:-}" in
  xvfb)
    xdpyinfo >/dev/null 2>&1 || { Xvfb "$DISPLAY" -screen 0 1280x960x24 -nolisten tcp & sleep 1; }
    xdpyinfo | grep dimensions ;;
  start)
    cd "$GAME"
    WINEDEBUG=fixme-all nohup wine yo3.exe >"$LOGS/client-stdout.log" 2>"$LOGS/client-wine.log" &
    for _ in $(seq 60); do [ -n "$(win)" ] && break; sleep 0.5; done
    echo "window $(win)" ;;
  shot)
    import -window "$(win)" "$LOGS/shots/${2:-shot}.png" && echo "$LOGS/shots/${2:-shot}.png" ;;
  click)
    W="$(win)"; xdotool mousemove --window "$W" "$2" "$3"; sleep 0.15; xdotool click --delay 120 1 ;;
  key)
    shift; xdotool key --delay 80 "$@" ;;
  type)
    xdotool type --delay 60 "$2" ;;
  login)
    sleep "${YO3_BOOT_WAIT:-12}"   # software rendering: the client needs a while to boot
    "$0" click 512 384; sleep 4    # skip the KONAMI splash
    "$0" click 297 301; sleep 4    # ONLINE
    "$0" click 402 459; sleep 4    # OK on the saved login
    "$0" click 656 231; sleep 6    # first player slot
    "$0" shot login ;;
  stop)
    wineserver -k || true ;;
  *)
    sed -n '2,12p' "$0"; exit 2 ;;
esac
