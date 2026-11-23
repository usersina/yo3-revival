#!/usr/bin/env python3
"""Prove native checkpoint coverage using matching server and game session logs (including rotated parts)."""
import argparse
from collections import Counter
import json
from pathlib import Path
import re

def records(lines, marker):
    result = []
    for line in lines:
        if marker in line:
            result.append(json.JSONDecoder().raw_decode(line.split(marker, 1)[1])[0])
    return result

def key(record):
    return record["schema"], record["sequence"], tuple(record["expected"])

def compare(expected, observed):
    problems = []
    if not expected:
        problems.append("No server checkpoints: native validation was not exercised.")
    for r in observed:
        if r.get("status") != "pass" or r.get("actual") != r.get("expected") or r.get("schema") != 1:
            problems.append(f'Native checkpoint {r.get("sequence")}: {r.get("status")} (expected {r.get("expected")}, actual {r.get("actual")}).')
    want, got = Counter(map(key, expected)), Counter(map(key, observed))
    if any(count != 1 for count in want.values()):
        problems.append("Duplicate server checkpoints: select matching session parts only.")
    if want - got:
        problems.append(f"Missing {sum((want-got).values())} native acknowledgements: incomplete validation.")
    if got - want:
        problems.append(f"Unexpected/duplicate {sum((got-want).values())} acknowledgements: check the session log selection.")
    return problems

def coverage_problems(lines):
    gaps = sorted({line.split("[YO3 GAP] ", 1)[1] for line in lines if "[YO3 GAP] " in line})
    return [f"Incomplete mechanic coverage: {gap}" for gap in gaps]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", nargs="+", type=Path, required=True, help="matching server session .log parts")
    parser.add_argument("--client", nargs="+", type=Path, required=True, help="both games' session .log parts")
    args = parser.parse_args()
    server = [line for p in args.server for line in p.read_text(errors="replace").splitlines()]
    client = [line for p in args.client for line in p.read_text(errors="replace").splitlines()]
    expected, observed = records(server, "[YO3 EXPECT] "), records(client, "[YO3 CHECK] ")
    problems = compare(expected, observed) + coverage_problems(server)
    if any(re.search(r"ERROR.*duel check", line) for line in server):
        problems.append("The server reported a duel-check or recording error; inspect its ERROR lines.")
    if any(re.search(r"page fault|unhandled exception|panicked at", line, re.I) for line in server + client):
        problems.append("A crash was recorded.")
    for problem in problems: print("FAIL:", problem)
    if not problems: print(f"PASS: {len(observed)} native checkpoints match the model; no engine/model errors in the supplied server logs.")
    return int(bool(problems))

if __name__ == "__main__": raise SystemExit(main())
