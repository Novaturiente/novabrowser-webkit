#!/bin/bash
# W9: Speedometer 3.1, installed WebKitGTK (sandbox on) vs installed CEF novabrowser
# (bare profile, ../../../novabrowser/spikes/s9-baseline kind novabare), interleaved rounds.
# usage: ./ab-cef.sh [rounds=4]  -> results/ab-cef.jsonl
cd "$(dirname "$0")"
here=$PWD
s9=$here/../../../novabrowser/spikes/s9-baseline
out=$here/results/ab-cef.jsonl
for r in $(seq "${1:-4}"); do
  res=$(target/release/w9-baseline --speedometer 2>/dev/null | grep '^{')
  echo "{\"round\":$r,\"variant\":\"webkit\",\"pkg\":\"$(pacman -Qqo /usr/lib/libwebkitgtk-6.0.so.4)\",\"load\":\"$(cut -d' ' -f1-3 /proc/loadavg)\",\"out\":${res:-null}}" | tee -a "$out"
  sleep 3
  load=$(cut -d' ' -f1-3 /proc/loadavg)
  res=$(cd "$s9" && node speedometer.mjs 1 novabare w9-cef-tmp 2>&1 | grep '^0 novabare' | sed 's/^0 novabare //')
  rm -f "$s9/results/w9-cef-tmp.json"   # keep the CEF repo clean
  echo "{\"round\":$r,\"variant\":\"cef\",\"pkg\":\"$(pacman -Q novabrowser-bin)\",\"load\":\"$load\",\"out\":${res:-null}}" | tee -a "$out"
  sleep 3
done
