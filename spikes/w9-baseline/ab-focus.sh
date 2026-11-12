#!/bin/bash
# FOCUS_JS cost (src/web.rs): Speedometer 3.1 with no focus script, the script before the
# 2026-11-12 pointerdown listener, and the current one; Safari identity, interleaved rounds.
# usage: [VARIANTS="off old new"] ./ab-focus.sh [rounds=3]  -> results/ab-focus.jsonl
cd "$(dirname "$0")"
out=results/ab-focus.jsonl
for r in $(seq "${1:-3}"); do
  for v in ${VARIANTS:-off old new}; do
    flag=$(case $v in old) echo --focus-old;; new) echo --focus;; esac)
    res=$(target/release/w9-baseline --speedometer --safari $flag 2>/dev/null | grep '^{')
    line="{\"round\":$r,\"focus\":\"$v\",\"load\":\"$(cut -d' ' -f1-3 /proc/loadavg)\",\"out\":${res:-null}}"
    echo "$line" | tee -a "$out"
    sleep 3
  done
done
