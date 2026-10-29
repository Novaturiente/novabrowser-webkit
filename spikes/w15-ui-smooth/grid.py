#!/usr/bin/env python3
"""Per-region flash check for a flash.sh recording (KEEP=1).

The whole-frame mean hides a flash in one part of the window. Every frame is scaled
to a 16x10 grid of grey cells. The recording is cut into transitions: runs of frames
between steady stretches (5+ frames with no cell moving more than 2 levels). In each
transition a cell overshoots when it goes more than 12 levels past both its value
before and after; that is a flash (a fade only moves between the two).
Timestamps are the recording's own (a nested compositor sends frames on damage only,
so they don't line up with `marks`).
Usage: grid.py <out-dir>   (needs rec.mp4 from flash.sh)
"""
import re, subprocess, sys

out = sys.argv[1]
W, H = 16, 10
probe = subprocess.run(["ffmpeg", "-v", "info", "-i", f"{out}/rec.mp4", "-vf", "showinfo",
                        "-f", "null", "-"], capture_output=True, text=True).stderr
ts = [float(m) for m in re.findall(r"pts_time:([\d.]+)", probe)]
raw = subprocess.run(["ffmpeg", "-v", "error", "-i", f"{out}/rec.mp4", "-vf",
                      f"scale={W}:{H}:flags=area,format=gray", "-f", "rawvideo", "-"],
                     capture_output=True).stdout
frames = [raw[i:i + W * H] for i in range(0, len(raw), W * H)][:len(ts)]
still = [i > 0 and max(abs(a - b) for a, b in zip(frames[i], frames[i - 1])) <= 2
         for i in range(len(frames))]
# steady[i]: frame i ends a run of 5 still frames
steady = [i >= 4 and all(still[i - 4:i + 1]) for i in range(len(frames))]
mean = lambda k: sum(frames[k]) // (W * H)
bad, i = 0, 1
while i < len(frames):
    if steady[i] or still[i]:
        i += 1
        continue
    a = i - 1  # last frame before the change
    while i < len(frames) and not steady[i]:
        i += 1
    z = min(i, len(frames) - 1)
    worst = (0, 0)
    for c in range(W * H):
        lo, hi = sorted((frames[a][c], frames[z][c]))
        for k in range(a, z + 1):
            v = frames[k][c]
            worst = max(worst, (max(lo - v, v - hi), c))
    over, c = worst
    tag = "FLASH" if over > 12 else "ok"
    bad += tag == "FLASH"
    print(f"{ts[a]:7.3f}-{ts[z]:7.3f}s {z - a:3} frames  mean {mean(a):3} -> {mean(z):3}  "
          f"{tag:5} overshoot {over:3} cell ({c % W},{c // W}): "
          + " ".join(str(frames[k][c]) for k in range(a, min(z + 1, a + 16))))
sys.exit(1 if bad else 0)
