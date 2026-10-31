#!/bin/sh
# usage: [ENV=..] scroll-run.sh <name> <url> <seconds>
# Test profile (/tmp/w16l, logged-in cookie copy) on the W16 virtual monitor (workspace 3);
# scrolls with IPC keys (10x j then 10x k, every 0.5 s) and logs web-process RSS every 5 s.
d=$(dirname "$(readlink -f "$0")")
name=$1; url=$2; secs=$3
export XDG_DATA_HOME=/tmp/w16l/data XDG_CONFIG_HOME=/tmp/w16l/config XDG_STATE_HOME=/tmp/w16l/state XDG_CACHE_HOME=/tmp/w16l/cache
nb=/usr/sbin/novabrowser-webkit
$nb --profile=Leak "$url" >"$d/$name.stderr" 2>&1 &
bp=$!
sleep 3
hyprctl dispatch "hl.dsp.window.move({ workspace = '3', window = 'pid:$bp', follow = false })" >/dev/null
sleep 7
sh "$d/sample.sh" "$bp" "$secs" 5 >"$d/$name.log" 2>&1 &
sp=$!
end=$(( $(date +%s) + secs ))
while [ "$(date +%s)" -lt "$end" ]; do
  timeout 2 $nb --profile=Leak --key jjjjjjjjjj >/dev/null 2>&1; sleep 0.5
  timeout 2 $nb --profile=Leak --key jjjjjjjjjj >/dev/null 2>&1; sleep 0.5
  timeout 2 $nb --profile=Leak --key kkkkkkkkkk >/dev/null 2>&1; sleep 0.5
  timeout 2 $nb --profile=Leak --key kkkkkkkkkk >/dev/null 2>&1; sleep 0.5
done
wait $sp
echo "browser alive: $(kill -0 $bp 2>/dev/null && echo yes || echo no)" >>"$d/$name.log"
grim -o W16 "$d/$name.png"
kill "$bp"
