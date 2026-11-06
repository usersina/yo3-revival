# yo3-revival: working guide

A private server for the retired Konami PC game Yu-Gi-Oh! ONLINE 3 (Duel Accelerator), so the user can play with
friends. The server is reverse-engineered from the client: **the client binary is the spec.**

Progress and next steps: [ROADMAP.md](ROADMAP.md). Protocol facts: [docs/protocol.md](docs/protocol.md). Client
addresses and data formats: [docs/client-internals.md](docs/client-internals.md).

## Ground rules

1. **Never touch the user's screen or mouse.** Run the game only on the private Xvfb display through
   `tools/client/vclient.sh` (display `:77`). The user asked for this explicitly.
2. **Evidence, not invention.** Every reply shape cites client evidence (an address, a `File.cpp Lnnn` trace line,
   or a logged packet) in `docs/protocol.md` with a status: confirmed / shape / guess / open. Send only fields the
   client reads. Label guesses and add them to the guesses table in ROADMAP.md.
3. **Test first.** New client requirement → failing contract test → implementation → green → live check. Keep
   `cargo nextest run --workspace` green and `cargo clippy --workspace --all-targets` clean before every commit.
4. **Keep four places in step:** `docs/protocol.md`, `crates/yo3-server/src/front.rs`,
   `crates/yo3-server/tests/front_contract.rs` and `ROADMAP.md`.
5. **Small, logical commits** (conventional style: `feat(server): …`, `fix(…)`, `docs: …`, `refactor(…)`,
   `chore: …`) on a feature branch per slice of work. Ask before merging into `main` or pushing.
6. **Code only in git.** Never commit game files, Konami data, decompiled code or packet logs. `re/`, `logs/` and
   `target/` are ignored. Docs may cite addresses and field names, but not paste decompiled code.

## Repo map

| Path | What |
|---|---|
| `crates/yo3-proto` | Wire codec: frame, Blowfish (`BlowfishLE`), key/value messages, the version-string opener |
| `crates/yo3-data` | Client data: `.ypk` archives, card table, regulations, preset decks, packs (golden tests skip without the game) |
| `crates/yo3-server` | Gate / front / patch-HTTP servers. `front.rs` = the reply contract, `net.rs` = framing, `log.rs` = packet log |
| `tools/client/vclient.sh` | Headless client: `xvfb`, `start`, `login`, `check`, `shot`, `click`, `pixel`, `key`, `type`, `stop` |
| `tools/re/` | Python reverse-engineering helpers (see Commands) |
| `tools/ghidra/` | Headless export script |
| `docs/` | Protocol catalog, client internals, original research |
| `re/` (ignored) | Ghidra project `re/ghidra`, `re/yo3_decompiled.c`, GhidraMCP jar |
| `logs/` (ignored) | `packets-*.jsonl`, `client-stdout.log` (client trace), `client-wine.log`, `shots/` |

## Commands

```bash
cargo nextest run --workspace                                   # all tests
cargo clippy --workspace --all-targets                          # lints
cargo run --release -p yo3-server -- --log-dir logs             # server (run in the background)
ss -ltnp | grep ':5730 '                                        # find the server's pid to stop it

tools/client/vclient.sh xvfb                                    # once per boot
tools/client/vclient.sh start && tools/client/vclient.sh login  # cold start to main screen, about 30 s
tools/client/vclient.sh check                                   # process, client trace, crash, last packets
tools/client/vclient.sh shot NAME                               # logs/shots/NAME.png, then view it
tools/client/vclient.sh stop                                    # also clears a crashed client

python3 tools/re/dispatch.py 0x4f4a4f 0x2398    # route a reply id through the global dispatcher; fields it reads
python3 tools/re/rtti.py ClassName              # vtable of a class (MSVC RTTI)
python3 tools/re/emp.py FILE.y3d                # decode a client data file
grep -n '0x2775' re/yo3_decompiled.c            # find a request id in the decompiled client
```

Ghidra export (stop GhidraMCP first, because Ghidra locks the project):
`/opt/ghidra/support/analyzeHeadless re/ghidra yo3 -import "<game dir>/yo3.exe" -overwrite -scriptPath tools/ghidra -postScript ExportDecompiled.java "$PWD/re/yo3_decompiled.c"`

GhidraMCP tools (`mcp__ghidra__*`): `decompile_function_by_address`, `get_xrefs_to`, `disassemble_function`,
`list_strings`, `rename_function_by_address`, `set_decompiler_comment`. Name what we understand, so the project keeps
the knowledge.

## The loop: answering a new request

1. **Observe.** Start the server and run `vclient.sh start && vclient.sh login && vclient.sh check`. Look for:
   - an unanswered request (the last `<-` in `check`, or `UNHANDLED` in the server output)
   - a "There is no response from the server. (N)" dialog (N is the request id in decimal)
   - a Wine "page fault … at address X" in `logs/client-wine.log`
2. **Locate.** Find the request's builder and the reply ids its screen compares (grep the id in
   `re/yo3_decompiled.c`, or use GhidraMCP xrefs). Read which fields the handler reads.
3. **Check the global dispatcher.** Run `tools/re/dispatch.py 0x4f4a4f <reply id>`. Every reply also passes through
   it, and unchecked reads there have crashed us before.
4. **Record** the pair in `docs/protocol.md` with its evidence and status.
5. **Red:** add a contract test in `front_contract.rs` naming the client address it protects.
6. **Green:** implement in `front.rs` with the minimum the client reads.
7. **Live:** restart the server, rerun the client, confirm the client's own trace line (e.g. `… recv success`) and
   take a screenshot.
8. **Commit** test, code and docs together; tick ROADMAP.md.

## Conventions

- Build messages with `Message::reply(0x2398).with("field", Value::…).item(Message::record()…)`. Request and reply
  ids are hex literals.
- Binary blobs are `Value::Bin`. Text (`Value::Text`) is read by the client up to the first NUL.
- Times sent to the client are JST (`jst_now()` in `front.rs`).
- Lists the client indexes (`item[0]`) need at least one record, or the client crashes.
- Game content (cards, banlists, decks, packs) comes from the client's files through `yo3-data`, never typed by
  hand. Decks and packs use CARD_Prop rows; regulations use Konami card ids.
- Unknown meaning: name the arm by its id, comment "meaning unknown", and list it as a guess.
- Python is only for `tools/re/`. Run it with system `python3` (capstone, pefile and pycryptodome are installed).

## Environment gotchas

- The user's shell aliases `ls` → eza and `cat` → bat. Use `command ls` / `command cat` in scripts.
- `pgrep -f` / `pkill -f` match the calling shell's own command line. Find processes by port (`ss -ltnp`) instead.
- Ports: gate 5730, front 1213, HTTP 8080. The game's `yo3.ini` points at 127.0.0.1; Konami's original is saved
  next to it as `yo3.ini.konami`.
- Game install: `~/.wine/drive_c/Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3` (override with `YO3_GAME_DIR`).
- A crashed client stays frozen in winedbg. Run `vclient.sh stop` before relaunching.
- Software rendering under Xvfb: boot takes about 20 s, and there's a "Click the screen" logo between the splash and
  the menu (`login` handles both).
- Ghidra 12.1.2 lives at `/opt/ghidra`. GhidraMCP 2.0 runs headless (`--no-analyze`) on `re/ghidra`, registered in
  Claude Code's local config. Only one Ghidra process can hold the project at a time.
- Ghidra's addresses equal the exe's virtual addresses (image base `0x400000`).
