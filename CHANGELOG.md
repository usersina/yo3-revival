# Changelog

Pre-alpha: expect bugs, and please [report them](https://github.com/usersina/yo3-revival/issues/new/choose).

Each section is a release. To release, rename the "Unreleased" section at the top to the next version
(`## v0.0.2 (date)`) and push to `main`: the release workflow builds the bundles and publishes the version under that
tag, with the section as its notes. A push whose newest version is already released publishes nothing, so changes can
gather under "Unreleased" first.

Older entries describe their release at the time. For current setup, use the [README](README.md) and
[host and player guide](docs/guide.md); the authenticated launcher below has not been published yet.

## Unreleased

- Added: authenticated revival launcher and connection DLL. Accounts sharing one IP stay separate; front and duel
  connections use validated TLS and single-use tickets. The game executable and INI stay unchanged.
- Changed: the launcher is now required. Initialize the host certificate, export a private profile per account, and
  allow TCP 7443. Raw game connections and the IP-based HTTP join page are no longer supported. Existing account data
  and keys carry over. Both bundles include the launcher, injector, DLL and new setup guide.
- Fixed: changing a card trade's offer after the other player confirmed can no longer execute the changed deal
  with that old confirmation. Both players must approve the current offers.
- Fixed: a refused attempt to delete the account's last player no longer removes their friends, mail and rentals.
- Fixed: existing key-based accounts (including those created through the retired join page) can complete card
  trades, rental offers check their owned cards, and their collection refreshes after another session changes it.
- Fixed: a third player can no longer enter or cancel another pair's duel by sending its duel id.
- Fixed: Create Duel keeps the host's chosen banlist after the challenger accepts.
- Fixed: documentation links and their checker follow the working guide's rename to `AGENTS.md`.
- Docs: clearer joining/local/hosting paths, complete Windows and Linux setup, corrected source-build and backup
  instructions, and current behavior separated from historical notes. Link checks now include the changelog and nested docs.

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
