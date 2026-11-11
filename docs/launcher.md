# Desktop guide

Use **Desktop** for a window with game setup, a Start button and logs. Use [Scripts](scripts.md) if you prefer a terminal.
Both editions include the server, rules engine, Node.js and required connection components. Game files are separate.

- [Desktop guide](#desktop-guide)
  - [First launch](#first-launch)
  - [Play locally](#play-locally)
  - [Join another server](#join-another-server)
  - [Host friends](#host-friends)
  - [Connection help](#connection-help)
  - [Open the game directly](#open-the-game-directly)
  - [Logs and support](#logs-and-support)
  - [Application updates](#application-updates)
  - [Your data and upgrades](#your-data-and-upgrades)
  - [Troubleshooting](#troubleshooting)
  - [Getting the original game](#getting-the-original-game)

## First launch

You need a complete original **Yu-Gi-Oh! ONLINE 3 v1.153** installation and **64-bit Windows 10/11** or **Linux with Wine**.
**Windows 7 is not supported in either Desktop or Scripts; compatibility work is out of scope for now.**
macOS does not have a supported build. [Game installation help](#getting-the-original-game).

1. [Download the latest release](https://github.com/usersina/yo3-revival/releases/latest). Choose the **Desktop**
   archive for your system and extract **all** of it.
2. Open `yo3-revival.exe` on Windows or `yo3-revival` on Linux inside `yo3-revival-desktop/`.
3. Stay in **Normal** mode. Click **Browse…** and select the original game folder containing `yo3.exe`, `yo3.idx`
   and `package/`. Select the installed folder, not a shortcut, installer or the extracted Revival folder.
4. Click **Validate game**. When it passes, click **Start**.
5. In the game, choose **ONLINE**, then select your player. [Start your first duel](playing.md#your-first-duel).

![Normal mode with validation, Start and visible logs](../screenshots/desktop.png)

Validation checks the game, matching connection helper/DLL and the components needed for your saved connection.
A successful validation saves the game folder. Changing the folder requires validating again before Start becomes
available. Start loads the connection mod automatically; it does not change `yo3.exe` or `yo3.ini`.

If Browse does not open a usable file dialog, choose **Use path instead** and paste the folder path into the box.
Close any remaining file dialog; its late selection will be ignored. A failed check shows its cause and recovery
steps, with **Show details**, **Copy error** and **Export logs…**. Missing Revival files need a complete matching
bundle restored; choosing a different game folder will not fix them.

**Normal** is one screen: game folder, validation, Start and logs. **Advanced**, at the top right, adds the
**Play**, **Hosting**, **Activity** and **Game setup** pages. Switching modes preserves saved settings and drafts.
In Advanced, game and Wine settings use **Save changes**; Wine controls appear only on Linux.

## Play locally

On a new setup, **Start** prepares your local account and starts everything needed automatically. You do not need
a sharing address, an invitation, firewall changes, Rust or a separate Node.js installation to play on your own PC.
There is no project-operated public server.

Animated dots and the logs show startup activity. Wine can take about 30 seconds on a cold start. **Close game**
ends the managed game session and can cancel its initialization once that button is available. Errors stay visible
until dismissed or another action replaces them; [troubleshooting](#troubleshooting) explains the recovery controls.

Your first legal starting deck is prepared and selected once, without opening Edit Deck. Existing decks and all later
manual edits or deletions are preserved. See [the playing guide](playing.md) for computer opponents, friends and decks.

Closing the game leaves your local server available while the app stays open. Closing the app in Normal mode asks
for confirmation and stops its managed game and local server. Advanced also offers **Keep hosting & exit**.

## Join another server

Ask the host for your **private invitation file**. Each friend needs their own account.

1. Switch to **Advanced → Play**.
2. Click **Import invitation…** and select the file. Review the connection and click **Save connection**.
3. Select it in **Server**. Use **Test connection** to check the address, compatibility and account from your PC.
4. Click **Play**, or return to Normal and click **Start**.

![Advanced Play with the server selection and invitation controls](../screenshots/desktop-advanced.png)

Switching back to Normal keeps this connection. To play on your own server again, choose **This computer** in
Advanced → Play. Close the game before changing its server or account. The account selected on Play is independent
of the account being managed under Hosting.

A host can instead provide an address and account key. On **Advanced → Play → Add server**, enter the DNS name or
IP, optionally with a port (default **7443**), then click **Check server**. Compare the identity code with the host,
choose **Trust this server**, enter the private key and **Save connection**. You do not need to handle certificates
manually. `yo3://host:port` addresses are also accepted. Later connections verify the saved identity automatically.

CLI profiles can also be imported if their public certificate is beside them. **Edit** changes a saved connection;
changing the address requires checking its identity again. **Remove** deletes the saved connection locally, leaving
the account and game data on the host. Never post invitations or account keys in a bug report.

## Host friends

Hosting and playing are independent: you can host without opening a game, or host while playing on another server.

1. Switch to **Advanced → Hosting → Advanced**. If your local server is running, close its game and choose
   **Overview → Stop server** before changing hosting settings.
2. Set **Sharing address** to the LAN/VPN address or hostname your friends can reach. Leave **Listen on** at
   **All interfaces (0.0.0.0)** unless you need a specific interface. Click **Save changes**.
3. Under **Overview**, click **Start server**.
4. Open **Players & invitations → Create account**. Enter a unique first-player name, **1–16 characters**, and
   click **Create account**.
5. Select that account and click **Export private invitation…**. Send the saved file only to that friend.
6. They follow [Join another server](#join-another-server). Repeat account creation for each friend.

![Hosting overview with independent server controls](../screenshots/desktop-hosting.png)

The default public port is **TCP 7443**. Friends connecting from outside your PC need network access to it; see
[Connection help](#connection-help). **Start server** does not request administrator access or change firewall rules.
**Running on this computer** confirms local status, not reachability from a friend's network.

**Players & invitations → Additional players** adds players to the selected account, up to four total. Players choose
their slot inside the game. Re-export an account's invitation when needed; managing it does not switch your login on Play.

Under **Hosting → Advanced**, you can choose an interface, change the port/data folder and set the first player for
a new server. A saved interface follows address changes; an unavailable interface reports an error rather than
opening access on every interface. Stop the server before saving hosting changes.

**All server flags** is searchable and comes from the actual server CLI, including `--duel-check` and future run flags.
Enter values directly, not shell commands. **Reset** removes an override. Settings owned by another control point to
that control; log paths stay app-managed. Use the [Scripts edition](scripts.md) for fully manual paths.

## Connection help

Check the saved sharing address/port, whether the host is running, and whether that address is reachable from the
friend's computer. Localhost only reaches the same PC; `0.0.0.0` is a listening setting, not an address to send friends.
An explicit external port in the sharing address can match a router/VPN mapping.

Only the chosen **TLS port** needs to be reachable by players. The internal HTTP/admin listener stays on loopback;
do not forward it. The DLL supplies private local game ports automatically. You do not need to edit `yo3.ini` or
forward the old game ports. A timeout alone does not identify a firewall or router as the cause.

Manage firewall access in the operating system and router/VPN routing outside the app. See the official
[Windows firewall tools](https://learn.microsoft.com/windows/security/operating-system-security/network-security/windows-firewall/tools)
or [firewalld port/service guide](https://firewalld.org/documentation/howto/open-a-port-or-service) if applicable.
An application-specific firewall allowance may need updating after an executable path changes.

The app maintains its self-managed certificate at startup while preserving the server's identity key. Normal joining
uses an invitation or remembered identity; it does not require copying certificates. Confirm unexpected identity
changes with the host. [Authentication details](authentication.md) and [CLI network settings](guide.md#network-settings)
are available for advanced setups.

## Open the game directly

This is optional. First save the game folder and select the server/account on Advanced → Play. Then open
**Advanced → Game setup → Advanced game options → Enable direct launch**. If using your local server, start it first.
You can then open the original `yo3.exe` or its shortcut; direct launch uses the saved connection and does not start
a stopped server for you.

The setup adds an owned `winmm.dll` loader and receipt beside the game, leaving the executable and INI unchanged.
Credentials stay in private app storage. A protected game folder may request permission for this explicit setup or
removal. It refuses to replace another mod.

**Repair direct launch** refreshes missing or outdated owned components; **Disable direct launch** removes that setup.
Disable it before moving the saved game location or changing the Wine prefix. Status shows Enabled, Disabled,
Needs repair or Mod conflict.

The loader checks required components and server compatibility before login. A missing DLL gives its expected path
and offers a matching-file picker, followed by restart. A game with no loader at all can only show the original
generic connection failure: the remote server cannot inspect files on your PC. Use the Desktop app to set it up.

## Logs and support

Normal mode always shows logs below Start, with **Copy** and **Export logs…**. Advanced → **Activity** separates
**Server** and **Game & launcher** logs, with filtering and pause controls. Startup output is captured even if a helper
fails before opening its own log session. Game stdout/stderr, DLL diagnostics, decoded server packets (including
nested fields) and engine stderr are captured automatically; there is no tracing checkbox to enable beforehand.
The ordinary view hides verbose payloads. Search **YO3 CLIENT**, **YO3 PACKET** or **YO3 ENGINE** in Activity to see
those retained traces, or export them.

To report a problem:

1. Reproduce it and note what you clicked or, for a duel, the card names and sequence of actions.
2. Use **Show details → Copy error** for the specific failure, then **Export logs…**.
3. Select **Current session** or **All retained sessions** and review the source file count/date range/size.
   Full detail is selected by default. Optionally enable **Remove personal data**, then **Save ZIP…**.
4. Review the ZIP and attach it to [a bug report](https://github.com/usersina/yo3-revival/issues/new/choose), with the app version.

Exporting keeps the original error visible. An export failure has its own message and **Retry preview** or **Save ZIP…**
actions. If saving is unavailable, use **Copy logs** or **Copy error**. Nothing is uploaded automatically.
Do not share private invitations, profiles, account keys, databases or `server.key`.

Full reports can contain player names, chat, decks, user paths and network addresses. **Remove personal data** omits
client/packet/engine payload traces and player-activity lines, and masks common user paths, IP addresses, server URLs
and email addresses. This reduces debugging detail; it is best-effort filtering, so review the remaining text before
sharing. Filtering changes only the exported ZIP: the original logs remain available for a later full report.
Credentials are scrubbed during capture and again at export, regardless of that option.

For a duel bug that needs state validation and exact replay, enable **Advanced → Hosting → Advanced → All server
flags → --duel-check**, save and restart the hosted server between games. Ask both players to collect matching logs.
This extra checking remains optional because it adds engine queries and a potentially slow dependency scan. Automatic
logging does not enable it. Disable the flag when the investigation is finished.
[Developer replay and validation](development.md#duel-diagnostics) explains the automated checks.

Logs rotate at 2 MiB and are retained for up to 14 days or 100 MiB, protecting active logs. The viewer reads a bounded
tail. Current-session exports include this app, its attached server and latest game run; all-session exports also
include retained history. Reports exclude credentials, databases and game files, and omit developer packet logs
outside the indexed session directory. Decoded packet traces are inside the session logs even when separate raw
packet files are disabled. Oversized lines/payloads are bounded; this is diagnostic capture, not a full memory dump.
Original game stdout from optional direct launch is not redirected into the helper's session; reproduce through
Desktop **Start** or Scripts **play** when that trace is needed. A remote player's export cannot include the host's
server logs: the host must export those separately.

## Application updates

The footer shows the app version and GitHub link. Click the version for **About & updates**. Startup checks leave
play available when offline or rate-limited; use **Check for updates** or **Releases** to retry or download manually.

For a newer compatible Desktop release, choose **Review & install…**, read the notes and confirm **Download & restart**.
You can cancel while downloading. The app verifies the download and component manifest before activation.
Finish duels first: the managed game closes and restarting hosting disconnects its players. Settings, accounts and
server data remain in place. Press **Start** yourself to reopen the game afterwards.

If verification or the new app's startup fails, the previous version remains available. Existing launcher shortcuts
route to the active version; keep the original bundle if using its shortcut or an older direct-launch setup.
Updates live under `updates/installs/` in private app storage, with the active/previous selections in `launcher.db`.
Direct launch may need **Repair direct launch** after an update. Development builds stay in place.

Archives have versioned filenames, but stable inner folders: `yo3-revival-desktop/` and `yo3-revival-scripts/`.
Do not overwrite a running bundle. [Scripts updates](scripts.md#updates) are manual.

## Your data and upgrades

App storage defaults to `%LOCALAPPDATA%\YO3 Revival` on Windows and
`${XDG_DATA_HOME:-~/.local/share}/yo3-revival` on Linux. Open it through
**Advanced → Activity → Open application data**.

| Location inside app storage              | Contents                                                              |
| ---------------------------------------- | --------------------------------------------------------------------- |
| `launcher.db`                            | Preferences and saved private connections                             |
| `server/server.db`                       | Hosted server state, including accounts, cards, BP and uploaded decks |
| `server/server.crt`, `server/server.key` | Public certificate and private server identity key                    |
| `logs/`, `profiles/`                     | Diagnostics and private connection profiles                           |

To reuse an existing server, **stop it and back up its entire data folder first**. Set that folder under
**Advanced → Hosting → Advanced → Data folder**, save, then start hosting. Changing the path selects data; it does
not move existing data. The default is the app's `server/` directory.

Old JSON state imports into SQLite in one transaction. Account/player IDs and credentials are preserved. Original
JSON files remain as migration copies and no longer receive writes. A failed import reports an error; repair its
source before retrying. Do not remove a migrated database to try to reimport stale JSON.

Each server startup creates a consistent database backup under the server data folder's `backups/`. These backups
exclude the identity key: separately back up the **whole data folder** securely. Players should also back up their
local `Documents/Yu-Gi-Oh! ONLINE 3` folder, which holds editor-only deck saves.

Restore only with the server stopped. Older releases cannot read the current SQLite state; downgrading requires
the original pre-upgrade snapshot and loses later database-only changes. Do not downgrade by copying back the old JSON.

## Troubleshooting

| Problem                                      | What to do                                                                                                                                                                                                                                               |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Downloaded Revival but have no game          | [Install the original game separately](#getting-the-original-game), then choose its installed folder.                                                                                                                                                    |
| Browse does not open a dialog                | Choose **Use path instead**, close any remaining dialog and paste the game folder path.                                                                                                                                                                  |
| Start is disabled                            | Choose the game folder and click **Validate game**. If it fails, use **Show details** and the repair steps.                                                                                                                                              |
| Incomplete game                              | Select the full installation containing `yo3.exe`, `yo3.idx` and `package/`, not an installer or shortcut.                                                                                                                                               |
| Unsupported game executable                  | Use the original **v1.153** installation. Validation checks the hook signatures before startup; replacing Revival helpers cannot fix a different game build.                                                                                             |
| Missing DirectX or VC90 runtime              | Follow [original game runtime repair](guide.md#original-game-runtimes), then **Validate game** again. The error identifies the dependency or runtime manifest.                                                                                           |
| Revival helper/DLL or engine missing         | Re-extract the complete matching Desktop archive, keeping every helper and `engine/`. A different game folder will not repair app files.                                                                                                                 |
| Antivirus flags Revival / a file disappeared | Check protection history and the official download. For a verified false positive, allow the affected file or exclude only the extracted Desktop folder. [Exact files, folders and recovery steps](guide.md#windows-smartscreen-and-antivirus-warnings). |
| App settings/logs cannot open                | Check the data folder's permissions/free space, then **Retry app setup**. Copy the error if it continues; keep your saved data.                                                                                                                          |
| Game startup fails or stalls                 | Read **Show details** and the logs. Use **Close game** when available, validate and try again; export logs if it repeats.                                                                                                                                |
| DLL did not initialize / unexpected EOF      | Current builds include the native failure stage and Windows error/exit code in details and logs. Keep those details when reporting; an EOF alone does not identify the cause.                                                                            |
| Trunk shows no cards                         | Clear the editor's filters and check server logs. The host must load complete game card/deck data; current builds refuse hosting if that fails. Correct the host's game folder and restart, keeping all account/deck data.                               |
| Cannot connect to a friend                   | Confirm the selected connection in **Advanced → Play**, then **Test connection**. Check [network access](#connection-help).                                                                                                                              |
| Identity or authentication refused           | Confirm identity changes with the host, or import the correct invitation for this server/account.                                                                                                                                                        |
| Cannot save a diagnostic ZIP                 | Check the error in the export dialog, choose a writable folder and retry. Copy logs/error as a fallback.                                                                                                                                                 |
| A long session disconnects                   | Sessions last 12 hours. Close the game and start it again.                                                                                                                                                                                               |
| A duel ends immediately as a draw            | Check server logs for **DUELS WON'T WORK** and restore the complete bundled engine. Source builds need the [engine dependencies](development.md#build-from-source).                                                                                      |
| A card seems to behave incorrectly           | Check [card rules](playing.md#card-effects-and-summon-rules), then [record the duel](#logs-and-support) if needed.                                                                                                                                       |
| Windows 7                                    | The current release requires a supported modern OS, even if the original game itself runs there.                                                                                                                                                         |

These executables and the injector/DLL are unsigned. [Windows warnings](guide.md#windows-smartscreen-and-antivirus-warnings)
explains what to check. Native Windows 11 checks cover standard-user startup in a writable game copy, Desktop and
Scripts hosting/play, starter setup, a complete NPC duel, log export and updater activation. Windows 10, protected-folder
direct-start installation and runtime reinstallation still need native checks. For terminal errors, see
[Scripts troubleshooting](scripts.md#when-something-fails).

## Getting the original game

Revival does not download or distribute the original game. Use a complete **v1.153** installation you are entitled
to use. A third-party archive reference is the
[MyAbandonware game listing](https://www.myabandonware.com/game/yu-gi-oh-online-3-duel-accelerator-qy4).
This link is a reference, not a Revival download or a guarantee about that archive.

A typical installed folder contains `yo3.exe`, `yo3.idx` and `package/`. On Windows it is often under
`C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3`; on Linux it is often inside your Wine prefix's `drive_c`.
Keep the original game's folder separate from the extracted `yo3-revival-desktop/` folder.
