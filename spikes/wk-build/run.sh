#!/bin/bash
# Start build.sh <stage> detached on novahome, capped at 20 GB RAM (user's limit: other
# services there must keep running). Over the cap only the build's cgroup is OOM-killed.
# usage: run.sh gen|use   -> log in ~/wk-build/<stage>.log, last line EXIT=<code>
stage=$1
if [ "${2:-}" != --inner ]; then
  # tmux runs commands with the login shell (fish on novahome): hand over to bash
  exec tmux new-session -d -s "wk$stage" "bash $0 $stage --inner"
fi
log=$HOME/wk-build/$stage.log
systemd-run --user --scope --quiet -p MemoryHigh=18G -p MemoryMax=20G -p MemorySwapMax=0 \
  nice -n 19 ionice -c3 "$HOME/wk-build/build.sh" "$stage" > "$log" 2>&1
echo "EXIT=$?" >> "$log"
