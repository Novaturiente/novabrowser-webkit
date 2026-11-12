#!/bin/sh
# Passive hang logger for novabrowser-webkit (w16). Every 5 s, one line:
#   time | focused window (class:title) | per browser: profile, active tab url
#   | every web process: pid:RSS-MB+swap-MB+gpu-MB/wait-channel (futex_wait at 0% CPU = hung)
#   gpu = i915 drm-total-system0 summed per DRM client (shmem-backed, not in RSS)
#   UI process entry: [profile pid url ui=RSS+swap cg=scope-mem+scope-swap] (MB)
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
    ui=$(awk '/VmRSS/{r=$2} /VmSwap/{s=$2} END{printf "%d+%d",r/1024,s/1024}' /proc/$bp/status 2>/dev/null)
    cg=/sys/fs/cgroup$(cut -d: -f3 /proc/$bp/cgroup)
    cgm=$(( $(cat $cg/memory.current 2>/dev/null || echo 0) >> 20 ))+$(( $(cat $cg/memory.swap.current 2>/dev/null || echo 0) >> 20 ))
    tabs="$tabs [${prof:-default} $bp $url ui=$ui cg=$cgm]"
  done
  procs=$(for w in $(pgrep -x WebKitWebProces); do
    gpu=$(cat /proc/$w/fdinfo/* 2>/dev/null | awk '/^drm-client-id/{id=$2} /^drm-total-system0/{t[id]=$2*($3=="MiB"?1024:1)} END{for(i in t)s+=t[i]; printf "%d",s/1024}')
    printf ' %s:%s+%s/%s' "$w" "$(awk '/VmRSS/{r=$2} /VmSwap/{s=$2} END{printf "%d+%d",r/1024,s/1024}' /proc/$w/status 2>/dev/null)" "$gpu" "$(cat /proc/$w/wchan 2>/dev/null)"
  done)
  echo "$(date +%T) | $focus |$tabs |$procs" >>"$out/$(date +%F).log"
  sleep 5
done
