# Scripts edition

Choose **Scripts** if you want terminal commands without the Desktop app. It includes the same server, rules engine,
Node.js, connection helper and DLL. Account/profile setup and updates are manual. For a Play button and log export,
use [Desktop](launcher.md).

You still need the original **Yu-Gi-Oh! ONLINE 3 v1.153** installation. Windows builds support **64-bit Windows 10/11**;
**Windows 7 is not supported.** Linux needs Wine for the game. No Rust or separate Node.js installation is needed
for a release bundle.

[Download Scripts for your system](https://github.com/usersina/yo3-revival/releases/latest) and extract the entire
archive. Keep every file together in `yo3-revival-scripts/`. Open a terminal in that folder.

- [Scripts edition](#scripts-edition)
  - [Play on your own PC](#play-on-your-own-pc)
    - [Windows](#windows)
    - [Linux](#linux)
  - [Join a friend's server](#join-a-friends-server)
  - [Host friends](#host-friends)
  - [Options and logs](#options-and-logs)
  - [When something fails](#when-something-fails)
  - [Updates](#updates)

## Play on your own PC

If you plan to host friends, use [Host friends](#host-friends) instead so their address is included in your initial
server identity. The commands below create a local-only setup. Run initialization and account creation **once**;
later, just run the start-server and play commands. Keep `data/` and your private profile/certificate.

### Windows

Use **Command Prompt** in the extracted folder. Adjust the game path if installed elsewhere.

```bat
yo3-server.exe init-auth localhost 127.0.0.1
yo3-server.exe add-account Duelist
yo3-server.exe profile Duelist client.json --advertise localhost
copy data\server.crt server.crt
start-server.cmd --bind 127.0.0.1 --game-dir "C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3"
```

Leave that window open. In a **second Command Prompt** in the same folder:

```bat
play.cmd --profile "%CD%\client.json" --game-dir "C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3"
```

### Linux

Adjust the game path if you use a different Wine prefix or installation location.

```bash
./yo3-server init-auth localhost 127.0.0.1
./yo3-server add-account Duelist
./yo3-server profile Duelist client.json --advertise localhost
cp data/server.crt server.crt
./start-server.sh --bind 127.0.0.1 --game-dir "$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3"
```

Leave that terminal open. In a **second terminal** in the same folder:

```bash
./play.sh --profile "$PWD/client.json" --game-dir "$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3"
```

Keep the play terminal open too; **Ctrl+C** closes the game and its authenticated session, including during startup.
In the game, choose **ONLINE** and select your player. The required mod skips the unused original login form and
prepares your first deck automatically. [Start your first duel](playing.md#your-first-duel).

## Join a friend's server

Ask the host for your private **`client.json` profile** and its matching public **`server.crt`**. Keep them together.
A Desktop invitation is a different format: import that in Desktop, or ask the host for a CLI profile using `profile`.

Windows, in Command Prompt:

```bat
play.cmd --profile "C:\Games\YO3-profile\client.json" --game-dir "C:\Games\Yu-Gi-Oh! ONLINE 3"
```

Linux:

```bash
./play.sh --profile "/absolute/path/client.json" --game-dir "/absolute/path/Yu-Gi-Oh! ONLINE 3"
```

Use absolute paths. Linux users can add `--wine-prefix "/absolute/path/private-prefix"`.
Do not edit `yo3.ini` or the game executable; the connection DLL supplies the private endpoints.

## Host friends

You need your own original game installation too, because the server reads its card/deck data.
Choose an address your friends can reach, such as your LAN/VPN IP or public hostname. Replace **192.168.1.50** below
with that address. Perform setup with the server stopped.

Windows, in Command Prompt in the extracted folder:

```bat
yo3-server.exe init-auth localhost 127.0.0.1 192.168.1.50
yo3-server.exe add-account Friend
yo3-server.exe profile Friend friend.json --advertise 192.168.1.50
start-server.cmd --game-dir "C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3"
```

Linux:

```bash
./yo3-server init-auth localhost 127.0.0.1 192.168.1.50
./yo3-server add-account Friend
./yo3-server profile Friend friend.json --advertise 192.168.1.50
./start-server.sh --game-dir "$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3"
```

Send **`friend.json`** and **`data/server.crt`** privately to that friend. They keep the files together, with the
certificate named `server.crt`. Repeat account creation/profile export for each friend, and create your own account
and localhost profile if you will also play. Names must be unique and **1–16 characters**; each account supports up
to four players. Do not share `server.key` or reuse one account for different friends.

The scripts listen on **all interfaces (`0.0.0.0`)** by default. Friends need access to **TCP 7443** through the host's
firewall and any router/VPN. The scripts do not configure these. The internal HTTP/admin port stays on loopback;
do not forward it. [Network details](guide.md#network-settings) and [existing-data upgrades](launcher.md#your-data-and-upgrades)
cover advanced setups.

## Options and logs

`start-server` forwards server options; explicit `--bind`, `--data-dir` and `--log-dir` values override its defaults.
Paths passed to that script are relative to the extracted bundle. `play` preserves your terminal's working directory,
so absolute profile/game paths avoid ambiguity. Arguments containing spaces must be quoted.

Use `yo3-server.exe --help` on Windows or `./yo3-server --help` on Linux for every server option, including
`--duel-check`. Use `play.cmd --help` or `./play.sh --help` for game-launch options.
[Account commands and advanced configuration](guide.md) are in the command-line reference.

Server logs default to `logs/`, with bounded diagnostic sessions under `logs/sessions/`. Game-helper sessions default
to `logs/sessions/` **beside the profile**, or the path supplied through `--diagnostics-dir`. Server packet logs may
contain player/chat/deck data. Game output, mod diagnostics, decoded server packets and engine stderr are captured
automatically. `--no-packet-log` disables the separate raw packet files, while decoded traces remain in session logs.
Desktop's [export dialog](launcher.md#logs-and-support) offers optional personal-data filtering for its indexed
sessions; manually shared Scripts logs should be reviewed before sharing. [Duel diagnostics](development.md#duel-diagnostics)
explains the optional `--duel-check` flag and replay tools.

## When something fails

- **Missing helper or DLL:** restore the complete matching Scripts bundle, including `yo3-auth-launch.exe` and
  `yo3-auth.dll`. Check security software's protection history if a file disappeared; verify the official download
  before restoring it. For a verified false positive, allow the affected file or exclude only the extracted
  `yo3-revival-scripts/` folder. [Antivirus help and DLL locations](guide.md#windows-smartscreen-and-antivirus-warnings).
  Do not delete helpers or combine files from different releases.
- **Game folder rejected:** choose the complete installation with `yo3.exe`, `yo3.idx` and `package/`.
  The executable must have the supported **v1.153** hook layout. Revival is not the original game installer.
- **Missing `d3dx9_40.dll`, `MSVCR90.dll`, `MSVCP90.dll` or runtime manifest failure:** follow
  [original game runtime repair](guide.md#original-game-runtimes), then retry. The checker does not start the game.
- **Cannot load host game data / no cards:** fix the host's `--game-dir`. Current builds refuse hosting with
  missing, corrupt or unusable card/deck data. Keep existing accounts, decks and identity files.
- **Connection mod did not initialize:** keep the `stage=...`, `windows_error=...` and any process exit code from
  the terminal/session log. These identify the failed startup step; EOF by itself does not establish the cause.
- **Already initialized / profile already exists:** reuse the existing `data/` and profile. These setup commands
  deliberately refuse to overwrite them. Do not delete account data just to rerun initialization.
- **Connection refused or timed out:** check the profile address/port and whether the host is running and reachable.
- **TLS or account refused:** use the correct private profile and adjacent certificate for this host. Confirm
  unexpected identity changes with the host; certificate checks cannot be bypassed.
- **Duels end immediately:** restore the complete `engine/` directory, including its Node runtime, and inspect the
  server's **DUELS WON'T WORK** message.
- **Need help:** keep the exact terminal error, application version and relevant session logs. Review them before
  sharing; do not send private profiles, account keys, databases or server keys.

Windows scripts pause on errors so a double-clicked window stays readable, and preserve the program's error code.
For more, see [general troubleshooting](launcher.md#troubleshooting).

## Updates

Scripts do not install updates automatically. Download the new Scripts archive and extract it into a **separate
folder first**. Stop the old server/game, back up and carry over your `data/`, private profiles and certificates.
Keep players' local `Documents/Yu-Gi-Oh! ONLINE 3` folders too. Read [backup and upgrade details](launcher.md#your-data-and-upgrades)
before restoring an older setup.

The archive filename includes the version; the inner directory is always `yo3-revival-scripts/`. This stable name
does not make it safe to overwrite a running server. [Desktop](launcher.md#application-updates) can download and
install a complete verified update after confirmation. Both editions share the same authenticated protocol and data format.
