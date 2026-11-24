# Problem report file

Desktop's **Report a problem** saves the whole report as one ZIP on the player's PC and sends nothing, so a report never
depends on a server. Players send that file to the developer as it is: on Discord, privately, or attached to a GitHub
issue. This page defines what is in it (format `yo3-revival-report`, version 3), for whoever reads one.

The file is `yo3-revival-report-YYYYMMDD-HHMMSS.zip`, in the player's **Downloads** folder (Windows: the user's
`Downloads`; Linux: XDG's download folder, else `~/Downloads`), or in the app's own `reports/` folder when there is
none. The result window shows its name, with **Open folder** and **Copy path**. Code: `crates/yo3-desktop/src/reports.rs`
(`save_report`, `export_full`), tested by `a_report_is_always_saved_as_one_file_in_the_documented_format`.

## What is inside

| File | What it holds |
| --- | --- |
| `README.txt` | The report for a person: what happened, how to reach the player, the error, Singleplayer or which server, the app's version and commit, the PC (system, processor and threads, memory, graphics card, Wine), the IDs of reports the game sent hosts by itself, and the recordings included. It tells the player to send the whole ZIP. |
| `report.json` | The same, for tools (below). |
| `bug-report.md` | The report as the GitHub form's Markdown, for pasting into an issue. |
| `sessions/` | This app session's log and the latest game's (the helper's, with the game's and the connection DLL's lines), and their metadata files. |
| `recordings/` | Singleplayer only: the newest three duels pinned with the `bug` chat command (`<id>.jsonl.gz`, up to 8 MB each), replayed exactly with `tools/monkey/duel-replay.py --recording FILE`. |

## `report.json`

```json
{
  "format": "yo3-revival-report",
  "version": 3,
  "created": 1795300000,
  "release": "v0.1.22", "commit": "…", "protocol": 3, "mod_abi": 3,
  "os": "windows", "arch": "x86_64",
  "system": { "os": "Windows 10 Home 22H2 (build 19045.6466)", "arch": "x86_64", "wine": null,
              "cpu": "AMD Ryzen 5 3600 6-Core Processor", "threads": 12, "memory_mb": 16334, "gpu": "NVIDIA GeForce GTX 1660" },
  "context": { "error": "…" },
  "report": {
    "what": "Black Brutdrago closed my game",
    "contact": "Discord: duelist",
    "error": "",
    "play": "singleplayer",
    "settings": { "resolution": "auto", "direct_start": false, "banlist": "", "problem_reports": true, "card_pictures": true,
                  "auto_updates": false, "game": "<the game's fingerprint>", "wine_prefix_set": false, "detailed_logs": false },
    "sent": [ { "to": "the server of a game that broke", "id": "R-7K3Q-M2PA" } ],
    "recordings": [ "recordings/20261124-101500-001.jsonl.gz" ]
  },
  "preview": { "files": 4, "bytes": 51234, "oldest": 1795290000, "newest": 1795300000 },
  "personal_data_filtered": true,
  "note": "…"
}
```

- `report.play` is `singleplayer` or the name of the server the player was signed in to on Multiplayer.
- `report.settings` holds only what explains a problem: no account, User ID, key, address or path. `game` is the
  setup check's fingerprint of `yo3.exe`, not its location.
- `report.sent` lists the IDs of reports the helper sent hosts by itself after recent games that broke (operation 7),
  for reference. Report a problem itself sends nothing: the file is the report.
- Version 2 (before) had the same top-level fields without `report`, `README.txt`, the PC's processor, memory and
  graphics card, and recordings; version 1 had no system details.

## Privacy

Passwords, keys and tokens are never written to the logs, and are removed again when the file is made. Personal details
are removed from the logs and the README (the user's folder in paths, network addresses); player names, chat and
decks can remain in the game's log. Nothing in the file is uploaded by itself: the player decides where to send it. See
[rules and privacy](rules-privacy.md).
