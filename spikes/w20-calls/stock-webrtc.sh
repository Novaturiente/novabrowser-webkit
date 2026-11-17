#!/usr/bin/env bash
# W20 spike (A2): does Arch's stock webkitgtk-6.0 have WebRTC? Throwaway container (--rm).
set -euo pipefail
docker run --rm archlinux bash -c '
  pacman -Syu --noconfirm --needed webkitgtk-6.0 binutils >/dev/null 2>&1
  pacman -Q webkitgtk-6.0
  echo "== ldd gstwebrtc/gstsdp:"; ldd /usr/lib/libwebkitgtk-6.0.so.4 | grep -E "gstwebrtc|gstsdp" || echo "(none)"
  echo "== PeerConnectionEnabled strings: $(strings /usr/lib/libwebkitgtk-6.0.so.4 | grep -c PeerConnectionEnabled || true)"
'
