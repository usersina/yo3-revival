# Desktop guide

> The Desktop window is the launcher of the public beta
> ([#20](https://github.com/usersina/yo3-revival/issues/20)). It downloads updates in the background and installs them
> when it restarts ([#32](https://github.com/usersina/yo3-revival/issues/32), [Application updates](#application-updates)).

Use **Desktop** for a window with setup, a Play button, settings and help. Use [Scripts](scripts.md) if you prefer a
terminal. Both editions include the server, rules engine and required connection components. Game files are separate.

- [Desktop guide](#desktop-guide)
  - [First launch](#first-launch)
  - [Singleplayer](#singleplayer)
  - [Join another server](#join-another-server)
    - [Several games at once](#several-games-at-once)
  - [Host friends](#host-friends)
  - [Settings](#settings)
    - [Card pictures](#card-pictures)
    - [Screen size](#screen-size)
    - [Anime cards](#anime-cards)
    - [Destiny Draw](#destiny-draw)
    - [Banlist](#banlist)
  - [Open the game directly](#open-the-game-directly)
  - [Help and errors](#help-and-errors)
  - [Logs and support](#logs-and-support)
    - [Problem reports](#problem-reports)
  - [Application updates](#application-updates)
  - [Your data and upgrades](#your-data-and-upgrades)
  - [Reset local game data](#reset-local-game-data)
  - [Other programs inside the game](#other-programs-inside-the-game)
  - [Troubleshooting](#troubleshooting)
  - [Windows 7](#windows-7)
  - [Getting the original game](#getting-the-original-game)

## First launch

You need a complete original **Yu-Gi-Oh! ONLINE 3 v1.153** installation and **64-bit Windows 10/11** or **Linux with Wine**.
**Windows 7 is not supported**, but Desktop can run there with a third-party compatibility layer: [Windows 7](#windows-7).
macOS does not have a supported build. [Game installation help](#getting-the-original-game).

1. [Download the latest release](https://github.com/usersina/yo3-revival/releases/latest). Choose the **Desktop**
   archive for your system and extract **all** of it.
2. Open `yo3-revival.exe` on Windows or `yo3-revival` on Linux inside `yo3-revival-desktop/`.
   On Windows, look for the Duel Disk icon with the Monster Reborn badge.
3. The first time, **Setup** opens. It has three steps: **Welcome**, **Game** and **Ready**.
4. On **Game**, YO3 Revival lists the copies of the game it found: Program Files on every drive (Windows), or your
   Wine prefixes, including Lutris and Bottles ones (Linux). Pick one, or use **Browse…** to choose the folder that
   holds `yo3.exe`. On Linux, **More options** sets the Wine prefix.
5. Wait for the checklist. Each line is green, amber (a warning) or red, with one sentence on how to fix it and
   **Details** for the technical text. **Next** opens once nothing is red.
6. On **Ready**, choose whether the game may open without the launcher, whether to put a shortcut on your
   desktop, whether to **download card pictures** (on unless you untick it: [card pictures](#card-pictures)), and whether
   to **send logs when something breaks** (on unless you untick it:
   [problem reports](#problem-reports)), then click **Done**.
7. Click **PLAY**. In the game, choose **ONLINE**, then select your player. [Start your first duel](playing.md#your-first-duel).

![Setup's Game step: the copy found in Program Files and its checklist](../screenshots/setup-game.png)

![Setup's Ready step: direct start, the desktop shortcut, card pictures and problem reports](../screenshots/setup-ready.png)

The checklist:

| Line | Red or amber means |
| --- | --- |
| Yu-Gi-Oh! ONLINE 3, the supported version | No `yo3.exe` in that folder, or another version of the game |
| Game data can be read | `yo3.idx` or the game's data files are missing or damaged: reinstall, or choose a complete copy |
| Wine is installed (Linux) | Wine is missing (the line says how to install it on your distribution), or the chosen prefix doesn't exist |
| DirectX 9 and Visual C++ 2008 runtimes | The line names the runtime to install, then **Retry** |
| Opening the game without the launcher | Amber only: the folder is protected (Windows asks for permission once, the first time it's needed) or read-only (the game then opens only from YO3 Revival) |
| YO3 Revival needs repair | Shown only when YO3 Revival's own files are missing or changed, often after an antivirus scan: **Repair** explains how to restore them |

Later starts are quick: YO3 Revival remembers a fingerprint of `yo3.exe` and runs the full check again only when the
game changed. If the game folder disappears (moved, or a drive unplugged), the main screen says **We can't find your
game** with **Fix**, which reopens Setup at the Game step. Settings → Game → **Change…** reopens it too.

## Singleplayer

![The main screen: Singleplayer, with the Information panel and PLAY](../screenshots/desktop.png)

**PLAY** starts your own server, then the game, and the line above the button names each step; after 30 seconds
without the game, the big button becomes **CANCEL**, which stops what it started: a game that appears while or just after you press it is closed, and PLAY comes back only once the start has settled. A notice about the game ("already running", a failed start) goes as soon as the game's state changes. While you play it is **CLOSE GAME**,
which asks the game to stop and forces it after 10 seconds. Your server stops 30 seconds after the game exits, so a quick relaunch reuses it. If YO3 Revival
crashes, the game keeps playing and your server stops by itself once nobody has played for two minutes. A game opened
without the launcher shows as **PLAYING**, so a second copy can't be started.

Your first legal starting deck is prepared and selected once, without opening Edit Deck. Existing decks and later
edits are kept. See [the playing guide](playing.md) for computer opponents, friends and decks.

The panel shows the [banlist](#banlist) your server uses, with **Change**. Cards you earn in Singleplayer stay on this
PC. If its data can't be opened, the notice offers **Restore a backup** (your server keeps one from each of its last
starts, newest first); Settings → Singleplayer has the backups, a reset and the data folder.

## Join another server

The **Multiplayer** tab signs in to servers with a User ID and password ([accounts](authentication.md#accounts)):

![The Multiplayer tab with the YO3 Revival server listed and online, the sign-in form and the server tools](../screenshots/multiplayer.png)

- **The server list**: the official servers (the project's YO3 Revival server, from a list the project signs) and the ones
  you added, the last one you played on first. Each shows whether it is online (the dot: green, amber or red), with how
  many players and its ping, or Maintenance, Offline, Needs a newer launcher, or Identity changed. While the tab is shown
  the statuses are asked again every 3 seconds (v0.1.17 (2026-11-22)), so a server coming back or players joining show without RETRY. The Information panel
  shows the chosen server's news (`code` in it shows as plain words). Your own hosted server, when it isn't running, shows
  Offline with **START HOSTING** (the same as Host for friends) in place of RETRY.
- **Server tools** (the **⋯** beside the list): **Add a server**, **Host for friends**, and **Remove this server** for one
  you added.
- **Add a server**: paste an invite link a host sent you (`yo3revival://server?…`), or type an address (`host` or
  `host:port`). The line under the box says what it found as you type: the link's server, or the name of the server
  that answers at that address; **Add** works once something is found. An invite link carries the server's identity,
  so it is added at once, and a link made for you carries your invite code too: making your account opens with it
  filled in. An address shows the server's name first; **Details** shows its identity, to compare with one the host
  gave you. YO3 Revival checks the identity on every connection after that. If it ever changes, the server's row says
  so and nothing connects: remove the server and add it again only if the host confirms the change.
- **Create account** (under the sign-in, **New here?**): a User ID (3 to 16 letters or digits), a password of 8 to 16
  letters or digits that isn't the User ID, typed twice, and the server's invite code when it asks for one. A list
  under the form ticks each rule green as you type; **Create account** works once all are met and the rules are
  accepted. You then see a **recovery code** once: **Copy** it or **Save** it as a text file, then **I saved it** signs
  you in.
- **LOG IN**: the User ID field has the typing when the form opens (the password, when a saved User ID is filled in). **Save User ID** keeps only the name. **Stay signed in** keeps a sign-in token in your system's credential
  store (Windows Credential Manager, or the Secret Service on Linux), never the password; without such a store the box is greyed out, off, and a line says why. Wrong User ID or password get the same message, and repeated tries wait longer each time.
- **Forgot password?**: your recovery code (or a reset code from the server's moderators, who check it's you first) and
  a new password. A recovery code is replaced by a new one, shown once.
- **PLAY** starts the game signed in to the server; **CLOSE GAME** and **CANCEL** work as in Singleplayer. Signed in,
  the panel shows your User ID with a red **Sign out**, which forgets this PC's sign-in. Signing in elsewhere ends this game's session: the newest sign-in wins. PLAY checks your sign-in first:
  if it ended (the server restarted, a moderator, or a sign-in on another PC), **Stay signed in** starts a new one, and
  otherwise you sign in again.

Settings → Accounts lists every server you're signed in to on this PC, with Sign out everywhere and Change password.

### Several games at once

**Since v0.1.5 (2026-11-20).** There is still one launcher per user (a second start brings the open window to the front). Signed in on a
server, **Play as another account…** under your User ID starts another game on that server signed in to a *different*
account: type its User ID and password and press **Play**. The password is used once and not kept, and the other account
doesn't "stay signed in".

- **Games running** lists every game with its account and server and a **Close** for each. Closing the launcher asks first,
  then closes every game it started. Singleplayer stays at one game, against your own server.
- **The same account twice is refused**, with a message, before anything is asked of the server (the server ends an
  account's older sign-in when it signs in again, so the first game would be cut off). That includes the account this
  launcher is signed in as: PLAY is for that one.
- **Each game is its own.** It has its own local ports and its own connection to the server, its own game data folder (the
  game's `options.y3o`, `users` and `players` folders: for an extra game they are in
  `Documents\YO3 Revival\servers\<a number made from the server and the account>`, so each account keeps its own saved
  settings, decks and duel logs next time), and on Linux its own Wine prefix (Wine allows one X display per prefix): a copy
  of the main prefix made when the game starts and removed when it closes. On a file system that can share data between
  files (btrfs, XFS) the copy is nearly free; elsewhere the copy shares the main prefix's `C:` drive instead of copying it.
  The main game keeps the data folder it always had. On Linux that folder is in the prefix's Documents: where Wine links
  it to your own Documents (its default) it is kept after the copy is removed, and in a prefix whose user folders are
  real folders (a sandboxed one) it goes with the copy.
- **Updates and Repair.** An update downloaded in the background installs on **Restart to install**, which waits until
  every game is closed; settings that change the game folder wait the same way.
- **A host can cap the games one PC runs** (`games-per-device`, **default 1**, which changes nothing until a host raises it):
  a second game on the same PC is then refused with "This server lets one PC run 1 game at once…". Hosts can also keep
  accounts on one PC or network from earning together: see [linked accounts](hosting.md#linked-accounts-and-fair-play).

## Host friends

**Host for friends**, on the Multiplayer tab, runs a server on this PC that your friends join, like a game's "Open to
LAN". It is the same server as Singleplayer's, with its own data (its own accounts, cards and decks), and it runs while
YO3 Revival is open.

![Host for friends: an invite link for a friend, the address and a friend online](../screenshots/host-friends.png)

1. On the Multiplayer tab, open the server tools (**⋯**) and click **Host for friends**.
2. Give your server a name. Keep port **7443** unless something else uses it. Leave **Address to share** empty for
   this PC's network address (friends on your home network reach it), or type your public address or a name such as
   `play.example.com` for friends elsewhere (see [Friends can't connect](#troubleshooting)).
3. Click **Start hosting**. Then, for each friend, click **Copy an invite link** and send them what it copied. Each
   friend gets their own link: it carries a single-use invite code that works for a week. The dialog lists the links
   made this time, with a copy button each, and the **Address** with its own.
4. Your friend pastes the link under Multiplayer → **⋯** → **Add a server**: making their account opens with the code
   filled in.
5. Your server is in your own Multiplayer list too, as **(yours)**: make your own account there with an invite link of
   your own, and play on it like everyone else.

The footer says **Hosting for friends** with how many players are online; click it to open the dialog again.
**Check reachability** asks your server at the address you share. From this PC that also needs your router to loop the
connection back, so a failure there means "maybe", not "no". **Admin panel** opens your server's
admin panel in your browser (accounts, moderation, news, settings, gifts). It signs in with an admin account: create
your account on your server from Multiplayer, then type its User ID in the box under it and click **Make admin**.
The red **Stop hosting** ends the server; friends' duels end as draws. The dialog's **?** opens this topic in Help, and
**×** closes the dialog while your server keeps running. Closing YO3 Revival stops it too, and asks first while friends are online.

Only the TLS port needs to be reachable. If another program has it, the dialog says so in one line with **Change
port**. Singleplayer and hosting can't share a port at the same time: with the defaults, your Singleplayer server gives
way when you start hosting while nobody plays on it, and PLAY in Singleplayer asks you to stop hosting first.

For a server that runs without the launcher (an always-on PC or a VPS), use the server on its own: the
[command-line reference](guide.md) and the [Scripts edition](scripts.md#host-friends).

## Settings

**Settings** at the bottom right opens **Settings**; **Back**, **Settings** again or Escape returns to the main screen. **Since v0.1.6 (2026-11-20):** it moved there from the title bar, and each section is grouped rows: a setting's name and one line on the left, its control (a switch for on and off) on the right.
Every change is saved at once and applies at the next PLAY (or your server's next start), without restarting YO3 Revival.

**Escape** closes the topmost thing first (**Since v0.1.5 (2026-11-20)**): an open dialog, then the bell's list or the server tools, then a
Multiplayer form (**Add a server**, **Create account**, **Forgot password?**; a recovery code shown once stays until you
press **I saved it**), then Settings. In a text box the first Escape only leaves the box.

![Settings, Game: the game folder, Verify and repair, the screen size, direct start and the shortcut](../screenshots/settings.png)

| Section | What it has |
| --- | --- |
| Game | The game folder (**Change…** opens Setup), **Verify** (checks the game and YO3 Revival's own files in full, and says how to fix what isn't right), the [screen size](#screen-size), [opening the game without the launcher](#open-the-game-directly) (a tick: pressing it sets it up at once, asking Windows for permission when the game folder needs it), **Add** for a desktop shortcut, and [**Card pictures**](#card-pictures) |
| Accounts | Each server you're signed in to on this PC: **Sign out**, **Sign out everywhere** (every PC signed in to the account must sign in again; asks for the password when this PC didn't stay signed in) and **Change password** (signs every PC out, this one too) |
| Singleplayer | **Anime cards** (your own server's `anime-cards` setting, [see below](#anime-cards); while some of their pictures are missing, a link under it leads to Card pictures), **Destiny Draw** (your own server's `destiny-draw` setting, [see below](#destiny-draw)), **Revival** (your own server's `revival-rule` setting: Official rules; Ignoring conditions: all; Anime: any revival, the default; from the next duel, [house rules](rules-privacy.md#house-rules)), the [banlist](#banlist), **Back up now**, **Restore…** (a backup from a recent start), **Unlock all cards** (three of every card, in every artwork, for every player of your own server, once; it asks first. Beside it, your player's cards out of every card, as 1,214 / 5,030; after the unlock it reads **All cards unlocked**, until an update adds cards, which **Unlock N new** gives), **Reset…** (starts your Singleplayer data again; the old data is kept beside it, renamed) and **Open data folder** |
| Advanced | On Linux, the Wine prefix and the Wine program (empty for Wine's defaults; **Save** checks it); **Detailed logs** for your servers; **Send logs when something breaks** ([problem reports](#problem-reports)); **Export logs…** and **Open logs folder**; [Reset local game data](#reset-local-game-data) |
| About | The version, **Check for updates**, release notes, the project, licences and credits |

### Card pictures

Settings → Game → **Card pictures** (v0.1.17 (2026-11-22); before, **Anime pictures** under the Anime cards switch in Settings →
Singleplayer) holds the pictures of the anime cards and of the [cards the game never had](anime-cards.md#artwork) (Shooting
Quasar Dragon, Divine Serpent Geh and the others your host gives): about 10 MB from ProjectIgnis' picture server. The game's own cards keep their artwork.

![Settings, Game, Card pictures: the switch, the bar and its buttons](../screenshots/settings-card-pictures.png)

- **Download automatically**: on, Desktop gets the missing pictures by itself, at each start and as soon as you turn it on.
  Setup's last step has the same tick (**Download card pictures**), ticked: untick it to download nothing. Off (the
  setting of a launcher set up before), nothing downloads unless you press **Get pictures**.
- The bar counts the pictures you have ("N of M pictures ready"). **Get pictures** (**Get the rest** once you have some)
  downloads the missing ones, with **Cancel**; each picture is tried three times before it counts as not fetched. The
  download goes on while you use the rest of the launcher: the footer then shows **Card pictures · N of M**, which opens
  this group, and it ends in a toast.
- **Add from a file…** adds a zip of pictures and **Save to a file…** makes one.
- **Your own pictures, as in EDOPro:** put a card's picture in the `pics` folder beside `anime-art` in the launcher's
  data folder (`%LOCALAPPDATA%\YO3 Revival\pics` on Windows, `~/.local/share/yo3-revival/pics` on Linux), named by
  its passcode (`513000137.png`, `82103466.jpg`), a PNG or JPEG in a card's shape, up to 4096 pixels a side and 24 MB (a
  `pics` folder beside `yo3.exe` works too). It shows from the next PLAY on that card, whether it is an
  anime or manga version or a card the game never had, and wins over every other picture. You can copy EDOPro's
  `pics` folder in as it is. The manga Gods and the anime Divine Serpent Geh come with pictures of their own.
- Pictures show from the next PLAY, and your own pictures are never replaced. Until its picture arrives, a card the game
  never had shows a stand-in picture that ships with YO3 Revival.

**Since v0.1.4 (2026-11-19):** clicking the version at the bottom (now in the middle of the footer, with an "About YO3 Revival" hint) also opens **About**, from Settings too.

### Screen size

Settings → Game → **Screen size** draws the game larger and sharp, in its own 4:3 shape. Menus, text and the duel
field are drawn at the new size; card pictures and backgrounds keep the detail their files have, so they are scaled up.
**Original (1024 × 768)** is the game as it always was. **Fit my screen** picks the largest window your screen holds;
a size such as **1440 × 1080** is the game window's size. If the window doesn't fit, Windows or Wine keeps it smaller
and the game fits inside, with black bars when the shapes differ.

The game's own **Full** screen setting (title screen → **Options** → **Screen**) becomes a borderless window the size
of your screen, the game centred with black bars beside it: on a 1920 × 1080 screen it is drawn at 1440 × 1080.
Changing **Screen** there closes the game, as it always has; the mode applies the next time you play.

The size applies at the next PLAY. If the higher resolution can't be used with your game files, the game doesn't start:
choose **Original** to play.

**Since v0.1.4 (2026-11-19):** opening `yo3.exe` [directly](#open-the-game-directly) draws the game at the size the last PLAY used (the
loader sets it up before the game makes its window). Change the size in Settings, PLAY once, and the next direct start follows.
A game this can't scale just opens at its own size.

### Anime cards

Settings → Singleplayer → **Anime cards** (off by default) plays some cards as they were in the anime: [the list](anime-cards.md). It
is your own server's `anime-cards` setting, so the switch shows what the server has (asked of it while it runs, read from its
saved settings while it is stopped) and writes it the same way. It takes effect from the next game start; with the game open,
**RESTART THE GAME** appears on the main screen. Turning it on while some pictures are missing opens a dialog: what is about to be
downloaded (about 10 MB from ProjectIgnis' picture server, how many of the cards still have no picture, that a card without one keeps
the game's artwork), **Get pictures** (the dialog closes and the bar under the switch shows the download; the result is a toast) or **Not now**. The switch stays on
whatever you choose, and [Card pictures](#card-pictures) downloads later. A server you join never shows this switch: its host
decides.

### Destiny Draw

Settings → Singleplayer → **Destiny Draw** (off by default) is your own server's `destiny-draw` setting, read from and written to it
the same way as Anime cards. It takes effect at your next duel against the computer, with no restart: once per duel, at your Draw
Phase with 1000 LP or less, the game asks whether to pick the card you draw ([playing](playing.md#destiny-draw)). A server you join
never shows this switch: its host decides.

### Banlist

Settings → Singleplayer → **Banlist** sets the Forbidden/Limited list your own server checks decks against. Auto Duel
and duels against the computer use it, and Edit Deck shows it as the **Latest Regulation**. **Unlimited** is the
default (v0.1.17 (2026-11-22); Season 16 before): the game's own list with no banned or limited cards, so any deck works,
the Yugi, Jaden and Yusei character decks included. Choose **Season 16**, the newest list the game has, to play as on the
original server, or any of the game's other lists. **Create Duel** can still pick another list for one duel. It changes while the game is
closed and applies at your server's next start. On a server, each player can choose their own with `banlist` in the
game's chat ([chat commands](playing.md#chat-commands)); the host's list is the one before they choose.

## Open the game directly

With **Open the game without the launcher** on (Setup's Ready step, or Settings → Game), PLAY sets things up so that
`yo3.exe`, or its Start menu or desktop shortcut, plays what you last played here, without this window. On Windows,
the original game asks for administrator rights each time it is opened this way (PLAY here doesn't); answer Yes. Each PLAY,
Singleplayer or Multiplayer, keeps it up to date. Settings → Game says whether it is set up, or that the next PLAY does
it.

- **Singleplayer:** your own server starts with the game if it isn't running, and stops about 30 seconds after the
  game closes. If it can't start, the game says "Failed to connect to server.".
- **Multiplayer:** with **Stay signed in**, the game goes straight to Select Player. Without it, or once that sign-in
  has ended, **ONLINE** shows the game's own login screen: type your User ID and password there. **Save Password**
  keeps this PC signed in (the next start skips the screen); **Save User ID** fills in your User ID next time. The game
  never stores the password itself. A wrong password gets "Incorrect User ID or
  Password", a ban "This User ID is currently restricted.", a server that is down or in maintenance "Failed to connect
  to server."; a server that needs a newer YO3 Revival adds one line saying to open it and update.

The setup adds an owned `winmm.dll` loader and a small receipt beside `yo3.exe`; the game's own files are never
changed. Your sign-in stays in the system's credential store (on Linux, also in the game's Wine prefix), never a
password. The connection helper is copied into the app's own data folder, so moving or updating YO3 Revival leaves
direct start working; if that copy is gone, the game opens offline and one line says to open YO3 Revival once. It never
replaces another mod's file of that name: the setting then says so, and PLAY here still works. It is a tick, in Setup's last step (off for a new player) and in Settings → Game:
pressing it sets it up at once, and in a protected game folder Windows asks for permission then. Say no and the tick
stays off, with one quiet line; nothing asks again at PLAY (**since v0.1.6 (2026-11-20)**: before, each PLAY's notice offered **Allow**). If
it was on but the game folder lost its files, Settings → Game says so beside the tick, with **Ask again**. Turning the
setting off takes the loader out of the game folder again.

When a game opened this way couldn't play (its server didn't start or refused the sign-in), YO3 Revival's main screen
says so once, the next time it is open with the game closed, with the technical reason behind **Details**.

**Delete Login Data** on the game's title screen ends this PC's saved sign-in for the server you last played on: the
server forgets it too, and the game's saved User ID is cleared. Singleplayer has nothing to delete. After an update or
a changed Wine prefix, the next PLAY refreshes the setup the same way. Choosing another game folder takes it out of the
old one first.

## Help and errors

**Help** at the bottom right, beside Settings (**since v0.1.6 (2026-11-20)**), opens **Need a hand?**: **Read the guide**, **Ask the community** (the
project's discussions) and **Report a problem**. The guide is inside the launcher, a topic at a time: getting started, Singleplayer,
Multiplayer, hosting, settings, the common problems and where to talk with other players (the project's
[discussions](https://github.com/usersina/yo3-revival/discussions)). Its **Report a problem** starts a report.

**Support us**, the red heart at the bottom left, says how to help the project: its discussions, and a donation link. YO3 Revival
is free, and giving changes nothing in the game.

Every error is one plain sentence with one action: its own (**Retry**, **Fix**, **Repair**, **Restore a
backup**) or **Report**. It shows as a toast above the footer (bottom left on the main screen, clear of PLAY; bottom right in Settings), so the page never moves
(**since v0.1.6 (2026-11-20)**): good news goes by itself after a few seconds, a problem stays until **Dismiss**. Settings' jobs show
there too, with a waiting bar. Those about both tabs (a game opened without the
launcher that couldn't play, an update that didn't install) go to the **bell** in the title
bar, which counts the new ones; click it to see them. **Help** opens the topic for that error. **Details** shows the technical text, with your
folders written as names such as `<game>` or `<app data>`, and **Copy** puts it on the clipboard.

![Help: the guide inside the launcher, at Host for friends](../screenshots/help.png)

## Logs and support

**Report a problem** (in **Help** at the bottom right, in the guide, or on a problem's toast):

1. Say what you were doing and what went wrong, and, if you'd like an answer, how to reach you (optional).
2. Click **Save report**. **Since v0.1.22 (2026-11-24):** it is saved as one file on your PC, in your **Downloads** folder: what
   you wrote, your recent logs (this session's and the latest game's) with personal details removed, this PC's details
   (system, processor, memory, graphics card) and, in Singleplayer, the duels you marked with `bug`
   ([what's inside](report-format.md)). Nothing is sent anywhere by itself.
3. The window shows the file with **Open folder** and **Copy path**. Send it to the developer: on Discord, privately, or
   attached to a GitHub issue. **Also talk about it on GitHub** opens the public report form, filled in, if you want to.

A report never depends on a server: it is a file on your PC. Before, Report a problem opened a GitHub form and saved a
file only in Singleplayer, which most reports lacked. Reports on GitHub are public, and the file can still hold player
names, chat and decks, so look through it before posting it there.

### Problem reports

When a game's connection breaks or the game closes with an error, YO3 Revival's helper sends that game's log to the
server you played on, before signing out. The host reads it by its report ID, beside the server's own log of the same
minutes, without asking you for anything. The game's log says the ID (`Problem report R-… sent`), and Report a problem's
file lists the recent ones. A game that ended well sends nothing.

The logs never hold your password, keys or tokens: they are removed when written, and again by the server. They do
hold your player names and what the game did. Untick it in Setup, or turn off Settings → Advanced → **Send logs when
something breaks**: it applies to the next game, and to a game opened [without the launcher](#open-the-game-directly)
too. A host can refuse them all (the server's `problem-reports` setting).

Settings → Advanced → **Export logs…** saves the logs in a ZIP of your choosing, also with personal details removed;
**Open logs folder** shows them. **Detailed logs** makes your servers write more (their debug level) from their next
start; turn it on to reproduce a problem for a report, then off again.

Logs rotate at 2 MiB and are kept for up to 14 days or 100 MiB. A remote player's export can't include the host's
server logs: the host exports those separately. For a duel bug that needs state checks and exact replay, the host runs
the server with `--duel-check` ([developer replay and validation](development.md#duel-diagnostics)).

## Application updates

YO3 Revival checks for a new release when it starts and every 6 hours while it is open (Settings → About → **Check for
updates** asks at once). When one is out:

- **"Update X is available"** beside the version in the footer (with a dot). Clicking it opens the **Update to X**
  window: that version's section of the [changelog](https://github.com/usersina/yo3-revival/blob/main/CHANGELOG.md),
  where it is, a **Download** button and a link to the release page. Settings → About's **Check for updates** opens the
  same window (**See the update**). **Since v0.1.22 (2026-11-24):** nothing downloads until you press **Download**, unless you turn
  on **Download updates automatically** in Settings → About (off by default; before, every new version downloaded by
  itself). While it downloads the footer says "Downloading update X…", then "Update X is ready: restart to install".
- **It downloads in the background, only if the project signed it.** Each release carries `SHA256SUMS` and the
  project's signature over it, `SHA256SUMS.sig`. Before downloading, YO3 Revival checks that signature with the
  project's key, which is built into the launcher, and checks that the signed list has the same SHA-256 that GitHub reports
  for the archive. The download is then checked against that SHA-256. A release without the signature, or with a
  signature or checksum that doesn't match, is refused with the reason, written to the logs and never installed; the
  window's **Download** tries again. The new
  version is unpacked beside this one: your settings, accounts, decks and server data stay where they are, and game
  files are never part of an update.
- **A ribbon above PLAY: "Update X is ready · Restart to install".** **Restart to install** (or the same button in the
  Update window) hides this window at once, opens the new version and closes this one once the new window is up (if the new version doesn't start, this window comes back). If you are hosting for friends it asks
  first, because restarting stops your server.
- **Never while the game runs.** While you play, the ribbon says the update installs after you close the game. Closing
  YO3 Revival is enough too: the next time you open it, it installs the update before its window opens (and waits again
  if a game is still open).
- **If the new version doesn't start,** this version stays, says "Update X didn't start, so YO3 Revival kept
  version Y" with the reason behind **Details**, and doesn't download that version again by itself (the release page
  still has it).
- **A server that needs a newer YO3 Revival** shows **UPDATE** on its Play button in Multiplayer, which opens the
  Update window (or the release page before the update is known). Each server names the oldest launcher it accepts (`min-launcher`). Let the
  signed update finish, then restart to install it. Your original game files do not need replacing. Singleplayer
  runs your own bundled server; other servers have their own version requirement.

You can still update by hand: download the new Desktop archive and extract it in place of the old folder, with
YO3 Revival closed. Your settings and data are kept in the app's storage.

**From v0.0.21 or older:** those versions update to the beta as before: version → **Review & install…** →
**Download & restart**. They
don't check signatures, only GitHub's checksum. After the update, the shortcut or folder you opened the old version
from still opens the new one, because the old version hands over to the installed update. Your settings stay in the
same `launcher.db`, and the old version can still read it. Keep that original folder: it is what the shortcut opens.

Archives have versioned filenames, but stable inner folders: `yo3-revival-desktop/` and `yo3-revival-scripts/`.
Do not overwrite a running bundle. [Scripts updates](scripts.md#updates) are manual.

## Your data and upgrades

App storage is `%LOCALAPPDATA%\YO3 Revival` on Windows and `${XDG_DATA_HOME:-~/.local/share}/yo3-revival` on Linux.
Settings → Singleplayer → **Open data folder** and Settings → Advanced → **Open logs folder** open parts of it.

| Location inside app storage | Contents |
| --- | --- |
| `launcher.db` | Settings and your Multiplayer servers (never a password) |
| `singleplayer/` | Your own server: `server.db` (accounts, cards, decks, BP, mail, duel logs), `server.crt` and `server.key` (its identity), `backups/` |
| `hosting/` | Host for friends' server, the same files as `singleplayer/` |
| `logs/`, `profiles/` | Diagnostics, and the private files the game's helper reads (no keys or passwords: those are in the system's credential store) |
| `reports/` | Report a problem's file when this PC has no Downloads folder |

Each server start makes a consistent database backup under its folder's `backups/`. These backups exclude the identity
key: to keep everything, back up the **whole folder** with the server stopped. Players should also back up their local
`Documents/Yu-Gi-Oh! ONLINE 3` folder, which holds editor-only deck saves, and `Documents/YO3 Revival/servers/`,
which holds the same for every server after the first ([details](playing.md)).

The beta starts with new data: it doesn't read a v0.0.x Desktop's hosting data or invitations. Restore only with the
server stopped. Older releases can't read newer data; downgrading needs the backup taken before the upgrade.

## Reset local game data

For broken game preferences or saved stock login details, close every game window and choose Settings → Advanced →
**Reset local game data** → **Reset…**, then:

- **Display and login settings** resets the game's options and cached stock logins, keeping player files, saved decks,
  deck selection and starter setup markers.
- **All local game data** moves the whole local folder into the backup, and the game makes fresh files next launch.

Server accounts, cards and BP stay on the server either way. Dated backups live in `.yo3-revival-backups/` beside the
game's local data folder in Documents; a full backup contains the original folder as `data/`. Windows Documents
redirection and the chosen Wine prefix are respected. Wine prefixes can share this folder, so close games in other
prefixes too. Reset works on `Documents/Yu-Gi-Oh! ONLINE 3`, the folder of the first server you played on; other
servers' folders under `Documents/YO3 Revival/servers/` are left alone.

To restore, close the game and move newly created files aside first. For a full reset, move the backup's `data/`
folder back to the original location; for the settings reset, move `options.y3o` and `users/` back from the backup.
Keep backups private: they can contain personal files and cached stock login details.

## Other programs inside the game

**Since v0.1.4 (2026-11-19).** Each PLAY, before the game is released, the connection DLL looks at what else runs inside it: other programs'
`.dll` and `.asi` files in the game folder, the programs Windows loaded into the game from outside its own folders, whether
anything has redirected the game's network functions, and the 32-bit Winsock providers the game's connection goes through
(the game is a 32-bit program; YO3 Revival's own helper never uses that list). Nothing on your PC is changed by looking, and
overlays and drivers that only sit in the game are not reported to you (they are in the logs for a problem report).

If something is found, a dialog says so in plain words, with the technical text behind **Details**:

- **A file in the game folder** (a mod loader, a plugin, a `winmm.dll` that isn't YO3 Revival's; YO3 Revival's own loader is recognised by its mark, with or without its receipt, and is never offered here): **Move it aside** renames it
  to end in `.yo3-disabled`, after you confirm, so Windows no longer loads it. Nothing is deleted; Settings → Advanced →
  **Files moved aside** lists each one with **Put back**. In a protected folder Windows may refuse the rename; move the
  file out yourself then.
- **A Winsock provider that isn't Microsoft's** (often from an old VPN, an ad blocker or security software): the dialog
  gives the command `netsh winsock reset`, which you run in a Command Prompt opened as administrator, then restart the PC.
  YO3 Revival never runs it or changes Windows' settings.
- **Security software** attached to the game's network functions: add an exception for the game folder in its settings, or
  switch off its web protection while you play. The product is named when YO3 Revival knows its files.
- **Continue anyway** starts the game as it is.

PLAY refuses to start the game in two cases only, with no Continue anyway: another program has *redirected* one of the game's
network functions (it sits between the game and the connection YO3 Revival guards), or a file is known to break the game's
connection (a list that ships empty). **Move it aside** is offered when the culprit is a file in the game folder.
Everything else stays a warning, unless the host asks for a clean game (next paragraph). The same check runs for Scripts' `play` (it prints what it found and stops; `--accept-foreign`
goes on past a warning). A game opened [directly](#open-the-game-directly) is not held: what was found goes in the logs.

**A server that asks for a clean game** (**Since v0.1.5 (2026-11-20)**, the host's `clean-game` setting, off by default): on that server PLAY
stops for anything the check lists, with no **Continue anyway**: another program's `.dll` or `.asi` in the game folder, a
Winsock provider that isn't Microsoft's, or a detour on the game's network functions. The dialog is titled "This server asks
for a clean game" and says so; **Move it aside** is still offered for files, and the other advice (the Winsock command,
security software) applies as above. It asks only on that server: other servers and your own Singleplayer are unchanged.
It is the host's tidiness rule for friends' games, not protection against cheating: anything on your PC can be hidden from
a check.

## Troubleshooting

Help in the footer has these topics too; an error's **Help** opens its own.

| Problem | What to do |
| --- | --- |
| Downloaded Revival but have no game | [Install the original game separately](#getting-the-original-game), then choose its installed folder in Setup. |
| PLAY says SET UP, REPAIR or RETRY | Click it: Setup opens at the game folder, Repair explains how to restore YO3 Revival's files, Retry checks again. Settings → Game → **Verify** checks everything in full. |
| Unsupported game executable | Use the original **v1.153** installation. Replacing Revival's helpers can't fix a different game build. |
| Missing DirectX or VC90 runtime | Setup's checklist names it. Follow [original game runtime repair](guide.md#original-game-runtimes), then **Retry**. |
| YO3 Revival needs repair / antivirus removed a file | Extract the complete matching Desktop archive again. For a verified false positive, allow the affected file or exclude only the extracted Desktop folder. [Exact files and recovery steps](antivirus.md). |
| The game doesn't start, or stops | Read the notice; **Details** has the technical text. Close any other copy of the game, then PLAY again. If it repeats, **Report** beside the error. |
| The game disconnects you in the shop or another screen | Another program is running inside the game: a mod loader or plugin file in the game folder (a `winmm.dll` that isn't YO3 Revival's, a `dinput8.dll`, an `.asi` file), software in Windows' network path, or security software. **Since v0.1.4 (2026-11-19):** PLAY now checks this each time and stops, with a dialog saying what it found ([below](#other-programs-inside-the-game)). Before that version the bell showed a notice for a foreign `winmm.dll`: close the game, move that file out of the game folder, then **PLAY** again. |
| The shop (or another screen) disconnects you with "An established connection was aborted by the software in your host machine" (error 10053) in the logs | **YO3 Revival's own `winmm.dll` is never the cause**: a `winmm.dll` in the game folder that carries YO3 Revival's mark (any release, even without its receipt file) is our loader, which does nothing while you press PLAY and only passes Windows' sound functions on. Desktop now says so and PLAY repairs its receipt. The 10053 is raised on your PC, not by the server: the log line says whether the game's side or the server's failed and, since the next release, whether it was reading or writing and how many bytes were in flight. Try, in this order: close **security software, VPN, "network protection" and firewall add-ons** (or add the game folder and `YO3 Revival` to their exclusions); run `netsh winsock reset` as administrator and restart the PC; update Windows (Insider builds can differ); and close **Cheat Engine, debuggers and process tools**, which can pause the game or change its time functions. The helper's log names a memory tool (Cheat Engine, x64dbg, Process Hacker and others) when one is running at PLAY and again when the game's connection fails. Nothing is blocked for it. |
| PLAY says "Another program is running in your game" | See [Other programs inside the game](#other-programs-inside-the-game): **Move it aside** for a file, `netsh winsock reset` for a network provider, an exception for security software, or **Continue anyway**. |
| PLAY says "YO3 Revival can't start the game safely" | Another program has taken over the game's network connection (or a file known to break it is present). Close that program or move its file out of the game folder. There is no Continue anyway for this one. |
| The game closes at once with `0xc0000139` or "waveInGetDevCapsW could not be located" | Update YO3 Revival, then PLAY. Versions up to v0.0.24 put a `winmm.dll` beside `yo3.exe` that Windows 10 version 1809 (LTSC 2019) couldn't use; PLAY replaces it with the current one. |
| Another program is using the port | Close the other server (a Scripts server, another YO3 Revival), or for hosting use **Change port**. |
| Friends can't connect to your hosted server | Friends elsewhere need your public address as the address to share and your router forwarding the port to this PC (look for Port forwarding in its settings). When Windows asks whether `yo3-server.exe` may use the network, allow it; if you clicked Cancel once, allow it in Windows Defender Firewall (**Allow an app through firewall**), again after each update. Some providers share one public address between homes (CGNAT): forwarding can't work then; ask for a public address, or use a VPN you both join. |
| Can't connect to a server | Check its row: Offline means it didn't answer. **Retry**, and ask the host whether it runs. Identity changed: don't sign in; ask the host. |
| "You have been disconnected from the Duel Server." in a duel, then the title | The server stopped answering for about a minute (it hung, or the host's network went away), so the launcher closed the game's connections instead of leaving the duel waiting. Ask the host to check the server, then sign in again. |
| Old game preferences or saved login details | [Reset local game data](#reset-local-game-data), display and login settings first. |
| A long session disconnects | Sessions last 12 hours. Close the game and start it again. |
| A duel ends immediately as a draw | The server's logs say **DUELS WON'T WORK**: restore the complete bundled engine. Source builds need the [engine dependencies](development.md#build-from-source). |
| A card seems to behave incorrectly | Check [card rules](playing.md#card-effects-and-summon-rules), then [report it](#logs-and-support). |
| "The program can't start because api-ms-win-core-synch-l1-2-0.dll is missing" (or `bcryptprimitives.dll`) | You're on Windows 7 or 8. Don't download that DLL: see [Windows 7](#windows-7). |

The Linux Desktop release was built against **glibc 2.43** in v0.1.0 (for example, Ubuntu 26.04); check the newest release's notes for what it needs.
On Ubuntu 24.04 it exits before opening a window with `GLIBC_2.43 not found`. Use a newer distribution or
[build from source](development.md#build-from-source) on your own system.

On minimal Debian/Ubuntu X11 systems, install Desktop's runtime libraries with
`sudo apt install libxcursor1 libx11-xcb1 libxi6 libxkbcommon-x11-0`. Missing-library errors prevent the app from
opening its window.

These executables and the injector/DLL are unsigned for now ([#38](https://github.com/usersina/yo3-revival/issues/38)).
[Antivirus warnings](antivirus.md) explains how to check a download against its published checksum. For terminal errors,
see [Scripts troubleshooting](scripts.md#when-something-fails).

## Windows 7

Unsupported and untested in releases: use it at your own risk. Checked once on 64-bit Windows 7 SP1 (2026-11-18):
Setup, a Singleplayer duel against Kaiba, and joining a server by address.

1. Install [VxKex](https://github.com/i486/VxKex/releases) (1.2.2.2316 checked). It warns about missing updates; it
   worked without them.
2. Right-click `yo3-revival.exe` → **Properties** → **VxKex** → tick **Enable VxKex for this program** → **OK**. The
   programs it starts (server, engine, game connection) inherit it.
3. Install the game's runtimes as usual: DirectX End-User Runtimes (June 2010) and Visual C++ 2008 (x86).
4. If the game says "Initialization of direct3d device failed" (no Direct3D acceleration, as in most virtual machines),
   put SwiftShader's software `d3d9.dll` in the game folder. Rendering is slow.
5. Releases through v0.1.2 can fail GitHub update checks with `UnknownIssuer`, and leave the server list empty when
   Windows lacks current root certificates. v0.1.3 bundles its own Mozilla certificates for GitHub;
   download it manually once to receive this fix. Until then, **Add a server** by its address
   (ours: `ygo-3-revival.work.gd:7443`).

Desktop v0.1.0's Setup wrongly reports `d3dx9_40.dll` missing (Windows error 87) on Windows 7; later releases fix it.
Scripts was not tried.

## Getting the original game

Revival does not download or distribute the original game. Use a complete **v1.153** installation you are entitled
to use. A third-party archive reference is the
[MyAbandonware game listing](https://www.myabandonware.com/game/yu-gi-oh-online-3-duel-accelerator-qy4).
This link is a reference, not a Revival download or a guarantee about that archive.

A typical installed folder contains `yo3.exe`, `yo3.idx` and `package/`. On Windows it is often under
`C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3`; on Linux it is often inside your Wine prefix's `drive_c`.
Keep the original game's folder separate from the extracted `yo3-revival-desktop/` folder.
On Windows, native checks used a writable copy of the game folder (for example under `C:\Games\`); an install left in
protected `Program Files` is not yet tested natively, so copy the whole folder somewhere your account can write if Play
or the game misbehaves there.
