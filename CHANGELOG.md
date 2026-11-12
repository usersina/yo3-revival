# Changelog

Pre-alpha: expect bugs, and please [report them](https://github.com/usersina/yo3-revival/issues/new/choose).

Each section is a release. To release, rename the "Unreleased" section at the top to the next version
(`## vX.Y.Z (date)`) and push to `main`: the release workflow builds the bundles and publishes the version under that
tag, with the section as its notes. A push whose newest version is already released publishes nothing, so changes can
gather under "Unreleased" first.

Older entries describe their release at the time. For current setup, use the [README](README.md) and
[graphical setup guide](docs/launcher.md). Releases through v0.0.3 predate the graphical launcher and TLS login.

## Unreleased

- Removed the starting-deck onboarding notice from in-game login news.
- Desktop folders now include `yo3-revival.ico` and `yo3-revival.png` for custom shortcuts. The Windows executable
  continues to contain the launcher icon itself.

## v0.0.8 (2026-11-12)

A new look for the Desktop app, BP that shows up right away, and BOOSTERPASS codes you can manage from the app.
Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server,
rules engine and required connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.8/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.8/docs/scripts.md).

- **A new look for the app:** calm dark panels with small corner labels and one blue accent, in the spirit of Master
  Duel's menus, over a backdrop of faint circuit traces like the icon's. Logs keep the original game's green-on-black
  chat style, with errors in red and warnings in amber.
- The app has a new Duel Disk icon with a Monster Reborn badge, shown in the app itself, its window and the Windows
  executable.
- **BP shows up right away:** BP from duels and from the host appears in the game on its next screen, without logging
  in again. Before, the game showed new BP only after a purchase or the next login. A gift from the host also arrives
  as a D-Mail "BP received" (the mail icon lights up, even during a duel).
- **Give BP and BOOSTERPASS codes in the app:** Advanced → Hosting → Players & invitations has Give BP for the selected
  account, and a BOOSTERPASS codes section to make codes (random ones too), see who claimed each and when, and remove
  them. The `codes` command lists claim times too.
- **Duel logs in the database:** the replays the game downloads after a duel between players are now kept in
  `server.db` (the newest 1000) instead of loose files. An old `duellogs/` folder is reported as safe to delete.
- **Clearer logs:** errors are red and warnings amber in the app, and in a server terminal. Old `accounts.json` copies
  are reported as a warning that they are no longer read and safe to delete (accounts live in `server.db`).
- **No firewall prompt when playing alone:** your local server only accepts this computer until you save a sharing
  address for friends. Before, Windows asked to allow `yo3-server.exe`, and choosing Cancel quietly blocked friends
  later. If that happened to you, see Connection help in the Desktop guide.
- **Coming from the Scripts edition:** Hosting → Advanced shows which data folder the app uses; choose your Scripts
  `data/` folder there to keep its accounts and BP. Starting while another server uses the same port now says so.
- Fixed: a shop purchase that could not be saved took the BP without giving the cards. Two games on one account
  can no longer buy the same protector or avatar item twice, and BP given during a session can be spent at once.
- Fixed: a command-line (Scripts) server's certificate expired after a year and friends could no longer connect.
  The server now renews it at start, keeping the same identity, so existing invitations and profiles keep working.

## v0.0.7 (2026-11-12)

A smoother Desktop experience, clearer startup errors and bug reports with diagnostics attached automatically.
Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server,
rules engine and required connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.7/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.7/docs/scripts.md).

### Built-in bug reports

- Added in-app GitHub sign-in and **Publish bug report**. Reports include version/system details, captured errors and
  linked diagnostic logs automatically; the review keeps the public-sharing notice and action buttons visible.
  The smaller **Exclude chat, decks, paths & addresses** option filters the report while preserving original logs.
- Failed report publishing preserves progress across reopening, reuses uploaded logs and checks uncertain requests
  before retrying. GitHub credentials use the system credential store, with a session-only fallback when unavailable.

### Startup and recovery

- Added **Help & support → Reset local game data…** with **Back up & reset**. The default preserves every player file, saved
  deck and selected deck; an explicit full reset moves the entire client folder into a dated backup. Server data
  and Revival connections are preserved, with running-game and folder-overlap checks before changes.
- Fixed intermittent Windows authentication failures caused by TLS response cleanup. Complete, validated replies
  remain successful when the peer resets during closure; incomplete replies and authentication failures still fail.
- Failed Desktop server startup now stops its own unfinished process and permits another Start. Shutdown waits
  before permitting a restart, including when Desktop reconnects to an already-running server.
- Connection timeouts identify the stalled stage in the logs, distinguishing TCP, TLS and discovery response waits.

### Simpler Desktop controls

- Desktop now uses one **Play** button with checking/starting progress. Choosing a game folder checks it and saves
  successful results automatically; Play rechecks required files, and imported invitations verify the server and account.
- Refined Desktop guidance with inline field errors and visible filenames, failure stages and native codes when
  available. Original launch failures stay visible during background checks. Status colors and contextual help remain
  consistent; formatted update notes scroll separately while restart warnings and action buttons stay visible.
- Direct launch is now a checkbox with contextual repair and clear prerequisites. Normal/Advanced controls fit the
  sidebar, and server/account selection visibly locks while an operation is running.
- Missing required files now appear together as exact paths, with a short antivirus/recovery hint. Local Play and
  hosting recheck the bundled server, Node.js and engine dependencies, including when hosting is already running.
- Error notices show the useful diagnosis directly, with a copy icon replacing the details expander. Normal mode
  has a smaller log box that groups repeats and hides Wine diagnostics; exports preserve the original output.
- Removed empty space in short log panels, release notes and forms, with consistent mode labels and dialog spacing.

## v0.0.6 (2026-11-11)

- Desktop logs open at the newest line and follow new output only while you are at the bottom. Scrolling up keeps
  older lines in view; scrolling back down resumes following. The scrollbar now sits at the right edge in both modes.
- Rust package versions now match the application release: `0.0.6`.

## v0.0.5 (2026-11-11)

A simpler launcher, native Windows startup fixes and more reliable duels. Choose **Desktop** for the graphical app
or **Scripts** for terminal start/play commands. Both editions include the server, rules engine and required
connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.5/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.5/docs/scripts.md).

### Simpler play and hosting

- Normal mode is one screen: choose the game folder, **Validate game**, **Start**, and visible logs.
  Switch to **Advanced** for server connections, hosting and full controls without losing settings or drafts.
- The launcher opens in a smaller, centered window so Version, GitHub and Help remain visible on laptop-sized screens.
- Advanced hosting lists the server's actual CLI flags, including independent `--duel-check` controls, with
  validation and per-option reset. New run flags appear automatically.
- Startup has animated feedback, live logs, cancellation and timeouts. Errors stay visible with details, copy and
  log export; diagnostic failures have their own recovery controls. Missing game files and missing Revival files
  get different repair guidance, and early helper/server failures are captured. A folder-picker fallback allows pasting a path.
- Releases provide **Desktop** and **Scripts** editions for Windows and Linux. Archives remain versioned, while
  their inner folders are `yo3-revival-desktop/` and `yo3-revival-scripts/`. Desktop still downloads and installs
  complete verified updates after confirmation; terminal users get start-server/play scripts and a dedicated guide.
- Script defaults can be overridden for bind/data/log paths. Ctrl+C during early game startup now cleans up.
- The README is a short first-play guide; detailed help is organized in `docs/`, with current screenshots in
  root `screenshots/`. Release bundles keep the same documentation layout and check their links.

### Windows startup and game validation

- Fixed Windows game startup requesting administrator access and then failing to inject the connection DLL.
  Desktop and Scripts now run the game with your normal user permissions.
- Fixed the connection DLL rejecting supported Windows games when ASLR changes their load address. Login,
  starter-deck setup and duel diagnostics now follow the actual image base without disabling ASLR.
- Fixed native Windows path handling: a supported game no longer fails runtime validation merely because its
  resolved path uses Windows' extended-path prefix. Game, connection DLL and direct-start paths use the compatible form.
- Game validation checks the supported v1.153 executable's hook signatures and probes its legacy DirectX/VC++ 2008
  dependencies before startup. Missing runtimes get specific repair instructions.
- DLL startup failures include their native stage and available Windows error/exit codes in launcher details and exported
  logs, with credential redaction. Logs also identify the executable fingerprint and connection mod version.
- Hosting now refuses missing, corrupt or unusable game card/deck data instead of accepting players with an empty
  collection. Fix the host's game-folder selection and retry; existing accounts, decks and identity files are kept.
- Added clear antivirus recovery guidance, including which Revival and game-folder files may need restoring after
  a verified false positive. The app does not change antivirus or firewall settings.

### Login and duels

- **ONLINE** now opens player selection directly. The required connection mod skips the unused original login
  form, including when opening the game through the installed direct-start loader.
- Fixed a local gate disconnect race exposed by automatic login, which could crash the client during connection.
- Fixed monsters such as Fortress Warrior and Shield Wing disappearing after surviving a protected battle.
  They remain visible and usable; destruction is shown only when the rules engine actually removes the card.
- Detailed diagnostics now also enables the duel checker on the next hosted-server start. The checker tracks
  battle animations that hide cards, closing a gap that let occupied but invisible slots go undetected.
- Added real-engine regressions for battle protection and Stardust's legal/illegal Debris Dragon combinations,
  plus simulations using all three original Yusei decks from the installed game.
- Fixed counters being removed during control changes and dynamic monster stats resetting during some swaps.

### Logs and bug reports

- Managed game launches automatically retain game output, connection-mod messages and decoded server packets;
  engine stderr is retained too. No capture checkbox needs enabling before a problem happens.
- Current-session and retained-history ZIP exports include full diagnostic detail by default. Optional
  **Remove personal data** filters only the exported ZIP, leaving local logs available for a later full report.
  Credentials remain scrubbed in every mode; nothing is uploaded automatically.
- Duel diagnostics now compare LP, monster stats, counters, turns and phases, and independently inspect the running
  game's state through the connection DLL. Reports reject missing snapshots and explicitly list coverage gaps.
  Exact engine recordings include deck order, choices and dependency fingerprints for developer replay.
- Detailed duel diagnostics allow cold dependency fingerprinting up to two minutes without extending the
  30-second deadline for game actions. Recording errors no longer claim that the engine and model disagree.

### Development and release checks

- All seven Rust crates inherit shared dependency versions, package metadata, one crate version and common lints
  from the workspace manifest. Resolved dependencies, enabled features and `Cargo.lock` are unchanged.
- Four test fixtures now exercise the same assertions on native Windows as on Linux.
- Publishing requires the final Desktop archive to pass the packaged-updater startup check after tests and builds.
- Fixed packaging with MinGW toolchains whose runtime code exceeds the DLL test fixture's old fixed reservation.
  Regression checks preserve its hook addresses, callback relocation and native Windows section layout.

## v0.0.4 (2026-11-10)

A graphical launcher for playing and hosting from one window. Download the bundle for your system, extract the
whole folder, and open **yo3-revival.exe** (Windows) or **yo3-revival** (Linux). Game files are not included;
Linux players also need Wine. This is still a pre-alpha.

### Play, join and host

- **Play** starts your local server automatically, or connects to a saved friend's server. Import a private
  invitation, or enter a server address and confirm its identity before supplying an account key.
- **My server** manages hosting independently of your game. Create accounts, add players and export private
  invitations without stopping hosting. Advanced interface, port and sharing options stay out of the normal flow.
- **Settings** checks your game folder and required connection components before login. Optional **direct launch**
  lets the original game executable or shortcut use your saved connection, with fail-fast component/server checks
  and repair guidance. The original executable and INI stay unchanged.
- **Activity** shows server and game logs. Export the current session or all retained sessions as a diagnostic ZIP,
  with a size/date preview and credential redaction. Nothing is uploaded automatically.
- The sidebar shows the version, Help and GitHub links. Updates are checked at startup; future updates download,
  verify and install a complete bundle **after confirmation**, with recovery to the previous version if startup fails.
- Fresh players receive a legal, selected starting deck automatically. Setup runs only once; existing decks,
  default selections, unfinished decks, and later manual edits or deletions are preserved by automatic setup.
- The launcher uses native Iced controls, icons, question-mark help and four clear pages. Wine options appear only
  on Linux. Starting a server does not request administrator access or change firewall rules.

### Upgrade from v0.0.3 or earlier

**Stop the old server and back up its entire data folder before upgrading.** Extract this bundle into a separate
folder, set the game folder in Settings, then select your existing data folder under **My server → Advanced**
**before starting hosting**. Accounts, cards, BP and records import into SQLite automatically; original JSON files
remain as migration copies. Keep players' local `Documents/Yu-Gi-Oh! ONLINE 3` folders too.

The revival launcher and connection DLL are now required. Accounts sharing one router remain separate through
TLS authentication. Allow the chosen TCP port (**7443** by default); the old raw game ports and browser `/join`
page are retired. Choose each existing account in **Players & invitations**, export its private invitation, and
send it only to its owner. Existing account credentials and player data carry over.

The GUI keeps its preferences and connections in a separate SQLite database in private application storage.
Self-managed server certificates are maintained when starting through the GUI, preserving the private identity key.
Existing launchers from v0.0.3 and earlier have no updater: download this release manually. The
[illustrated setup and upgrade guide](https://github.com/usersina/yo3-revival/blob/main/docs/launcher.md)
walks through the complete process, backups and rollback limits.

### Other fixes

- Changing a trade after confirmation requires both players to approve the new offer. Existing key-based accounts
  can trade, rental offers check ownership, and collections refresh after another session changes them.
- Refusing deletion of an account's last player preserves that player's friends, mail and rentals.
- An unrelated player cannot enter or cancel another pair's duel. Create Duel keeps the host's chosen banlist.
- Exporting another player's invitation does not switch the account used on Play. Start uses saved settings.
- Game exit, hosting after closing/reopening the manager, Unicode game/DLL paths and cold headless startup behave
  correctly. Missing or incompatible connection components stop startup before account login.

## v0.0.3 (2026-11-09)

Fixes a locked main screen and a crash. Download the bundle for your system below, unpack it, copy the `data` folder
from your old one into it (your accounts, cards and decks), and run it as before.

- Fixed: for a player with a friend (or someone ignored), the main screen stayed locked after login: only the chat
  answered clicks. The game was waiting for an old Yu-Gi-Oh! Online 2 step the server didn't answer.
- Fixed: clicking the eagle icon at the top left crashed the game. It now shows your Duelist premium.
- The "Duels are unavailable" message now says what to do with a release bundle (unpack it again) instead of asking
  for Node.js.

## v0.0.2 (2026-11-09)

BOOSTERPASS codes and BP from the host. Download the bundle for your system below, unpack it, copy the `data` folder
from your old one into it (your accounts, cards and decks), and run it as before.

- BOOSTERPASS codes: the host makes codes (`add-code`) that players type in the game's BOOSTERPASS window (Main Menu →
  BOOSTERPASS) for BP, each account once; the game shows the points and "Acquired Duelist premium!".
- `give-bp` gives BP to one player's account. Both work while the server runs, from the server's own PC.
- The admin commands that need the server stopped now say so instead of having their change overwritten.

## v0.0.1 (2026-11-09)

The first pre-alpha. A private server for Yu-Gi-Oh! ONLINE 3 (ver. 1.153): log in with the original game client and
duel the computer or your friends. Download the bundle for your system below, unpack it and run `run.cmd` (Windows) or
`./run.sh` (Linux); nothing else to install. Then follow the [README](https://github.com/usersina/yo3-revival#readme).

### What works

- Duels against the computer opponents at Kaiba Corporation and between players (Auto Duel, Create Duel, Single and
  Match Mode with side decks, spectating), on EDOPro's rules engine, in the game's own dialogs and animations. Duel
  logs and replays.
- Edit Deck, the starter deck and nine official decks for Yugi, Jaden and Yusei, banlists and a server-side deck check.
- The shop (booster packs, structure decks, protectors, avatar items) paid in BOOSTERPASS Points earned in duels, card
  trades and rentals.
- Chat rooms and whispers, player search, friends and ignore lists, D-Mail, teams, user events (knockout tournaments).
- A join page (`http://HOST:8080/join`) where friends make their own account in a browser.

### Fixed since the first test builds

- The rules engine ships inside the bundle with its own Node.js: no Node.js, Git or curl to install. A duel whose
  engine can't run ends as a draw instead of stopping at the first Draw Phase.
- The game client no longer crashes after a search (Sangan, and any card that adds from the Deck): the server now
  follows the engine's hand shuffles.
- Creature Swap, Magical Hats, Exchange of the Spirit, stolen tokens and Xyz materials show where the engine has them.
- Hovering a monster shows markers on its equip cards and on the cards that target it.
