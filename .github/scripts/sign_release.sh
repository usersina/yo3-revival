#!/usr/bin/env bash
# Signs a release's SHA256SUMS (#32): DIR/SHA256SUMS.sig, Ed25519 over the file's bytes (64 raw bytes), then checks
# that signature with the public key Desktop has built in, so a release Desktop would refuse is never published.
#
#   sign_release.sh DIR KEY_FILE [PUBLIC_KEY_HEX]
#
# KEY_FILE is an Ed25519 private key in PEM (the Release workflow writes the UPDATE_SIGNING_KEY secret to a private
# temporary file and removes it afterwards). PUBLIC_KEY_HEX (64 hex digits) defaults to the key in
# crates/yo3-desktop/src/updates/signature.rs; tools/test-packaged-update.sh passes the key it made for the test.
# Prints nothing secret: only the public key and the result.
set -euo pipefail
if [[ $# -lt 2 || $# -gt 3 ]]; then echo 'usage: sign_release.sh DIR KEY_FILE [PUBLIC_KEY_HEX]' >&2; exit 2; fi
dir="$1" key="$2"
root="$(cd "$(dirname "$0")/../.." && pwd)"
if [[ $# == 3 ]]; then
  public="$3"
else
  # The bytes between `PUBLIC_KEY: Option<[u8; 32]> = Some([` and `]);`, as hex.
  public="$(sed -n '/pub const PUBLIC_KEY: Option<\[u8; 32\]> = Some(\[/,/\]);/p' "$root/crates/yo3-desktop/src/updates/signature.rs" \
    | grep -o '0x[0-9a-fA-F]\{2\}' | sed 's/0x//' | tr -d '\n' | tr 'A-F' 'a-f')"
fi
[[ "$public" =~ ^[0-9a-f]{64}$ ]] || { echo "No valid public key to check against (got '${public}')" >&2; exit 1; }
[[ -f "$dir/SHA256SUMS" ]] || { echo "$dir/SHA256SUMS is missing" >&2; exit 1; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
# The key's own public half must be the one Desktop checks against: a wrong secret fails here, not on players' machines.
openssl pkey -in "$key" -pubout -outform DER -out "$scratch/own.der"
own="$(tail -c 32 "$scratch/own.der" | od -An -v -tx1 | tr -d ' \n')"
[[ "$own" == "$public" ]] || { echo "The signing key's public key ($own) is not Desktop's ($public)" >&2; exit 1; }

openssl pkeyutl -sign -rawin -inkey "$key" -in "$dir/SHA256SUMS" -out "$dir/SHA256SUMS.sig"
[[ "$(stat -c %s "$dir/SHA256SUMS.sig")" == 64 ]] || { echo "The signature is not 64 bytes" >&2; exit 1; }

# Checked with the public key alone (an Ed25519 SubjectPublicKeyInfo: a fixed 12-byte header, then the key).
python3 -c 'import sys; sys.stdout.buffer.write(bytes.fromhex(sys.argv[1]))' "302a300506032b6570032100$public" > "$scratch/public.der"
openssl pkeyutl -verify -rawin -pubin -keyform DER -inkey "$scratch/public.der" \
  -in "$dir/SHA256SUMS" -sigfile "$dir/SHA256SUMS.sig" >/dev/null
echo "SHA256SUMS signed and verified with $public"
