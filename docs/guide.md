# Command-line reference

For step-by-step setup, use [Desktop](launcher.md) or [Scripts](scripts.md). This page is for explicit server/account
commands and network configuration. [Source builds and test tools](development.md) have their own guide.

Examples use `./yo3-server` on Linux. On Windows use `yo3-server.exe` in Command Prompt or `.\yo3-server.exe` in
PowerShell. Run from the extracted bundle, or substitute `./target/release/yo3-server` from the repository root.

- [Server options](#server-options)
- [Account commands](#account-commands)
- [Network settings](#network-settings)
- [Existing data and backups](#existing-data-and-backups)
- [Windows SmartScreen and antivirus warnings](#windows-smartscreen-and-antivirus-warnings)
- [Original game runtimes](#original-game-runtimes)

## Server options

`yo3-server --help` is the complete reference. Useful examples:

```bash
./yo3-server --bind 0.0.0.0 --game-dir "/absolute/path/Yu-Gi-Oh! ONLINE 3"
./yo3-server --data-dir "/absolute/path/server data" --auth-port 17443 --http-port 18080
./yo3-server --regulation "Season 16" --duel-check
```

Initialize the identity and accounts before running; [Scripts setup](scripts.md#play-on-your-own-pc) shows the sequence.
Without `--game-dir`, the server uses its default original-game location. Release bundles carry `engine/` and its
Node runtime; run the raw server from the bundle directory so those files can be found. The start-server wrapper
changes to that directory automatically.

Hosting refuses missing, corrupt or unusable game card/deck data before accepting players or migrating player
storage. Select the complete installation with `--game-dir` and retry; keep existing accounts, decks and identity files.

| Option | Purpose |
|---|---|
| `--bind` | Listening interface/IP; raw server defaults to `127.0.0.1`, scripts and Desktop default to `0.0.0.0` |
| `--auth-port` | TLS listener, default `7443` |
| `--advertise` | Address exported in profiles/asset URLs; does not change the listening interface |
| `--game-dir` | Complete original game installation |
| `--data-dir` | Persistent SQLite state and server identity, default `data/` |
| `--log-dir` | Logs, default `logs/` |
| `--http-port` | Internal loopback HTTP/admin listener, default `8080`; Desktop uses an automatically assigned port |
| `--auth-cert`, `--auth-key` | Explicit TLS certificate/key files |
| `--regulation` | Login banlist by ID/name; default unlimited |
| `--duel-check` | Compare each player's modeled client state with the rules engine after each step |
| `--no-packet-log` | Disable raw game packet logging |

Other duel-testing flags and diagnostic paths are listed by `--help`; see [duel diagnostics](development.md#duel-diagnostics).
The game's connection ports are private loopback endpoints assigned by its launcher. Do not configure those in `yo3.ini`.

## Account commands

Most commands require the server **stopped**. BP/code commands can reach a running server through its private local
admin token. Add `--data-dir PATH` to target a non-default setup. Keep each friend's profile/key private.

```bash
./yo3-server init-auth localhost 127.0.0.1 192.168.1.50
./yo3-server add-account Alice
./yo3-server profile Alice Alice.json --advertise 192.168.1.50
./yo3-server accounts
./yo3-server add-player Alice Joey
./yo3-server rename Joey Joseph
./yo3-server set-country Alice TN
./yo3-server give-bp Alice 5000
./yo3-server add-code ABCD1234EFGH5678 2500
```

- `init-auth` creates the host's TLS identity once; include every hostname/IP friends will use. It refuses to replace
  an existing identity.
- `add-account NAME` creates an account and first player. Names are 1–16 characters and unique. `--also B,C` adds
  players to that account, up to four total.
- `profile PLAYER FILE --advertise HOST` exports a CLI profile for an existing account. The profile references
  `server.crt`; copy the host's public certificate beside it. Existing profile files are not overwritten.
- `add-player`, `rename` and `set-country` manage an account's players. `countries` lists accepted country codes;
  `auto` clears an override.
- `give-bp`, `add-code`, `remove-code` and `codes` manage points and redemption codes while hosting can continue.

`--help` on each subcommand lists the exact parameters. If playing from source, [the development guide](development.md#build-from-source)
shows the explicit DLL/injector paths. For a release bundle, use its [play script](scripts.md#join-a-friends-server).

## Network settings

Friends reach one **TLS listener**, normally TCP **7443**. The internal HTTP/admin listener remains on loopback;
never forward it. A launcher supplies the game's gate/front/duel/HTTP ports locally through the required DLL.
There is no stock-client/IP-auth fallback, browser `/join` setup or INI download.

- **Bind** chooses which local interfaces accept connections. `0.0.0.0` listens on all IPv4 interfaces;
  `127.0.0.1` permits only this PC.
- **Advertise** is the reachable LAN/VPN/public address put into a profile. It does not open a listener or change
  firewall policy. Include the address in the identity's certificate names when creating it.
- **External port mappings:** if the public port differs from the local listener, use the external address/port
  when exporting the profile. Keep `--auth-port` and advertised ports consistent with the intended mapping.
- **Firewall/router/VPN:** manage these outside Revival. Check the address from a friend's PC; a successful host-local
  test does not prove remote reachability.

A CLI profile and its adjacent public certificate pin the host's identity and expected name. A Desktop invitation
bundles those details with the account key. They are different formats; Desktop can import either, while the play
script takes a CLI profile. Accounts sharing a router keep separate identities.

For an existing localhost-only identity, do not delete `data/` and rerun `init-auth`. Desktop can keep the same
identity while updating its self-managed certificate: stop the server, select that data folder, save a reachable
sharing address under **Advanced → Hosting → Advanced**, then start hosting. Re-export profiles/invitations as needed.
[Authentication](authentication.md) explains trust, certificate handling and session lifetimes.

## Existing data and backups

`server.db` is SQLite. Old JSON state imports once and stays untouched as migration copies. A path change selects a
different server's data; it does not move it. The running server locks its data directory.

Back up the entire data folder securely, including `server.key`. Startup database snapshots in `backups/` do not
contain that key. Keep players' local deck-editor folders too. Follow [the backup and upgrade procedure](launcher.md#your-data-and-upgrades)
before moving data, restoring or downgrading.

## Windows SmartScreen and antivirus warnings

These pre-alpha executables and the connection injector/DLL are unsigned. Windows or security software may block
or quarantine them. Check the exact warning and protection history, and verify the official release source before
restoring a file. Do not disable protection or assume every detection is harmless.

`yo3-auth-launch.exe` and `yo3-auth.dll` are both required. Keep the complete matching bundle together; removing them
or mixing releases prevents startup. The injector resumes the game only after required hooks initialize. Source
builds are documented [here](development.md#build-from-source) for people who want to inspect/build the code.

For a detection report, include the security product, exact detection name, affected filename, application version
and file SHA-256 if available. A missing file alone does not establish that antivirus removed it. Scripts use the
same required injector/DLL as Desktop and can encounter the same detections.

## Original game runtimes

Validation checks the supported v1.153 executable's hook signatures and probes its known legacy runtime dependencies
without starting the game. A compatible game build and intact Revival helpers are separate requirements.

- **`d3dx9_40.dll`:** install or repair Microsoft's
  [DirectX End-User Runtimes (June 2010)](https://www.microsoft.com/en-us/download/details.aspx?id=8109).
  The DirectX included with modern Windows does not supply every legacy D3DX component.
- **`MSVCR90.dll`, `MSVCP90.dll`, or the VC90 runtime manifest:** install or repair Microsoft's
  [Visual C++ 2008 SP1 Redistributable](https://www.microsoft.com/en-us/download/details.aspx?id=26368),
  choosing **`vcredist_x86.exe`**, because the original game is 32-bit even on 64-bit Windows.

Do not fetch individual DLLs from download sites. On Linux, the same check runs in the chosen Wine prefix; fix
that prefix's runtime setup. A successful dependency probe does not prove graphics-driver or native Windows UAC behavior.
If Windows reports **740 / elevation required**, the original game requests administrator permission: close Revival,
use **Run as administrator**, and retry. Hosting itself does not require elevation.
