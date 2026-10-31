#!/bin/sh
# Passive hang logger for novabrowser-webkit (w16). Every 5 s, one line:
#   time | focused window (class:title) | per browser: profile, active tab url
#   | every web process: pid:RSS-MB/wait-channel (futex_wait at 0% CPU = hung)
# Output: ~/.local/state/novabrowser-webkit/memlog/<date>.log
# Run: systemd-run --user --unit=nb-memlog sh <this file>; stop: systemctl --user stop nb-memlog
out=${XDG_STATE_HOME:-$HOME/.local/state}/novabrowser-webkit/memlog
mkdir -p "$out"
while :; do
  focus=$(hyprctl activewindow -j 2>/dev/null | jq -r '"\(.class):\(.title)"' | cut -c1-60)
  tabs=""
  for bp in $(pgrep -x novabrowser-web); do
    prof=$(tr '\0' '\n' </proc/$bp/cmdline | sed -n 's/^--profile=//p')
    url=$(timeout 2 novabrowser-webkit ${prof:+--profile="$prof"} --query state 2>/dev/null | jq -r '.url' | cut -c1-80)
    tabs="$tabs [${prof:-default} $bp $url]"
  done
  procs=$(for w in $(pgrep -x WebKitWebProces); do
    printf ' %s:%s/%s' "$w" "$(awk '/VmRSS/{printf "%d",$2/1024}' /proc/$w/status 2>/dev/null)" "$(cat /proc/$w/wchan 2>/dev/null)"
  done)
  echo "$(date +%T) | $focus |$tabs |$procs" >>"$out/$(date +%F).log"
  sleep 5
done
