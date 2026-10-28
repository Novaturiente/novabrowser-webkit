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

# back / forward in one tab
"$bin" --cmd "open $(page four)" >/dev/null
wait_for '"title":"four"' && "$bin" --cmd "back" >/dev/null && wait_for '"url":"data:text/html,<title>two' 5 \
    && ok "back" || fail "back: $(q)"
"$bin" --cmd "forward" >/dev/null
wait_for '"url":"data:text/html,<title>four' 5 && ok "forward" || fail "forward: $(q)"

# pinned tabs refuse tab-close; undo brings a closed tab back
"$bin" --cmd "tab-pin" >/dev/null
"$bin" --cmd "tab-close" >/dev/null
wait_for 'Tab is pinned' 5 && ok "pinned tab refuses tab-close" || fail "pinned close: $(q)"
"$bin" --cmd "tab-pin" >/dev/null
n=$(ntabs); "$bin" --cmd "tab-close" >/dev/null; sleep 0.5
"$bin" --cmd "undo" >/dev/null
wait_for '"title":"four"' 5 && [ "$(ntabs)" = "$n" ] && ok "undo reopens the closed tab" || fail "undo: $(q)"

# find: match count, not found
"$bin" --cmd "/link" >/dev/null
wait_for '"message":"info: 2 matches"' 5 && ok "find counts matches" || fail "find: $(q)"
"$bin" --cmd "/zzqqxx" >/dev/null
wait_for 'not found on page' 5 && ok "find reports no match" || fail "find none: $(q)"

# JS dialog lands in the tab-bar prompt; closing its tab drops it
"$bin" --cmd "open -t data:text/html,<title>dlg</title><script>setTimeout(()=>alert(424242),300)</script>" >/dev/null
wait_for '"prompt":"[^"]*424242' 10 && ok "alert() shows in the prompt" || fail "alert: $(q)"
"$bin" --cmd "tab-close" >/dev/null
wait_for '"prompt":null' 5 && ok "closing the tab drops its dialog" || fail "dialog drop: $(q)"

# crash page: kill the active tab's web process, other tabs keep running, reload brings it back
"$bin" --cmd "open -t $(page victim)" >/dev/null
wait_for '"title":"victim"' 10
# every web process: tabs show the crash page, chrome views (tab bar…) reload themselves
# (they may sit under bwrap: walk up from every web process to our pid)
desc() { local p=$1; while [ "$p" -gt 1 ]; do [ "$p" = "$pid" ] && return 0; p=$(awk '{print $4}' /proc/$p/stat 2>/dev/null || echo 1); done; return 1; }
for wp in $(pgrep WebKitWebProces); do desc $wp && kill -9 $wp; done
wait_for '"crashed":true' 10 && ok "crash page after kill -9 of the web process" || fail "crash: $(q)"
"$bin" --cmd "reload" >/dev/null
wait_for '"title":"victim"' 10 && ! q | grep -q '"active":[0-9]*,[^]]*"crashed":true,[^}]*"title":"victim"' && ok "reload after a crash" || fail "crash reload: $(q)"
"$bin" --cmd "hint" >/dev/null   # the tab bar must be back to show the mode
wait_for '"label":"-- HINT MODE --"' 5 && "$bin" --cmd "mode-leave" >/dev/null && ok "tabs work after their web processes died" || fail "after crash: $(q)"
"$bin" --cmd "tab-close" >/dev/null

# spawn -o and a userscript writing commands to QUTE_FIFO
"$bin" --cmd "spawn -o echo spawned-output" >/dev/null
wait_for '"title":"Output"' 10 && ok "spawn -o shows output in a tab" || fail "spawn -o: $(q)"
"$bin" --cmd "tab-close" >/dev/null
mkdir -p $XDG_CONFIG_HOME/$app/userscripts
printf '#!/bin/sh\necho "message-info fifo-ok $QUTE_TITLE" >> "$QUTE_FIFO"\n' > $XDG_CONFIG_HOME/$app/userscripts/hello
chmod +x $XDG_CONFIG_HOME/$app/userscripts/hello
"$bin" --cmd "spawn -u hello" >/dev/null
wait_for 'fifo-ok' 10 && ok "userscript: QUTE_FIFO line runs as a command" || fail "userscript: $(q)"

# sessions by name, quickmarks, bookmarks
"$bin" --cmd "session-save smoke" >/dev/null; sleep 0.5
[ -s "$XDG_DATA_HOME/$app/sessions/smoke.json" ] && ok "session-save NAME" || fail "session-save"
"$bin" --cmd "session-save ../evil" >/dev/null
wait_for '"message":"error:' 5 && [ ! -e "$XDG_DATA_HOME/$app/evil.json" ] && ok "session names can't leave the dir" || fail "session path: $(q)"
"$bin" --cmd "quickmark-add data:text/html,<title>qm</title> smokeqm" >/dev/null
"$bin" --cmd "quickmark-load -t smokeqm" >/dev/null
wait_for '"title":"qm"' 10 && ok "quickmark add + load" || fail "quickmark: $(q)"
"$bin" --cmd "bookmark-add" >/dev/null; sleep 0.3
grep -q 'qm' "$XDG_CONFIG_HOME/$app/bookmarks/urls" 2>/dev/null && ok "bookmark-add writes bookmarks/urls" || fail "bookmark-add"
"$bin" --cmd "tab-close" >/dev/null

# windows: open -w and a private window
w=$(q | grep -o '"windows":[0-9]*')
"$bin" --cmd "open -w $(page win2)" >/dev/null
wait_for '"windows":2' 10 && ok "open -w makes a second window" || fail "open -w: $(q) (was $w)"
"$bin" --cmd "close" >/dev/null
wait_for '"windows":1' 10 && ok ":close closes only that window" || fail "close window: $(q)"

# downloads: attachment from a local server lands in downloads.location.directory
srv=$tmp/www; mkdir -p $srv/dl
echo 'payload' > $srv/dl/file.bin
echo '<title>notify</title><script>navigator.geolocation.getCurrentPosition(()=>{},()=>{})</script>' > $srv/notify.html
port=$((20000 + RANDOM % 20000))
(cd $srv && exec python3 -m http.server $port --bind 127.0.0.1 2>$tmp/http.log) &
http=$!
sleep 0.5
"$bin" --cmd "set downloads.location.directory $tmp/dl" >/dev/null
"$bin" --cmd "download http://127.0.0.1:$port/dl/file.bin" >/dev/null
for _ in $(seq 50); do [ -s $tmp/dl/file.bin ] && break; sleep 0.2; done
[ "$(cat $tmp/dl/file.bin 2>/dev/null)" = payload ] && ok "download (WebKit backend)" || fail "download: $(ls $tmp/dl 2>&1) $(q)"
"$bin" --cmd "download http://127.0.0.1:$port/dl/file.bin" >/dev/null
for _ in $(seq 50); do [ -s "$tmp/dl/file (1).bin" ] || [ -s "$tmp/dl/file.1.bin" ] && break; sleep 0.2; done
[ -s "$tmp/dl/file (1).bin" ] || [ -s "$tmp/dl/file.1.bin" ]; [ $? = 0 ] && ok "second download gets a (1) name" || fail "unique name: $(ls $tmp/dl)"

# a permission request set to ask lands in the prompt; tab close answers no
"$bin" --cmd "open -t http://127.0.0.1:$port/notify.html" >/dev/null
wait_for '"prompt":"Allow http://127.0.0.1:[0-9]* to see your location' 10 && ok "geolocation permission prompt" || fail "permission prompt: $(q)"
"$bin" --cmd "tab-close" >/dev/null
wait_for '"prompt":null' 5 && ok "closing the tab drops its permission prompt" || fail "prompt drop: $(q)"

# https-toggle on a local http page writes https_excludes; dark-toggle writes darkmode_excludes
"$bin" --cmd "open -t http://127.0.0.1:$port/dl/" >/dev/null; sleep 1
"$bin" --cmd "https-toggle" >/dev/null; sleep 0.5
grep -q "127.0.0.1:$port" "$XDG_CONFIG_HOME/$app/https_excludes" 2>/dev/null && ok "https-toggle writes https_excludes" || fail "https-toggle: $(q)"
"$bin" --cmd "dark-toggle" >/dev/null; sleep 0.5
wait_for 'Dark mode on for 127.0.0.1' 5 && ok "dark-toggle" || fail "dark-toggle: $(q)"
"$bin" --cmd "tab-close" >/dev/null
# dark mode on for 127.0.0.1 now: a light page is filtered; once it turns dark itself it is left alone
cp "$(dirname "$0")/pages/late-dark.html" $srv/
"$bin" --cmd "open -t http://127.0.0.1:$port/late-dark.html" >/dev/null
wait_for '"title":"light filtered"' 5 && wait_for '"title":"dark marked"' 5 \
  && ok "dark mode filters a light page, skips it once it turns dark" || fail "late dark: $(q)"
"$bin" --cmd "tab-close" >/dev/null
kill $http

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

saved_tabs=$(ntabs)
stop "quit -s"
grep -q '"panicked\|panicked at' "$tmp/stderr" && fail "panic in stderr ($tmp/stderr)" || ok "no panics"
grep -q "web process extension not found" "$tmp/stderr" && fail "extension .so not found next to the binary" || ok "extension found"
[ -s "$XDG_DATA_HOME/$app/sessions/default.json" ] && ok "session saved" || fail "quit -s saved no session"

# second run: auto_save.session restores both tabs (lazily: titles come from the session)
start
wait_for '"title":"four"' 30 && [ "$(ntabs)" = "$saved_tabs" ] && ok "session restored at start" || fail "restore: $(q)"
stop

echo; [ $fails = 0 ] && echo "smoke: all passed" || { echo "smoke: $fails failed"; cat "$tmp/stderr" | grep -vi enchant | tail -20; }
exit $fails
