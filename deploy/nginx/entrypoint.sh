#!/bin/sh
set -eu
case "${YO3_DOMAIN:-}" in ''|*[!a-zA-Z0-9.-]*) echo 'YO3_DOMAIN must be a DNS name (or localhost for a local check).' >&2; exit 1;; esac
case "${YO3_HTTPS_PORT:-443}" in *[!0-9]*|'') echo 'YO3_HTTPS_PORT must be a port number.' >&2; exit 1;; esac
[ "${YO3_HTTPS_PORT:-443}" -ge 1 ] && [ "${YO3_HTTPS_PORT:-443}" -le 65535 ]
YO3_AUTHORITY=$YO3_DOMAIN
if [ "${YO3_HTTPS_PORT:-443}" != 443 ]; then YO3_AUTHORITY="$YO3_DOMAIN:$YO3_HTTPS_PORT"; fi
export YO3_AUTHORITY
# Substitute only our variable; preserve nginx's $remote_addr, $uri, etc.
# shellcheck disable=SC2016
envsubst '${YO3_AUTHORITY}' < /etc/nginx/templates/dashboard.conf.template > /tmp/dashboard.conf
nginx -t
exec nginx -g 'daemon off;'
