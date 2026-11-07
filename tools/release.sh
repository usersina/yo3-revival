#!/bin/sh
# Builds the host's bundle: dist/yo3-revival-<version>-linux-x86_64.tar.gz with the server, the rules engine's
# sources (its packages, scripts and card database are fetched on the host by engine/fetch.sh), a launcher and the
# setup guide. No game files go in: the server reads the host's own game install.
set -e
cd "$(dirname "$0")/.."
version=$(git describe --tags --always --dirty 2>/dev/null || echo dev)
name="yo3-revival-$version-linux-x86_64"
out="dist/$name"
cargo build --release -p yo3-server
rm -rf "$out"
mkdir -p "$out/engine"
cp target/release/yo3-server "$out/"
cp engine/duelcore.ts engine/package.json engine/package-lock.json engine/fetch.sh "$out/engine/"
cp docs/friends.md "$out/README.md"
cat > "$out/run.sh" <<'EOF'
#!/bin/sh
# Starts the server for friends: every network interface, accounts and records in ./data (backed up at each start).
# Extra options go to the server, e.g. ./run.sh --advertise my.host.name   (see ./yo3-server --help)
cd "$(dirname "$0")"
[ -s engine/vendor/cardinfo.json ] || engine/fetch.sh
exec ./yo3-server --bind 0.0.0.0 --data-dir data --log-dir logs "$@"
EOF
chmod +x "$out/run.sh" "$out/engine/fetch.sh"
tar -C dist -czf "dist/$name.tar.gz" "$name"
echo "dist/$name.tar.gz"
