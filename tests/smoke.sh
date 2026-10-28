#!/usr/bin/env bash
# Smoke test over the IPC socket: start, open/close tabs, :set, hints, internal pages,
# quit, session restore, web process extension. Needs a running Wayland or X11 session.
# Throwaway profile dirs, no network (data: URLs only).
# Usage: tests/smoke.sh [path/to/novabrowser-webkit]   (default: target/release/novabrowser-webkit)
set -u
bin=$(realpath "${1:-target/release/novabrowser-webkit}")
tmp=$(mktemp -d /tmp/nova-wk-smoke.XXXXXX)
export XDG_CONFIG_HOME=$tmp/config XDG_DATA_HOME=$tmp/data XDG_STATE_HOME=$tmp/state XDG_CACHE_HOME=$tmp/cache
app=novabrowser-webkit
mkdir -p $XDG_DATA_HOME/$app && touch $XDG_DATA_HOME/$app/welcomed   # returning user: no welcome tab
fails=0
ok()   { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }
q()    { "$bin" --query state 2>/dev/null; }
page() { echo "data:text/html,<title>$1</title><p>$1 <a href='%23a'>link a</a> <a href='%23b'>link b</a>"; }
# wait up to $2 s for `q` output to contain $1
wait_for() { for _ in $(seq $(( ${2:-10} * 5 ))); do q | grep -q -- "$1" && return 0; sleep 0.2; done; return 1; }
ntabs() { q | grep -o '"title"' | wc -l; }
pid=
start() {
    "$bin" "$@" 2>>"$tmp/stderr" &
    pid=$!
}
stop() {
    "$bin" --cmd "${1:-quit}" >/dev/null
    for _ in $(seq 50); do kill -0 $pid 2>/dev/null || break; sleep 0.2; done
    if kill -0 $pid 2>/dev/null; then fail "quit: still running after 10 s"; kill $pid
    else wait $pid; rc=$?; [ $rc = 0 ] && ok "quit exits 0" || fail "quit exit code $rc"; fi
}
trap '[ -n "$pid" ] && kill $pid 2>/dev/null; rm -rf "$tmp"' EXIT

start "$(page one)"
wait_for '"title":"one"' 30 && ok "starts, first tab loads" || fail "first tab (stderr: $tmp/stderr)"

"$bin" --cmd "open -t $(page two)" >/dev/null
wait_for '"title":"two"' && [ "$(ntabs)" = 2 ] && ok "open -t over IPC" || fail "open -t: $(q)"

"$bin" "$(page three)"   # plain launch while running hands over to the running instance
wait_for '"title":"three"' && ok "second launch hands over" || fail "handover: $(q)"

"$bin" --cmd "hint" >/dev/null
wait_for '"label":"-- HINT MODE --"' 5 && ok "hint mode on a page with links" || fail "hint: $(q)"
"$bin" --cmd "mode-leave" >/dev/null
wait_for '"label":""' 5 && ok "mode-leave ends hint mode" || fail "mode-leave: $(q)"

"$bin" --cmd "tab-close" >/dev/null
sleep 1
n=$(ntabs); [ "$n" = 2 ] && ok "tab-close ($n tabs left)" || fail "tab-close: $(q)"

"$bin" --cmd "set theme light" >/dev/null
grep -q '"theme": *"light"' "$XDG_CONFIG_HOME/$app/autoconfig.json" 2>/dev/null \
    && ok ":set persists to autoconfig.json" || fail ":set persistence"
"$bin" --cmd "set auto_save.session true" >/dev/null

"$bin" --cmd "history" >/dev/null
wait_for '"url":"nova://page/' 5 && ok "internal page (:history) opens" || fail "history page: $(q)"
"$bin" --cmd "tab-close" >/dev/null

"$bin" --cmd "zoom 150" >/dev/null
wait_for 'Zoom level: 150' 5 && ok "zoom" || fail "zoom: $(q)"
"$bin" --cmd "zoom 100" >/dev/null

"$bin" --cmd "nonexistent-cmd" >/dev/null
wait_for '"message":"error:' 5 && ok "unknown command reports an error" || fail "error message: $(q)"

# adblock network rule in the web process extension, against a local server
srv=$tmp/www; mkdir -p $srv/ads
echo '<title>ads</title><script src="/ads/x.js"></script><img src="/ok.png">' > $srv/index.html
echo '' > $srv/ads/x.js; echo '' > $srv/ok.png
port=$((20000 + RANDOM % 20000))
(cd $srv && exec python3 -m http.server $port --bind 127.0.0.1 2>$tmp/http.log) &
http=$!
echo '/ads/x.js' > $tmp/list.txt
"$bin" --cmd "set content.blocking.adblock.lists '[\"file://$tmp/list.txt\"]'" >/dev/null
"$bin" --cmd "adblock-update" >/dev/null
wait_for 'adblock: 1 lists' 20 || fail "adblock-update: $(q)"
"$bin" --cmd "open -t http://127.0.0.1:$port/" >/dev/null
wait_for '"title":"ads"' 10; sleep 1
grep -q 'GET /ok.png' $tmp/http.log && ! grep -q 'GET /ads/x.js' $tmp/http.log \
    && ok "extension blocks a listed request, lets others through" || fail "adblock: $(cat $tmp/http.log)"
kill $http
"$bin" --cmd "tab-close" >/dev/null

stop "quit -s"
grep -q '"panicked\|panicked at' "$tmp/stderr" && fail "panic in stderr ($tmp/stderr)" || ok "no panics"
grep -q "web process extension not found" "$tmp/stderr" && fail "extension .so not found next to the binary" || ok "extension found"
[ -s "$XDG_DATA_HOME/$app/sessions/default.json" ] && ok "session saved" || fail "quit -s saved no session"

# second run: auto_save.session restores both tabs (lazily: titles come from the session)
start
wait_for '"title":"two"' 30 && [ "$(ntabs)" = 2 ] && ok "session restored at start" || fail "restore: $(q)"
stop

echo; [ $fails = 0 ] && echo "smoke: all passed" || { echo "smoke: $fails failed"; cat "$tmp/stderr" | grep -vi enchant | tail -20; }
exit $fails
