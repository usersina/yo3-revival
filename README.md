# yo3-revival

A private server for **Yu-Gi-Oh! ONLINE 3 (Duel Accelerator)**, Konami's online card game that shut down in 2012. Log
in through the authenticated revival launcher and duel again: against the computer, or with your friends.

<p align="center">
  <img src="docs/images/duel.png" alt="The original Yu-Gi-Oh! ONLINE 3 client in a duel against Seto Kaiba, served by this project" width="800">
</p>

<p align="center">
  <sub>The original game client (ver. 1.153), mid-duel on this server (screenshot of Konami's game).</sub>
</p>

## Start here

There is no public server. You can host on your own PC or join a friend's server. Everyone needs their own install
of the original game **ver. 1.153**; Linux players also need Wine. Game files are not included.

**Current source requires the revival launcher.** Published releases through v0.0.3 predate it and do not follow these
instructions. Until a new release is published, a developer must [build the current bundles](docs/guide.md#run-from-source).
Each bundle includes the server, rules engine, launcher and connection DLL.

- **Joining a friend?** Get your private profile and `server.crt` from the host, then follow
  [the player instructions](docs/guide.md#for-every-player). You do not need to run a server or install build tools.
- **Hosting friends?** Choose the address they will use **before** creating the certificate. Follow
  [the host setup](docs/guide.md#for-the-host); only TCP **7443** needs to reach your PC.
- **Playing alone?** Follow [the local setup](docs/guide.md#play-on-your-own-pc), then duel the computer at
  **Kaiba Corporation**.

Authentication happens in the launcher; the game's original username/password fields are unused. The game executable
and `yo3.ini` stay unchanged. Each account has its own private profile, so friends sharing a router stay separate.

## Your first duel

1. Keep the server running, then start the launcher with your profile and game folder.
2. Press **ONLINE**, dismiss the original login form and select your player.
3. Open **Main Menu → Edit Deck** once, then leave it to save your starting decks.
4. Visit **Kaiba Corporation** for a computer opponent. With a friend, both choose **Auto Duel → Confirm**, or use
   **Create Duel** to arrange a specific match.

The [host and player guide](docs/guide.md) has complete Windows and Linux commands, account management and pictures.

## Something wrong?

Check [When something goes wrong](docs/guide.md#when-something-goes-wrong) first. Still stuck, or found a bug?
[Open an issue](https://github.com/usersina/yo3-revival/issues/new/choose) and attach a screenshot of the server's
window if you can.

## What works

Duels against the computer and between players (Auto Duel, Create Duel, Match Mode, spectating) with the real card
rules, the official Yugi, Jaden and Yusei decks, Edit Deck, the shop and booster packs, trades, chat, friends, D-Mail,
teams and tournaments. The full list, and what is left, is in [the guide](docs/guide.md#what-works-and-what-doesnt-yet)
and the [roadmap](docs/roadmap.md).

| Duel against Kaiba | Official character decks |
|---|---|
| ![Seto Kaiba on the map's Kaiba Corporation](docs/images/kaiba.png) | ![The Select Deck screen listing the official decks](docs/images/decks.png) |

## For developers

The server is written in Rust, reverse-engineered from the game client, and runs duels on
[EDOPro](https://github.com/edo9300/edopro)'s rules engine. Everything technical (building from source, the protocol,
the client's internals, the roadmap) is in [docs/](docs/README.md). To contribute, read [AGENTS.md](AGENTS.md) first.

## Legal

This is a non-commercial fan preservation project. It is **not affiliated with or endorsed by Konami**. Yu-Gi-Oh!
and Yu-Gi-Oh! ONLINE 3 belong to Konami; so do the cards, art and the game client, and the screenshots above show
Konami's game. This repository holds only original code and notes: no game files, card data, art, decompiled code or
packet logs are distributed (the screenshots in `docs/images/` are the one exception, and they show the game, not its
data). You need your own copy of the client, and the server reads your install's data files at run time. The release
bundles also carry the rules engine, which is other people's open-source work (see `engine/THIRD-PARTY.md` in a
bundle).

## License

[MIT](LICENSE), covering this project's own code and documentation. It does not cover Konami's game or any of its
content.

---

<sub>If you'd like to support the work: [Buy me a coffee](https://buymeacoffee.com/usersina) · BTC
`bc1qawq699ewyq0vgyux6nmyyx29r4vhj573l77elj`</sub>
