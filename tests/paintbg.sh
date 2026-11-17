#!/usr/bin/env bash
# View background while a page loads: dark (no white flash), then `colors.webpage.bg` once the
# page is set up; dark again under dark mode. A local server holds back the body (/) or the
# app script that gives the page its dark background (/app) for 3 s.
# usage: tests/paintbg.sh [binary]   (needs a Wayland/X11 session)
set -u
. "$(dirname "$0")/lib.sh" "${1:-}"
mkdir -p $XDG_CONFIG_HOME/$app
printf 'import { c } from "nova";\nc.colors.webpage.bg = "#ffffff";\n' > $XDG_CONFIG_HOME/$app/config.ts
port=$((20000 + RANDOM % 20000))
python3 -c '
import http.server, sys, time
class H(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"
    def do_GET(self):
        self.send_response(200); self.send_header("Content-Type", "text/html" if not self.path.endswith(".js") else "text/javascript"); self.end_headers()
        if self.path == "/app":
            self.wfile.write(b"<html><head><title>app</title></head><body><p>Loading the app</p><script async src=/app.js></script></body></html>")
        elif self.path == "/app.js":
            time.sleep(3)
            self.wfile.write(b"document.documentElement.style.background = \"#000\"; document.title = \"app ready\";")
        else:
            self.wfile.write(b"<html><head><title>slow</title></head><body>" + b" " * 4096); self.wfile.flush()
            time.sleep(3)
            self.wfile.write(b"<p>hello</p></body></html>")
    def log_message(self, *a): pass
http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()' $port &
http=$!
sleep 0.5
start "$(page one)"
wait_for '"title":"one"' 30 || { fail "did not start"; finish; }
# mean luma of the middle of the window (the page area)
luma() { "$bin" --query "snapshot $tmp/s.png" >/dev/null; python3 -c "from PIL import Image; im=Image.open('$tmp/s.png').convert('L'); w,h=im.size; c=im.crop((w//4,h//3,3*w//4,2*h//3)); print(round(sum(c.tobytes())/(c.width*c.height)))"; }
"$bin" --cmd "open -t http://127.0.0.1:$port/" >/dev/null
sleep 1.5
l=$(luma); [ "$l" -lt 60 ] && ok "new tab, nothing painted yet: dark ($l)" || fail "new tab before paint: luma $l (white flash)"
"$bin" --cmd "open -t http://127.0.0.1:$port/app" >/dev/null
sleep 1.5
l=$(luma); [ "$l" -lt 60 ] && ok "app shell, its script still loading: dark ($l)" || fail "app shell before its script: luma $l (white flash)"
wait_for '"title":"app ready"' 10 && sleep 1
l=$(luma); [ "$l" -lt 60 ] && ok "app set its own dark background: dark ($l)" || fail "app ready: luma $l"
"$bin" --cmd "tab-focus 2" >/dev/null
wait_for '"title":"slow"' 10
sleep 4
l=$(luma); [ "$l" -gt 200 ] && ok "painted page without a background: colors.webpage.bg ($l)" || fail "after paint: luma $l"
"$bin" --cmd dark-toggle >/dev/null; sleep 1
l=$(luma); [ "$l" -lt 60 ] && ok "dark-toggle: dark ($l)" || fail "dark-toggle: luma $l"
"$bin" --cmd dark-toggle >/dev/null; sleep 1
l=$(luma); [ "$l" -gt 200 ] && ok "dark-toggle off: light again ($l)" || fail "dark-toggle off: luma $l"
stop
finish
