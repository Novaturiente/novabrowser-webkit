#!/bin/sh
# usage: [ENV=..] hide-run.sh <name> <visible-secs> <hidden-secs> <visible-again-secs>
# Test profile (/tmp/w16l) on the W16 virtual monitor (workspace 3), then moved to
# workspace 'w16hide' (on no monitor = hidden), then back. RSS every 5 s into <name>.log.
d=$(dirname "$(readlink -f "$0")")
name=$1
export XDG_DATA_HOME=/tmp/w16l/data XDG_CONFIG_HOME=/tmp/w16l/config XDG_STATE_HOME=/tmp/w16l/state XDG_CACHE_HOME=/tmp/w16l/cache
/usr/sbin/novabrowser-webkit --profile=Leak https://learning.eecglobal.com/learning >"$d/$name.stderr" 2>&1 &
bp=$!
mv() { hyprctl dispatch "hl.dsp.window.move({ workspace = '$1', window = 'pid:$bp', follow = false })" >/dev/null; echo "$(date +%T) -> workspace $1" >>"$d/$name.log"; }
sleep 3
: >"$d/$name.log"
mv 3
sh "$d/sample.sh" "$bp" $(( $2 + $3 + $4 + 10 )) 5 >>"$d/$name.log" 2>&1 &
sp=$!
sleep "$2"; mv name:w16hide
sleep "$3"; mv 3
sleep "$4"
kill $sp
echo "browser alive: $(kill -0 $bp 2>/dev/null && echo yes || echo no)" >>"$d/$name.log"
grim -o W16 "$d/$name.png"
kill "$bp"
