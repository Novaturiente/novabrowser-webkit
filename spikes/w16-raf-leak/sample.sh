#!/bin/sh
# usage: sample.sh <browser-pid> <seconds> [step]   RSS + wait channel of its web processes every step (15) s
anc() { p=$1; while [ "$p" -gt 1 ] 2>/dev/null; do echo "$p"; p=$(sed 's/.*) //' /proc/$p/stat 2>/dev/null | awk '{print $2}'); [ -z "$p" ] && break; done; }
t=0
while [ "$t" -le "$2" ]; do
  out=""
  for w in $(pgrep -x WebKitWebProces); do
    anc "$w" | grep -qx "$1" && out="$out $w:$(awk '/VmRSS/{printf "%dMB",$2/1024}' /proc/$w/status)/$(cat /proc/$w/wchan)"
  done
  echo "$(date +%T) t=${t}s$out"
  sleep "${3:-15}"; t=$((t+${3:-15}))
done
