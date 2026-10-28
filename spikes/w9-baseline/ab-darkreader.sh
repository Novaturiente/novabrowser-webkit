#!/bin/bash
# W13: Speedometer 3.1 with and without Dark Reader (world "novabrowser", all frames),
# Safari identity on both, interleaved rounds. Rule (D6): cost <= 3%.
# usage: [VARIANTS="off on css"] ./ab-darkreader.sh [rounds=3]  -> results/ab-darkreader.jsonl
# on = Dark Reader, css = the inverting style sheet fallback
cd "$(dirname "$0")"
out=results/ab-darkreader.jsonl
for r in $(seq "${1:-3}"); do
  for v in ${VARIANTS:-off on}; do
    flag=$(case $v in on) echo --darkreader;; css) echo --cssdark;; esac)
    res=$(target/release/w9-baseline --speedometer --safari $flag 2>/dev/null | grep '^{')
    line="{\"round\":$r,\"darkreader\":\"$v\",\"load\":\"$(cut -d' ' -f1-3 /proc/loadavg)\",\"out\":${res:-null}}"
    echo "$line" | tee -a "$out"
    sleep 3
  done
done
