#!/usr/bin/env python3
"""Read YO3 Revival's logs as one timeline: session logs, server logs, packet logs and report files, in yo3-log/1
(docs/logs.md) or the older text format, put in time order and filtered.

    tools/logs/yo3log.py PATH...                      files, folders (searched) or report ZIPs, readable, oldest first
    tools/logs/yo3log.py PATH --level warn            warnings and errors only (error, warn, info, debug, trace)
    tools/logs/yo3log.py PATH --src dll,wine          only these sources (app, helper, server, client, dll, wine,
                                                      engine, packet); traces (client, engine, packet) need --traces
    tools/logs/yo3log.py PATH --user 1001             one account's lines (also --player, --duel, --report, --session)
    tools/logs/yo3log.py PATH --since 10:00 --until 10:30    a time window (HH:MM[:SS] on the logs' first day, or ISO)
    tools/logs/yo3log.py PATH --grep "page fault"     lines whose message or fields match (a regular expression)
    tools/logs/yo3log.py PATH --json | jq .           yo3-log/1 lines (older logs converted), for other tools

Times are UTC. Nothing is written: it only reads.
"""
import argparse
import datetime
import io
import json
import re
import sys
import zipfile
from pathlib import Path

LEVELS = ["trace", "debug", "info", "warn", "error"]
TRACES = {"client", "engine", "packet"}
FIRST = ["ts", "level", "src", "session", "msg"]
SESSION_NAME = re.compile(r"^((?:app|server|game|direct)-\d+-\d+-[0-9a-f]+)\.\d{4}\.log$")
SERVER_TEXT = re.compile(r"^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?)Z\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+(.*)$")
WINE = re.compile(r"^[0-9a-f]{4}:(fixme|err|warn|trace):")
WORD = {"ERROR": "error", "WARN": "warn", "INFO": "info", "DEBUG": "debug", "TRACE": "trace"}


def iso(seconds, millis=0):
    return datetime.datetime.fromtimestamp(seconds, datetime.UTC).strftime("%Y-%m-%dT%H:%M:%S") + f".{millis:03d}Z"


def classify(text, default):
    """An old text line's source, level, own time (a server's text) and message (as the launcher classifies them)."""
    for marker, src in (("[YO3 MOD] ", "dll"), ("[YO3 CLIENT] ", "client"), ("[YO3 PACKET] ", "packet"), ("[YO3 ENGINE] ", "engine")):
        if text.startswith(marker):
            rest = text[len(marker):]
            level = word_level(rest) if src == "dll" else "debug"
            return src, level, None, rest
    m = WINE.match(text)
    if m:
        return "wine", {"err": "error", "warn": "warn"}.get(m.group(1), "debug"), None, text
    for prefix, level in (("info:  ", "debug"), ("warn:  ", "warn"), ("err:   ", "error")):  # DXVK, Direct3D under Wine
        if text.startswith(prefix):
            return "wine", level, None, text
    m = SERVER_TEXT.match(text)
    if m:
        return "server", WORD[m.group(2)], m.group(1)[:23] + "Z", m.group(3)
    return default, word_level(text), None, text


def word_level(text):
    for word in text.split()[:4]:
        if word in WORD:
            return WORD[word]
    return "info"


def own_src(session):
    kind = session.split("-", 1)[0] if session else ""
    return {"app": "app", "server": "server"}.get(kind, "helper" if kind in ("game", "direct") else "server")


def parse(line, session):
    """One line as a yo3-log/1 dict, or None for an empty or unreadable line."""
    line = line.rstrip("\r\n")
    if not line.strip():
        return None
    if line.startswith("{"):
        try:
            d = json.loads(line)
        except ValueError:
            d = None
        if isinstance(d, dict):
            if "ts" in d and "msg" in d:
                d.setdefault("session", session or "")
                return d
            if "t" in d and "dir" in d:  # a packet log line from before yo3-log/1
                m = d.get("msg") if isinstance(d.get("msg"), dict) else {}
                rid = (m.get("fields") or {}).get("request")
                t = datetime.datetime.fromisoformat(d["t"]).astimezone(datetime.UTC)
                out = {"ts": t.strftime("%Y-%m-%dT%H:%M:%S.") + f"{t.microsecond // 1000:03d}Z", "level": "warn" if d.get("error") else "debug",
                       "src": "packet", "session": session or "", "msg": f"{d['dir']} {hex(rid) if isinstance(rid, int) else m.get('version', '?')}",
                       "conn": d.get("server"), "dir": d["dir"], "packet": m}
                if isinstance(rid, int):
                    out["req"] = f"0x{rid:04x}"
                if d.get("error"):
                    out["error"] = d["error"]
                return out
    head, _, rest = line.partition(" ")
    if head.isdigit() and len(head) == 10:  # an older session log line: `UNIX_SECONDS text`
        src, level, ts, msg = classify(rest, own_src(session))
        return {"ts": ts or iso(int(head)), "level": level, "src": src, "session": session or "", "msg": msg}
    src, level, ts, msg = classify(line, own_src(session))  # a server's console text, or anything else
    return {"ts": ts or "", "level": level, "src": src, "session": session or "", "msg": msg}


MARKERS = {"dll": "[YO3 MOD] ", "client": "[YO3 CLIENT] ", "packet": "[YO3 PACKET] ", "engine": "[YO3 ENGINE] "}


def text(line):
    """A log line as the older text form, for tools that look for markers (`[YO3 ENGINE] {...}`, `ERROR`): a yo3-log/1
    line becomes `LEVEL <marker of its src>message`, any other line stays as it is."""
    if not line.startswith("{"):
        return line
    try:
        d = json.loads(line)
    except ValueError:
        return line
    if not isinstance(d, dict) or "msg" not in d or "ts" not in d:
        return line
    return f"{d['ts']}  {str(d.get('level', 'info')).upper()} {MARKERS.get(d.get('src'), '')}{d['msg']}"


def session_of(name):
    m = SESSION_NAME.match(Path(name).name)
    return m.group(1) if m else ""


def is_log(name):
    n = Path(name).name
    return bool(SESSION_NAME.match(n)) or n.endswith(".jsonl") or n.endswith(".log")


def sources(paths):
    """(name, text stream) for every log file in `paths`: files, folders (searched) and report ZIPs."""
    for path in map(Path, paths):
        if path.is_dir():
            for f in sorted(p for p in path.rglob("*") if p.is_file()):
                if f.suffix == ".zip":
                    yield from sources([f])
                elif is_log(f.name):
                    yield f.name, f.open(encoding="utf-8", errors="replace")
        elif path.suffix == ".zip":
            with zipfile.ZipFile(path) as z:
                for name in z.namelist():
                    if is_log(name):
                        yield name, io.TextIOWrapper(z.open(name), encoding="utf-8", errors="replace")
        elif path.exists():
            yield path.name, path.open(encoding="utf-8", errors="replace")
        else:
            sys.exit(f"yo3log.py: no such file or folder: {path}")


def when(text, day):
    """--since/--until: `HH:MM[:SS]` on `day` (the logs' first), or a full ISO time; as a yo3-log ts prefix to compare."""
    if re.fullmatch(r"\d\d:\d\d(:\d\d(\.\d{1,3})?)?", text):
        return f"{day}T{text}"
    return text.replace(" ", "T")


def human(d):
    rest = "".join(f"  {k}={v if isinstance(v, str) else json.dumps(v)}" for k, v in d.items() if k not in FIRST and k not in ("packet", "raw", "raw_len"))
    return f"{d.get('ts', '')[:23].replace('T', ' ')} {d.get('level', '').upper():<5} {d.get('src', ''):<6} {d.get('msg', '')}{rest}"


def ordered(d):
    return json.dumps({**{k: d[k] for k in FIRST if k in d}, **{k: v for k, v in d.items() if k not in FIRST}}, ensure_ascii=False)


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("paths", nargs="+")
    a.add_argument("--level", choices=LEVELS, help="this level and above")
    a.add_argument("--src", help="comma-separated sources")
    a.add_argument("--traces", action="store_true", help="include the game's, the engine's and packet traces")
    for key in ("user", "player", "duel", "report", "session"):
        a.add_argument(f"--{key}")
    a.add_argument("--since")
    a.add_argument("--until")
    a.add_argument("--grep", help="a regular expression over the message and fields")
    a.add_argument("--json", action="store_true", help="print yo3-log/1 lines")
    args = a.parse_args(argv)
    lines = []
    for n, (name, stream) in enumerate(sources(args.paths)):
        session = session_of(name)
        with stream:
            for i, line in enumerate(stream):
                d = parse(line, session)
                if d:
                    lines.append((d.get("ts", ""), n, i, d))
    lines.sort(key=lambda x: (x[0], x[1], x[2]))
    srcs = set(args.src.split(",")) if args.src else None
    least = LEVELS.index(args.level) if args.level else 0
    day = next((x[0][:10] for x in lines if x[0]), "1970-01-01")
    since, until = (when(args.since, day) if args.since else None), (when(args.until, day) if args.until else None)
    pattern = re.compile(args.grep, re.I) if args.grep else None
    out = sys.stdout
    for ts, _, _, d in lines:
        src = d.get("src", "")
        if srcs is not None and src not in srcs or srcs is None and src in TRACES and not args.traces:
            continue
        if LEVELS.index(d.get("level", "info")) < least if d.get("level") in LEVELS else False:
            continue
        # --until takes in the whole second (or minute) it names: compared on its own length.
        if since and ts and ts < since or until and ts and ts[:len(until)] > until:
            continue
        if any(getattr(args, k) is not None and str(d.get(k, "")) != getattr(args, k) for k in ("user", "player", "duel", "report", "session")):
            continue
        if pattern and not pattern.search(ordered(d)):
            continue
        out.write((ordered(d) if args.json else human(d)) + "\n")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except BrokenPipeError:
        sys.exit(0)
