# yo3-revival

A private server for **Yu-Gi-Oh! ONLINE 3 (Duel Accelerator)**, Konami's online card game that shut down in
September 2012. It was reverse-engineered from the original game client and written in Rust, so you and your friends
can log in with the real client and duel again. Duels run on [EDOPro](https://github.com/edo9300/edopro)'s rules
engine.

<p align="center">
  <img src="docs/images/duel.png" alt="The original Yu-Gi-Oh! ONLINE 3 client in a duel against Seto Kaiba, served by this project" width="800">
</p>

<p align="center">
  <sub>The unmodified game client (ver. 1.153), mid-duel on this server (screenshot of Konami's game).</sub>
</p>

**Contents:** [What works](#what-works) · [Play](#play) · [Host](#host) · [Documentation](#documentation) ·
[Legal](#legal) · [License](#license)

## What works

- **Duels** between players (Auto Duel, Create Duel, Single and Match Mode with side decks, spectating) and against
  the computer opponents at Kaiba Corporation, with the real card rules (chains, tributes, Synchro summons, and so
  on) in the game's own dialogs and animations. Duel logs and replays are kept.
- **Cards and decks:** the Edit Deck screen, official character decks for Yugi, Jaden and Yusei, banlists
  (regulations) and a server-side deck check.
- **Economy:** the shop (booster packs, structure decks, protectors, avatar items) paid in BOOSTERPASS Points that
  duels earn, card trades and card rentals between players.
- **Social:** chat rooms and whispers, player search, friends and ignore lists, D-Mail, teams, and user events
  (knockout tournaments).
- **Self-service join page:** friends make their own account in a browser, no host work per player.
- **Hosting:** Linux and Windows server builds, a LAN, a VPN or a forwarded port.

What is left is in the [roadmap](docs/roadmap.md).

| Duel against Kaiba | Official character decks |
|---|---|
| ![Seto Kaiba on the map's Kaiba Corporation](docs/images/kaiba.png) | ![The Select Deck screen listing the official decks](docs/images/decks.png) |

## Play

You need your own copy of the game (the last released client, ver. 1.153, installed on Windows or under Wine on
Linux) and a host running this server. The original installer (the 2011-07-28 setup, about 900 MB) is preserved by a
third party on [My Abandonware](https://www.myabandonware.com/game/yu-gi-oh-online-3-duel-accelerator-qy4); it is
Konami's software, not part of this project, and you download it at your own discretion. The host sends you an
address; then:

1. Open **`http://HOST:8080/join`** in a browser, type the player name you want and press **Join**.
2. Press **Download yo3.ini** and put it in the game's install folder, replacing the original (keep a copy of it).
   On Windows that is `C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3`.
3. Start the game, press **ONLINE**, log in with any name and password, and pick your player.
4. Once: **Main Menu → Edit Deck**, then leave it. That saves your starter deck.
5. Press the duel button on the bottom bar and pick **Auto Duel** (when two players wait for the same format they are
   matched) or **Create Duel** (your friend sends you a request from your Player Information), or walk the map to
   Kaiba Corporation for a computer opponent.

The full player guide, with troubleshooting, is [docs/guide.md](docs/guide.md#for-every-player).

## Host

You need Linux or Windows, the game installed (the server reads its card data from it), Node.js 22.18 or newer, Git
and curl.

1. Download the bundle for your system from the [latest release](https://github.com/usersina/yo3-revival/releases/latest)
   (Linux `.tar.gz`, Windows `.zip`; or build both with `tools/release.sh`), unpack it, and run `./run.sh` (Linux) or
   `run.cmd` (Windows). The first start downloads the rules engine's packages, card scripts and card database (about
   160 MB).
2. Open TCP ports **5730**, **1213**, **1214** and **8080**, or put everyone on one VPN (Tailscale, ZeroTier) so
   nothing needs forwarding.
3. Share `http://YOUR-ADDRESS:8080/join` with your friends. On a server reachable from the internet, start it with
   `--no-join` and make accounts by hand, because the join page's keys travel over plain HTTP.

Details (firewalls, forwarding, banlists, accounts, moving a friend's account, backups) are in
[docs/guide.md](docs/guide.md#for-the-host). To build and run from source instead:

```bash
cargo run --release -p yo3-server -- --bind 0.0.0.0 --log-dir logs
```

## Documentation

Everything detailed lives in [docs/](docs/README.md), which says what to read in what order.

| Read | For |
|---|---|
| [docs/guide.md](docs/guide.md) | Hosting and joining: setup, every feature from a player's side, troubleshooting |
| [docs/roadmap.md](docs/roadmap.md) | What is done, what is open, and every guess we made |
| [docs/protocol.md](docs/protocol.md) | Every request and reply, with the client evidence behind it |
| [docs/client-internals.md](docs/client-internals.md) | Client addresses, data formats, crash history |
| [docs/research.md](docs/research.md) | The original feasibility study |
| [CLAUDE.md](CLAUDE.md) | How to work on the project (for people and AI agents): rules, commands, the reverse-engineering loop |

To contribute, read [CLAUDE.md](CLAUDE.md) first. The short version: the game client is the spec, every reply cites
client evidence, tests come first, and `cargo nextest run --workspace` and `cargo clippy --workspace --all-targets`
stay clean.

## Legal

This is a non-commercial fan preservation project. It is **not affiliated with or endorsed by Konami**. Yu-Gi-Oh!
and Yu-Gi-Oh! ONLINE 3 belong to Konami; so do the cards, art and the game client, and the screenshots above show
Konami's game. This repository holds only original code and notes: no game files, card data, art, decompiled code or
packet logs are distributed (the screenshots in `docs/images/` are the one exception, and they show the game, not its
data). You need your own copy of the client, and the server reads your install's data files at run time.

## License

[MIT](LICENSE), covering this project's own code and documentation. It does not cover Konami's game or any of its
content.
