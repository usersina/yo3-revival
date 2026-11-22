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
  - [Other programs inside the game](#other-programs-inside-the-game)
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
| `--dashboard-port`          | Opt-in staff API on loopback TCP; disabled when omitted, `0` assigns a free port. Separate from host commands |
| `--dashboard-socket`        | Unix staff API socket mounted by nginx; mutually exclusive with `--dashboard-port`. TOML: `dashboard_socket` |
| `--dashboard-origin`        | Exact web origin for the Unix socket, e.g. `https://play.example.com`; required with the socket. No paths or public plain HTTP. [Proxy setup](docker.md) |
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
| `min-launcher` | Server release | At once | Raise the minimum launcher version; it cannot be lower than the server release. Older launchers are told to update |
| `login-regulation` | Season 16 | At each player's next login | The banlist for Auto Duel and NPC duels, by id or name, for players who haven't chosen their own with `/banlist` in the chat ([playing](playing.md#chat-commands); `set-banlist` overrides one account). Players already online keep theirs; Auto Duel pairs only the same banlist |
| `start-lp` | 8000 | At the next duel | LP at the start of an Auto Duel or NPC duel; a hosted duel has its own |
| `anime-cards` | `off` | At the next duel | `on`: cards with a reviewed anime version play by their anime effects in NPC and player duels and show as the anime cards (the card profile, fetched when a game starts: after a change an open game is refused every duel until it is restarted); the others, and every card the profile does not cover yet, keep the game's ([playing](playing.md#anime-card-versions)) |
| `destiny-draw` | `off` | At the next duel | `on`: in NPC duels (casual duels against the computer; not duels between players, events or the Ultimate Match) the player may, once per duel at the start of their Draw Phase with 1000 LP or less, pick any card left in their Deck as the card they draw ([playing](playing.md#destiny-draw)) |
| `turn-seconds`, `total-seconds` | 180, 600 | At the next duel | The clocks of duels between players that set none (0: off) |
| `pack-cost`, `deck-cost` | 100, 500 | At the game's next request | BP a booster pack and a structure deck cost in Blister's shop |
| `trade-cards` | 10 | At the game's next request | Cards of their own a player gives for one card in Blister's Trade Card |
| `rental-cards`, `rental-bp`, `rental-days` | 5, 100, 7 | At the game's next request | The shop Rental's offer: how many cards, for how many BP, for how many days |
| `report-limit` | 5 | At the game's next request | Reports (`/report` in the chat) one player may send in an hour; 0 turns them off |
| `games-per-device` | 1 | At once | Games one PC may run at once (the launcher's Play as another account): a launcher sends a random install id with its sign-ins, and a second game from the same one is refused past this number (admins and moderators are exempt). The default changes nothing until a host raises it ([linked accounts](hosting.md#linked-accounts-and-fair-play)) |
| `linked-accounts` | `fair` | At once | `fair`: accounts seen on one PC or address are linked, and a duel between them pays no BP and counts no record, rating or title, and they can't trade or rent to each other. `off`: they are strangers |
| `problem-reports` | `on` | At once | `on`: players' launchers send a game's log here when it breaks, and with Report a problem; `problem-reports` and `problem-report ID` read them ([hosting](hosting.md#logs-and-privacy)). `off`: none are taken or kept |
| `clean-game` | `off` | At once | `on`: the launcher refuses to start the game here while another program's file or network software is inside it ([hosting](hosting.md#clean-game)) |
| `news` | A welcome | At the game's next request | The Information list: `TITLE \| BODY` for one entry, or a JSON list of `{title, body}` (1 to 10) |
| `address-burst`, `address-refill-seconds`, `backoff-max-seconds` | 10, 3, 900 | At once | The [throttle](authentication.md#passwords-and-throttling) on account requests |
| `max-players` | 500 | At once | Players signed in; one more is told "The server is full" (admins and moderators still come in) |
| `ultimate-cpu-wait` | 30 | At once | Seconds an Ultimate Match entrant waits for another player before the computer plays them (0: only players) |
| `max-duels` | 30 | At once | Duels running (an engine each, up to about 100 MB): beyond it an NPC duel or Create Duel is refused with "Duels are busy" in the player's chat, and Auto Duel waits until one ends |
| `max-connections` | 2048 | At once | TLS connections at once; a player's game holds several |
| `handshake-burst`, `handshakes-per-second` | 120, 10 | At once | New TLS connections one address may open; this PC's are not counted |
| `request-burst`, `requests-per-second` | 200, 20 | At each new connection | Requests one game connection may make at once, then how fast more are answered. Past the burst requests wait; none is dropped. Real play peaks near 32 a second, briefly |
| `outbox-max` | 4096 | At each new connection | Messages that may wait for a game that reads nothing; past it, or after a minute with no read, its connection ends |
| `recording-days` | 14 | At once | Days every duel's rules-engine recording is kept (`recordings/` in the data folder, about 10 KB a duel); older days are removed at start and daily, pinned ones (a player's `bug`) stay; 0: duels aren't recorded |
| `audit-keep-days` | 365 | At once | Days the audit log keeps a row; older rows are removed at start, daily and when this changes (0: for ever) |
| `maintenance-at`, `maintenance-message` | (none) | At once | Set by `maintenance` ([moderation](#moderation-and-maintenance)) |

A value that doesn't pass its check is refused and changes nothing. Desktop writes its Banlist choice as
`login-regulation` before it starts its server. The testing aids (`--duel-lp`, `--duel-time`, `--duel-deck` and the
like) stay flags and win over the settings.

`yo3-server card-profile FILE [--profile anime]` writes the card profile's client file (what `anime-cards on` makes each game
fetch at start, [how it works](anime-cards.md#the-card-profile)); it needs the game's files and the engine folder, and works on
the files, so a running server is not touched.

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
./yo3-server delete-account Yugi --yes --request-id delete-yugi-0001                           # an admin's: the account and its players, for good
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
- **Deleting an account** (`delete-account`, an admin's; the panel's **Delete account** asks for the User ID typed
  out): the account, its User ID, password, recovery code and devices, and its players with their cards, decks, BP,
  items and titles go. So do their friends and ignore entries, friend, trade, rental and team invitations, rentals,
  the D-Mail they sent and that waits for them, a team they lead (disbanded, its members told) or their place in one,
  their duel records, user events they host, and their own duel logs. Their game is disconnected. The audit log
  keeps one row naming the User ID, actor and time, with non-secret request/disconnect metadata; earlier rows about
  the account go. What stays: other players' records against them, the other player's copy of a duel log, and the
  backups until they are replaced (the newest 24, hourly). Without `--yes` the command refuses.

  Cleanup of every saved domain, duel logs and old audit rows commits with the deletion audit and retry receipt
  before live changes or disconnects. Failures leave the account intact. Retain the printed request ID and retry with
  `--request-id` after a lost response; this never deletes a replacement account with the same name. The last admin
  cannot be deleted until another admin exists. In the React preview, type the numeric account number instead of
  the User ID. An unknown disconnect outcome is reported separately from the committed deletion.

## Official events (the Stadium)

The map's Stadium lists the events you schedule. Five kinds, each with its own score: `gunslinger` (win streak),
`duelist-trial` and `anniversary` (wins), `evolution` and `ultimate` (stars, played as matches). Players enter an event's
lobby from the Stadium and press Ready, which pairs them with another entrant of the same event (an Ultimate Match entrant
nobody joins within `ultimate-cpu-wait` seconds, 30 by default, plays the computer instead; set it to 0 to keep that event
players-only); after each duel they are matched again until they press Not Ready. Everyone counts as premium. The commands work while the server runs, and the
ones that change something also with it stopped; they are admin API operations like the rest (an admin's, audited), and
the [dashboard](dashboard.md)'s Events page does the same.

```bash
./yo3-server create-event duelist-trial "Winter Trial" --start "2026-12-01 18:00" --end "2026-12-08 18:00" \
    --participation-bp 100 --threshold 5 --threshold-bp 500    # UTC; or +2h, now, an RFC 3339 time
./yo3-server events                                           # each event, its kind, time, status and entrants
./yo3-server edit-event 10000 --end +3d --regulation unlimited # what is left out stays; `--regulation any` clears the banlist
./yo3-server cancel-event 10000                               # it leaves the list; nothing is paid
```

`--regulation` makes entrants log in with that banlist (`banlists` lists them; players choose theirs with `/banlist`):
Ready tells them so in the chat instead of matching. Prizes are BP, paid by D-Mail from "Host" when the event ends:
`--participation-bp` to everyone who played a duel of it, and `--threshold-bp` more to everyone whose best score reached
`--threshold`. Every win (a duel, or a match) also earns the player one **Special Pack** from the server itself (up to 10 in
one event; five of the 96 promo-style cards no pack, deck or rental gives), a present with a D-Mail from Host: nothing to
set. Wins, streaks and stars count towards the event titles (TRIALIST, GUNSLINGER, EVOLUTION and their upgrades) and first
place of an ended event with two players or more earns the kind's CHAMPION title, as the [playing guide](playing.md) lists.
A win adds one to the score; a Gunslinger's streak ends with a loss or a draw, but the best streak is what
counts. Scores are kept in `server.db` and go with a deleted account. Tournament Points on My Profile are 5 for each
event a player played plus their best score in it. The Stadium shows times in each player's time zone.

## The Championship (the map's Tournament tile)

The Championship is one single-day knockout you schedule: an entry window, then a start time on the same day. Players enter by
opening the Tournament tile while entries are open (the client has no other way in: nothing to press, the lobby is the entry).
At the start the server draws the entrants (more than 16: 16 of them at random, the others get a D-Mail saying the draw
did not pick them; fewer than two: it is cancelled, with a D-Mail to the entrant) and plays the client's Final: 16 seats,
four rounds (1st Round, Quarter-Final, Semi-Finals, Final). Empty seats are byes: with two entrants they meet in the Final, with
more the byes are spread so nobody gets a second one where the four fixed rounds allow it. The commands work while the
server runs, and the ones that change something also with it stopped; they are admin API operations (an admin's, audited),
and the [dashboard](dashboard.md)'s Championship page does the same.

```bash
./yo3-server create-championship "Winter Cup" --entry-close "2026-12-05 17:30" --start "2026-12-05 18:00" \
    --min-duels 10 --participation-bp 50 --finalist-bp 200 --winner-bp 1000   # UTC; or +2h, now; --entry-open defaults to now
./yo3-server championships                                    # each Championship, its state and how many entered
./yo3-server edit-championship 1 --start "2026-12-05 18:30"     # what is left out stays; only before it starts
./yo3-server cancel-championship 1                             # it leaves the map; nothing is drawn
```

Entry opening, closing and the start fall within one day, and no two Championships may be within an hour of each other.
`--min-duels` is the number of duels (against players or NPCs) a player must have played to enter; it defaults to 0, and
premium, which the client's text asks for too, is not checked (everyone counts as premium). A drawn player who does not
show up loses the round: a match nobody sits down at ends after three minutes with both out, and one whose opponent never
comes ends after 90 seconds in favour of the player who is there; a player who leaves the janken, the turn order choice or the duel loses it. Duels use the server's `start-lp`, `turn-seconds`,
`total-seconds` and `login-regulation`. Titles follow the rounds: Y.O.C. QUALIFIER when drawn, Y.O.C. Q-FINALIST through the
first round (a bye counts), Y.O.C. S-FINALIST through the second, Y.O.C. FINALIST through the third, Y.O.C WINNER (the client's
own spelling) for winning. Prizes are BP, paid by D-Mail when it ends: `--participation-bp` to everyone drawn,
`--finalist-bp` more to both finalists, `--winner-bp` more to the winner. Each Championship is kept in `server.db` for Duel
Log's list of past Championships (`year` is the Japan-time year, `season` its number that year); a server restart during the
bracket ends it with no winner.

## Player reports and banlists

Players send reports from the game's chat (`/p Server report PLAYER reason`, [playing](playing.md#chat-commands)), and
choose their own banlist with `/p Server banlist NAME`. The commands below work while the server runs, and the ones
that change something also with it stopped. They are admin API operations like the rest: audited, and open to a moderator
(`--by NAME`). The [dashboard](dashboard.md) has a Reports page and shows each account's banlist.

```bash
./yo3-server reports                                   # every report, newest first (--status open|handled|dismissed, --limit N)
./yo3-server --by Anna handle-report 3 --note "warned"  # marks report 3 handled
./yo3-server --by Anna handle-report 4 --dismiss         # or dismissed
./yo3-server banlists                                  # the game's banlists and the server's default
./yo3-server --by Anna set-banlist Yugi unlimited       # WHO's next login uses it (an id or a name); `default` is the server's
```

A report keeps who sent it, who it is about, the reason (300 characters), the time and, once closed, who handled it and
the note. It stays in `server.db` (so in backups) until its account is deleted. A player can send `report-limit` an hour.
The reported player is never told.

**Duel problems.** `bug what happened` in the chat (playing: [chat commands](playing.md#chat-commands)) is a report too,
listed with the others: its reason starts with "Duel problem:", it is about the duel's opponent (an NPC by name) and it
names the duel's recording. Every duel writes one, `recordings/<date>/<id>.jsonl.gz` in the data folder, kept for
`recording-days`; `bug` pins the player's last duel as `recordings/pinned/<id>.jsonl.gz`, which the pruning leaves.
A recording holds the decks, the moves and the rules engine's answers, no account or address. Replay it with the same
release's engine (the header names the server version):

```bash
python3 tools/monkey/duel-replay.py --recording recordings/pinned/20261122-075334-001.jsonl.gz --engine engine
```

It replays the duel call by call and says PASS when every answer matches; from there the problem gets a scenario test
([development](development.md#which-tool-sees-what)). Delete pinned recordings by hand once their reports are closed.

## The admin panel

On the `beta` branch (#36). The server serves a web panel on its loopback HTTP port, at
`http://localhost:8080/panel/` (`--http-port`). It answers this PC only: on a server you reach it through an SSH
tunnel ([hosting guide](hosting.md#the-admin-panel-through-an-ssh-tunnel)). Sign in with a password account whose
role is admin or moderator:

```bash
./yo3-server add-user Anna          # prints a setup code; finish Forgot password/--recover before set-role
./yo3-server set-role Anna admin    # or moderator
```

| Page | What it shows and does | Moderator |
| --- | --- | --- |
| Dashboard | Players online, duels, rules engines, memory, uptime, version, last backup's age, whether saving works, recent warnings and errors | Sees it |
| Accounts | Search by User ID or player name; an account's role, standing, mute, devices, BP, banlist (and set it), and each player's cards, decks, items and record; disconnect, mute, restrict, ban, lift, sign out everywhere, reset code, rename; give BP, set country, set role; give a player cards, another artwork or a deck recipe (as `give-cards` and `give-recipe` below); delete the account (type its User ID) | Moderation and reset codes, on players only; not BP, country, roles, gifts or deleting |
| Invites and codes | Invite codes (make, list) and BOOSTERPASS codes (add, remove, who claimed them) | Makes invites; doesn't add or remove codes |
| News and maintenance | The game's Information list, planning or calling off maintenance | Sees it |
| Settings | Every live setting with its value, when it applies and what it does | Sees it |
| Moderation | Who is banned, restricted or muted now, and the latest moderation | Yes |
| Reports | The players' reports from the game's `/report`, newest first, filtered by Open, Handled or Dismissed; links to both players' accounts; Handle or Dismiss with a note | Yes |
| Audit log | The newest 200 changes, from the panel, the commands and the API | Yes |
| Backups | The database backups; back up now; download one (the download is audited) | Sees the list |
| Logs | The server's newest lines since it started (warnings and errors only, or all) | Yes |

- Every change runs one admin API operation as the signed-in account: the audit log names it, from `panel`. A
  moderator's refused change says why and changes nothing; a moderator can't act on an admin's or another moderator's
  account.
- Sign-in has the launcher's throttle: a wrong password makes the next try for that User ID wait 1 s, then 2, 4, …
  up to `backoff-max-seconds`. A session ends after 30 idle minutes, 12 hours, **Sign out**, or at once when the
  account changes role or is banned.
- The panel keeps no data of its own and needs no restart: a changed role ends existing staff sessions at their next
  request, even if the account is promoted back. Setting the same role again leaves current sessions valid.

## Account commands

For unattended first deployment only, `bootstrap-admin USER` reads a password from stdin and atomically creates the
first admin with an audit row and permanent bootstrap receipt. It requires the server stopped and an empty account
directory. Matching retries keep the current password; deletion/demotion or a different target closes bootstrap
with an error. It is offline-only and never an HTTP operation. [One-time setup](deployment.md).

On the `beta` branch every command works while the server runs (#25): it goes to the running server's admin API
(loopback, with the private token in `data/admin.json`) and changes it at once. With no server running, the command
works on the data folder itself, holding its lock (`server.lock`) meanwhile. `init-auth`, `bootstrap-admin`, `restore` and
`backups` always work on the files. Every change is recorded in the audit log: `audit` lists the newest, with who,
from where, what, on what, and the values before and after. `problem-reports [--who WHO] [--limit N]` lists players'
problem reports and `problem-report ID [--out FILE]` shows one ([hosting](hosting.md#logs-and-privacy)). Released versions still need the server stopped for all
but the BP and code commands. Add `--data-dir PATH` to target a non-default setup. Players sign in with password
accounts (`invite`, `add-user`); `share` prints the invite link to send them. `add-account` and `profile` make key
accounts for the Scripts edition on your own PC. Keep each key profile private ([accounts](authentication.md#accounts)).

```bash
./yo3-server init-auth localhost 127.0.0.1 192.168.1.50
./yo3-server add-account Alice
./yo3-server profile Alice Alice.json --advertise 192.168.1.50
./yo3-server accounts
./yo3-server add-player Alice Joey
./yo3-server rename Joey Joseph
./yo3-server set-country Alice TN
./yo3-server give-bp Alice 5000
./yo3-server give-bp Alice 5000 --request-id <the-id-printed-by-the-first-try>  # retry a lost answer, not a second grant
./yo3-server grant-title Alice "GAME MASTER"
./yo3-server give-cards Alice "Blue-Eyes White Dragon" --art 2 --count 3
./yo3-server give-cards Alice "Shooting Quasar Dragon"   # an added card (#75) too: no pack holds one yet
./yo3-server give-recipe Alice "E-HERO EVOLUTION DECK" --cards
./yo3-server unlock-cards                    # three of every card, every artwork, for every player (or: unlock-cards Alice)
./yo3-server add-code ABCD1234EFGH5678 2500
./yo3-server add-code "" 500 --uses 10       # generate a code; keep the printed request ID
./yo3-server remove-code ABCD1234EFGH5678
./yo3-server invite --uses 5 --days 7
./yo3-server --advertise 192.168.1.50 share --invite
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
- `share` prints what a friend needs to join: the server's name, its address (`--advertise` and `--auth-port`), its
  identity and the invite link carrying both. `--invite` adds an invite code for one sign-up, working for a week, and
  the link for one friend that carries it. A Desktop friend pastes the link under Multiplayer → **⋯** → Add a server;
  a Scripts friend runs `play --register LINK USER
  --profile client.json` ([Scripts](scripts.md#join-a-friends-server)).
- `add-account NAME` creates a key account and its first player, for the Scripts edition on your own PC. Names are 1–16 characters and
  unique. `--also B,C` adds players to that account, up to four total.
- `profile PLAYER FILE --advertise HOST` exports a key account's private profile for the play script. The profile
  references `server.crt`; copy the host's public certificate beside it. Existing profile files are not overwritten.
  `key WHO` prints a private profile too. All three commands commit account changes, audit and retry receipt before
  revealing a key and accept `--request-id ID`. After a lost answer, repeat the same input and ID: no duplicate
  account or key is created. Existing keys are reused; retries cannot recreate a deleted account. Profile files and
  keys grant game access without a password: share privately. Friends should use password accounts and invites.
- `add-user USER` prepares a one-time setup code instead of a password. The owner chooses a password through
  Desktop’s **Forgot password** or Scripts `--recover`, and saves their private recovery code. Finish setup before
  promoting the account. `reset-code USER` replaces a lost setup/reset code after you verify the owner’s identity.
  Both commands commit the change, audit and receipt together. They print a prepared code and request ID before
  sending; a prepared code is not a confirmation. For a lost response, reuse both through
  `--request-id ID --code-stdin < /path/to/private-code-file`. Never put the code in arguments or shell history.
  Codes work once for 24 hours. A retry reports current availability without reissuing or extending the code.
- `give-cards`, `give-recipe` and `grant-title` accept `--request-id ID`. After a lost answer, repeat the identical
  command with its printed ID. Gifts commit the player, associated D-Mail, audit and receipt before notifying the game;
  retries never give another copy or send another letter. Card/recipe D-Mail is kept for offline grants too. Recipe
  cards are counted together before checking the limit of 255 additional copies per artwork. The dashboard’s player
  **Gift** action uses the same operations with searchable game metadata and an explicit recipient confirmation.
- `set-role USER ROLE` is admin-only and commits the role, audit and retry receipt together. It accepts
  `--request-id ID`; repeat the same command with that ID after a lost response. A retry never reapplies an older role.
  Changing your own role can remove management access; another admin or the host must restore it with `set-role`.
- `invite`, `invites`, `add-user`, `reset-code` and `set-role` manage password accounts (beta) and work while the
  server runs: invite codes for sign-up, an account awaiting its owner’s password setup, a one-day reset code for a
  player who lost their password and recovery code, and roles. [Accounts](authentication.md#making-accounts-invites-and-recovery)
  explains each. `invite` and `share --invite` print a request ID before creation and accept `--request-id ID`.
  After a lost response, repeat the same command and limits with that ID to retrieve the original code and expiry.
  A retry does not add sign-ups back or extend the expiry. `invite --uses 0` means unlimited sign-ups; omit `--days`
  for no expiry. Positive limits must be at least 1, and expiry must fit the supported RFC 3339 date range.
- Commands that name an account (`key`, `add-player`, `set-country`, `give-bp`) take its User ID or one of its
  players' names.
- `add-player`, `rename` and `set-country` manage an account's players. `countries` lists accepted country codes;
  `auto` clears an override.
  `add-player`, `rename` and `set-country` print a request ID and accept `--request-id ID`. After an uncertain
  response, repeat the same arguments and ID; a rename retry still works when the old name is gone. Names are
  1–16 printable characters; accounts hold up to four players. Saved profile changes appear in game after the next
  sign-in. Country codes still come from the client's table; `auto` clears the override without needing that table.
- `grant-title PLAYER TITLE` gives a player a title by its name as the game shows it (`titles` lists them), such as
  GAME MASTER or JUMP FESTA, which no duel earns. The player wears it in Avatar Settings. A rename or title reaches
  a player who is online at their next login.
- `unlock-cards [PLAYER]` unlocks every card: each artwork of every card topped up to three copies beyond the starting
  cards, for PLAYER or, without one, every player; each player who got cards gets a D-Mail. It is Desktop's **Unlock all
  cards** for your own server. Accepts `--request-id ID` like the gifts.
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
  `give-bp`, `add-code` and `remove-code` print a request ID before sending. After a timeout or lost answer, repeat
  the same command and values with `--request-id ID`: the original result returns without a second payment, letter,
  generated code or audit row. For `add-code`, an empty CODE (`""`) asks for a generated code. Changing `--by` does
  not make it a new request. Successful receipts do not expire or follow audit pruning. The balance and D-Mail (for
  Give BP), or the code change, commit with audit and receipt; a failed commit publishes nothing. A removal retry
  does not delete a newly recreated code with the same name. Already claimed BP stays with players.
  Use them rather than editing `accounts.json`: after the first start of a current server, accounts live in
  `server.db`. The JSON files are old copies the server no longer reads; it warns while they exist, and they are safe
  to delete. A player who is online sees BP changes (gifts, duel rewards) on the game's next screen. `codes` lists
  who claimed each code and when. The server's terminal output is coloured by level (`NO_COLOR=1` turns it off).

`--help` on each subcommand lists the exact parameters. If playing from source, [the development guide](development.md#build-from-source)
shows the explicit DLL/injector paths. For a release bundle, use its [play script](scripts.md#join-a-friends-server).

`set`, `reset` and `maintenance` also print a request ID and accept `--request-id ID`. After a lost response,
repeat the same command and values with that ID. Settings, audit and receipt save together; scheduling saves both
the message and deadline or neither. A retry returns the original confirmation without restoring an old setting
or moving the countdown. Use `settings` to inspect the current values. `maintenance --off` must be used alone,
without `--in` or `--message`. The React development dashboard exposes the same controls; see the
[dashboard guide](dashboard.md#local-development).

Moderation commands (`kick`, `ban`, `restrict`, `lift`, `mute`, `unmute`, `sign-out`) print and accept
`--request-id ID`. After an uncertain answer, repeat the same arguments and ID. A receipt cannot repeat a disconnect,
reinstate an older ban/mute or extend its expiry. Ban/sign-out save device revocation first; live disconnect intent
saves in the same transaction. If its outcome cannot be recorded, the account change stays saved and the response
warns that the disconnect is unknown. Startup labels it interrupted, without repeating it. Inspect the current
account/session before requesting a new disconnect. Reasons are single-line, at most 200 characters. Use a positive
duration or omit it until lifted. The React dialog offers the same actions from View account → Moderate account.

`stop` first captures the running process's identity and prints its request/run IDs. After a lost response, copy
its printed `stop --request-id ID --run-id RUN` command. Never substitute a new run ID when retrying. The saved
receipt cannot stop a restarted process, and an old run is refused even if the original request never committed.
Stop intent is saved before signalling; an uncertain outcome means inspect the process/logs, not send another stop.
A confirmed signal still allows final saves and the shutdown backup to finish. Desktop uses the same run-bound
operation. The connected React preview shows live checked status and requires confirmation before stopping.

The connected dashboard's **Audit log** reads saved administrative history with search and older/newer pages. IDs
are text so large values remain exact. **Server logs** shows up to 500 of the 2,000 most recent captured lines,
newest first, with warnings/errors filtering, optional refresh and Copy visible logs. This view clears on restart;
full diagnostics remain in the server's log folder. Fetch failures are shown explicitly and do not become empty results.

## Network settings

Friends reach one **TLS listener**, normally TCP **7443**. The internal HTTP/admin listener remains on loopback;
never forward it. A launcher supplies the game's gate/front/duel/HTTP ports locally through the required DLL.
There is no stock-client/IP-auth fallback, browser `/join` setup or INI download.

- **Bind** chooses which local interfaces accept connections. `0.0.0.0` listens on all IPv4 interfaces;
  `127.0.0.1` permits only this PC.
- **Advertise** is the reachable LAN/VPN/public address put into `share`'s invite link and a key profile. It does not open a listener or change
  firewall policy. Include the address in the identity's certificate names when creating it.
- **External port mappings:** if the public port differs from the local listener, use the external address/port
  when exporting the profile. Keep `--auth-port` and advertised ports consistent with the intended mapping.
- **Firewall/router/VPN:** manage these outside Revival. Check the address from a friend's PC; a successful host-local
  test does not prove remote reachability.

Invite links (`yo3revival://server?address=…&identity=…`, from Desktop's Host for friends or `yo3-server share`) carry
the address and the identity; players then sign in with a password account, in Desktop or with the play script's
`--sign-in`. Its profile, like a key profile and its adjacent public certificate, pins the host's identity. Accounts sharing a router keep separate
identities.

For an existing localhost-only identity, do not delete `data/` and rerun `init-auth`: that makes a new key, and every
player would have to trust the server again.
[Authentication](authentication.md) explains trust, certificate handling and session lifetimes.

## Existing data and backups

`server.db` is SQLite in WAL mode, with one row per account (and its players), record, friendship, mail, team, event,
rental and BP code; a save writes only the rows that changed. This dashboard branch uses schema 6 (row-based state, retry receipts and disconnect/stop/backup outcomes). Schemas 1–5
upgrade transactionally in place; older server binaries cannot open the upgraded database. Keep a pre-upgrade backup
if you need to return to an older binary. JSON files from versions before
`server.db` are not read (the beta starts fresh). A path change selects a different server's data; it does not move it.
The running server holds its data directory: a second server on the same folder is refused, whatever its ports. A
`server.lock` left after a crash holds nothing.

If a save fails (a full disk), the server says so in its log and refuses new logins until a save works again; players
already in keep playing.

The server backs up `server.db` into `backups/` when it starts, every hour while it runs, and when it stops, keeping the
newest 24. Keep the printed request ID for a manual backup; retry with `backup --request-id ID` after a lost
response. A retry checks the same outcome and never creates another snapshot. Only restore requires the server stopped:

```bash
./yo3-server --data-dir data backup           # create a checked snapshot, also while the server runs
./yo3-server --data-dir data backups          # list them, newest first
./yo3-server --data-dir data restore latest   # or a name from the list; the replaced database is kept beside it
```

Back up the entire data folder securely, including `server.key`: the database backups in `backups/` don't contain that
key. Keep players' local deck-editor folders too. Follow [the backup and upgrade procedure](launcher.md#your-data-and-upgrades)
before moving data, restoring or downgrading.

## Windows SmartScreen and antivirus warnings

The executables, the injector and the connection DLL are unsigned for now, so Windows or security software may block
or quarantine them. [Antivirus warnings](antivirus.md) is the player's page: which files and folders are ours, checking
a download against the release's `SHA256SUMS`, restoring a quarantined file, allowing only files you checked, and
reporting a false positive. Scripts use the same injector and DLL as Desktop.

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

## Other programs inside the game

**Since v0.1.4 (2026-11-19).** `play` (and `yo3-launcher`) check what else runs inside the game before releasing it (#68): other programs'
`.dll` and `.asi` files in the game folder, modules loaded from outside Windows' folders, redirected network functions and
non-Microsoft 32-bit Winsock providers. The connection DLL writes what it finds to the game's session log (`[YO3 MOD] Survey: …`,
with each module's size and SHA-256) and, for the launcher, a JSON summary at `--survey-file FILE` (default: beside the
session log, removed at the end).

| Found | `play` |
| ----- | ------ |
| another program redirected a game network function, or a file on the known-bad list | stops: `The game was not started: … It would break the game's connection.` |
| a `.dll`/`.asi` in the game folder, a detour at the start of a Windows network function (usually security software), a Winsock provider that isn't Microsoft's | stops, saying so; `--accept-foreign` starts the game anyway |
| anything else (an overlay's DLL, a driver's) | only in the log |

For a foreign file, rename it yourself (Desktop's **Move it aside** appends `.yo3-disabled`). For a provider, run
`netsh winsock reset` in an administrator Command Prompt and restart; YO3 Revival never runs it. Desktop shows the same findings
in a dialog: [Other programs inside the game](launcher.md#other-programs-inside-the-game).

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
startup with `stage=mod-resolution`. **Since v0.1.4 (2026-11-19):** opening `yo3.exe` directly uses the size the last Desktop PLAY saved (Desktop only; Scripts has no direct start). How it works:
[client internals](client-internals.md#higher-resolution).
