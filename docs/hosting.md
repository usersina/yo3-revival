# Hosting a server

How to run `yo3-server` as an always-on service on a Linux server (a VPS, or a spare PC) from the **server bundle**,
and how to look after it: the admin panel through an SSH tunnel, backups on and off the machine, monitoring and
alerts, and updates. On the `beta` branch (#36, #37); the released Desktop and Scripts editions host from a PC
([Desktop](launcher.md), [Scripts](scripts.md#host-friends)).

The guide doesn't name a provider. Replace the placeholders with your own values:

| Placeholder | What it is |
| --- | --- |
| `<server-address>` | The address players and you reach the server at: a domain name such as `play.example.org`, or its IP |
| `<server-ip>` | The server's public IP address |
| `<admin>` | Your own login on the server, for SSH |

- [Hosting a server](#hosting-a-server)
  - [What you need](#what-you-need)
  - [The server bundle](#the-server-bundle)
  - [Set up a fresh server](#set-up-a-fresh-server)
  - [The admin panel through an SSH tunnel](#the-admin-panel-through-an-ssh-tunnel)
  - [Admin commands on the server](#admin-commands-on-the-server)
  - [Backups](#backups)
  - [Monitoring and alerts](#monitoring-and-alerts)
  - [Logs and privacy](#logs-and-privacy)
  - [Updates](#updates)

## What you need

- **A 64-bit (x86-64) Linux server with systemd**: Debian 12, Ubuntu 24.04 or similar. A 4 GB, 2–4 vCPU server
  covers twice the beta's expected peak of 50 players and 15 duels ([capacity](development.md#capacity-r7-27)).
  About 2 GB of disk for the bundle, the game data and backups.
- **Your own copy of the game's data:** `package/data002.ypk` and `yo3.idx` from an installed Yu-Gi-Oh! ONLINE 3
  v1.153. The server reads its cards, decks and banlists from them. They are Konami's files: they are never in the
  repository or a release, and you copy them from your own installation.
- **SSH access** to the server, with a user that may use `sudo`.

Only the game's TLS port (7443) is open to the internet. The admin panel and the admin API stay on the server's
loopback address; you reach them through SSH.

## The server bundle

`yo3-revival-server-VERSION-linux-x86_64.tar.gz` extracts to `yo3-revival-server/`:

| File | What it is |
| --- | --- |
| `yo3-server` | The server; `yo3-server --help` lists its options and admin commands |
| `yo3-engine`, `engine/` | The rules engine and its card scripts and database (`engine/THIRD-PARTY.md` says whose they are) |
| `server.toml.example` | The [server file](guide.md#the-server-file) for this layout |
| `systemd/yo3-server.service` | The service |
| `systemd/yo3-monitor.service`, `.timer`, `yo3-monitor.sh`, `alerts.env.example` | The [monitor and its alerts](#monitoring-and-alerts) |
| `systemd/yo3-offsite-backup.service`, `.timer`, `yo3-offsite-backup.sh`, `offsite.env.example` | The [off-machine backup](#off-machine-backups) |
| `README.md`, `docs/` | This guide and the others |

It has no launcher, no connection DLL and no game files: players use the Desktop or Scripts edition.

## Set up a fresh server

The paths below are the ones the unit and the example file use: the bundle in `/opt/yo3-revival`, the game's data in
`/opt/yo3-game`, the configuration in `/etc/yo3`, the server's data in `/var/lib/yo3` and its logs in
`/var/log/yo3`.

1. **A user for the server, and the folders.** The server never runs as root.

   ```bash
   sudo useradd --system --home-dir /var/lib/yo3 --shell /usr/sbin/nologin yo3
   sudo install -d -o yo3 -g yo3 -m 0700 /var/lib/yo3 /var/log/yo3
   sudo install -d -m 0755 /etc/yo3 /opt/yo3-game/package
   ```

2. **The bundle**, from the release page, into `/opt/yo3-revival` (owned by root: the server can't change itself):

   ```bash
   sudo tar -xzf yo3-revival-server-VERSION-linux-x86_64.tar.gz -C /opt
   sudo mv /opt/yo3-revival-server /opt/yo3-revival
   ```

3. **The game's data**, copied from your own installation (from your PC: `scp`, then move them into place):

   ```bash
   scp "Yu-Gi-Oh! ONLINE 3/package/data002.ypk" "Yu-Gi-Oh! ONLINE 3/yo3.idx" <admin>@<server-address>:
   sudo install -m 0644 data002.ypk /opt/yo3-game/package/
   sudo install -m 0644 yo3.idx /opt/yo3-game/
   ```

4. **The server file.** Copy the example and put your address in `advertise`:

   ```bash
   sudo cp /opt/yo3-revival/server.toml.example /etc/yo3/server.toml
   sudoedit /etc/yo3/server.toml
   ```

5. **The server's identity**, once, with every name players will use (the domain and the IP):

   ```bash
   sudo -u yo3 /opt/yo3-revival/yo3-server --config /etc/yo3/server.toml init-auth <server-address> <server-ip>
   ```

   It writes `server.crt` and `server.key` into `/var/lib/yo3`. Players' launchers pin this identity: keep
   `server.key` (it is in the [off-machine backup](#off-machine-backups)) and never run `init-auth` again on a server
   players already use.

6. **The service:**

   ```bash
   sudo cp /opt/yo3-revival/systemd/yo3-server.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable --now yo3-server
   systemctl status yo3-server        # active (running)
   journalctl -u yo3-server -f        # its log; Ctrl+C to leave
   ```

   The unit runs the server as `yo3` with a read-only system (`ProtectSystem=strict`): it may write only its data and
   log folders. It restarts the server if it fails. `systemctl stop yo3-server` stops it gracefully, as
   `yo3-server stop` does: no new logins, duels between players end as draws, and a last backup.

7. **The firewall:** SSH and the game's port only. With `ufw`:

   ```bash
   sudo ufw allow OpenSSH
   sudo ufw allow 7443/tcp
   sudo ufw enable
   ```

   With `firewalld`: `sudo firewall-cmd --permanent --add-port=7443/tcp && sudo firewall-cmd --reload`. Many
   providers also have a firewall in their control panel: allow the same there. Never open the HTTP port (8080):
   it serves the admin panel and API, on loopback only.

8. **Your admin account**, a password account with the admin role:

   ```bash
   sudo -u yo3 /opt/yo3-revival/yo3-server --config /etc/yo3/server.toml add-user <your-user-id>
   ```

   `add-user` prints a one-time setup code, valid for 24 hours. On your own computer, add the server in Desktop
   and use **Forgot password** with your User ID and that code to choose a password. Save the recovery code it
   gives you. Terminal-only users can use `yo3-launcher --recover LINK USER --profile client.json` instead
   ([Scripts](scripts.md#password-setup-and-recovery)); the host’s `share` command prints LINK. No game files are needed
   for that command. Complete setup **before** assigning a staff role:

   ```bash
   sudo -u yo3 /opt/yo3-revival/yo3-server --config /etc/yo3/server.toml set-role <your-user-id> admin
   ```

   Sign in to the panel with the password you chose. Moderators get `set-role USER moderator`.
   Role changes invalidate outstanding setup/reset codes; an account awaiting its first password cannot be promoted.

9. **Players** sign up in the launcher with an invite code (`registration` is `invite` by default). Make codes in the
   panel's **Invites and codes**, or with `invite --uses 5 --days 7`.

## The admin panel through an SSH tunnel

The panel is a web page the server serves on its loopback HTTP port (D7: no public admin page in the beta). The
[React dashboard under development](dashboard.md) uses a separate opt-in loopback API socket; it does not change
this panel's port or the CLI's access, and must not be proxied publicly before its MFA/proxy gate is complete. From your
own PC, open a tunnel to the server and leave it open while you use the panel:

```bash
ssh -N -L 8080:localhost:8080 <admin>@<server-address>
```

Then open **<http://localhost:8080/panel/>** in your browser and sign in with your admin or moderator account. Close
the tunnel with Ctrl+C when you are done. On Windows the same `ssh` command works in PowerShell or Command Prompt
(Windows 10 and 11 have OpenSSH); PuTTY calls it Connection → SSH → Tunnels, source port 8080, destination
`localhost:8080`.

- **Port 8080 is busy on your PC:** forward another local port, `ssh -N -L 18080:localhost:8080 …`, and open
  `http://localhost:18080/panel/`.
- **A server on your own PC** (a spare machine at home): no tunnel; open `http://localhost:8080/panel/` there.
- The panel refuses any address but `localhost`, `127.0.0.1` and `[::1]`: open it through the tunnel, not by the
  server's name.

[The admin panel](guide.md#the-admin-panel) lists its pages and what a moderator may do. An admin gives a player cards
no pack hands out, another artwork of a card or a deck recipe from the player's account page (**Give cards**, **Give
deck recipe**): pick the player, then the card by its name or Konami id, or the deck by its title. The player gets a D-Mail
from "Host" saying what they got, and the game shows the cards with their pictures, like an opened pack, the next time
they come back to the map. A recipe with its cards comes the same way; a recipe alone shows at their next login or
after their next duel.

## Admin commands on the server

Every admin command works on the running server through its admin API, as the panel does. Run them as the `yo3`
user, with the server file, so they find the server's data folder:

```bash
alias yo3='sudo -u yo3 /opt/yo3-revival/yo3-server --config /etc/yo3/server.toml'
yo3 accounts
yo3 --by <your-user-id> ban Someone --reason "cheating" --days 7
yo3 maintenance --in 15 --message "Updating, back in 20 minutes"
yo3 audit --limit 20
yo3 give-bp Someone 500 --request-id <the-id-printed-by-the-first-try>  # only to retry that exact grant after a lost answer
yo3 delete-account Someone --yes      # a player asked for their account to go: everything kept for it, for good
```

Give BP prints a retry ID before sending. Reuse it with the same target and amount after an uncertain answer;
changing the audit name (`--by`) does not permit a second grant. Details: [account commands](guide.md#account-commands).

A deleted account still sits in the backups until they are replaced: the newest 24 on the server (a day), and the
off-machine copies for as long as they keep them (below). [Moderation](guide.md#moderation-and-maintenance) and [account commands](guide.md#account-commands) list them all.

## Backups

**On the server.** The server copies its database into `/var/lib/yo3/backups/` at start, every hour and at stop, and
keeps the newest 24. The panel's **Backups** page lists them, takes one now, and lets an admin download one; the
dashboard shows the newest one's age. To restore one, stop the server first:

```bash
sudo systemctl stop yo3-server
sudo -u yo3 /opt/yo3-revival/yo3-server --config /etc/yo3/server.toml backups
sudo -u yo3 /opt/yo3-revival/yo3-server --config /etc/yo3/server.toml restore latest
sudo systemctl start yo3-server
```

### Off-machine backups

A server that is lost takes its backups with it, so the bundle has a daily job that copies the newest backup, with
the server's identity (`server.crt`, `server.key`), to storage elsewhere. It uses [restic](https://restic.net)
(encrypted, deduplicated; any object storage it supports, or an SFTP server), or [rclone](https://rclone.org) to copy
the files to any remote rclone supports. Install one of them, then:

```bash
sudo cp /opt/yo3-revival/systemd/offsite.env.example /etc/yo3/offsite.env
sudo chmod 600 /etc/yo3/offsite.env
sudoedit /etc/yo3/offsite.env      # the destination and its credentials
sudo cp /opt/yo3-revival/systemd/yo3-offsite-backup.{service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now yo3-offsite-backup.timer
sudo systemctl start yo3-offsite-backup   # once now; journalctl -u yo3-offsite-backup says how it went
```

With restic it keeps 14 daily and 8 weekly copies. Each run that succeeds writes the time to
`/var/lib/yo3-offsite/last-success`, which the monitor checks. A restore is the reverse: `restic restore latest
--target /tmp/restore` (or `rclone copy`), then stop the server, put `server.db` into a new folder under
`/var/lib/yo3/backups/` and `restore` it as above. Rehearse a restore before players depend on the server (#39).

## Monitoring and alerts

The server answers `GET http://localhost:8080/health` on loopback with its state as JSON: status 200 when it is
healthy, 503 when saving fails or the rules engine is missing. The bundle's monitor checks it every minute, with the
disk and the backups, and sends an alert when a problem starts and when it ends:

| Alert | When |
| --- | --- |
| Server down | The health check doesn't answer (systemd restarts a server that failed; this says it stayed down) |
| Saving fails | The server can't save (a full disk, a broken database): logins pause until it can |
| Duels don't work | The rules engine can't start |
| Backup stale | The newest backup is older than 2 hours (`YO3_BACKUP_MAX_AGE`) |
| Off-machine backup stale | The last off-machine copy is older than 26 hours (`YO3_OFFSITE_MAX_AGE`; only once one has run) |
| Disk filling | The data folder's disk is over 90 % full (`YO3_DISK_MAX_PERCENT`) |
| Errors repeat | 5 or more errors in the server's log in 10 minutes (`YO3_ERRORS_MAX`) |

Where alerts go is yours to choose, in `/etc/yo3/alerts.env`: a Discord webhook (`YO3_ALERT_WEBHOOK`), or any
command that takes the message as its argument (`YO3_ALERT_COMMAND`: mail, ntfy, another chat's webhook). With
neither, alerts only go to the journal (`journalctl -u yo3-monitor`).

```bash
sudo cp /opt/yo3-revival/systemd/alerts.env.example /etc/yo3/alerts.env
sudo chmod 600 /etc/yo3/alerts.env
sudoedit /etc/yo3/alerts.env
sudo cp /opt/yo3-revival/systemd/yo3-monitor.{service,timer} /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now yo3-monitor.timer
sudo systemctl start yo3-monitor && journalctl -u yo3-monitor -n 5   # "all clear", or the alerts it sent
```

The monitor runs on the server itself, so it can't report the whole machine going down. Add an **external uptime
check** too: any uptime service's TCP check on `<server-address>:7443`, alerting the same channel.

## Logs and privacy

- The server logs to the journal (`journalctl -u yo3-server`) and keeps session logs in `/var/log/yo3/sessions`,
  pruned by size and age. The journal's own size limit applies (`SystemMaxUse=` in `/etc/systemd/journald.conf`).
- **Packet logs are off** in the example server file (`packet_log = false`): no raw packet files and no packet
  traces (which carry chat lines and names) in the session logs. Turn them on only to investigate a problem, and off
  again after.
- The panel's **Logs** page shows the newest lines since the server started, with credentials masked.

## Updates

1. Plan a short maintenance, so players see the countdown (panel: **News and maintenance**, or
   `yo3 maintenance --in 10 --message "Updating"`).
2. When it starts, stop the server, replace the bundle and start it again:

   ```bash
   sudo systemctl stop yo3-server
   sudo tar -xzf yo3-revival-server-NEW-linux-x86_64.tar.gz -C /tmp
   sudo rm -rf /opt/yo3-revival.old && sudo mv /opt/yo3-revival /opt/yo3-revival.old
   sudo mv /tmp/yo3-revival-server /opt/yo3-revival
   sudo systemctl start yo3-server
   yo3 maintenance --off
   ```

3. Compare the new bundle's `systemd/` files with yours in `/etc/systemd/system/` and the example server file with
   `/etc/yo3/server.toml`; the changelog says when they change. `/opt/yo3-revival.old` is the way back: the stop took
   a backup first.
