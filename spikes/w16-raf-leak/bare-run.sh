#!/bin/sh
# Bare WebKit view on logged-in /learning + RSS samples for 180 s, into bare.log.
d=$(dirname "$(readlink -f "$0")")
log=$d/bare.log
/home/nova/personal/novabrowser-webkit/target/release/w16-bare "$d/run-bare/cookies.sqlite" https://learning.eecglobal.com/learning >"$d/bare.stderr" 2>&1 &
bp=$!
sleep 8
echo "title: $(hyprctl clients -j | jq -r ".[] | select(.pid==$bp) | .title")" >"$log"
sh "$d/sample.sh" "$bp" 180 >>"$log" 2>&1
echo "browser alive: $(kill -0 $bp 2>/dev/null && echo yes || echo no)" >>"$log"
kill "$bp" 2>/dev/null
