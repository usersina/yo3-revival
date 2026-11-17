#!/bin/sh
# Certbot deploy hook. Run as root: deploy/install-certificate.sh DOMAIN [DESTINATION]
set -eu
umask 077
if [ "$(id -u)" != 0 ]; then echo 'Run this certificate hook as root.' >&2; exit 1; fi
domain=${1:?Usage: install-certificate.sh DOMAIN [DESTINATION]}
case "$domain" in ''|*[!a-zA-Z0-9.-]*) echo 'Invalid DNS name.' >&2; exit 1;; esac
dest=${2:-/srv/yo3/tls}
source_dir="/etc/letsencrypt/live/$domain"
openssl x509 -in "$source_dir/fullchain.pem" -noout -checkend 604800
openssl x509 -in "$source_dir/fullchain.pem" -noout -checkhost "$domain"
cert_public=$(openssl x509 -in "$source_dir/fullchain.pem" -pubkey -noout | openssl pkey -pubin -outform DER | openssl dgst -sha256)
key_public=$(openssl pkey -in "$source_dir/privkey.pem" -pubout -outform DER | openssl dgst -sha256)
[ "$cert_public" = "$key_public" ] || { echo 'Certificate and key do not match.' >&2; exit 1; }
install -d -o 10001 -g 10001 -m 0750 "$dest"
# Renewal and an Ansible rerun can coincide. Serialize export/reload, including the two file renames.
exec 9>"$dest/.certificate.lock"
flock 9
if ! cmp -s "$source_dir/fullchain.pem" "$dest/fullchain.pem" || ! cmp -s "$source_dir/privkey.pem" "$dest/privkey.pem"; then
    trap 'rm -f "$dest/.fullchain.new" "$dest/.privkey.new"' EXIT
    install -o 10001 -g 10001 -m 0644 "$source_dir/fullchain.pem" "$dest/.fullchain.new"
    install -o 10001 -g 10001 -m 0600 "$source_dir/privkey.pem" "$dest/.privkey.new"
    mv -f "$dest/.fullchain.new" "$dest/fullchain.pem"
    mv -f "$dest/.privkey.new" "$dest/privkey.pem"
    echo 'Certificate exported'
fi
chown 10001:10001 "$dest/fullchain.pem" "$dest/privkey.pem"
chmod 0644 "$dest/fullchain.pem"
chmod 0600 "$dest/privkey.pem"
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo"
# Retry a reload even with unchanged files: a previous run may have exported successfully then lost the reload.
if [ -n "$(docker compose ps --status running --quiet dashboard)" ]; then
    docker compose exec -T dashboard nginx -t
    docker compose exec -T dashboard nginx -s reload
fi
