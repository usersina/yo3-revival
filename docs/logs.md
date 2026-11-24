# Log format (yo3-log/1)

Every log file YO3 Revival writes is JSON Lines in one schema, **yo3-log/1**: the launcher's and the helper's session
logs (with the game's, the connection DLL's and Wine's lines in them), the server's session log and its packet log. One
object per line, so `jq`, [lnav](https://lnav.org) and `tools/logs/yo3log.py` read them all, and lines from different
programs can be put in one timeline. The names follow the OpenTelemetry log data model (timestamp, severity, body,
attributes), flattened for `jq`. Consoles stay readable text: a host's terminal shows the same events as before.

Code: `crates/yo3-tunnel/src/diagnostics.rs` (every session log), the server's JSON layer in
`crates/yo3-server/src/log.rs`; the viewer `tools/logs/yo3log.py`.

## A line

```json
{"ts":"2026-11-24T08:12:37.123Z","level":"info","src":"server","session":"server-1795504323-808643-033499225f7f","msg":"read the card profile anime fbb63cc4552f0c3c","topic":"tls","peer":"127.0.0.1:57390","user":1001}
```

| Key | Always | What it is |
| --- | --- | --- |
| `ts` | yes | When, in UTC, RFC 3339 with milliseconds (`2026-11-24T08:12:37.123Z`). |
| `level` | yes | `error`, `warn`, `info`, `debug` or `trace`. |
| `src` | yes | Who wrote it: `app` (the Desktop window), `helper` (the connection helper `yo3-launcher` that runs a game, and the game's output without a marker), `server`, `client` (the game's network trace, `[YO3 CLIENT]`), `dll` (the connection DLL, `[YO3 MOD]`), `wine` (Wine's and DXVK's lines), `engine` (the rules engine's traffic), `packet` (a game packet). |
| `session` | yes | The session log's id (`app-…`, `game-…`, `direct-…`, `server-…`): its metadata is `<session>.json` beside it, with the build, the parent session and the process. |
| `msg` | yes | The text, credentials already removed. |

Other keys are the event's fields. These have a fixed meaning; any other key is allowed:

| Key | What it is |
| --- | --- |
| `topic` | The area a server line is about (`tls`, `seat`, `report`, `panel`, `http`...). |
| `user` | An account number (User ID's number, `user_id`). |
| `player` | A player's id. |
| `peer` | The other end of a connection, `address:port`. |
| `duel` | A duel's id (a match) or its recording's id. |
| `report` | A problem report's id (`R-XXXX-XXXX`) or a player report's number. |
| `req` | A game request or reply id, in hex (`0x2398`), as the docs write them. |
| `conn` | The game connection: `gate`, `front` or `duel`. |
| `dir` | A packet's direction: `<-` from the game, `->` to it. |
| `error` | An error's text, when the line is about one. |

## Packets

The packet log (`packets-*.jsonl`, `--log-dir`) uses the same envelope with `src` `packet`, `conn`, `dir` and `req`, and
the message as `packet`: its fields with their real types (text as a string, numbers as numbers, binary as
`"bin:<hex>"`), its items as an array of the same. `raw` keeps the frame's bytes in hex.

## Older logs

Logs written before yo3-log/1 are text: each line `UNIX_SECONDS text` (session logs; a server's text has its own ISO time
after it), and packet logs with `t`, `server`, `dir` and `msg`. `tools/logs/yo3log.py` reads both and shows them in the
same form, and a report file (format 4) can hold either. Nothing needs converting.

## Reading logs

```bash
python3 tools/logs/yo3log.py ~/.local/share/yo3-revival/logs/sessions          # one timeline of every session, readable
python3 tools/logs/yo3log.py REPORT.zip --level warn                             # a report file's warnings and errors
python3 tools/logs/yo3log.py server.log --user 1001 --since 10:00                # one account's lines from 10:00 (UTC)
python3 tools/logs/yo3log.py logs/ --duel 20261124-101500-001 --json | jq .      # one duel's lines, as yo3-log/1
python3 tools/logs/yo3log.py logs/ --src dll,wine --grep "page fault"            # the game side of a crash
```

`--json` prints yo3-log/1 lines (old logs converted), for `jq`. For lnav, install `tools/logs/yo3-log.lnav.json`
(`lnav -i tools/logs/yo3-log.lnav.json`) and open the files.
