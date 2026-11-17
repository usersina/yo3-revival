# One-time VPS setup

Run Ansible **from your local checkout**, supplying the VPS IP and a DNS name pointing to it. It sets up Docker,
HTTPS, the server and dashboard. Later releases publish container images to GitHub; automatic VPS deployment is
separate future work. This is Unreleased work on the dashboard branch. Public password sign-in is enabled for
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

The playbook builds the current checkout **on your machine**, uploads the images by their exact IDs, installs Docker
and Compose on the VPS, configures the host firewall without resetting existing rules, obtains an HTTPS certificate,
sets up renewal/reload checks, creates the game identity and first administrator, and waits for real HTTPS health.
The VPS does not need Rust, Node or your source checkout.

Open **`https://play.example.com/`**, then sign in as **`HostAdmin`**. The generated password is in the local ignored
file **`deploy/ansible/.bootstrap-password-play.example.com`**. Keep it in your password manager. To choose the
initial password instead, supply `YO3_BOOTSTRAP_PASSWORD` through your shell's secret environment: 8–16 letters/digits,
uncommon. It is passed through stdin, hidden from Ansible output, and never set in the runtime containers.

Optional settings are listed in [options.example.yml](../deploy/ansible/options.example.yml): ACME contact email,
game path and admin User ID. Pass overrides with `-e`, or copy that file to ignored `deploy/ansible/config.yml` and
add `-e @deploy/ansible/config.yml`. Email is optional; renewal runs through the host's Certbot timer either way.

## Reruns and operations

This is first setup, **not an update command**. A completed rerun reuses the saved image IDs and domain; it does not
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
- **A health check fails:** inspect Compose logs before retrying. The playbook does not delete data to repair it.
- **Renewal/expiry check fails:** inspect `journalctl -u certbot.service` and
  `journalctl -u yo3-https@play.example.com.service`. Renewal runs independently of GitHub Actions.
- **VPS is lost:** restore the off-machine database snapshot and original game certificate/key before starting.

## Images after a GitHub release

The existing Release workflow calls [Release containers](../.github/workflows/containers.yml) **after the release
job succeeds**. This explicit dependency also works for releases created using `GITHUB_TOKEN`; those do not trigger
another workflow through the `release` event.

CI checks runtime configuration and Ansible syntax/templates, builds the server/engine and dashboard/nginx images,
and pushes Linux x86_64 images to GHCR with release and commit tags. The dashboard build checks generated-client
drift. The release receives `container-images.json` containing the exact image digests. The job uses the automatic
GitHub token with package permission; it has no SSH key and does not contact the VPS. Check package visibility in
GitHub before expecting anonymous pulls.

A later CD workflow can use GitHub Secrets for the VPS address, SSH key and verified host key. It is deliberately
not part of first setup yet. Local builds and the Compose rehearsal work before any images are published.

The local container rehearsal has passed; a fresh VPS run, unchanged rerun, reboot, real CA renewal and off-machine
recovery are still required. Then run the game/load soak and implement MFA. See the
[Ansible Compose module](https://docs.ansible.com/projects/ansible/latest/collections/community/docker/docker_compose_v2_module.html)
and [GitHub container publishing documentation](https://docs.github.com/en/actions/tutorials/publish-packages/publish-docker-images)
for the underlying tools.
