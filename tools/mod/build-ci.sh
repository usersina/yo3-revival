#!/usr/bin/env bash
# tools/mod/build.sh as the release builds it, into tools/mod/build-ci/: Ubuntu's mingw-w64 in Docker, which links
# msvcrt.dll. A local Arch mingw-w64 links the Universal CRT instead, which Windows 7 lacks, so use this to test a C
# helper on a real Windows VM. YO3_BUILD_VERSION and YO3_BUILD_COMMIT pass through: give a release's (v0.1.0,
# c0c497b206a0) to swap one file into that release's bundle for a test.
set -euo pipefail
cd "$(dirname "$0")/../.."
image=${YO3_MOD_CI_IMAGE:-ubuntu:26.04}
out=tools/mod/build-ci
rm -rf "$out" && mkdir -p "$out"
commit=${YO3_BUILD_COMMIT:-$(git rev-parse --short=12 HEAD)}
docker run --rm -e YO3_BUILD_VERSION -e YO3_BUILD_COMMIT="$commit" -e OWNER="$(id -u):$(id -g)" \
  -v "$PWD:/src:ro" -v "$PWD/$out:/out" "$image" sh -euc '
    apt-get update -qq >/dev/null
    DEBIAN_FRONTEND=noninteractive apt-get install -yqq --no-install-recommends mingw-w64 python3 >/dev/null
    mkdir -p /work/crates/yo3-tunnel/src /work/tools
    cp /src/Cargo.toml /work/ && cp /src/crates/yo3-tunnel/src/lib.rs /work/crates/yo3-tunnel/src/
    cp -r /src/tools/mod /work/tools/ && rm -rf /work/tools/mod/build /work/tools/mod/build-ci
    /work/tools/mod/build.sh
    cp -r /work/tools/mod/build/. /out/ && chown -R "$OWNER" /out'
echo "Built into $out/ (msvcrt.dll, as released)"
