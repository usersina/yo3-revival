# Linux VPS with Docker Compose

Run the Rust server/native duel engine and the React/nginx dashboard as separate services. This arrived in **v0.1.0**. The dashboard is public HTTPS with staff password sign-in, as requested for
VPS testing; **MFA is not implemented yet**. The old HTML panel and host-token API remain container-local. A web
session never grants host authority. Public database downloads are disabled; use SSH for backups.

Want one setup command instead? [Local Ansible setup](deployment.md) uses these same runtime files.

## What you need

- A Linux VPS with Docker Engine and the Compose plugin, Git, OpenSSL and Certbot. Use the
  [official Docker installation instructions](https://docs.docker.com/engine/install/). Task is optional.
- A domain whose A record points to the VPS. Add an AAAA record only if IPv6 really reaches the same service.
- TCP **7443** for the game, **80** for certificate challenges/HTTPS redirects and **443** for the dashboard, allowed
  in the provider firewall. Keep SSH available. Docker publishes its own ports; do not rely only on a host UFW rule.
  [Docker port-publishing behavior](https://docs.docker.com/engine/network/port-publishing/).
- Your own game's `yo3.idx` and `package/data002.ypk`. For card previews, also copy `package/data000.ypk` and
  `package/data001.ypk` (about 574 MB together). All stay mounted read-only, outside the images.
- Enough space for a source build and backups. The first Rust/native-engine build is heavier than running the server;
  it uses two compilation jobs by default. Neither Node.js nor Rust needs installing on the VPS.

Use a DNS name such as `play.example.com` below. The same name is advertised to game clients. Do not put it behind a
normal HTTP-only CDN proxy: the game speaks its own TLS protocol on 7443.

## 1. Prepare the checkout and game files

```sh
sudo install -d -o "$USER" -g "$(id -gn)" /opt/yo3-revival
git clone --branch main https://github.com/usersina/yo3-revival.git /opt/yo3-revival
cd /opt/yo3-revival
cp deploy/.env.example .env
sudo install -d -m 0755 /srv/yo3/game/package /srv/yo3/acme
sudo install -d -o 10001 -g 10001 -m 0750 /srv/yo3/tls
```

Copy the required game files from your PC, keeping this layout. Include the two optional image archives for card previews:

```text
/srv/yo3/game/
├── yo3.idx
└── package/
    ├── data000.ypk  # Optional: card thumbnails
    ├── data001.ypk  # Optional: full card images
    └── data002.ypk
```

Rust serves individual card images through authenticated same-origin HTTPS routes. No bucket or additional
image service is required. Missing artwork leaves gifts usable. After replacing/adding archives on an existing
host, restart the server during maintenance so its cached index and image revisions refresh. See
[card previews and caching](dashboard.md#staff-interface).

They need to be readable by container UID 10001 (`0644` files, `0755` parent directories). Edit `.env`: set
`YO3_DOMAIN`, the three absolute paths, and the version/commit for the build. For example:

```dotenv
YO3_DOMAIN=play.example.com
YO3_GAME_DIR=/srv/yo3/game
YO3_TLS_DIR=/srv/yo3/tls
YO3_ACME_DIR=/srv/yo3/acme
YO3_IMAGE_TAG=beta-test
YO3_BUILD_VERSION=beta-test
YO3_BUILD_COMMIT=REPLACE_WITH_CHECKED_OUT_COMMIT
```

`git rev-parse --short=12 HEAD` supplies the last value. `.env` is ignored by Git. Custom host ports and loopback-only
bindings are shown in the example. Public access is the default; the local management ports are never published.

```sh
docker compose config --quiet
docker compose build
```

The build fetches the pinned EDOPro/Lua sources and card data, builds both binaries, and packages the engine data and
licenses using the same script as releases. Base images are digest-pinned. The dashboard build uses `npm ci` and the
committed generated API client. Application images run as UID/GID **10001**, with read-only roots and no capabilities.

## 2. Issue the HTTPS certificate

The web certificate is separate from the game's identity. Certbot owns the certificate under `/etc/letsencrypt`;
nginx receives only an exported certificate/key pair, not Certbot's account credentials or Docker's socket.

Start the temporary HTTP challenge service; it needs no existing certificate or running game server:

```sh
docker compose -f compose.yaml -f deploy/compose.acme.yaml --profile certificate up -d --no-deps acme
sudo certbot certonly --webroot -w /srv/yo3/acme \
  --cert-name play.example.com -d play.example.com \
  --deploy-hook '/opt/yo3-revival/deploy/install-certificate.sh play.example.com'
docker compose -f compose.yaml -f deploy/compose.acme.yaml --profile certificate stop acme
docker compose -f compose.yaml -f deploy/compose.acme.yaml --profile certificate rm -f acme
```

Certbot asks for your contact email and agreement to its terms. Its saved deploy hook validates the certificate/key,
exports it with the required ownership/permissions, and reloads a running nginx after renewal. If you changed
`YO3_TLS_DIR`, pass the same directory as the hook's second argument. Keep port 80 reachable: the full dashboard
continues serving the same challenge webroot once started. [Certbot webroot and renewal hooks](https://eff-certbot.readthedocs.io/en/stable/using.html).

If issuance fails, check DNS, stale AAAA records, both firewalls and whether another service owns port 80. Retry
Certbot after correcting the cause. Do not repeatedly force issuance; Let's Encrypt applies rate limits.

## 3. Initialize and start

Generate the **game** identity once, using every DNS name/IP players will use:

```sh
docker compose run --rm --no-deps server init-auth play.example.com YOUR_PUBLIC_IP
docker compose up -d --wait
```

`init-auth` refuses to overwrite an existing identity. Keep its `server.key`: players pin this identity, so losing it
changes what they trust. Rebuilding or recreating containers preserves it and SQLite in the named `data` volume.
The named `staff-socket` volume contains only the staff API socket/lock. nginx mounts it read-only and has no access
to `data`, game files, the game private key or the host-token management API. The two containers use ordinary Compose
networking; neither uses host networking or shares a network namespace.

Open **https://play.example.com/**. Use an admin/moderator password account. To create the first account:

```sh
docker compose exec server /opt/yo3/yo3-server --config /etc/yo3/server.toml add-user YourUserID
docker compose exec server /opt/yo3/yo3-server --config /etc/yo3/server.toml share
```

On your PC, add that server in Desktop and choose **Forgot password** with the printed setup code to choose your
password. Save the recovery code. Then promote the account:

```sh
docker compose exec server /opt/yo3/yo3-server --config /etc/yo3/server.toml set-role YourUserID admin
```

Finish password setup before promotion; role changes invalidate outstanding setup codes. Give friends invites through
the dashboard or `invite --uses 5 --days 7`. The [account commands](guide.md#account-commands) work with the same
`docker compose exec server /opt/yo3/yo3-server --config /etc/yo3/server.toml` prefix.

Task is optional and has exactly five convenience tasks: `task build`, `task start`, `task stop`, `task status` and
`task logs`. The equivalent Compose commands are `build`, `up -d --wait`, `stop`, `ps` and `logs --tail=100 --follow`.
`stop` preserves volumes. Use Compose/Task to stop a deployment: the dashboard's process Stop can be restarted by
Docker's `unless-stopped` policy. **Do not use `docker compose down --volumes` on a deployment.**

## HTTPS renewal and health

Enable your distro's Certbot renewal timer, then verify a staging renewal. The ordinary dry run tests renewal;
the saved deploy hook runs after successful real renewals. You can also run the hook manually to verify export/reload.

```sh
sudo systemctl enable --now certbot.timer
sudo certbot renew --dry-run
systemctl list-timers certbot.timer
sudo journalctl -u certbot.service --since '1 day ago'
sudo deploy/check-https.sh play.example.com
```

The server container checks `/health` internally: a missing engine or persistence failure makes it unhealthy.
nginx checks HTTP readiness and at least seven days remaining on its certificate. Docker reports unhealthy
containers but does **not** restart a still-running process solely for a failed health check. Check `docker compose
ps`, logs and external HTTPS/TCP monitoring; `restart: unless-stopped` covers process exits and host reboots.
For a scheduled HTTPS/expiry check:

```sh
sudo cp deploy/systemd/yo3-https@.{service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now yo3-https@play.example.com.timer
sudo journalctl -u yo3-https@play.example.com.service
```

The template assumes `/opt/yo3-revival` and `/srv/yo3/tls`; adjust it if using other paths. Connect service failures to
your own monitoring alerts. Check both the deployed expiry and renewal failures, rather than waiting for browsers to
refuse access. If renewal succeeded but nginx still serves the old certificate, verify `docker compose exec dashboard nginx -t`,
then re-run the certificate export hook (it retries reload even when the files are unchanged), and inspect the dashboard logs.

nginx overwrites forwarded client/protocol headers, clears `Forwarded` and `Authorization`, checks the exact web
host and proxies only listed staff routes. Rust checks that origin again, validates CSRF and uses the single forwarded
client IP for its sign-in throttle. TCP management never trusts those headers. nginx adds static-file CSP/HSTS; Rust
owns API security headers. CSP permits inline **styles** for the existing React UI library, never inline scripts.
No automatic upstream mutation retries are enabled. The remaining security work is [MFA and recovery](roadmap.md#open-now).

## Backups, restore and updates

Create a checked snapshot and list its name:

```sh
docker compose exec server /opt/yo3/yo3-server --config /etc/yo3/server.toml backup
docker compose exec server /opt/yo3/yo3-server --config /etc/yo3/server.toml backups
```

The dashboard also creates/lists snapshots. It explains that public downloads are disabled. Copy a selected snapshot
with `docker compose cp server:/var/lib/yo3/backups/NAME/server.db ./server.db`, then copy it **off the VPS**. Keep it
private: it includes password hashes and audit history. Also back up `server.crt`, `server.key` and your deployment
configuration. A second copy on the same disk does not survive losing the VPS. [Off-machine backup options](hosting.md#off-machine-backups).

Restore is offline and exclusively locks the data directory. The server container must be stopped; the dashboard
can stay up and will report the unavailable server:

```sh
docker compose stop server
docker compose run --rm --no-deps server restore NAME
docker compose up -d --wait server
```

Use the exact snapshot name from `backups`. Restore keeps the replaced database as a safety copy and does not replace
the game identity. Rehearse this before depending on the VPS. A restore can roll back grants and receipts made after
the snapshot; avoid accepting player changes during a rehearsal.

For an update, announce maintenance, take/copy a snapshot and preserve the current game identity before changing
source or images. Update the checkout and build metadata in `.env`, then:

```sh
docker compose build
docker compose stop
docker compose up -d --wait
```

The server receives SIGTERM and has 60 seconds to finish duels, save and take its final backup. New SQLite schemas
may refuse older binaries; rollback means restoring a compatible snapshot, not just switching the image tag. When
all checks pass, cancel maintenance. Never delete the data volume as an update step.

## Repeat the deployment rehearsal

The local/VPS rehearsal uses the real images and read-only game assets, but a **randomly named disposable Compose
project**, temporary volumes and unused loopback ports. It verifies its own self-signed localhost certificate and
never contacts a live server. Build the images with `YO3_IMAGE_TAG=local` first, then:

```sh
python3 tools/containers/test_config.py
python3 tools/containers/test_stack.py --game-dir /srv/yo3/game
```

It exercises real game account registration, HTTPS staff sign-in, nginx header spoofing/route restrictions, CSRF,
retry-safe BP grants, persistent data, live-restore refusal, offline restore, unchanged game identity, read-only game
files, HTTP redirects and ACME. `--keep` retains only its own temporary project for browser inspection and prints its
address; it does not keep production data. It otherwise removes only its own test containers/volumes. This rehearsal
needs game assets and is manual; the lightweight configuration and Rust socket contracts can run without them.
