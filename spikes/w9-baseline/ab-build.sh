#!/bin/bash
# W9: Speedometer 3.1, Arch webkitgtk-6.0 vs the novahome PGO build, interleaved rounds.
# Custom builds run from a prefix without the bubblewrap sandbox, so "sys-nosb" is the fair baseline.
# usage: ./ab-build.sh [rounds=3]  -> results/ab-build.jsonl
cd "$(dirname "$0")"
out=results/ab-build.jsonl
NOSB=WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
variants=(
  "sys:"
  "sys-nosb:$NOSB"
  "pgo-nosb:$NOSB LD_LIBRARY_PATH=$HOME/.local/opt/wk-pgo/lib"
)
for r in $(seq "${1:-3}"); do
  for v in "${variants[@]}"; do
    name=${v%%:*}; envs=${v#*:}
    res=$(env $envs target/release/w9-baseline --speedometer 2>/dev/null | grep '^{')
    echo "{\"round\":$r,\"variant\":\"$name\",\"load\":\"$(cut -d' ' -f1-3 /proc/loadavg)\",\"out\":${res:-null}}" | tee -a "$out"
    sleep 3
  done
done
