# Command-line reference

For step-by-step setup, use [Desktop](launcher.md) or [Scripts](scripts.md). This page is for explicit server/account
commands and network configuration. [Source builds and test tools](development.md) have their own guide.

Examples use `./yo3-server` on Linux. On Windows use `yo3-server.exe` in Command Prompt or `.\yo3-server.exe` in
PowerShell. Run from the extracted bundle, or substitute `./target/release/yo3-server` from the repository root.

- [Command-line reference](#command-line-reference)
  - [Server options](#server-options)
  - [Account commands](#account-commands)
  - [Network settings](#network-settings)
  - [Existing data and backups](#existing-data-and-backups)
  - [Windows SmartScreen and antivirus warnings](#windows-smartscreen-and-antivirus-warnings)
  - [Original game runtimes](#original-game-runtimes)
  - [Higher resolution](#higher-resolution)

## Server options

`yo3-server --help` is the complete reference. Useful examples:

```bash
./yo3-server --bind 0.0.0.0 --game-dir "/absolute/path/Yu-Gi-Oh! ONLINE 3"
./yo3-server --data-dir "/absolute/path/server data" --auth-port 17443 --http-port 18080
./yo3-server --config "/absolute/path/server.toml"
```

Initialize the identity and accounts before running; [Scripts setup](scripts.md#play-on-your-own-pc) shows the sequence.
Without `--game-dir`, the server uses its default original-game location. Release bundles carry `engine/` (the card
scripts and database) and the `yo3-engine` program beside the server, which finds them there from any directory. The
default `--data-dir data` and `--log-dir logs` are relative to the current directory, so run the raw server from the
bundle directory (or pass both) to keep using the bundle's `data/`. The start-server wrapper changes to that directory
automatically.

Hosting refuses missing, corrupt or unusable game card/deck data before accepting players or migrating player
storage. Select the complete installation with `--game-dir` and retry; keep existing accounts, decks and identity files.

| Option                      | Purpose                                                                                              |
| --------------------------- | ---------------------------------------------------------------------------------------------------- |
| `--bind`                    | Listening interface/IP; raw server `127.0.0.1`, scripts `0.0.0.0`; Desktop: see below                |
| `--auth-port`               | TLS listener, default `7443`                                                                         |
| `--advertise`               | Address exported in profiles/asset URLs; does not change the listening interface                     |
| `--game-dir`                | Complete original game installation                                                                  |
| `--data-dir`                | Persistent SQLite state and server identity, default `data/`                                         |
| `--log-dir`                 | Logs, default `logs/`                                                                                |
| `--http-port`               | Internal loopback HTTP/admin listener, default `8080`; Desktop uses an automatically assigned port   |
| `--auth-cert`, `--auth-key` | Explicit TLS certificate/key files                                                                   |
| `--duel-check`              | Compare each player's modeled client state with the rules engine after each step                     |
| `--no-packet-log`           | Disable separate raw packet files; decoded packet traces remain in bounded session diagnostics       |
| `--no-packet-trace`         | Keep decoded packet traces out of the session diagnostics too (beta; `packet_log = false` sets both) |
| `--config`                  | The [server file](#the-server-file) (beta); default `server.toml` in the current folder when it is there |
| `--owned`                   | The launcher's own server (Singleplayer, on the `beta` branch): standard input stays open while the launcher lives; once it closes, the server stops after nobody has played for `--owned-idle` seconds (default 120)|

Desktop listens on `127.0.0.1` while no sharing address is saved, even when **Listen on** is all interfaces.

The banlist, a duel's LP and clocks, the news and the account rules are [live settings](#live-settings), not flags.
Other duel-testing flags and diagnostic paths are listed by `--help`; see [duel diagnostics](development.md#duel-diagnostics).
The game's connection ports are private loopback endpoints assigned by its launcher. Do not configure those in `yo3.ini`.

## The server file

On the `beta` branch, what only a restart changes can live in a TOML file instead of flags: the listeners, the TLS
files, the folders and the rules engine's program. The server reads `--config FILE`, or `server.toml` in the current
folder when there is one. A flag wins over the file, and relative paths in the file are from its folder.

```toml
bind = "0.0.0.0"
auth_port = 7443
http_port = 8080
advertise = "play.example"
game_dir = "/games/Yu-Gi-Oh! ONLINE 3"
data_dir = "data"
log_dir = "logs"
engine = "yo3-engine"
packet_log = false
```

`packet_log = false` keeps players' packets out of every log: no raw packet files (`--no-packet-log`) and no packet
traces in the session logs (`--no-packet-trace`), which otherwise carry chat lines and names. A hosted server should
use it ([hosting](hosting.md#logs-and-privacy)); the server bundle's `server.toml.example` does. `auth_cert` and
`auth_key` name explicit TLS files. A key the server doesn't know, such as a live setting, is refused
at start with the list of keys it takes.

## Live settings

On the `beta` branch, these change without a restart. They are kept in the server's database (`server.db`), work
while the server runs or stopped, and each applies at its own moment:

```bash
./yo3-server settings                             # each one's value, whether it is the default, when it applies
./yo3-server set login-regulation "Season 14"
./yo3-server set news "Maintenance | Sunday at 10:00"
./yo3-server reset login-regulation
```

| Setting | Default | Applies | What |
| --- | --- | --- | --- |
| `server-name` | Yu-Gi-Oh! ONLINE 3 server | At once | The name in the launcher's server list |
| `headline` | (none) | At once | A line of news under the name in the server list |
| `registration` | `invite` | At once | How accounts are made: `invite`, `open` or `closed` ([accounts](authentication.md#accounts)) |
| `min-launcher` | (none) | At once | Refuse sign-ins from older launchers; they are told to update |
| `login-regulation` | Season 16 | At each player's next login | The banlist for Auto Duel and NPC duels, by id or name. Players already online keep theirs |
| `start-lp` | 8000 | At the next duel | LP at the start of an Auto Duel or NPC duel; a hosted duel has its own |
| `turn-seconds`, `total-seconds` | 180, 600 | At the next duel | The clocks of duels between players that set none (0: off) |
| `news` | A welcome | At the game's next request | The Information list: `TITLE \| BODY` for one entry, or a JSON list of `{title, body}` (1 to 10) |
| `address-burst`, `address-refill-seconds`, `backoff-max-seconds` | 10, 3, 900 | At once | The [throttle](authentication.md#passwords-and-throttling) on account requests |
| `max-players` | 500 | At once | Players signed in; one more is told "The server is full" (admins and moderators still come in) |
| `max-duels` | 30 | At once | Duels running (an engine each, up to about 100 MB): beyond it an NPC duel or Create Duel is refused with "Duels are busy" in the player's chat, and Auto Duel waits until one ends |
| `max-connections` | 2048 | At once | TLS connections at once; a player's game holds several |
| `handshake-burst`, `handshakes-per-second` | 120, 10 | At once | New TLS connections one address may open; this PC's are not counted |
| `maintenance-at`, `maintenance-message` | (none) | At once | Set by `maintenance` ([moderation](#moderation-and-maintenance)) |

A value that doesn't pass its check is refused and changes nothing. Desktop writes its Banlist choice as
`login-regulation` before it starts its server. The testing aids (`--duel-lp`, `--duel-time`, `--duel-deck` and the
like) stay flags and win over the settings.

## Moderation and maintenance

On the `beta` branch. WHO is a User ID or one of its players' names; `--by NAME` records the moderator in the audit
log (default: host). They work on a running server; the ones that change an account also work with it stopped.

```bash
./yo3-server --by Anna kick Yugi                                  # the game shows its own disconnect dialog
./yo3-server --by Anna mute Yugi --reason "flooding" --minutes 30 # their lines go nowhere; their chat says why
./yo3-server --by Anna unmute Yugi
./yo3-server --by Anna restrict Yugi --reason "spam" --days 1     # sign-in refused; a game already in plays on
./yo3-server --by Anna ban Yugi --reason "cheating" --days 7      # sign-in refused, devices signed out, game disconnected
./yo3-server --by Anna lift Yugi                                  # ends a ban or a restriction
./yo3-server --by Anna sign-out Yugi                              # every device signed out, the game disconnected
./yo3-server maintenance --in 30 --message "Back at 10:00"        # the game counts down; --off calls it off
./yo3-server set news "Season 16 | The new banlist is live"        # pushed to everyone online
```

- **Restricted or banned**: the launcher shows the reason and the end. The game itself can only say "This User ID is
  currently restricted." (it has no ban text it uses), which a restricted player's game shows at its next ONLINE.
- **Maintenance**: until it starts, the game shows a wrench with the time left, and its notice (the message). Then
  everyone but admins and moderators (`set-role`) is disconnected, duels between players end as draws, sign-ins wait
  and the launcher's server list says Maintenance. Run `maintenance --off` when it is over. `stop` after the countdown
  shuts the server down for an update.
- **News**: the `news` setting; a change reaches the Information panel of everyone online at once.

## The admin panel

On the `beta` branch (#36). The server serves a web panel on its loopback HTTP port, at
`http://localhost:8080/panel/` (`--http-port`). It answers this PC only: on a server you reach it through an SSH
tunnel ([hosting guide](hosting.md#the-admin-panel-through-an-ssh-tunnel)). Sign in with a password account whose
role is admin or moderator:

```bash
./yo3-server add-user Anna          # prints Anna's first password; or sign up in the launcher with an invite
./yo3-server set-role Anna admin    # or moderator
```

| Page | What it shows and does | Moderator |
| --- | --- | --- |
| Dashboard | Players online, duels, rules engines, memory, uptime, version, last backup's age, whether saving works, recent warnings and errors | Sees it |
| Accounts | Search by User ID or player name; an account's role, standing, mute, devices, BP, and each player's cards, decks, items and record; disconnect, mute, restrict, ban, lift, sign out everywhere, reset code, rename; give BP, set country, set role; give a player cards, another artwork or a deck recipe (as `give-cards` and `give-recipe` below) | Moderation and reset codes, on players only; not BP, country, roles or gifts |
| Invites and codes | Invite codes (make, list) and BOOSTERPASS codes (add, remove, who claimed them) | Makes invites; doesn't add or remove codes |
| News and maintenance | The game's Information list, planning or calling off maintenance | Sees it |
| Settings | Every live setting with its value, when it applies and what it does | Sees it |
| Moderation | Who is banned, restricted or muted now, and the latest moderation | Yes |
| Audit log | The newest 200 changes, from the panel, the commands and the API | Yes |
| Backups | The database backups; back up now; download one (the download is audited) | Sees the list |
| Logs | The server's newest lines since it started (warnings and errors only, or all) | Yes |

- Every change runs one admin API operation as the signed-in account: the audit log names it, from `panel`. A
  moderator's refused change says why and changes nothing; a moderator can't act on an admin's or another moderator's
  account.
- Sign-in has the launcher's throttle: a wrong password makes the next try for that User ID wait 1 s, then 2, 4, …
  up to `backoff-max-seconds`. A session ends after 30 idle minutes, 12 hours, **Sign out**, or at once when the
  account loses its role or is banned.
- The panel keeps no data of its own and needs no restart: a role change applies at the account's next page.

## Account commands

On the `beta` branch every command works while the server runs (#25): it goes to the running server's admin API
(loopback, with the private token in `data/admin.json`) and changes it at once. With no server running, the command
works on the data folder itself, holding its lock (`server.lock`) meanwhile. Only `init-auth`, `restore` and
`backups` always work on the files. Every change is recorded in the audit log: `audit` lists the newest, with who,
from where, what, on what, and the values before and after. Released versions still need the server stopped for all
but the BP and code commands. Add `--data-dir PATH` to target a non-default setup. Players sign in with password
accounts (`invite`, `add-user`); `add-account` and `profile` make key accounts for the Scripts edition's play script.
Keep each key profile private ([accounts](authentication.md#accounts)).

```bash
./yo3-server init-auth localhost 127.0.0.1 192.168.1.50
./yo3-server add-account Alice
./yo3-server profile Alice Alice.json --advertise 192.168.1.50
./yo3-server accounts
./yo3-server add-player Alice Joey
./yo3-server rename Joey Joseph
./yo3-server set-country Alice TN
./yo3-server give-bp Alice 5000
./yo3-server grant-title Alice "GAME MASTER"
./yo3-server give-cards Alice "Blue-Eyes White Dragon" --art 2 --count 3
./yo3-server give-recipe Alice "E-HERO EVOLUTION DECK" --cards
./yo3-server add-code ABCD1234EFGH5678 2500
./yo3-server invite --uses 5 --days 7
./yo3-server add-user Alice
./yo3-server reset-code Alice
./yo3-server set-role Alice moderator
./yo3-server audit --limit 20
./yo3-server stop
```

- `init-auth` creates the host's TLS identity once; include every hostname/IP friends will use. It refuses to replace
  an existing identity. Its certificate lasts a year; the server renews it within 30 days of expiry, with the same
  key and names, so existing profiles and their copy of `server.crt` keep working. On the `beta` branch it also
  checks while running (every six hours), and serves a certificate replaced on disk without a restart.
- `stop` stops the running server gracefully, as SIGTERM and Ctrl+C do: no new logins, the duels between players end
  as draws, and a last backup. Every change is already saved as it happens.
- `add-account NAME` creates a key account and its first player, for a Scripts player. Names are 1–16 characters and
  unique. `--also B,C` adds players to that account, up to four total.
- `profile PLAYER FILE --advertise HOST` exports a key account's private profile for the play script. The profile
  references `server.crt`; copy the host's public certificate beside it. Existing profile files are not overwritten.
  `key WHO` prints the key itself.
- `invite`, `invites`, `add-user`, `reset-code` and `set-role` manage password accounts (beta) and work while the
  server runs: invite codes for sign-up, an account the host makes with a first password, a one-day reset code for a
  player who lost their password and recovery code, and roles. [Accounts](authentication.md#making-accounts-invites-and-recovery)
  explains each.
- Commands that name an account (`key`, `add-player`, `set-country`, `give-bp`) take its User ID or one of its
  players' names.
- `add-player`, `rename` and `set-country` manage an account's players. `countries` lists accepted country codes;
  `auto` clears an override.
- `grant-title PLAYER TITLE` gives a player a title by its name as the game shows it (`titles` lists them), such as
  GAME MASTER or JUMP FESTA, which no duel earns. The player wears it in Avatar Settings. A rename or title reaches
  a player who is online at their next login.
- `give-cards PLAYER CARD` gives a player cards that no pack or deck hands out, or another artwork of a card (#19).
  CARD is its English name or Konami id; a name two cards share (Dark Magician) needs the id. `--art N` picks the
  artwork (the card's first when omitted; Blue-Eyes White Dragon has five) and `--count N` how many copies (1 when
  omitted, at most 255). On a running server the player gets a D-Mail from "Host" saying what they got, and an
  online player's game gets the new collection at once (during a duel between players, a trade or a rental, at the
  next screen that asks for it). The game also shows the cards with their pictures, as when a pack is opened, the next
  time the player comes back to the map (or at their next login).
- `give-recipe PLAYER DECK` gives a player a deck recipe for Load Deck's Official tab (#19), by its title or preset
  id; `recipes` lists them (two recipes share some titles: use the id). `--cards` gives the recipe's cards too, and
  then the game shows the deck as a present the next time the player comes back to the map, as a bought structure deck,
  and the recipe is on the tab from then on. A recipe alone shows at the player's next login or after their next duel.
  The D-Mail says which. Only an admin gives cards or recipes,
  and each gift is in the audit log.
- `give-bp`, `add-code`, `remove-code` and `codes` manage points and redemption codes while hosting can continue.
  Use them rather than editing `accounts.json`: after the first start of a current server, accounts live in
  `server.db`. The JSON files are old copies the server no longer reads; it warns while they exist, and they are safe
  to delete. A player who is online sees BP changes (gifts, duel rewards) on the game's next screen. `codes` lists
  who claimed each code and when. The server's terminal output is coloured by level (`NO_COLOR=1` turns it off).

`--help` on each subcommand lists the exact parameters. If playing from source, [the development guide](development.md#build-from-source)
shows the explicit DLL/injector paths. For a release bundle, use its [play script](scripts.md#join-a-friends-server).

## Network settings

Friends reach one **TLS listener**, normally TCP **7443**. The internal HTTP/admin listener remains on loopback;
never forward it. A launcher supplies the game's gate/front/duel/HTTP ports locally through the required DLL.
There is no stock-client/IP-auth fallback, browser `/join` setup or INI download.

- **Bind** chooses which local interfaces accept connections. `0.0.0.0` listens on all IPv4 interfaces;
  `127.0.0.1` permits only this PC.
- **Advertise** is the reachable LAN/VPN/public address put into a key profile. It does not open a listener or change
  firewall policy. Include the address in the identity's certificate names when creating it.
- **External port mappings:** if the public port differs from the local listener, use the external address/port
  when exporting the profile. Keep `--auth-port` and advertised ports consistent with the intended mapping.
- **Firewall/router/VPN:** manage these outside Revival. Check the address from a friend's PC; a successful host-local
  test does not prove remote reachability.

The Desktop launcher's invite links (`yo3revival://server?address=…&identity=…`, from Host for friends) carry the
address and the identity; players then sign in with a password account. A key profile and its adjacent public
certificate pin the host's identity and expected name for the play script. Accounts sharing a router keep separate
identities.

For an existing localhost-only identity, do not delete `data/` and rerun `init-auth`: that makes a new key, and every
player would have to trust the server again.
[Authentication](authentication.md) explains trust, certificate handling and session lifetimes.

## Existing data and backups

`server.db` is SQLite in WAL mode, with one row per account (and its players), record, friendship, mail, team, event,
rental and BP code; a save writes only the rows that changed. On the `beta` branch it is schema 2: a server from `main`
(schema 1) can't open it, and a schema 1 database is converted once, in place. JSON files from versions before
`server.db` are not read (the beta starts fresh). A path change selects a different server's data; it does not move it.
The running server holds its data directory: a second server on the same folder is refused, whatever its ports. A
`server.lock` left after a crash holds nothing.

If a save fails (a full disk), the server says so in its log and refuses new logins until a save works again; players
already in keep playing.

The server backs up `server.db` into `backups/` when it starts, every hour while it runs, and when it stops, keeping the
newest 24. With the server stopped:

```bash
./yo3-server --data-dir data backups          # list them, newest first
./yo3-server --data-dir data restore latest   # or a name from the list; the replaced database is kept beside it
```

Back up the entire data folder securely, including `server.key`: the database backups in `backups/` don't contain that
key. Keep players' local deck-editor folders too. Follow [the backup and upgrade procedure](launcher.md#your-data-and-upgrades)
before moving data, restoring or downgrading.

## Windows SmartScreen and antivirus warnings

These pre-alpha executables and the connection injector/DLL are unsigned. Windows or security software may block
or quarantine them. No exclusion is needed unless your security software actually blocks a file.

1. Check your antivirus's protection history for the exact detection and affected file. A missing file alone does
   not establish that antivirus removed it.
2. Confirm you downloaded the complete matching archive from the
   [official releases](https://github.com/usersina/yo3-revival/releases/latest). Do not assume every detection is harmless;
   report an uncertain detection before allowing it.
3. For a verified false positive, allow or restore the affected file. If you choose a folder exclusion instead,
   exclude only the extracted **`yo3-revival-desktop/`** or **`yo3-revival-scripts/`** folder, then re-extract any
   quarantined files from the matching archive. After an in-app update, Desktop runs from
   `%LOCALAPPDATA%\YO3 Revival\updates\installs\<version>-<platform>\` instead, a new folder for each version:
   allow the file there, or exclude that version's folder. In Desktop, choose **Retry check**, then **Play** once the check passes.

**Where are our DLLs?**

| Location                            | Revival files                                                                                                    |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Extracted Desktop or Scripts folder | `yo3-auth.dll` (required connection mod), `yo3-auth-launch.exe` (required injector), and the Revival executables |
| Same folder                         | `yo3-engine.exe` (the rules engine, run by the server) and `engine\` (its card scripts and database)              |
| Extracted Desktop folder            | `yo3-bootstrap.dll`, the loader that Play puts beside `yo3.exe` so the game can be opened directly             |
| After a Desktop in-app update       | the same Desktop files in `%LOCALAPPDATA%\YO3 Revival\updates\installs\<version>-<platform>\`, a new folder per version      |
| Game folder, beside `yo3.exe`       | `winmm.dll`, put there by Desktop Play so [the game opens directly](launcher.md#open-the-game-directly)         |

Desktop **Play** and Scripts **play** load `yo3-auth.dll` from the folder they run from (the extracted Revival folder,
or Desktop's update folder after an in-app update); they do not copy it into the game folder. Excluding just
`yo3-revival.exe` does not cover the injector or DLL. The whole game folder does not normally need an exclusion. If the optional direct-launch `winmm.dll` is flagged, allow that
specific file after verifying it came from your Revival setup.

Keep antivirus protection on. Prefer a file exception to a folder exclusion, which stops scanning everything
inside that folder. Do not exclude all of Downloads, Games or Program Files. See
[Microsoft's Windows Security guidance](https://support.microsoft.com/en-us/windows/security/threat-malware-protection/virus-and-threat-protection-in-the-windows-security-app)
for Defender's exclusion settings.

`yo3-auth-launch.exe` and `yo3-auth.dll` are both required. Keep the complete matching bundle together; removing them
or mixing releases prevents startup. The injector resumes the game only after required hooks initialize. Source
builds are [documented here](development.md#build-from-source) for people who want to inspect/build the code.

For a detection report, include the security product, exact detection name, affected filename, application version
and file SHA-256 if available. Scripts use the same required injector/DLL as Desktop and can encounter the same
detections. [Report a detection](https://github.com/usersina/yo3-revival/issues/new/choose).

## Original game runtimes

Validation checks the supported v1.153 executable's hook signatures and probes its known legacy runtime dependencies
without starting the game. A compatible game build and intact Revival helpers are separate requirements.
Revival resolves native Windows paths automatically. Paste the game folder's normal path in Desktop or quote it
in Scripts; no manual extended-path conversion is needed.

- **`d3dx9_40.dll`:** install or repair Microsoft's
  [DirectX End-User Runtimes (June 2010)](https://www.microsoft.com/en-us/download/details.aspx?id=8109).
  The DirectX included with modern Windows does not supply every legacy D3DX component.
- **`MSVCR90.dll`, `MSVCP90.dll`, or the VC90 runtime manifest:** install or repair Microsoft's
  [Visual C++ 2008 SP1 Redistributable](https://www.microsoft.com/en-us/download/details.aspx?id=26368),
  choosing **`vcredist_x86.exe`**, because the original game is 32-bit even on 64-bit Windows.

Do not fetch individual DLLs from download sites. On Linux, the same check runs in the chosen Wine prefix; fix
that prefix's runtime setup. A successful dependency probe does not prove graphics-driver behavior.
Desktop and Scripts launch the original game with the current user's permissions, avoiding its obsolete administrator
request. Run Revival normally. Keep the selected game folder and your game Documents writable by your account;
hosting itself does not require elevation.

## Higher resolution

`play.sh` / `play.cmd` (and `yo3-launcher`) take `--resolution`:

| Value          | The game                                                                                      |
| -------------- | --------------------------------------------------------------------------------------------- |
| `off` (absent) | the original 1024×768 (or 1024×600) window                                                    |
| `WIDTHxHEIGHT` | a window with that client area, 320 to 8192 each, e.g. `1440x1080`                             |
| `auto`         | the largest window of the game's shape that the screen's work area holds                       |

```bash
./play.sh --profile "$PWD/client.json" --game-dir "$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3" --resolution 1440x1080
```

The game keeps laying out its 1024×768 frame; the connection DLL draws it at the larger size, centred, with black bars
when the shapes differ, and maps the mouse back. In the game's own **Full** screen mode the window is borderless and the
size of the screen, whatever the value. The launcher hands the value to the game as `YO3_RESOLUTION`; setting that
variable yourself works the same. A value the DLL refuses, or a game whose drawing code differs from v1.153's, stops
startup with `stage=mod-resolution`. Opening `yo3.exe` directly always uses the original size. How it works:
[client internals](client-internals.md#higher-resolution).
