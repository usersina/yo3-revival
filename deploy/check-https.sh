#!/bin/sh
# Local check for the timer/monitor: certificate expires in >7 days and the real HTTPS endpoint answers.
set -eu
domain=${1:?Usage: check-https.sh DOMAIN [TLS_DIRECTORY]}
case "$domain" in ''|*[!a-zA-Z0-9.-]*) echo 'Invalid DNS name.' >&2; exit 1;; esac
openssl x509 -in "${2:-/srv/yo3/tls}/fullchain.pem" -noout -checkend 604800
curl --fail --silent --show-error --max-time 15 "https://$domain/" > /dev/null
