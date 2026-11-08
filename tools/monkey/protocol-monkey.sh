#!/usr/bin/env bash
# The protocol monkey (crates/yo3-monkey): builds the server and the monkey in release mode, starts the server
# on a scratch data dir (never the repo's data/) and sends it mutated, replayed, random and garbage traffic.
#
#   tools/monkey/protocol-monkey.sh fuzz --seed 1 --plans 2000 --par 8   plans one batch after another
#   tools/monkey/protocol-monkey.sh duel --seed 1 --pairs 10             two clients, Auto Duel, random duel commands
#   tools/monkey/protocol-monkey.sh soak --secs 300 --conns 24 --duel-pairs 3
#   tools/monkey/protocol-monkey.sh repro FILE | minimize FILE           a saved failing plan
#
# SCRATCH (default /tmp/yo3-monkey) holds server.log, the packet log, data/, stats.csv and failures/ (each failure:
# info.txt, the server log's tail and the plans sent around it as plan-*.txt, which `repro` replays). The corpus is
# the packet logs in logs/ (the real client's requests). Seeds reproduce a run only while the corpus is the same;
# the saved plan files always do. Exit status 1 when anything failed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
SCRATCH="${SCRATCH:-/tmp/yo3-monkey}"
for p in 5730 1213 1214 8080; do
  if ss -ltn "sport = :$p" | grep -q LISTEN; then echo "port $p is in use: stop the server first" >&2; exit 2; fi
done
cargo build --release -p yo3-server -p yo3-monkey 2>&1 | tail -1
exec target/release/yo3-monkey --scratch "$SCRATCH" "$@"
