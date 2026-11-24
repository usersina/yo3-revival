#!/bin/sh
# tools/engine/test-shipped.sh BUNDLE: the server's and the engine's tests against the rules engine a release bundle
# ships (BUNDLE is an extracted Linux bundle: its engine/ folder, pruned by tools/engine/engine-scripts.py, and its
# yo3-engine program), so a card script the bundle lacks fails the release instead of a player's duel. The test
# harness's own files that no bundle carries (engine/client-tokens.tsv, the added/ and anime/ tables...) are linked in
# beside the bundle's in a scratch folder; the bundle itself is not changed. Run from the repository root; arguments after
# BUNDLE go to cargo nextest (--release).
case "${1:-}" in ""|-h|--help) awk 'NR>1 && /^#/{sub(/^# ?/,""); print; next} NR>1{exit}' "$0"; [ -n "${1:-}" ]; exit $? ;; esac
set -e
bundle=$(cd "$1" && pwd)
[ -d "$bundle/engine/vendor" ] && [ -x "$bundle/yo3-engine" ] || { echo "test-shipped.sh: $bundle is no extracted Linux bundle" >&2; exit 2; }
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
for f in "$bundle"/engine/*; do ln -s "$f" "$scratch/"; done
for f in "$PWD"/engine/*; do [ -e "$scratch/$(basename "$f")" ] || ln -s "$f" "$scratch/"; done
# The bundle's program inside the engine folder (engine::program takes one there), not YO3_ENGINE, which would also
# replace the stand-in engines the robustness tests make.
ln -s "$bundle/yo3-engine" "$scratch/yo3-engine"
shift
YO3_ENGINE_DIR="$scratch" cargo nextest run -p yo3-server -p yo3-engine "$@"
