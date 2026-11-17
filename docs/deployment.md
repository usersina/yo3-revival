# One-time VPS setup

Run Ansible **from your local checkout**, supplying the VPS IP and a DNS name pointing to it. It sets up Docker,
HTTPS, the server and dashboard. After the one-time deployment-key setup below, a new changelog/Cargo version
pushed or merged to `main` runs checks, publishes the release and its images, then updates this VPS automatically. This arrived in v0.1.0. Public password sign-in is enabled for
VPS testing; MFA and recovery are still pending.

## Before running

- A fresh **x86_64 Ubuntu/Debian VPS**, reachable over SSH as root or a user with sudo. Verify its SSH host key on
  your first connection. Python 3 must be installed on the VPS.
- A lowercase DNS name with an A record pointing to that VPS. Remove a stale AAAA record if IPv6 is not configured.
  Allow SSH and TCP **7443, 80 and 443** in the provider's firewall. Use DNS-only records: the game port is not HTTP.
- Locally: Linux x86_64, Docker with Compose and enough space to build both images, Python and Ansible.
- Your original game installation. The playbook copies only `yo3.idx` and `package/data002.ypk` privately to the VPS;
  they never go into Git, GitHub or the container images.

Install the pinned Ansible version in your preferred Python environment, then its Docker collection:

```sh
python3 -m venv ~/.venvs/yo3-ansible
. ~/.venvs/yo3-ansible/bin/activate
python3 -m pip install ansible-core==2.22.0
ansible-galaxy collection install -r deploy/ansible/requirements.yml
```

## Set it up

From the repository root, replace the example IP and DNS name:

```sh
export YO3_GAME_SOURCE_DIR='/path/to/Yu-Gi-Oh! ONLINE 3'
ansible-playbook -i '203.0.113.10,' -u root deploy/ansible/site.yml \
  -e yo3_domain=play.example.com
```

The comma after the IP makes it an inline inventory. The game path defaults to the usual Wine installation when
`YO3_GAME_SOURCE_DIR` is omitted. For a sudo user, change `-u root`; use `--ask-become-pass` if sudo requires a password.
SSH uses your normal agent/config. No GitHub Secrets are needed for this local setup.

The playbook builds the current checkout **on your machine**, exports images by dedicated versioned tags and installs Docker
and Compose on the VPS, configures the host firewall without resetting existing rules, obtains an HTTPS certificate,
sets up renewal/reload checks, creates the game identity and first administrator, and waits for real HTTPS health.
The VPS does not need Rust, Node or your source checkout.

Open **`https://play.example.com/`**, then sign in as **`HostAdmin`**. The generated password is in the local ignored
file **`deploy/ansible/.bootstrap-password-play.example.com`**. Keep it in your password manager. To choose the
initial password instead, supply `YO3_BOOTSTRAP_PASSWORD` through your shell's secret environment: 8–16 letters/digits,
uncommon. It is passed through stdin, hidden from Ansible output, and never set in the runtime containers.

Optional settings are listed in [options.example.yml](https://github.com/usersina/yo3-revival/blob/main/deploy/ansible/options.example.yml): ACME contact email,
game path and admin User ID. Pass overrides with `-e`, or copy that file to ignored `deploy/ansible/config.yml` and
add `-e @deploy/ansible/config.yml`. Email is optional; renewal runs through the host's Certbot timer either way.

## Reruns and operations

This is first setup, **not an update command**. A completed rerun reuses the saved image references and domain; it does not
rebuild, change the password or replace the game identity. An interrupted first setup can be retried from the same
checkout. The admin bootstrap has a durable receipt and refuses to adopt an existing account database, re-promote a
demoted account or replace a deleted admin. Use the local administrative commands for recovery.

Runtime files are under `/opt/yo3-revival`, game assets and HTTPS files under `/srv/yo3`. The database and game
identity are in the Compose data volume. From `/opt/yo3-revival`, `sudo docker compose ps` and
`sudo docker compose logs --tail=100` inspect the stack. [Docker operations](docker.md) covers backups, restore,
graceful shutdown and the manual equivalent of setup. Off-machine backup storage and alert destinations still need
to be chosen; first provisioning does not configure them.

- **Certificate issuance fails:** check DNS/AAAA and provider firewall rules, then retry. No plain HTTP password
  screen is exposed while the certificate is missing.
- **DNS was just changed:** save/apply the record in the DNS provider's panel, then check its authoritative
  nameservers. Recursive resolvers may retain the previous address until its old TTL expires. Once ACME succeeds,
  setup checks local nginx with the domain's hostname and normal CA validation, so a stale VPS resolver cannot block
  completion. Browsers and the hourly HTTPS monitor still use DNS and may need that cache to expire.
- **A health check fails:** inspect Compose logs before retrying. The playbook does not delete data to repair it.
- **Renewal/expiry check fails:** inspect `journalctl -u certbot.service` and
  `journalctl -u yo3-https@play.example.com.service`. Renewal runs independently of GitHub Actions.
- **VPS is lost:** restore the off-machine database snapshot and original game certificate/key before starting.

## Images after a GitHub release

The existing Release workflow calls [Release containers](https://github.com/usersina/yo3-revival/blob/main/.github/workflows/containers.yml) **after the release
job succeeds**. This explicit dependency also works for releases created using `GITHUB_TOKEN`; those do not trigger
another workflow through the `release` event.

Before either image builds, CI verifies that this is the newest published stable `vX.Y.Z` release and that its Git
tag resolves to the exact source commit. Ordinary pushes, older tags, draft releases and failed release jobs do not
build published images. Local Ansible provisioning can still build the checkout without publishing anything.

CI checks runtime configuration and Ansible syntax/templates, builds the server/engine and dashboard/nginx images,
and pushes Linux x86_64 images to GHCR with release and commit tags. The dashboard build checks generated-client
drift. The release receives `container-images.json` containing the exact image digests. The image-publication job uses the automatic
GitHub token with package permission. A separate dependent deployment job uses the restricted SSH key described below. Check package visibility in
GitHub before expecting anonymous pulls.

After deployment passes its health checks, cleanup keeps the **newest two published stable releases** and their
server/dashboard images. For example, v0.1.1 keeps v0.1.0; v0.1.2 removes v0.1.0 and keeps v0.1.1. Older release
objects and their downloads are deleted, together with unused versions of these two GHCR packages. The retained
images' platform and provenance manifests stay even when GitHub calls them untagged. Drafts and tags outside the
stable release naming scheme are left alone. Git tags and the changelog remain as source history.

Cleanup validates both retained images and their metadata before any deletion. A failed build or deployment skips
cleanup. A cleanup API failure fails that job without stopping the healthy server; **Re-run failed jobs** retries
it. The workflow's GitHub token needs package admin access, granted to repositories that publish their packages;
no additional token is needed. GitHub may refuse deletion of a public package version with over 5,000 downloads:
see [package deletion permissions and limits](https://docs.github.com/en/rest/packages/packages#delete-package-version-for-a-user).
The packaged-updater gate downloads the latest existing release, so it never depends on a pruned archive.

The server runtime is assembled **from scratch** using the same Debian builder's ELF loader and shared libraries
as the compiled server and C++ engine. It includes CA certificates, files/DNS resolution, UTC time data and the
small shell/cat/id/curl tools used by provisioning, administration and health checks. No compiler, package manager,
Rust toolchain or source checkout goes into that runtime. Copied packages retain their copyright and version
metadata so vulnerability scanners can identify the libraries. The dashboard stays on nginx Alpine, without Node.

Both programs run as UID/GID **10001:10001**. Executables and libraries belong to root; only the server's data and
shared socket directories belong to that user. Compose keeps the root filesystem read-only, drops all capabilities,
forbids privilege escalation and makes `/tmp` a `noexec,nosuid,nodev` tmpfs. Server startup uses `umask 077`.
The game and web certificates remain read-only mounts; the dashboard never receives game files or the database.

Builds use pinned cargo-chef to cache compiled Rust dependencies in an ordinary Docker layer, exported by CI's
GitHub cache. The planner masks local version numbers; the real manifests and release metadata are restored before
compiling the shipped binaries. Engine fetching depends only on the fetch script and pinned card IDs; editing a
card override repackages data without recompiling Rust. Runtime binaries are stripped. Tests and generated-client
drift checks run in release CI, not inside the Dockerfiles; the dashboard build still typechecks its source.

After image publication, CI smoke-tests the exact digests before attaching deployment references: the runtime
user/permissions, certificates, hostname resolution, packaged native engine, real cards and a started duel. To check
local images without touching a running setup:

```sh
python3 tools/containers/test_images.py --server yo3-revival-server:local --dashboard yo3-revival-dashboard:local
python3 tools/containers/test_stack.py --image-tag local --game-dir '/path/to/Yu-Gi-Oh! ONLINE 3'
```

The second command needs your game files read-only and creates its own Compose project, ports, certificates and
volumes, removing them afterwards. Neither command rebuilds images or changes the live host. To test a separately
tagged build, replace `local` with that tag (and use the corresponding image names in the first command).

## Enable automatic updates once

For an existing provisioned host, generate a dedicated key locally. Keep its private file outside the checkout:

```sh
ssh-keygen -t ed25519 -N '' -C yo3-release-deployment -f ~/.ssh/yo3-release-deploy
ansible-playbook -i '203.0.113.10,' -u root deploy/ansible/cd.yml \
  -e yo3_deploy_public_key_file=$HOME/.ssh/yo3-release-deploy.pub
```

For first provisioning, pass that same variable to `site.yml` instead. For a sudo user, use the same sudo access as
first setup. This creates `yo3-deploy` with a root-owned forced SSH command and permission to run only the release
updater. It has no shell access through that key, port forwarding, general sudo or Docker-group membership.

Set repository **Actions secrets** using the GitHub CLI or Settings → Secrets and variables → Actions:

| Secret | Value |
| --- | --- |
| `VPS_HOST` | VPS IP or SSH hostname |
| `VPS_PORT` | SSH port; optional, defaults to 22 |
| `VPS_SSH_KEY` | The dedicated private key |
| `VPS_KNOWN_HOSTS` | Host-key entries already verified for that exact address/port |

For example, `gh secret set VPS_SSH_KEY --repo usersina/yo3-revival < ~/.ssh/yo3-release-deploy` uploads the private
key without displaying it. Obtain `VPS_KNOWN_HOSTS` from your verified SSH known-hosts file; do not trust a fresh
`ssh-keyscan` result without checking its fingerprint. No runner agent is installed on the VPS.

The automatic path is **main version bump → checks and bundles → published GitHub release → both GHCR images →
SSH deployment → retain two releases**. There is no approval button. A failed build or image publication never runs deployment.
`Cargo.toml` and the newest changelog version must agree; an already-released version publishes/deploys nothing.
The deployment uses the successful build's image digests and verifies their release/commit labels. An ephemeral
GitHub token can pull private GHCR packages; it is not retained on the VPS.

Both images are pulled before shutdown. The updater stops the stack gracefully, snapshots the complete stopped data
volume, installs the images, checks the running server's release/save/engine health and validates HTTPS. On a detected
failure it restores the old images and data before restarting. A successful retry checks health without restarting;
older releases and attempts to reuse a version with different images are refused. The previous snapshot/configuration
is retained privately at `/srv/yo3/release-rollback/`; it contains credentials and is not an off-machine backup.
The active image references are also recorded for future Ansible reruns, so rerunning setup cannot silently downgrade.
Infrastructure/Compose layout changes still require updating the provisioned runtime files explicitly.

Deployment briefly disconnects players and ends active duels. A server requires at least its own release version:
a `v0.1.1` server refuses a `v0.1.0` launcher, including password, remembered-device and key logins. Discovery shows
**Update required** before opening the game. Hosts may raise `min-launcher` further, but cannot lower that release
floor. Desktop checks and downloads signed updates automatically; Scripts users replace their complete bundle.
Other people's self-hosted servers are not changed by this workflow.

If deployment fails, inspect its job output. After correcting the cause, **Re-run failed jobs** retries the same
immutable release safely; do not move its tag or republish different files under that version. Local provisioning
and rehearsals do not require a published release.

An Ubuntu 24.04 VPS rehearsal passed public bootstrap, a zero-change rerun, reboot with preserved state, real CA
issuance, staging renewal with nginx reload, isolated off-machine restore and a 100-player protocol soak. This does
not yet establish native-engine duel capacity. MFA, an off-machine backup destination and the first actual GitHub
image publication remain; DNS-based monitoring may report the previous address during propagation. See the
[Ansible Compose module](https://docs.ansible.com/projects/ansible/latest/collections/community/docker/docker_compose_v2_module.html)
and [GitHub container publishing documentation](https://docs.github.com/en/actions/tutorials/publish-packages/publish-docker-images)
for the underlying tools.
