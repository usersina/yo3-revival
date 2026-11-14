# Changelog

Pre-alpha: expect bugs, and please [report them](https://github.com/usersina/yo3-revival/issues/new/choose).

Each section is a release. To release, rename the "Unreleased" section at the top to the next version
(`## vX.Y.Z (date)`) and push to `main`: the release workflow builds the bundles and publishes the version under that
tag, with the section as its notes. A push whose newest version is already released publishes nothing, so changes can
gather under "Unreleased" first.

Older entries describe their release at the time. For current setup, use the [README](README.md) and
[graphical setup guide](docs/launcher.md). Releases through v0.0.3 predate the graphical launcher and TLS login.

## v0.0.13 (2026-11-14)

Duel windows that speak the card's words, a bigger and sharper game, NPC duel logs, fairer clocks and the shop's
Trade Card and Rental. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions
include the server, rules engine and required connection components. Original game files are not included.
**Windows 7 is not supported.**

**Everyone needs this version:** the connection files changed, so a host and their friends must all update together.
Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.13/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.13/docs/scripts.md).

- **You choose the zones.** Ground Collapse, Seismic Shockwave and the other cards that make zones unusable now ask you
  with the game's own message, and you click the zones; Senet Switch asks where to move. Unusable zones show a red
  cross. Cards you summon, Set or activate fill the middle zones first, as on the original server.
- **You choose the order.** Big Eye and the other cards that put cards back on your Deck, and effects that trigger
  together, let you click them in order; the first click goes on top.
- **Revealed cards are shown.** Ancient Telescope, Reversal Quiz and other looks at a Deck turn its top cards over on
  the pile, a Set card shown to you flips up for a moment, and with Convulsion of Nature both Decks show their top card.
- **Changed Types and Attributes show.** After Scroll of Bewitchment, DNA Surgery or Zombie World, the card's label and
  info panel show the new Attribute or Type.
- **The duel windows speak for themselves.** A card's effect choices now show as buttons with the card's own words
  (Pyramid Energy: ATK or DEF), Synchro Summons ask for "Synchro Material", and a card's own question or hint shows in
  its window. Big Eye no longer asks you to "Declare monster level". Choices with more than four effects still list
  them in the chat window.
- **Duel logs for NPC duels.** A duel against an NPC is now saved like one against a player: find it in Duel Log →
  Latest Duel Logs and press Play to watch it again.
- **Fairer duel clocks.** Your clock no longer runs while the game is still playing animations: your game now tells
  the server when it has finished them, so a slower computer no longer loses thinking time.
- Your avatar speaks its line when you flip a Set card or reveal a hand card to activate it, as in the original.
- **A bigger, sharper game (optional).** Desktop: Advanced → Game setup → Display → Resolution; Scripts: add
  `--resolution 1440x1080` (or `auto`) to `play`. The game keeps its 4:3 shape, with black bars on a wider screen;
  card pictures stay as detailed as their files. Opening `yo3.exe` directly still uses the original size.
- **Trade Card at the shop.** Give Blister 10 cards you don't need and get any card you pick from the first 48
  booster packs.
- **Rent from the shop.** Borrow 5 cards of your choice for 100 BP for 7 days, to try a deck before you build it.
- **Fixed:** a token equipped to Allure Queen stayed on the field after it left.
- **Fixed:** summoning a Psychic with Brain Research Lab's extra Normal Summon could stop the duel with an error.
- **Fixed:** Card Trooper could send a different number of cards than the one you picked.
- **Goyo Guardian plays by its card text:** any Tuner will do for its Synchro Summon, not only an EARTH one.
- **Fixed:** with two Double Spells in your opponent's Graveyard, a duel could freeze or stop with an error.

## v0.0.12 (2026-11-13)

Decks that follow you, an Official deck tab and fixes for getting stuck or thrown out. Choose **Desktop** for the
graphical app or **Scripts** for terminal commands. Both editions include the server, rules engine and required
connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.12/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.12/docs/scripts.md).

- **Your decks follow you as soon as you save them.** Saving a deck in Edit Deck (Save or New) now sends it to the
  server, so it shows up in Load Deck when you play from another computer, without using it in a duel first. A
  computer that already has a deck of that name keeps its own copy.
- **Official decks have their own tab.** The Yugi, Jaden and Yusei character decks, your starter deck's original
  recipe and the structure decks you bought are now under Edit Deck → Load Deck → **Official**, which used to be
  empty. Your **Deck** tab keeps your own decks, so they all fit. To duel with an official deck, load it and save it
  with New. Copies you already have stay where they are.
- **Fixed: stuck at rock-paper-scissors.** If your opponent left a duel while you were choosing rock, paper or
  scissors, you stayed on that screen for good and had to close the game. You now see "(name) has disconnected." and
  return to the map, with nothing counted.
- **Fixed: the game closing when the host goes away.** If the host's server stopped or crashed, or the network
  dropped, the game itself crashed. You now see "You have been disconnected" and go back to the title screen. A host
  whose computer went to sleep or lost its network is noticed within about a minute and a half too.
- **Fixed: waiting for an opponent who never shows up.** If your opponent's game closed after choosing a deck, you
  waited for good. After 90 seconds the duel is cancelled and you're sent back. In events, a match nobody joins now
  ends on its own so the tournament goes on.
- **Each server keeps its own decks.** Everyone starts as "Duelist", so joining a second server showed the first
  server's decks (which it might refuse) and skipped its starter deck. Each server now has its own local game data.
  The first server you play on keeps your current decks; any other server starts with your display settings and a
  fresh starter deck.
- **Fixed:** a duel against a computer opponent the server doesn't have now stops at deck selection instead of
  leaving you stuck after rock-paper-scissors.

## v0.0.11 (2026-11-13)

A fairness fix for duels against the computer. Choose **Desktop** for the graphical app or **Scripts** for terminal
commands. Both editions include the server, rules engine and required connection components. Original game files are
not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.11/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.11/docs/scripts.md).

- **No more peeking at computer opponents' face-down cards.** In NPC duels the attack-target window and the card
  panel showed a face-down monster's name, art and ATK/DEF. They now show a card back until the card is revealed, as
  in duels between players.

## v0.0.10 (2026-11-13)

Computer opponents that think, talk and have a character of their own, and much smaller downloads. Choose
**Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the server, rules engine
and required connection components. Original game files are not included. **Windows 7 is not supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.10/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.10/docs/scripts.md).

- **No more Node.js inside:** the rules engine is now a small program of its own, built from the same EDOPro core,
  instead of the core running inside a bundled copy of Node.js. Downloads are much smaller and hosts still install
  nothing. Duels play exactly as before: every test duel was played through both engines and they agreed.
  Updating from v0.0.9 or older works as usual from the app.
- **Computer opponents actually play now.** Kaiba and the other NPCs used to summon their first monster and attack
  with everything. They now think about each move: they try their options ahead within their turn, use their Spells
  and Traps (removal when you summon, Ritual Summons, Set Traps waiting for your attack), choose good targets and
  keep blockers up, and they think through your next turn before committing, in about a second at most. They
  play fair: they never peek at your hand, your Set cards or anyone's Deck (with a few famous exceptions below).
- **Each opponent has a character:** Kaiba and Jack Atlas attack relentlessly, Syrus and Luna play it safe, and
  Maximillion Pegasus... reads your mind. His Millennium Eye sees your face-down monsters and Set cards.
- **The heroes believe in the Heart of the Cards:** Yami Yugi, Yugi Muto, Jaden Yuki and Yusei Fudo always draw the
  card they need. Good luck.
- **The opponents talk.** Every NPC has its own lines in the duel's chat window: a greeting, taunts, its big
  monsters by name, its direct attacks, its reaction to a heavy hit, and a word when the duel ends. The heroes call
  on the Heart of the Cards as they draw, and Pegasus tells you which card is in your hand and which one you just
  Set. He is not bluffing.

## v0.0.9 (2026-11-12)

Every card checked against the rules engine, duels that look and sound like the original game, and choices back in
your hands. Choose **Desktop** for the graphical app or **Scripts** for terminal commands. Both editions include the
server, rules engine and required connection components. Original game files are not included. **Windows 7 is not
supported.**

Before updating, close the game and stop hosting. Keep your server data and local game Documents; see the
[Desktop update guide](https://github.com/usersina/yo3-revival/blob/v0.0.9/docs/launcher.md#application-updates) or
[Scripts guide](https://github.com/usersina/yo3-revival/blob/v0.0.9/docs/scripts.md).

- **Duels show what the original game showed,** worked out from duel logs recorded on Konami's own server in
  2010–2012: Life Point changes show their number and sound, a Field Spell changes the duel field's background,
  Sets and activations play their effects, and your avatar speaks your Avatar Settings lines when drawing,
  summoning, activating, Setting, ending a turn and taking damage (the game's "speech" duel option turns it off).
  Moves play the right sound, and a surrender reads as in the original.
- **Every card checked:** each of the game's cards now has to load in the rules engine with the game's own Level,
  ATK and DEF, and each is played through a full test duel, as are the decks people used on Konami's server. This
  found and fixed:
  - Barrel Dragon and Red-Eyes Darkness Metal Dragon didn't work in v0.0.8.
  - A deck's second-print Dark Magician or Polymerization silently vanished from the duel.
  - Placing a card on the opponent's field (Give and Take, Black Garden), disabling two zones (Ground Collapse) and
    removing counters from several cards (Cloudian - Altus) could stall the duel.
  - Machina Fortress couldn't be Special Summoned, and Doomsday Token, Grinder Golem's tokens and Slime Tokens never
    appeared.
- **Your choices are yours:** a card with two effects you can activate at once asks which one; an effect's options
  the game has no label for show as cards to pick from instead of the first option being chosen for you; and when
  cards must add up to a Level (Machina Fortress's discards, a Ritual Summon's tributes) you pick them, then confirm
  with a right-click.
- **Replays show both players' cards,** as the original's did, and duel log files match the original's layout.
- **Decks the duel can't play are refused:** a Fusion or Synchro Monster in the main deck (the game's deck editor
  doesn't allow it either), and the 12 game exclusives the rules engine doesn't have (Ulevo, Xexex and others; no
  pack or preset deck holds them).
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
