# Hosting and joining: playing Yu-Gi-Oh! ONLINE 3 with friends

Everyone uses the revival launcher and connection DLL with their own original game install (ver. 1.153). The
server and launcher authenticate each account over TLS. No game files are modified or shipped. These instructions
are for current source; the last published release predates the authenticated launcher.

**Contents**

- [Play on your own PC](#play-on-your-own-pc)
- [For the host](#for-the-host)
  - [LAN checklist](#lan-checklist)
  - [Windows SmartScreen and antivirus warnings](#windows-smartscreen-and-antivirus-warnings)
  - [macOS](#macos)
  - [Run from source](#run-from-source)
- [For every player](#for-every-player)
  - [Joining, step by step (with pictures)](#joining-step-by-step-with-pictures)
- [What works and what doesn't yet](#what-works-and-what-doesnt-yet)
- [When something goes wrong](#when-something-goes-wrong)

## Play on your own PC

Install the original game (on Linux, under Wine), build the bundles with `tools/release.sh`, and unpack the one for
your system. On Windows substitute `.\yo3-server.exe` for `./yo3-server` in these commands:

```sh
./yo3-server init-auth localhost 127.0.0.1
./yo3-server add-account Yugi
./yo3-server profile Yugi client.json --advertise localhost
```

Copy `data/server.crt` next to `client.json` as `server.crt`. Keep `data/server.key` on the server. Start `run.cmd`
(Windows) or `./run.sh` (Linux), then launch the game as described below. Initialization runs once; accounts and the
server identity persist in `data/` and are backed up at server startup.

## For the host

Before initializing the certificate, choose the DNS name or IP your friends reach. Include it in `init-auth`, for
example `./yo3-server init-auth localhost 127.0.0.1 192.168.1.50`. A certificate initialized only for localhost does
not authenticate a LAN IP. Existing certificate/key files are never overwritten by this command.

With the server stopped, make an account per friend and export their profile:

```sh
./yo3-server add-account Alice
./yo3-server profile Alice Alice.json --advertise 192.168.1.50
```

Privately give Alice `Alice.json` and the public `data/server.crt`; she keeps them together, with the certificate
named `server.crt`. Each friend needs their own profile, even behind one router. The profile contains a secret
account key; the certificate is public. Keep the server private key on the host. On Windows keep private profiles
and the server key in folders private to their intended users; the tools set mode 0600 on Unix.

Run `run.cmd` or `./run.sh`. The host exposes TCP **7443** for TLS (`--auth-port` changes it). The internal HTTP/admin
port, 8080 by default, stays on loopback and needs no firewall opening. A VPN is fine; TLS remains required.

Existing players keep their cards, BP and ids. Export `profile EXISTING_PLAYER client.json --advertise HOST`; an
address-only account receives a key without moving its data. The old `/join` page and `yo3.ini` downloads are gone.
Old clients cannot log in; the launcher is required. See [authentication.md](authentication.md) for the protocol and
migration details.

Useful account commands (server stopped except the BP operations):

```sh
./yo3-server accounts
./yo3-server add-player Yugi Joey
./yo3-server rename Joey Joseph
./yo3-server set-country Yugi TN
./yo3-server give-bp Yugi 5000
./yo3-server add-code ABCD1234EFGH5678 2500
```

`--regulation NAME_OR_ID` sets the login banlist; by default it is unlimited. Hosted duels keep their host's choice.

### LAN checklist

- Confirm the server reports `authenticated TLS` on the intended address and port.
- Allow TCP 7443 on the host firewall; with a VPN or router, route/forward that port to the host.
- Check the profile's server address and certificate name against the host's certificate names.
- Give each friend the correct public certificate and their own private profile.
- Launch through `yo3-launcher`; changing `yo3.ini` or starting `yo3.exe` directly cannot authenticate.

### Windows SmartScreen and antivirus warnings

These are unsigned pre-alpha executables and an injector/DLL, which Windows or an antivirus may flag. Build from
source or review the code before trusting them. The injector loads only the DLL paths supplied to it; it terminates
the game before startup if a required hook fails. Restore files removed by an antivirus only after reviewing them.

### macOS

No macOS bundle or live check is provided. The supported builds are Linux with Wine and Windows.

### Run from source

```sh
engine/fetch.sh
cargo build --release -p yo3-server -p yo3-launcher
tools/mod/build.sh
```

Follow the host setup above. For a headless development client set `YO3_AUTH_PROFILE` to its private profile and use
`tools/client/vclient.sh start --trace`; the working guide contains the private-display commands.

## For every player

Start the launcher, using the private profile and game folder:

```powershell
.\yo3-launcher.exe --profile .\Alice.json --game-dir "C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3"
```

On Linux:

```sh
./yo3-launcher --profile Alice.json --game-dir "$HOME/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3"
```

The bundle's injector and DLL sit beside the launcher. Linux needs Wine; `--wine-prefix` selects another prefix.
No `yo3.ini` edits are required. Authentication happens before the game starts; its original username/password
fields are unused. A bad key, certificate or DLL stops startup and reports the reason in the launcher's terminal.
Keep that terminal open while playing.

### Joining, step by step (with pictures)

1. Start the revival launcher with your private profile and the host certificate.
2. Press **ONLINE**, dismiss the original login form, then select your player.
3. Once: **Main Menu → Edit Deck**, then leave it. This saves the starter and official character decks.

![Main Menu and Edit Deck](images/guide/6-edit-deck.png)

4. **Auto Duel → Confirm** matches two waiting players. To pick a friend, use **Create Duel**, then your friend's
   Player Information → **Send Duel Request**. Choose your deck and play. Computer opponents are at Kaiba Corporation.

![Auto Duel](images/guide/7-auto-duel.png)

Only the matched players can submit decks or start that duel. Create Duel keeps the chosen banlist after acceptance.

## What works and what doesn't yet

Working: chat rooms and whispers, player search, NPC duels, Auto Duel and Create Duel between players (Single
and Match Mode), choosing targets, tributes, positions, coin calls and chains in the game's own dialogs, attack, summon,
chain-link and destruction animations, the result screens, avatar settings, and the shop's booster packs and structure decks: every
account starts with 5000 BOOSTERPASS Points (100 a pack), duels earn more (100 for a win, 50 for a draw, 30 for a
loss; the shop shows them after your next login or purchase), so do codes the host gives out (Main Menu → BOOSTERPASS),
and the cards you pull are kept on the server. A
structure deck you buy shows up in Edit Deck's deck list, ready to duel with.

Duels between players have clocks: 3 minutes a turn and 10 minutes a duel (or the host's choice in Create Duel), and
running out loses the duel.

To add a friend, open their Player Information (click them in player search) and press Send Friend Request; they
get a friends icon at the top left, and accepting adds you to each other's lists. Main Menu → Friend List shows all your
friends; the search tab's two-people button shows the ones who are online. Friends are kept by the host's server.
In Create Duel, Opponent's Limits → Friend lets only your friends send you a duel request.

Main Menu → D-Mail sends a friend a message (Write Mail, then the friends button to pick them); they see a mail icon
at the top left, or get it at their next login if they were away.

Main Menu → Team forms a team; its leader invites players from their Player Information. A team has its own chat tab
and D-Mail (Send to Team), and Create Duel's Opponent's Limits → Team takes requests only from your team.

Satellite Plaza holds user events: Create User Event makes a knockout tournament, friends press Participate in its
details, and the host starts it from the duel button's Event Menu in the lobby; the server pairs everyone and each
pair duels, round by round, until one is left.

You can watch a friend's duel: find them in player search (they show as duelling), open Player Information and press
Watch Duel. You see the duel from one player's side with both hands hidden, and go back to the map when it's over.

In Match Mode, a player whose deck has a side deck gets the Card Swap screen between duels: drag cards between the
side deck and the main or extra deck, then press the arrow at the top left to go on. Build a side deck in Edit Deck.

The shop also sells card protectors (200 BP) and avatar items (100 BP): Avatar Settings and the deck editor offer
them once bought.

To trade cards, open a player's Player Information and press Send Trade Request; once they accept, drag cards from
your trunk into your half of the table, press Ready to Trade., and answer Yes to "Execute Trade?". A deck that used a
card you traded away no longer counts as valid until you edit it.
Changing either offer withdraws both players' approvals: both must press Ready to Trade. and confirm the new offer.
Existing accounts and accounts made through `add-account` can trade and rent cards too; their collection follows the player.

To lend cards, open a player's Player Information and press Send Rental Request. Once they accept, the lender sets the
conditions (a term of 1, 3, 7 or 14 days, and Free) and double-clicks cards from their trunk into the list, then presses
Confirm; the borrower answers "Borrow these cards?" and the cards join their trunk (marked as borrowed, so they can be
put in decks but not traded) while the lender's copies are out of theirs. When the term is over the cards are back with
their owner, even across a server restart (they are kept in `data/rentals.json`); there is no early return. Both players'
Main Menu > Rental Card List shows what they lent and borrowed. Players' decks stay on their own machines (in
`Documents\Yu-Gi-Oh! ONLINE 3`): a deck that used a borrowed card is no longer valid once the rental ends.

## When something goes wrong

- **Authentication failed:** check that this is your private profile for this host. The game has not been started.
- **Certificate or TLS handshake failed:** check the public certificate and the expected DNS name/IP. Use a name
  included by the host when initializing the certificate; certificate checks cannot be bypassed.
- **Cannot connect:** check TCP 7443, the profile's server address, and the host's firewall/VPN.
- **Connection mod did not initialize:** rebuild/restore `yo3-auth.dll` and `yo3-auth-launch.exe` from this version.
  Unsupported imports leave the game suspended and it is terminated; there is no fallback connection.
- **Game cannot find the server:** start it through the launcher, and keep the launcher running.
- **Duels end at once as draws:** inspect the server's `DUELS WON'T WORK` message. Restore the complete `engine/`
  folder from the bundle, or fetch the dependencies and install a compatible Node.js when building from source.
- **A long session disconnects:** launcher sessions expire after 12 hours. Restart the launcher to authenticate again.
- **Found a bug:** open <https://github.com/usersina/yo3-revival/issues/new/choose> with the steps and logs. Do not attach
  private profiles, account keys or `server.key`. Game packet logs exclude transport credentials.
