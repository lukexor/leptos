#!/usr/bin/env bash
# Usage: stress.sh <path> <requests> <parallel>. Restarts the server, loads
# <path> in parallel, and reports whether the server survived.
set -uo pipefail
cd "$(dirname "$0")" || exit
[ -f server.pid ] && kill "$(cat server.pid)" 2>/dev/null; sleep 0.3
./run-server.sh > server.pid
pid=$(cat server.pid)
for _ in $(seq 1 50); do curl -sf -o /dev/null localhost:3000/ && break; sleep 0.1; done
seq 1 "$2" | xargs -P"$3" -I{} curl -s -o /dev/null -w '%{http_code}\n' "localhost:3000$1" | sort | uniq -c
if [ -d /proc/"$pid" ] && ! grep -q panicked server.log; then echo "server ALIVE, no panic"; else echo "server DEAD or panicked"; grep -m1 -B2 -A25 panicked server.log; fi
