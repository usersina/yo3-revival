# Playing Yu-Gi-Oh! ONLINE 3 with friends

One person (the host) runs the server; everyone, the host included, plays with the original game client pointed at
it. This server is unofficial and has nothing to do with Konami. It ships no game files: every player needs their own
install of the client (the last version, from 2012).

## For the host

You need Linux (on Windows, use WSL), the game installed (the server reads its card data from your install), Node.js
22.18 or newer (it runs the engine's TypeScript directly), `git` and `curl`.

1. Unpack the release (or build it from the repository with `tools/release.sh`) and run `./run.sh`. The first start
   downloads what the rules engine needs (EDOPro's core, card scripts and card database, about 160 MB). Your game
   install is found in `~/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3`; pass `--game-dir DIR`
   otherwise.
2. Make the server reachable. Friends connect to TCP ports **5730** (gate), **1213** (front), **1214** (duels) and
   **8080** (patch check):
   - **Same network:** friends use your LAN address (`ip -4 addr`, e.g. `192.168.1.2`). Open the ports if a firewall
     runs: `sudo ufw allow 5730,1213,1214,8080/tcp`, or for firewalld `sudo firewall-cmd --add-port={5730,1213,1214,8080}/tcp`.
   - **Over the internet, easiest:** a mesh VPN (Tailscale, ZeroTier, NetBird). Everyone joins your network and uses
     your VPN address (e.g. `100.x.y.z`). Nothing to forward, and only your friends can reach the server.
   - **Over the internet with port forwarding:** forward the four ports on your router to your machine, and start the
     server with your public address or DNS name: `./run.sh --advertise my.name.example`.

   The server sends each client to the address it used to reach the server, so one server answers on all of these at
   once. `--advertise` is only needed when the address clients dial isn't one of the machine's own (port forwarding).
3. Name your friends' players. There's no sign-up: the server tells players apart by the address they connect from,
   and each address gets its own account with up to four players. A new address starts with a player called
   "Duelist" (then "Duelist2", "Duelist3"...). With the server stopped:

   ```sh
   ./yo3-server add-player 100.64.0.7 Anis   # before Anis first connects: their account gets just this player
   ./yo3-server rename Duelist2 Joey         # or rename one afterwards
   ./yo3-server accounts                     # who's who
   ```

   Two friends behind the same router (the same public address) share one account; a VPN gives each their own.
4. Accounts and win/loss records live in `data/` (`accounts.json`, `records.json`), backed up into `data/backups/` at
   every start (the last 10 are kept). Back up `data/` to keep them.

## For every player

1. Install Yu-Gi-Oh! ONLINE 3. On Windows it runs as it is; on Linux it runs under Wine.
2. Open `yo3.ini` in the install folder (`C:\Program Files (x86)\Konami\Yu-Gi-Oh! ONLINE 3`), keep a copy of the
   original, and point it at the host (here `100.64.0.5`; the host itself can use `127.0.0.1`):

   ```ini
   [Yo3]
   GateServerAddress=100.64.0.5
   GateServerPort=5730
   PatchUpdateUrl=http://100.64.0.5:8080/yo3/
   PatchDataUrl=http://100.64.0.5:8080/ygoda/patch/
   InfoURL=http://100.64.0.5:8080/app/
   ```

3. Start the game and log in with any name and password (they aren't checked), then pick your player.
4. Once: open **Main Menu → Edit Deck** and leave it again. That saves your starter deck; without a deck the duel
   button says "There are no valid Decks available."
5. To duel a friend, press the duel button on the bottom bar, then either:
   - **Auto Duel → Confirm**: when two players wait for the same Rule Format they're matched;
   - **Create Duel → Confirm** (with a password if you like): your friend opens the player list (the first tab of
     the panel on the left), clicks your name, and presses **Send Duel Request**; you accept.

   Then pick your deck and play. Kaiba Corporation on the map has the computer opponents.

## What works and what doesn't yet

Working: chat rooms and whispers, player search, NPC duels, Auto Duel and Create Duel between players (Single
and Match Mode), choosing targets, tributes, positions, coin calls and chains in the game's own dialogs, attack, summon
and chain-link animations, the result screens, avatar settings.

Not yet: changing the side deck between a match's duels, duel timers, watching duels, effects that make you choose between a
card's own options (the first is taken), packs and an economy. Players' decks stay on their own machines (in `Documents\Yu-Gi-Oh! ONLINE 3`).

## When something goes wrong

- **"There is no response from the server."** Check the address in `yo3.ini`, that the server runs, and that the four
  ports are open (`nc -vz HOST 5730` from the player's machine).
- **Stuck right after logging in, or after the duel's janken:** the duel port 1214 is blocked, or the host needs
  `--advertise`.
- **Both friends get the same players:** they reach the server from the same address (one router, port forwarding).
  Use a VPN, or have them pick different players of the one account.
- The server's log is in the terminal; every packet is also written to `logs/packets-*.jsonl`.
