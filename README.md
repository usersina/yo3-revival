# yo3-revival

A private server for **Yu-Gi-Oh! ONLINE 3 (Duel Accelerator)**, Konami's online card game that shut down in 2012. Log
in with the original game client and duel again: against the computer, or with your friends.

<p align="center">
  <img src="docs/images/duel.png" alt="The original Yu-Gi-Oh! ONLINE 3 client in a duel against Seto Kaiba, served by this project" width="800">
</p>

<p align="center">
  <sub>The unmodified game client (ver. 1.153), mid-duel on this server (screenshot of Konami's game).</sub>
</p>

## Play on your own PC

There is no public server: you run one yourself, on your PC. It takes about five minutes, on Windows or Linux.

1. **Get the game.** Download the original installer (about 900 MB) from
   [My Abandonware](https://www.myabandonware.com/game/yu-gi-oh-online-3-duel-accelerator-qy4) and install it (on
   Linux, with Wine). It is Konami's software, kept online by a third party, not part of this project.
2. **Get the server.** Download the file for your system from the
   [latest release](https://github.com/usersina/yo3-revival/releases/latest) (Windows: the `.zip`, Linux: the
   `.tar.gz`) and unpack it anywhere. Nothing else to install.
3. **Start the server.** Double-click `run.cmd` (Windows) or run `./run.sh` (Linux). Keep its window open while you
   play. If Windows says "Windows protected your PC", press *More info*, then *Run anyway*.
4. **Make your account.** Open <http://127.0.0.1:8080/join> in your browser, type a player name and press **Join**.
   Press **Download yo3.ini** and put that file in the game's folder (Windows:
   `C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3`), replacing the one there (keep a copy of the old one).
5. **Log in.** Start the game, press **ONLINE**, log in with any name and password, and pick your player.
6. **Once:** open **Main Menu → Edit Deck**, then leave it. That saves your starter deck.
7. **Duel!** Walk the map to **Kaiba Corporation** for a computer opponent.

Every step has a screenshot in the [picture guide](docs/guide.md#joining-step-by-step-with-pictures).

## Play with friends

One of you runs the server (steps 2 and 3 above) and tells the others their address. Everyone else skips steps 2 and
3 and, in step 4, opens `http://THEIR-ADDRESS:8080/join` instead of `127.0.0.1`. Then press the duel button on the
bottom bar and pick **Auto Duel** to be matched with a friend.

The host must let friends reach ports 5730, 1213, 1214 and 8080. On the same Wi-Fi or LAN that is usually just a
firewall prompt to allow. Over the internet, the easiest way is a free VPN such as Tailscale. The
[host guide](docs/guide.md#for-the-host) has the details, and a [LAN checklist](docs/guide.md#lan-checklist) for when
friends can't connect.

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
the client's internals, the roadmap) is in [docs/](docs/README.md). To contribute, read [CLAUDE.md](CLAUDE.md) first.

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
