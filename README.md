# yo3-revival

A private server for **Yu-Gi-Oh! ONLINE 3 (Duel Accelerator)**, Konami's online card game that shut down in
September 2012, so we can play it with friends again. The server is reverse-engineered from the game client and
written in Rust, test-first.

**Where it stands:** the real client logs in, enters the world and sits on a stable main screen. Chat, lobbies,
collection and duels are ahead. See [ROADMAP.md](ROADMAP.md).

## Quick start

You need the game installed under Wine (or on Windows) and a Rust toolchain.

1. Point the game at your server. In the game folder's `yo3.ini`:

   ```ini
   GateServerAddress=127.0.0.1
   GateServerPort=5730
   PatchUpdateUrl=http://127.0.0.1:8080/yo3/
   PatchDataUrl=http://127.0.0.1:8080/ygoda/patch/
   InfoURL=http://127.0.0.1:8080/app/
   ```

2. Run the server:

   ```bash
   cargo run --release -p yo3-server -- --log-dir logs
   ```

   Useful options: `--advertise <ip>` (the address friends should reach), `--bind`, `--game-dir`. The server reads
   banlists, preset decks and packs from the game's own files.

3. Start the game and click ONLINE. To run it headless instead, on a private virtual display:

   ```bash
   tools/client/vclient.sh xvfb && tools/client/vclient.sh start && tools/client/vclient.sh login
   ```

## Development

```bash
cargo nextest run --workspace     # or: cargo test --workspace
cargo clippy --workspace --all-targets
```

| Read | For |
|---|---|
| [CLAUDE.md](CLAUDE.md) | How to work on this project: rules, commands, the reverse-engineering loop, gotchas |
| [ROADMAP.md](ROADMAP.md) | Milestones, open questions and guesses |
| [docs/protocol.md](docs/protocol.md) | Every request/reply pair, with client evidence and status |
| [docs/client-internals.md](docs/client-internals.md) | Client addresses, data formats, crash history |
| [docs/research.md](docs/research.md) | The original feasibility study |

## Legal

A non-commercial fan preservation project, not affiliated with Konami. This repository contains only original code
and notes. No game files, card data, art or decompiled code are distributed; you need your own copy of the game.
