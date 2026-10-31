#!/bin/sh
# usage: hypr-run.sh <name> <workspace> <seconds> <command...>
# Runs <command> on Hyprland, moves its window to <workspace> (the W16 headless output),
# samples web-process RSS every 15 s into <name>.log, screenshot <name>.png at the end.
d=$(dirname "$(readlink -f "$0")")
name=$1; ws=$2; secs=$3; shift 3
"$@" >"$d/$name.stderr" 2>&1 &
bp=$!
sleep 3
hyprctl dispatch "hl.dsp.window.move({ workspace = '$ws', window = 'pid:$bp', follow = false })" >/dev/null
sleep 5
sh "$d/sample.sh" "$bp" "$secs" >"$d/$name.log" 2>&1
echo "browser alive: $(kill -0 $bp 2>/dev/null && echo yes || echo no)" >>"$d/$name.log"
grim -o W16 "$d/$name.png"
kill "$bp"
