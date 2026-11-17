#!/bin/sh
set -eu
# Also used by offline init/restore and docker compose exec's explicit yo3-server command.
# No database edits or implicit identity creation at startup.
if [ "${YO3_HTTPS_PORT:-443}" = 443 ]; then YO3_DASHBOARD_ORIGIN="https://${YO3_DOMAIN:?}"; fi
if [ -n "${YO3_DASHBOARD_ORIGIN:-}" ]; then
    set -- --dashboard-origin "$YO3_DASHBOARD_ORIGIN" "$@"
fi
if [ -n "${YO3_DOMAIN:-}" ]; then
    set -- --advertise "$YO3_DOMAIN" "$@"
fi
exec /opt/yo3/yo3-server --config /etc/yo3/server.toml "$@"
