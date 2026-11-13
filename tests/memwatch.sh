#!/bin/bash
# Memory watchdog (window.rs memory_watch), slow: ~4 min. A background tab holding
# 1.5 GB of WebGL textures is hibernated after 2 hidden minutes; a 4.4 GB active tab
# survives while the system has memory to spare (kill line 8 GB or low system memory;
# not tested here: it would push the user's own browser into its danger path).
# usage: tests/memwatch.sh [binary]
set -u
cd "$(dirname "$0")/.."
. tests/lib.sh
serve tests/pages
start "http://127.0.0.1:$port/gpu-hog.html?mb=1500"
wait_for '"title":"gpu-hog 1504 MB"' 20 && ok "hog tab holds 1.5 GB" || fail "hog tab did not load: $(q)"
"$bin" --cmd "open -t $(page other)" >/dev/null
wait_for '"active":1' 10 || fail "second tab not active"
wait_for '"asleep":true' 200 && ok "heavy hidden tab hibernated" || fail "heavy tab still awake: $(q)"
"$bin" --cmd "open http://127.0.0.1:$port/gpu-hog.html?mb=4400" >/dev/null
wait_for '"title":"gpu-hog 44[0-9]{2} MB"' 30 || fail "4.4 GB tab did not load: $(q)"
wait_for '"crashed":true' 90 && fail "4.4 GB tab killed on a healthy system: $(q)" || ok "4.4 GB tab survives"
stop
finish
