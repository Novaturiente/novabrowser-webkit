#!/bin/bash
# BOLT the staged libwebkitgtk on novarch (needs perf + llvm-bolt, an Intel CPU with LBR).
# Input: a `BOLT=1 run.sh pkg` stage copied to $IN (rsync novahome:wk-build/stage-pkg/usr $IN/).
# Profiles Speedometer 3.1 via ../w9-baseline, writes the optimized lib to $OUT/usr/lib.
# libjavascriptcoregtk is left alone: BOLTed it segfaults on the first JS run
# (executeModuleProgram -> LLInt; 28k relocations BOLT can't analyze), 2026-10-31.
# usage: bolt.sh [runs=3]   IN=/tmp/wkb OUT=/tmp/wkbolt
set -euo pipefail
IN=${IN:-/tmp/wkb} OUT=${OUT:-/tmp/wkbolt} W=${W:-$HOME/.cache/wkbolt}
lib=libwebkitgtk-6.0.so.4.19.5
w9=$(dirname "$(realpath "$0")")/../w9-baseline
readelf -SW "$IN/usr/lib/$lib" | grep '\.rela\.text' >/dev/null || { echo "no .rela.text: build with BOLT=1" >&2; exit 1; }
mkdir -p "$W"
fdata=()
for i in $(seq "${1:-3}"); do
  # -F 20000: one run at 4000 gave BOLT ~100x too few samples
  (cd "$w9" && perf record -q -e cycles:u -j any,u -F 20000 -o "$W/sp$i.data" -- \
    env WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 LD_LIBRARY_PATH="$IN/usr/lib" \
    target/release/w9-baseline --speedometer </dev/null >/dev/null 2>&1)
  perf2bolt "$IN/usr/lib/$lib" -p "$W/sp$i.data" -o "$W/wk$i.fdata" </dev/null | grep -E 'non-empty'
  fdata+=("$W/wk$i.fdata")
done
merge-fdata "${fdata[@]}" > "$W/wk-m.fdata"
rm -rf "$OUT" && mkdir -p "$OUT" && cp -a "$IN/usr" "$OUT/"
llvm-bolt "$IN/usr/lib/$lib" -o "$OUT/usr/lib/$lib" -data="$W/wk-m.fdata" \
  -reorder-blocks=ext-tsp -reorder-functions=cdsort -split-functions -split-all-cold -split-eh \
  -icf=all -use-gnu-stack -dyno-stats </dev/null > "$W/wk-bolt.log" 2>&1
grep -E 'BOLT-ERROR|taken branches \(' "$W/wk-bolt.log" || true
echo "bolted: $OUT/usr/lib/$lib"
