#!/bin/bash
# Start build.sh <stage> detached on novahome, capped at 20 GB RAM (user's limit: other
# services there must keep running). Over the cap only the build's cgroup is OOM-killed.
# usage: [MEM_MAX=20G MEM_HIGH=18G JOBS=8] run.sh gen|use|pkg -> log in ~/wk-build/<stage>.log,
# last line EXIT=<code>. Lower MEM_MAX below what `free` shows as available so the build never
# pushes the other services into memory pressure.
stage=$1
if [ "${2:-}" != --inner ]; then
  # tmux runs commands with the login shell (fish on novahome): hand over to bash
  exec tmux new-session -d -s "wk$stage" "MEM_MAX=${MEM_MAX:-20G} MEM_HIGH=${MEM_HIGH:-18G} JOBS=${JOBS:-8} bash $0 $stage --inner"
fi
log=$HOME/wk-build/$stage.log
# CPUWeight=idle: ananicy-cpp on novahome renices clang++ to -4, so per-process nice can't keep
# the build behind the services; an idle cgroup only gets CPU nothing else wants.
systemd-run --user --scope --quiet -p MemoryHigh="$MEM_HIGH" -p MemoryMax="$MEM_MAX" -p MemorySwapMax=0 \
  -p CPUWeight=idle -p IOWeight=1 \
  env JOBS="$JOBS" nice -n 19 ionice -c3 "$HOME/wk-build/build.sh" "$stage" > "$log" 2>&1
echo "EXIT=$?" >> "$log"
