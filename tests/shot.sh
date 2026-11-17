#!/usr/bin/env bash
# UI screenshot helper for the native chrome: start a throwaway instance on a few local
# pages, run each command over IPC (a command starting with "key " sends keys instead,
# "sleep N" waits), then save the window as a PNG (rendered by GTK, so it works while
# another window covers it).
# Usage: tests/shot.sh <binary> <out.png> [command ...]
#   env: SHOT_TABS (default 4 local pages), SHOT_WAIT (seconds before the shot, default 1)
set -u
out=$(realpath -m "$2")
. "$(dirname "$0")/lib.sh" "$1"
shift 2
start "$(page one)"
wait_for '"title":"one"' 30 || { echo "did not start"; cat "$tmp/stderr"; exit 1; }
for t in $(seq 2 "${SHOT_TABS:-4}"); do "$bin" --cmd "open -t $(page "tab$t")" >/dev/null; done
"$bin" --cmd "tab-focus 1" >/dev/null
sleep 1
for c in "$@"; do
    case "$c" in
        key\ *) "$bin" --key "${c#key }" >/dev/null ;;
        sleep\ *) sleep "${c#sleep }" ;;
        *) "$bin" --cmd "$c" >/dev/null ;;
    esac
done
sleep "${SHOT_WAIT:-1}"
"$bin" --query "snapshot $out"
echo
q
echo
"$bin" --cmd quit >/dev/null
sleep 1
