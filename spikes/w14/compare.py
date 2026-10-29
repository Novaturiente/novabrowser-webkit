#!/usr/bin/env python3
"""W14: WebKit edition vs CEF edition, same machine, throwaway profiles.
- startup: launch -> first tab (a local page) has its title, via `--query state`; median of 5
  after one warm-up run.
- memory: 5 real sites in 5 tabs, all shown once, then 20 s idle; PSS summed over the whole
  process tree (/proc/<pid>/smaps_rollup), so shared libraries are not counted twice.
Scrolling feel stays a manual check.
Usage: spikes/w14/compare.py <webkit-binary> <cef-binary>  -> prints + results/w14.json
W14_ONLY=memory skips startup; W14_EXTRA=webkit-noext adds a WebKit run without the web
process extension (adblock off), the W7 measure."""
import http.server, json, os, statistics, subprocess, sys, tempfile, threading, time

SITES = [
    "https://en.wikipedia.org/wiki/Rust_(programming_language)",
    "https://github.com/WebKit/WebKit",
    "https://news.ycombinator.com/",
    "https://stackoverflow.com/questions/11227809",
    "https://www.bbc.com/news",
]


def serve():
    d = tempfile.mkdtemp()
    open(f"{d}/t.html", "w").write("<title>ready</title><p>ready")
    h = lambda *a: http.server.SimpleHTTPRequestHandler(*a, directory=d)
    s = http.server.HTTPServer(("127.0.0.1", 0), h)
    http.server.SimpleHTTPRequestHandler.log_message = lambda *a: None
    threading.Thread(target=s.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{s.server_port}/t.html"


def profile(app):
    tmp = tempfile.mkdtemp(prefix="w14-")
    env = dict(os.environ, **{v: f"{tmp}/{d}" for v, d in
               [("XDG_CONFIG_HOME", "c"), ("XDG_DATA_HOME", "d"), ("XDG_STATE_HOME", "s"), ("XDG_CACHE_HOME", "k")]})
    os.makedirs(f"{tmp}/d/{app}", exist_ok=True)
    open(f"{tmp}/d/{app}/welcomed", "w").close()
    return env


def state(binary, env):
    r = subprocess.run([binary, "--query", "state"], env=env, capture_output=True, text=True, timeout=10)
    try:
        return json.loads(r.stdout)
    except ValueError:
        return None


def cmd(binary, env, line):
    subprocess.run([binary, "--cmd", line], env=env, capture_output=True, timeout=10)


def quit(binary, env, p):
    cmd(binary, env, "quit")
    try:
        p.wait(timeout=20)
    except subprocess.TimeoutExpired:
        p.kill()


def tree(root):
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


def pss_mb(root):
    total = 0
    for p in tree(root):
        try:
            for line in open(f"/proc/{p}/smaps_rollup"):
                if line.startswith("Pss:"):
                    total += int(line.split()[1])
        except OSError:
            pass
    return total / 1024


def startup(binary, app, url):
    env = profile(app)
    p = subprocess.Popen([binary, url], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    t0 = time.monotonic()
    while time.monotonic() - t0 < 30:
        s = state(binary, env)
        if s and any(t.get("title") == "ready" for t in s.get("tabs", [])):
            break
        time.sleep(0.02)
    dt = time.monotonic() - t0
    quit(binary, env, p)
    return dt


def memory(binary, app, extra_env=None):
    env = dict(profile(app), **(extra_env or {}))
    p = subprocess.Popen([binary, SITES[0]], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(6)
    for s in SITES[1:]:
        cmd(binary, env, f"open -t {s}")
        time.sleep(6)
    for i in range(1, len(SITES) + 1):  # every tab shown once (lazy views)
        cmd(binary, env, f"tab-focus {i}")
        time.sleep(1)
    time.sleep(20)
    mb = pss_mb(p.pid)
    n = len(tree(p.pid))
    quit(binary, env, p)
    return mb, n


url = serve()
res = {}
runs_list = [("webkit", sys.argv[1], "novabrowser-webkit", None), ("cef", sys.argv[2], "novabrowser", None)]
if os.environ.get("W14_EXTRA") == "webkit-noext":
    runs_list.insert(1, ("webkit-noext", sys.argv[1], "novabrowser-webkit", {"NOVA_WEBKIT_EXT_DIR": "/nonexistent"}))
for name, binary, app, extra in runs_list:
    binary = os.path.realpath(binary)
    r = {}
    if os.environ.get("W14_ONLY") != "memory" and extra is None:
        startup(binary, app, url)  # warm-up (page cache)
        runs = [startup(binary, app, url) for _ in range(5)]
        r = {"startup_s": [round(x, 3) for x in runs], "startup_median_s": round(statistics.median(runs), 3)}
    mb, procs = memory(binary, app, extra)
    res[name] = dict(r, pss_mb_5_sites=round(mb), processes=procs)
    print(name, json.dumps(res[name]))
os.makedirs(os.path.join(os.path.dirname(__file__), "results"), exist_ok=True)
with open(os.path.join(os.path.dirname(__file__), "results", "w14.json"), "w") as f:
    json.dump({"date": time.strftime("%Y-%m-%d %H:%M"), "sites": SITES, **res}, f, indent=1)
