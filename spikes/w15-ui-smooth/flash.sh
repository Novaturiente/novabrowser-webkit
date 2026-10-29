#!/usr/bin/env bash
# Records the browser window while overlays open and close, then prints the mean
# brightness (luma 0-255) of every frame. A flash shows as a dip or spike that a
# plain fade would not make. Throwaway profile; data: pages only.
# Runs inside a nested niri (spikes/w15-ui-smooth/niri.kdl) so the recording sees only
# the browser: on the live desktop other windows and workspace switches got recorded.
# Start it first: `niri -c spikes/w15-ui-smooth/niri.kdl`, then NESTED=wayland-2 (its socket).
# Needs wf-recorder and ffmpeg. The recording is deleted after the analysis (KEEP=1 keeps it).
# Usage: NESTED=wayland-2 spikes/w15-ui-smooth/flash.sh [binary] [out-dir]
set -u
cd "$(dirname "$0")/../.."
export WAYLAND_DISPLAY=${NESTED:?start the nested niri and set NESTED to its socket}
unset DISPLAY
. tests/lib.sh
out=${2:-spikes/w15-ui-smooth/results/$(date +%H%M%S)}
mkdir -p "$out"
white="data:text/html,<title>white</title><body style='background:%23fff;margin:0'><h1 style='font:40px sans-serif;padding:40px'>White page</h1>"
dark="data:text/html,<title>dark</title><body style='background:%23123;margin:0'><h1 style='color:%23fff;font:40px sans-serif;padding:40px'>Dark page</h1>"

start "$white"
wait_for '"title":"white"' 30 || { fail "start"; finish; }
sleep 2.5
wf-recorder -y -r 60 -c libx264 -p crf=12 -f "$out/rec.mp4" >"$out/rec.log" 2>&1 &
rec=$!
t0=$(date +%s.%N)
sleep 1.5
mark() { echo "$(date +%s.%N) $1" >>"$out/marks"; }
: >"$out/marks"
step() { mark "$1"; "$bin" --cmd "$2" >/dev/null; sleep "${3:-1.2}"; }
step "palette-open-white" "cmd-set-text :open "
step "palette-close" "mode-leave" 0.8
step "goto-dark" "open $dark" 1.5
step "palette-open-dark" "cmd-set-text :open "
step "palette-close" "mode-leave" 0.8
step "goto-white" "back" 1.5
step "search-open" "cmd-set-text /"
step "search-close" "mode-leave" 0.8
mark "keyhint-g"; "$bin" --key g >/dev/null; sleep 1.2
mark "keyhint-esc"; "$bin" --key Escape >/dev/null; sleep 1
kill -INT $rec; wait $rec 2>/dev/null
stop
echo "$t0 start" >>"$out/marks"
ffmpeg -v error -i "$out/rec.mp4" -vf "signalstats,metadata=print:key=lavfi.signalstats.YAVG:file=$out/luma.txt" -f null -
python3 - "$out" <<'E'
import re, sys
out = sys.argv[1]
ts, ys = [], []
for l in open(f"{out}/luma.txt"):
    m = re.search(r"pts_time:([\d.]+)", l)
    if m: ts.append(float(m.group(1)))
    m = re.search(r"YAVG=([\d.]+)", l)
    if m: ys.append(float(m.group(1)))
marks = [l.split(None, 1) for l in open(f"{out}/marks")]
t0 = float(marks.pop()[0])  # recorder launch; its first frame is ~this
for t, name in marks:
    t = float(t) - t0
    seq = [round(y) for tt, y in zip(ts, ys) if t - 0.1 <= tt <= t + 0.8]
    # collapse runs of equal values: value x count
    runs, prev = [], None
    for v in seq:
        if prev is not None and abs(v - prev[0]) <= 1: prev[1] += 1
        else:
            prev = [v, 1]; runs.append(prev)
    print(f"{name.strip():20} " + " ".join(f"{v}" + (f"x{n}" if n > 1 else "") for v, n in runs))
E
[ "${KEEP:-0}" = 1 ] || rm -f "$out/rec.mp4"
finish
