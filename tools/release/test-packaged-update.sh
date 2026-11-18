#!/usr/bin/env bash
# Exercise the real release archive on an owned display and fresh application data; never publishes anything.
#
#   tools/release/test-packaged-update.sh VERSION DESKTOP-LINUX.tar.gz [--from RELEASED-DESKTOP-LINUX.tar.gz]
#
# 1. Signed updates (#32): SHA256SUMS over the archive, signed by the Release workflow's own script
#    (.github/scripts/sign_release.sh) with a key made here for this run only; the launcher's check accepts it and
#    refuses a changed signature, another key, a missing SHA256SUMS.sig and a download the manifest doesn't list.
# 2. The archive's launcher, staged and installed at the next start, opens its window before it becomes active.
# 3. With --from (the latest Desktop archive, e.g. `gh release download -p '*desktop*linux*'`): that release's
#    launcher makes its data, this release opens on it and is promoted, then the released binary (the player's original
#    shortcut) must hand over to the installed update.
set -euo pipefail
usage() { echo 'usage: tools/release/test-packaged-update.sh VERSION DESKTOP-LINUX.tar.gz [--from RELEASED-DESKTOP-LINUX.tar.gz]' >&2; exit 2; }
[[ $# == 2 || ( $# == 4 && "$3" == --from ) ]] || usage
export YO3_UPDATE_TEST_TAG="$1"
export YO3_UPDATE_TEST_ARCHIVE YO3_UPDATE_TEST_FROM=""
YO3_UPDATE_TEST_ARCHIVE="$(realpath "$2")"
[[ -f "$YO3_UPDATE_TEST_ARCHIVE" ]] || { echo "Archive not found: $YO3_UPDATE_TEST_ARCHIVE" >&2; exit 2; }
if [[ $# == 4 ]]; then
  YO3_UPDATE_TEST_FROM="$(realpath "$4")"
  [[ -f "$YO3_UPDATE_TEST_FROM" ]] || { echo "Released archive not found: $YO3_UPDATE_TEST_FROM" >&2; exit 2; }
fi
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
export YO3_UPDATE_TEST_ROOT="$scratch/app"

# The release's signed manifest, as the workflow makes it, with a throwaway key (never the project's).
export YO3_UPDATE_TEST_SIGNED="$scratch/signed" YO3_UPDATE_TEST_PUBLIC_KEY
mkdir -p "$YO3_UPDATE_TEST_SIGNED"
(umask 077 && openssl genpkey -algorithm ed25519 -out "$scratch/test-key.pem" 2>/dev/null)
YO3_UPDATE_TEST_PUBLIC_KEY="$(openssl pkey -in "$scratch/test-key.pem" -pubout -outform DER | tail -c 32 | od -An -v -tx1 | tr -d ' \n')"
(cd "$(dirname "$YO3_UPDATE_TEST_ARCHIVE")" && sha256sum "$(basename "$YO3_UPDATE_TEST_ARCHIVE")") > "$YO3_UPDATE_TEST_SIGNED/SHA256SUMS"
"$ROOT/.github/scripts/sign_release.sh" "$YO3_UPDATE_TEST_SIGNED" "$scratch/test-key.pem" "$YO3_UPDATE_TEST_PUBLIC_KEY"
if "$ROOT/.github/scripts/sign_release.sh" "$YO3_UPDATE_TEST_SIGNED" "$scratch/test-key.pem" 2>/dev/null; then
  echo "The signing script signed with a key that isn't Desktop's" >&2; exit 1
fi
"$ROOT/.github/scripts/sign_release.sh" "$YO3_UPDATE_TEST_SIGNED" "$scratch/test-key.pem" "$YO3_UPDATE_TEST_PUBLIC_KEY" >/dev/null
rm -f "$scratch/test-key.pem"

filter='test(=packaged_update_is_signed_and_opens_its_window_before_activation)'
[[ -n "$YO3_UPDATE_TEST_FROM" ]] && filter="$filter | test(=a_released_launcher_updates_to_this_release_and_its_shortcut_reaches_it)"
export filter
cd "$ROOT"
xvfb-run --auto-servernum --server-num=97 --server-args='-screen 0 1280x960x24 -nolisten tcp' sh -c '
  unset WAYLAND_DISPLAY
  export YO3_UPDATE_TEST_DISPLAY="$DISPLAY"
  cargo nextest run -p yo3-desktop --test update_activation \
    --run-ignored only --no-tests fail --test-threads 1 \
    -E "$filter" \
    --success-output immediate
'
