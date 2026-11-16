#!/bin/sh
# The YO3 Revival off-machine backup (#37), run daily by yo3-offsite-backup.timer as the server's user: copies the
# newest database backup and the server's identity (server.crt, server.key: players' launchers pin it) elsewhere.
# Configuration: /etc/yo3/offsite.env (offsite.env.example). `init` creates the restic repository once.
# docs/hosting.md, "Off-machine backups".
set -eu
data_dir=${YO3_DATA_DIR:-/var/lib/yo3}
stamp_dir=${YO3_OFFSITE_STATE:-${STATE_DIRECTORY:-/var/lib/yo3-offsite}}
tool=${YO3_OFFSITE:-}
fail() { echo "off-machine backup: $1" >&2; exit 1; }

case "$tool" in
    restic) [ -n "${RESTIC_REPOSITORY:-}" ] || fail "RESTIC_REPOSITORY is not set in /etc/yo3/offsite.env" ;;
    rclone) [ -n "${YO3_RCLONE_REMOTE:-}" ] || fail "YO3_RCLONE_REMOTE is not set in /etc/yo3/offsite.env" ;;
    "") fail "not configured: set YO3_OFFSITE (restic or rclone) and its destination in /etc/yo3/offsite.env" ;;
    *) fail "YO3_OFFSITE is restic or rclone, not $tool" ;;
esac
command -v "$tool" >/dev/null || fail "$tool is not installed"
[ -n "${CACHE_DIRECTORY:-}" ] && export RESTIC_CACHE_DIR="${RESTIC_CACHE_DIR:-$CACHE_DIRECTORY}"

if [ "${1:-}" = init ]; then
    [ "$tool" = restic ] || fail "init is for restic; rclone needs no setup here"
    exec restic init
fi

newest=$(find "$data_dir/backups" -mindepth 2 -maxdepth 2 -name server.db 2>/dev/null | sort | tail -n 1)
[ -n "$newest" ] || fail "no backup in $data_dir/backups yet (the server makes one at start)"
name=$(basename "$(dirname "$newest")")
for f in server.crt server.key; do [ -f "$data_dir/$f" ] || fail "$data_dir/$f is missing"; done

if [ "$tool" = restic ]; then
    restic backup --tag yo3 --host yo3-server "$newest" "$data_dir/server.crt" "$data_dir/server.key"
    restic forget --tag yo3 --host yo3-server --keep-daily 14 --keep-weekly 8 --prune
else
    stage=$(mktemp -d)
    trap 'rm -rf "$stage"' EXIT
    cp "$newest" "$data_dir/server.crt" "$data_dir/server.key" "$stage/"
    rclone copy "$stage" "$YO3_RCLONE_REMOTE/$name"
    rclone delete --min-age "${YO3_RCLONE_KEEP_DAYS:-30}d" "$YO3_RCLONE_REMOTE"
    rclone rmdirs --leave-root "$YO3_RCLONE_REMOTE"
fi
mkdir -p "$stamp_dir"
date +%s > "$stamp_dir/last-success"
echo "off-machine backup of $name done ($tool)"
