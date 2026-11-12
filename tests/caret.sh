#!/bin/bash
# Insert mode on click and caret position on `gi`. Needs real input: ydotool (running
# ydotoold) and Hyprland, scale 1 (ydotool absolute moves are 2x).  usage: tests/caret.sh [binary]
set -u
cd "$(dirname "$0")/.."
. tests/lib.sh
export HYPRLAND_INSTANCE_SIGNATURE=$(ls -t /run/user/$(id -u)/hypr | head -1)
serve tests/pages
start "http://127.0.0.1:$port/caret.html"
wait_for '"title":"caret"' 15 || fail "caret page did not load"
sleep 1.5
read X Y W H < <(hyprctl clients -j | jq -r ".[] | select(.pid==$pid) | \"\(.at[0]) \(.at[1]) \(.size[0]) \(.size[1])\"")
# click at $1 % of the window height (fields: a top, b middle, c bottom)
click() { ydotool mousemove -a -x $(( (X + W/2) / 2 )) -y $(( (Y + H*$1/100) / 2 )) >/dev/null; sleep 0.2; ydotool click 0xC0 >/dev/null; sleep 0.6; }
esc() { "$bin" --key "<Escape>" >/dev/null; sleep 0.3; }
mode() { q | grep -q "\"mode\":\"$1\""; }

"$bin" --key "gi" >/dev/null; sleep 0.6
wait_for '"title":"a:11-11"' 3 && ok "gi: caret at the end" || fail "gi caret: $(q | grep -o '"title":"[^"]*"' | head -1)"
mode insert && ok "gi: insert mode" || fail "gi: not insert"
esc
click 20; mode insert && ok "click focused field after Esc: insert" || fail "click focused field after Esc: still normal"
esc
click 82; mode insert && ok "click other field after Esc: insert" || fail "click other field after Esc: still normal"
click 50; mode insert && ok "click field while insert: insert" || fail "click field while insert: left insert"
stop
finish
