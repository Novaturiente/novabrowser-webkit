#!/bin/bash
# Memory watchdog (window.rs memory_watch), slow: ~4 min. A background tab holding
# 1.5 GB of WebGL textures is hibernated after 2 hidden minutes; a tab over 4 GB
# RSS + GPU is killed with "out of memory".  usage: tests/memwatch.sh [binary]
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
wait_for '"crashed":true' 90 && ok "4.4 GB tab killed" || fail "4.4 GB tab not killed: $(q)"
stop
finish
