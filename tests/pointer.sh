#!/bin/bash
# Mouse on the native chrome: tab click, middle-click close, hover × close, live drag
# reorder, hover tooltip, right-click menu, click outside the palette, prompt button.
# Needs real input: ydotool (running ydotoold) and Hyprland, scale 1 (ydotool absolute
# moves are 2x). Moves the real pointer.  usage: tests/pointer.sh [binary]
set -u
cd "$(dirname "$0")/.."
. tests/lib.sh
export HYPRLAND_INSTANCE_SIGNATURE=$(ls -t /run/user/$(id -u)/hypr | head -1)
start "$(page one)"
wait_for '"title":"one"' 30 || fail "did not start"
for t in two three four; do "$bin" --cmd "open -t $(page $t)" >/dev/null; done
"$bin" --cmd "tab-focus 1" >/dev/null
sleep 1.5
read X Y W H < <(hyprctl clients -j | jq -r ".[] | select(.pid==$pid) | \"\(.at[0]) \(.at[1]) \(.size[0]) \(.size[1])\"")
BAR=13   # middle of the tab bar row
# window coords -> pointer
to() { ydotool mousemove -a -x $(( (X + $1) / 2 )) -y $(( (Y + $2) / 2 )) >/dev/null; sleep 0.15; }
# middle x of tab i (0-based) of n equal tabs (4 px gaps)
tabx() { local w=$(( (W - 4 * ($2 + 1)) / $2 )); echo $(( 4 + $1 * (w + 4) + w / 2 )); }
tabend() { local w=$(( (W - 4 * ($2 + 1)) / $2 )); echo $(( 4 + $1 * (w + 4) + w )); }
titles() { q | grep -o '"title":"[^"]*"' | tr -d '\n'; }
active() { q | grep -o '"active":[0-9]*' | cut -d: -f2; }

to $(tabx 2 4) $BAR; ydotool click 0xC0 >/dev/null; sleep 0.5
[ "$(active)" = 2 ] && ok "click selects tab 3" || fail "click: active $(active)"

to $(tabx 3 4) $BAR; ydotool click 0xC2 >/dev/null; sleep 0.8
[ "$(ntabs)" = 3 ] && ok "middle-click closes tab 4" || fail "middle-click: $(ntabs) tabs"

to $(tabx 0 3) $BAR; sleep 1
q | grep -q '"popup":\[' && ok "hover shows the tooltip" || fail "tooltip: $(q | grep -o '"popup":[^,]*')"
[ -n "${SHOTS:-}" ] && "$bin" --query "snapshot $SHOTS/tip.png" >/dev/null

# drag tab 1 to the far right: it follows the pointer, lands last
to $(tabx 0 3) $BAR; ydotool click 0x40 >/dev/null; sleep 0.2
for f in 1 2 3 4 5 6; do to $(( $(tabx 0 3) + f * (W / 8) )) $BAR; done
"$bin" --query "snapshot $tmp/drag.png" >/dev/null
ydotool click 0x80 >/dev/null; sleep 0.8
[ -n "${SHOTS:-}" ] && cp "$tmp/drag.png" "$SHOTS/drag-mid.png"
titles | grep -q '"title":"two""title":"three""title":"one"' && ok "drag moves tab 1 to the end" || fail "drag: $(titles)"
[ "$(active)" = 2 ] && ok "dragged tab stays active" || fail "drag active: $(active)"

# × on hover closes that tab without selecting it (tab 1 = "two")
to $(tabx 0 3) $BAR; sleep 0.4; to $(( $(tabend 0 3) - 14 )) $BAR; sleep 0.4
[ -n "${SHOTS:-}" ] && "$bin" --query "snapshot $SHOTS/hover-x.png" >/dev/null
ydotool click 0xC0 >/dev/null; sleep 0.8
[ "$(ntabs)" = 2 ] && ! titles | grep -q '"two"' && ok "hover × closes its tab" || fail "×: $(titles)"
[ "$(active)" = 1 ] && ok "× doesn't select" || fail "× active: $(active)"

to $(tabx 0 2) $BAR; ydotool click 0xC1 >/dev/null; sleep 0.6
q | grep -q '"menu":true' && ok "right-click opens the tab menu" || fail "menu: $(q | grep -o '"menu":[a-z]*')"
[ -n "${SHOTS:-}" ] && "$bin" --query "snapshot $SHOTS/menu.png" >/dev/null
to $(( W / 2 )) $(( H / 2 )); ydotool click 0xC0 >/dev/null; sleep 0.6
q | grep -q '"menu":false' && ok "click outside closes the menu" || fail "menu still open"

"$bin" --cmd "cmd-set-text :open " >/dev/null; sleep 0.6
q | grep -q '"mode":"command"' || fail "palette did not open"
to $(( W / 2 )) $(( H - 60 )); ydotool click 0xC0 >/dev/null; sleep 0.6
q | grep -q '"mode":"normal"' && ok "click outside the palette closes it" || fail "palette: $(q | grep -o '"mode":"[a-z]*"')"

"$bin" --cmd "open -t data:text/html,<title>al</title><script>setTimeout(()=>alert(424242),200)</script>" >/dev/null
# (the command parser drops quotes, so the alert text is a number)
wait_for '"prompt":"[^"]*424242' 10 || fail "no alert prompt"
sleep 0.6
[ -n "${SHOTS:-}" ] && "$bin" --query "snapshot $SHOTS/prompt.png" >/dev/null
# the only (OK) button: centred under the question card
to $(( W / 2 )) 130; ydotool click 0xC0 >/dev/null; sleep 0.6
q | grep -q '"prompt":null' && ok "prompt button answers" || fail "prompt: $(q | grep -o '"prompt":[^,]*')"
stop
finish
