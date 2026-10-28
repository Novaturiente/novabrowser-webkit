#!/bin/bash
# W9 A/B of WebKitGTK runtime env knobs on Speedometer 3.1, interleaved rounds.
# usage: ./ab.sh [rounds=3]  -> results/ab-env.jsonl
cd "$(dirname "$0")"
out=results/ab-env.jsonl
variants=(
  "base:"
  "gpu4:WEBKIT_SKIA_GPU_PAINTING_THREADS=4"
  "cpu8:WEBKIT_SKIA_ENABLE_CPU_RENDERING=1 WEBKIT_SKIA_CPU_PAINTING_THREADS=8"
  "nodmabuf:WEBKIT_DISABLE_DMABUF_RENDERER=1"
  "nocomp:WEBKIT_DISABLE_COMPOSITING_MODE=1"
  "vblank:WEBKIT_FORCE_VBLANK_TIMER=1"
)
for r in $(seq "${1:-3}"); do
  for v in "${variants[@]}"; do
    name=${v%%:*}; envs=${v#*:}
    res=$(env $envs target/release/w9-baseline --speedometer 2>/dev/null | grep '^{')
    line="{\"round\":$r,\"variant\":\"$name\",\"env\":\"$envs\",\"load\":\"$(cut -d' ' -f1-3 /proc/loadavg)\",\"out\":${res:-null}}"
    echo "$line" | tee -a "$out"
    sleep 3
  done
done
