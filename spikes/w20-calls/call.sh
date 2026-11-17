#!/usr/bin/env bash
# W20 spike (A3): one call run with a log bundle.
# Usage (from the repo root): spikes/w20-calls/call.sh <name> <url>
# Bundle: ~/.local/state/novabrowser-webkit/calls/<YYYY-MM-DD-HHMM>-<name>/ (outside the repo:
# GStreamer logs can contain meeting IDs and IP addresses).
set -uo pipefail
name=${1:?name}; url=${2:?url}
here=$(cd "$(dirname "$0")" && pwd)
bin=${NOVA_BIN:-target/release/novabrowser-webkit}
dir=$HOME/.local/state/novabrowser-webkit/calls/$(date +%F-%H%M)-$name
mkdir -p "$dir" && chmod 700 "$dir"
start=$(date '+%F %T')
{
  pacman -Q webkitgtk-6.0-nova-bin gst-plugin-pipewire xdg-desktop-portal-hyprland \
    gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav libnice 2>&1
  echo "browser: $("$bin" --version 2>&1 | head -1) ($(git -C "$here" rev-parse --short HEAD))"
  echo "NOVA_WEBKIT_FEATURES=${NOVA_WEBKIT_FEATURES:-}"
} > "$dir/env.txt"
# no GST_DEBUG_FILE: every process (sandboxed web processes too) writes to inherited stderr
GST_DEBUG='webrtc*:4,pipewire*:4,nice:3,webkit*:4' GST_DEBUG_NO_COLOR=1 \
  "$bin" --profile calls "$url" 2> "$dir/browser.stderr"
echo "browser exit: $?" >> "$dir/env.txt"
journalctl --user --since "$start" -u xdg-desktop-portal -u xdg-desktop-portal-hyprland \
  --no-pager > "$dir/portal.log" 2>&1
mem=$HOME/.local/state/novabrowser-webkit/memlog/$(date +%F).log
[ -f "$mem" ] && awk -v s="${start#* }" '$0 >= s || /^[^0-9]/' "$mem" > "$dir/memlog.txt"
cp "$here/result-template.md" "$dir/result.md"
echo "bundle: $dir"
