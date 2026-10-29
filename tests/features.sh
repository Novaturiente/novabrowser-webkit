#!/usr/bin/env bash
# Phase 8 feature checks over the IPC socket (sessions, hibernation, windows, marks,
# downloads, aria2, devtools, fullscreen, profiles, private windows, welcome, import).
# Same setup as smoke.sh: throwaway profile, local pages only, needs a Wayland/X11 session.
# Usage: tests/features.sh [path/to/novabrowser-webkit]
set -u
. "$(dirname "$0")/lib.sh"
# field $2 of tab $1 (0-based) in the active window, e.g. `tabf 1 asleep` -> true
tabf() { q | python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin)['tabs'][$1]['$2']))" 2>/dev/null; }
statef() { q | python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin)['$1']))" 2>/dev/null; }
wait_tabf() { for _ in $(seq $(( ${4:-10} * 5 ))); do [ "$(tabf $1 $2)" = "$3" ] && return 0; sleep 0.2; done; return 1; }
sess=$XDG_DATA_HOME/$app/sessions

# --- 8.1 autosave without quitting; 8.3 lazy restore; command-line URL added as a tab
start "$(page one)"
wait_for '"title":"one"' 30 || fail "start"
"$bin" --cmd "set auto_save.session true" >/dev/null
"$bin" --cmd "open -t $(page two)" >/dev/null
wait_for '"title":"two"'
for _ in $(seq 25); do grep -q 'two' $sess/default.json 2>/dev/null && break; sleep 0.2; done
grep -q 'two' $sess/default.json 2>/dev/null && ok "8.1 autosave writes the session without quitting" || fail "8.1 autosave: $(ls $sess 2>&1)"
"$bin" --cmd "open -t $(page three)" >/dev/null; wait_for '"title":"three"'
"$bin" --cmd "tab-close" >/dev/null; sleep 1.5          # closed 'three': undo after restart (8.5)
stop
start "$(page cli)"
wait_for '"title":"cli"' 30 && [ "$(ntabs)" = 3 ] && ok "8.1 restore + command-line URL as an extra tab" || fail "8.1 restore: $(q)"
[ "$(tabf 0 loaded)" = false ] && ok "8.3 background tabs restore without a WebView" || fail "8.3 lazy: $(q)"
"$bin" --cmd "tab-focus 1" >/dev/null
wait_tabf 0 loaded true 10 && wait_for '"title":"one"' && ok "8.3 a restored tab loads on first show" || fail "8.3 show: $(q)"

# --- 8.5 undo across a restart
"$bin" --cmd "undo" >/dev/null
wait_for '"title":"three"' 10 && ok "8.5 undo brings back a tab closed before the restart" || fail "8.5 undo: $(q)"

# --- 8.4 named sessions: load -c, delete
"$bin" --cmd "session-save s1" >/dev/null; sleep 0.5
n=$(ntabs)
"$bin" --cmd "open -t $(page extra)" >/dev/null; wait_for '"title":"extra"'
"$bin" --cmd "session-load -c s1" >/dev/null
for _ in $(seq 50); do [ "$(ntabs)" = "$n" ] && ! q | grep -q '"title":"extra"' && break; sleep 0.2; done
[ "$(ntabs)" = "$n" ] && ! q | grep -q '"title":"extra"' && ok "8.4 session-load -c replaces the tabs" || fail "8.4 load -c: $(q) (want $n tabs)"
"$bin" --cmd "session-delete s1" >/dev/null; sleep 0.3
[ ! -e $sess/s1.json ] && ok "8.4 session-delete" || fail "8.4 delete"

# --- 8.6 hibernation, keep-awake
"$bin" --cmd "tab-focus 1" >/dev/null; sleep 0.5
"$bin" --cmd "tab-focus 2" >/dev/null; wait_tabf 1 loaded true 10   # load it first
"$bin" --cmd "tab-focus 1" >/dev/null; sleep 0.5
"$bin" --cmd "tab-hibernate 2" >/dev/null
wait_tabf 1 asleep true 5 && [ "$(tabf 1 loaded)" = false ] && ok "8.6 tab-hibernate N unloads that tab" || fail "8.6 hibernate: $(q)"
"$bin" --cmd "tab-hibernate" >/dev/null
wait_for "Can't hibernate the tab you're looking at" 5 && ok "8.6 the active tab is never hibernated" || fail "8.6 active: $(q)"
"$bin" --cmd "tab-focus 2" >/dev/null
wait_tabf 1 loaded true 10 && [ "$(tabf 1 asleep)" = false ] && ok "8.6 a hibernated tab comes back on show" || fail "8.6 wake: $(q)"
"$bin" --cmd "tab-keep-awake" >/dev/null
wait_tabf 1 keep true 5 && ok "8.6 tab-keep-awake marks the tab" || fail "8.6 keep: $(q)"

# --- 8.10 tab-move, 8.11 mute
t0=$(tabf 0 title)
"$bin" --cmd "tab-focus 1" >/dev/null; sleep 0.3
"$bin" --cmd "tab-move +" >/dev/null; sleep 0.3
[ "$(tabf 1 title)" = "$t0" ] && ok "8.10 tab-move + moves the tab right" || fail "8.10 move: $(q)"
"$bin" --cmd "tab-mute" >/dev/null
wait_tabf 1 muted true 5 && ok "8.11 tab-mute" || fail "8.11 mute: $(q)"
"$bin" --cmd "tab-mute" >/dev/null

# --- 8.7 windows restore, 8.8 tab-give
"$bin" --cmd "tab-give" >/dev/null
wait_for '"windows":2' 10 && ok "8.8 tab-give moves the tab to a new window" || fail "8.8 give: $(q)"
stop "quit -s"
start
wait_for '"windows":2' 30 && ok "8.7 the session restores both windows" || fail "8.7 windows: $(q)"
"$bin" --cmd "close" >/dev/null; wait_for '"windows":1' 10

# --- 8.12 / 8.13 quickmarks + bookmarks: delete, manager page
"$bin" --cmd "open $(page marked)" >/dev/null; wait_for '"title":"marked"'
"$bin" --cmd "quickmark-save" >/dev/null   # opens the command line prefilled, like qutebrowser
wait_for '"mode":"command"' 5 && ok "8.12 quickmark-save opens :quickmark-add <url>" || fail "8.12 qm save: $(q)"
"$bin" --cmd "mode-leave" >/dev/null
"$bin" --cmd "quickmark-add data:text/html,<title>qm1</title> qm1" >/dev/null; sleep 0.3
grep -q '^qm1 ' $XDG_CONFIG_HOME/$app/quickmarks 2>/dev/null && ok "8.12 quickmark-add writes quickmarks" || fail "8.12 qm add: $(cat $XDG_CONFIG_HOME/$app/quickmarks 2>&1)"
"$bin" --cmd "quickmark-del qm1" >/dev/null; sleep 0.3
! grep -q '^qm1 ' $XDG_CONFIG_HOME/$app/quickmarks 2>/dev/null && ok "8.12 quickmark-del" || fail "8.12 qm del"
"$bin" --cmd "bookmark-add" >/dev/null; sleep 0.3
"$bin" --cmd "bookmark-del" >/dev/null; sleep 0.3
! grep -q 'marked' $XDG_CONFIG_HOME/$app/bookmarks/urls 2>/dev/null && ok "8.12 bookmark-add then bookmark-del" || fail "8.12 bm del"
"$bin" --cmd "bookmarks" >/dev/null
wait_for '"url":"nova://page/' 5 && ok "8.13 bookmarks page opens" || fail "8.13 page: $(q)"
"$bin" --cmd "tab-close" >/dev/null

# --- 8.14 / 8.15 downloads: page, downloads.json, clear; 8.16 aria2
srv=$tmp/www; mkdir -p $srv; echo payload > $srv/f.bin; serve $srv
"$bin" --cmd "set downloads.location.directory $tmp/dl" >/dev/null
"$bin" --cmd "set downloads.backend webkit" >/dev/null
"$bin" --cmd "download http://127.0.0.1:$port/f.bin" >/dev/null
for _ in $(seq 50); do [ -s $tmp/dl/f.bin ] && break; sleep 0.2; done
for _ in $(seq 25); do grep -q 'f.bin' $XDG_DATA_HOME/$app/downloads.json 2>/dev/null && break; sleep 0.2; done
grep -q 'f.bin' $XDG_DATA_HOME/$app/downloads.json 2>/dev/null && ok "8.15 finished download recorded in downloads.json" || fail "8.15 json: $(cat $XDG_DATA_HOME/$app/downloads.json 2>&1)"
"$bin" --cmd "downloads" >/dev/null
wait_for '"url":"nova://page/' 5 && ok "8.15 :downloads page opens" || fail "8.15 page: $(q)"
"$bin" --cmd "tab-close" >/dev/null
"$bin" --cmd "download-clear" >/dev/null; sleep 0.3
! grep -q 'f.bin' $XDG_DATA_HOME/$app/downloads.json 2>/dev/null && ok "8.15 download-clear" || fail "8.15 clear"
if command -v aria2c >/dev/null; then
    echo payload2 > $srv/g.bin
    "$bin" --cmd "set downloads.backend aria2" >/dev/null
    "$bin" --cmd "download http://127.0.0.1:$port/g.bin" >/dev/null
    for _ in $(seq 75); do [ -s $tmp/dl/g.bin ] && break; sleep 0.2; done
    [ "$(cat $tmp/dl/g.bin 2>/dev/null)" = payload2 ] && ok "8.16 aria2 backend downloads" || fail "8.16 aria2: $(ls $tmp/dl) $(q)"
    a=$(pgrep -a aria2c | grep -- "--stop-with-process=$pid " | head -1)   # ours, not another browser's
    [ -n "$a" ] && ! echo "$a" | grep -q -- '--rpc-secret' && ok "8.16 aria2 RPC secret not on argv (A4)" || fail "8.16 argv: $a"
    "$bin" --cmd "set downloads.backend webkit" >/dev/null
else
    echo "skip 8.16 (no aria2c)"
fi
kill $http; http=

# --- 8.18 devtools, 8.19 fullscreen
"$bin" --cmd "devtools" >/dev/null; sleep 1
! q | grep -q '"message":"error' && ok "8.18 devtools opens without error" || fail "8.18 devtools: $(q)"
"$bin" --cmd "fullscreen" >/dev/null
for _ in $(seq 25); do [ "$(statef fullscreen)" = true ] && break; sleep 0.2; done
[ "$(statef fullscreen)" = true ] && ok "8.19 fullscreen" || fail "8.19 fullscreen: $(q)"
"$bin" --cmd "fullscreen" >/dev/null
for _ in $(seq 25); do [ "$(statef fullscreen)" = false ] && break; sleep 0.2; done
[ "$(statef fullscreen)" = false ] && ok "8.19 fullscreen again leaves" || fail "8.19 leave: $(q)"

# --- 8.21 private window
"$bin" --cmd "open -p $(page priv)" >/dev/null
wait_for '"title":"priv"' 10 && [ "$(statef private)" = true ] && ok "8.21 open -p opens a private window" || fail "8.21 private: $(q)"
"$bin" --cmd "close" >/dev/null; wait_for '"windows":1' 10

# --- 8.20 profiles
"$bin" --list-profiles | grep -q '^default' && ok "8.20 --list-profiles" || fail "8.20 list: $("$bin" --list-profiles 2>&1)"
mkdir -p $XDG_DATA_HOME/$app/profiles/work && touch $XDG_DATA_HOME/$app/profiles/work/welcomed
"$bin" --cmd "profile-open work" >/dev/null
pw=
for _ in $(seq 75); do "$bin" --profile work --query state >/dev/null 2>&1 && pw=1 && break; sleep 0.2; done
[ -n "$pw" ] && ok "8.20 profile-open starts its own instance (own IPC socket)" || fail "8.20 profile: $(ls $XDG_DATA_HOME/$app/profiles/work)"
"$bin" --profile=work --cmd quit >/dev/null 2>&1
"$bin" --list-profiles | grep -q '^work' && ok "8.20 --list-profiles shows the new profile" || fail "8.20 list work"
stop

# --- 8.23 --import from a Chromium-style profile dir; 8.22 welcome once per profile
imp=$tmp/chromium-profile; mkdir -p $imp
sqlite3 $imp/History "CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INT, typed_count INT, last_visit_time INT, hidden INT);
  INSERT INTO urls(url,title,visit_count,typed_count,last_visit_time,hidden) VALUES
  ('https://example.org/a','A',3,1,13300000000000000,0),('https://example.org/b','B',1,0,13300000000000000,0);"
out=$("$bin" --import $imp 2>&1)
echo "$out" | grep -q 'imported 2 Chromium' && ok "8.23 --import <dir> (Chromium History)" || fail "8.23 import: $out"
# --import novabrowser: the CEF edition's data (merge history/marks, copy files we lack)
cd_=$XDG_CONFIG_HOME/novabrowser; cdd=$XDG_DATA_HOME/novabrowser; mkdir -p $cd_ $cdd/sessions
sqlite3 $cdd/history.sqlite "CREATE TABLE history(id INTEGER PRIMARY KEY, url TEXT NOT NULL UNIQUE, title TEXT NOT NULL DEFAULT '',
  last_atime INTEGER NOT NULL, visits INTEGER NOT NULL DEFAULT 1, typed INTEGER NOT NULL DEFAULT 0);
  CREATE TABLE cmd_history(id INTEGER PRIMARY KEY, cmd TEXT NOT NULL);
  INSERT INTO history(url,title,last_atime,visits,typed) VALUES ('https://cef.example/x','From CEF',1790000000,5,2);
  INSERT INTO cmd_history(cmd) VALUES ('open cef-cmd');"
echo 'cq https://cef.example/q' > $cd_/quickmarks
echo 'cef.example' > $cd_/https_excludes
echo '{"https://cef.example":{"content.geolocation":true}}' > $cdd/permissions.json
echo '{"windows":[]}' > $cdd/sessions/cefsess.json
mkdir -p $XDG_CONFIG_HOME/$app; echo 'mine' > $XDG_CONFIG_HOME/$app/darkmode_excludes; echo 'theirs' > $cd_/darkmode_excludes
out=$("$bin" --import novabrowser 2>&1)
h=$XDG_DATA_HOME/$app/history.sqlite
[ "$(sqlite3 $h "SELECT title||visits||typed FROM history WHERE url='https://cef.example/x'")" = "From CEF52" ] \
  && [ "$(sqlite3 $h "SELECT count(*) FROM cmd_history WHERE cmd='open cef-cmd'")" = 1 ] \
  && ok "8.23 --import novabrowser merges history and cmd history" || fail "8.23 cef history: $out"
grep -q '^cq https://cef.example/q' $XDG_CONFIG_HOME/$app/quickmarks && ok "8.23 --import novabrowser merges quickmarks" || fail "8.23 cef marks: $out"
[ -f $XDG_CONFIG_HOME/$app/https_excludes ] && [ -f $XDG_DATA_HOME/$app/permissions.json ] && [ -f $XDG_DATA_HOME/$app/sessions/cefsess.json ] \
  && [ "$(cat $XDG_CONFIG_HOME/$app/darkmode_excludes)" = mine ] \
  && ok "8.23 --import novabrowser copies files, keeps existing ones" || fail "8.23 cef files: $out"
out=$("$bin" --import novabrowser 2>&1)
echo "$out" | grep -q 'copied 0 files' && echo "$out" | grep -q 'and 0 commands' && echo "$out" | grep -q '0 new quickmarks' \
  && ok "8.23 --import novabrowser twice changes nothing" || fail "8.23 cef rerun: $out"

rm -f $XDG_DATA_HOME/$app/welcomed
start
wait_for '"url":"nova://page/' 30 && ok "8.22 welcome page on a new profile" || fail "8.22 welcome: $(q)"
[ -e $XDG_DATA_HOME/$app/welcomed ] && ok "8.22 welcomed marker written" || fail "8.22 marker"
stop

finish
