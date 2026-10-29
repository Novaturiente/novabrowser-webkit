#!/usr/bin/env python3
"""Idle network capture (PROGRESS 7.13): start the browser on about:blank with a throwaway
profile, sit idle, and list every remote TCP/UDP peer its process tree (UI, network and web
processes, curl/aria2 children) talks to. Polls /proc every 0.1 s, so connections shorter
than that can be missed; no root needed.
Usage: [IDLE_PROFILE=dir] tests/idle-net.py [binary] [seconds=90]
(IDLE_PROFILE: reuse that profile dir, e.g. to see a second start with adblock lists cached)"""
import os, socket, subprocess, sys, tempfile, time

binary = os.path.realpath(sys.argv[1] if len(sys.argv) > 1 else "target/release/novabrowser-webkit")
secs = float(sys.argv[2]) if len(sys.argv) > 2 else 90
tmp = os.environ.get("IDLE_PROFILE") or tempfile.mkdtemp(prefix="nova-idle-")
env = dict(os.environ, **{v: f"{tmp}/{d}" for v, d in
           [("XDG_CONFIG_HOME", "c"), ("XDG_DATA_HOME", "d"), ("XDG_STATE_HOME", "s"), ("XDG_CACHE_HOME", "k")]})
os.makedirs(f"{tmp}/d/novabrowser-webkit", exist_ok=True)
open(f"{tmp}/d/novabrowser-webkit/welcomed", "w").close()


def tree(root):
    """root and all its descendants."""
    kids = {}
    for p in os.listdir("/proc"):
        if p.isdigit():
            try:
                ppid = int(open(f"/proc/{p}/stat").read().rsplit(")", 1)[1].split()[1])
                kids.setdefault(ppid, []).append(int(p))
            except OSError:
                pass
    out, todo = [], [root]
    while todo:
        p = todo.pop()
        out.append(p)
        todo += kids.get(p, [])
    return out


def inodes(pids):
    found = {}
    for p in pids:
        try:
            for fd in os.listdir(f"/proc/{p}/fd"):
                link = os.readlink(f"/proc/{p}/fd/{fd}")
                if link.startswith("socket:["):
                    found[link[8:-1]] = p
        except OSError:
            pass
    return found


def addr(hexaddr):
    ip, port = hexaddr.split(":")
    raw = bytes.fromhex(ip)
    if len(raw) == 4:
        a = socket.inet_ntop(socket.AF_INET, raw[::-1])
    else:  # /proc stores IPv6 as four little-endian 32-bit words
        a = socket.inet_ntop(socket.AF_INET6, b"".join(raw[i:i + 4][::-1] for i in range(0, 16, 4)))
    return a, int(port, 16)


def comm(p):
    try:
        return open(f"/proc/{p}/comm").read().strip()
    except OSError:
        return "?"


proc = subprocess.Popen([binary, "about:blank"], env=env, stderr=subprocess.DEVNULL)
seen = {}
end = time.time() + secs
while time.time() < end and proc.poll() is None:
    ino = inodes(tree(proc.pid))
    for f, kind in [("tcp", "tcp"), ("tcp6", "tcp"), ("udp", "udp"), ("udp6", "udp")]:
        try:
            lines = open(f"/proc/net/{f}").read().splitlines()[1:]
        except OSError:
            continue
        for line in lines:
            c = line.split()
            if c[9] not in ino:
                continue
            ip, port = addr(c[2])
            if port == 0 or ip.startswith("127.") or ip in ("::1", "::ffff:127.0.0.1", "0.0.0.0", "::"):
                continue  # unconnected or loopback (aria2 RPC, our own servers)
            key = (kind, ip, port)
            if key not in seen:
                seen[key] = (round(secs - (end - time.time()), 1), comm(ino[c[9]]))
    time.sleep(0.1)
subprocess.run([binary, "--cmd", "quit"], env=env, stdout=subprocess.DEVNULL)
proc.wait(timeout=15)

print(f"idle {secs:.0f} s on about:blank, fresh profile: {len(seen)} remote peer(s)")
for (kind, ip, port), (t, who) in sorted(seen.items(), key=lambda x: x[1][0]):
    try:
        name = socket.gethostbyaddr(ip)[0]
    except OSError:
        name = "-"
    print(f"  {t:>6}s  {who:<16} {kind} {ip}:{port}  ({name})")
