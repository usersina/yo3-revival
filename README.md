# yo3-revival

A private server for **Yu-Gi-Oh! ONLINE 3 (Duel Accelerator)**, Konami's online card game that shut down in 2012. Log
in through the authenticated revival launcher and duel again: against the computer, or with your friends.

<p align="center">
  <img src="docs/images/duel.png" alt="The original Yu-Gi-Oh! ONLINE 3 client in a duel against Seto Kaiba, served by this project" width="800">
</p>

<p align="center">
  <sub>The unmodified game client (ver. 1.153), mid-duel on this server (screenshot of Konami's game).</sub>
</p>

## Play on your own PC

There is no public server: you run one yourself. Current source requires the revival launcher and connection DLL.
The last published release predates this setup; build current bundles with `tools/release.sh`.

1. Install your own copy of the original game (ver. 1.153; on Linux, under Wine).
2. Build/unpack the bundle for your system. It includes the server, rules engine, launcher and our connection DLL.
3. Once, initialize the server identity and account (Windows: use `.\yo3-server.exe`):

   ```sh
   ./yo3-server init-auth localhost 127.0.0.1
   ./yo3-server add-account Yugi
   ./yo3-server profile Yugi client.json --advertise localhost
   cp data/server.crt server.crt
   ```

4. Start `run.cmd` on Windows or `./run.sh` on Linux. Keep it open.
5. Start `yo3-launcher.exe --profile client.json --game-dir "C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3"`
   on Windows. On Linux use `./yo3-launcher` and your Wine game directory.
6. Press **ONLINE**, pick your player, open **Main Menu → Edit Deck** once, then visit **Kaiba Corporation** to duel.

Authentication happens in the launcher. The original game's username/password fields are unused. Its binary and
`yo3.ini` stay unchanged. The [host and player guide](docs/guide.md) has the full commands.

## Play with friends

Initialize the certificate with the DNS name or IP your friends will reach. Create a separate account and private
launcher profile for each friend, and give each the public `server.crt`. Only TCP **7443** needs to reach the host;
all game connections use TLS. Accounts behind the same router stay separate. Then use **Auto Duel** or **Create Duel**.

The [host guide](docs/guide.md#for-the-host) covers setup and existing-account migration. See the
[authentication design](docs/authentication.md) for credential handling and verification.

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
