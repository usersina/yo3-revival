#!/usr/bin/env bash
# Exercise the real release archive on an owned display and fresh application data; never publishes anything.
set -euo pipefail
if [[ $# != 2 ]]; then
  echo 'usage: tools/test-packaged-update.sh VERSION DESKTOP-LINUX.tar.gz' >&2
  exit 2
fi
export YO3_UPDATE_TEST_TAG="$1"
export YO3_UPDATE_TEST_ARCHIVE
YO3_UPDATE_TEST_ARCHIVE="$(realpath "$2")"
[[ -f "$YO3_UPDATE_TEST_ARCHIVE" ]] || { echo "Archive not found: $YO3_UPDATE_TEST_ARCHIVE" >&2; exit 2; }
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
export YO3_UPDATE_TEST_ROOT="$scratch/app"
cd "$ROOT"
xvfb-run --auto-servernum --server-num=97 --server-args='-screen 0 1280x960x24 -nolisten tcp' sh -c '
  unset WAYLAND_DISPLAY
  export YO3_UPDATE_TEST_DISPLAY="$DISPLAY"
  cargo nextest run -p yo3-desktop --test update_activation \
    --run-ignored only --no-tests fail \
    -E "test(=packaged_update_opens_its_window_before_activation)" \
    --success-output immediate
'
