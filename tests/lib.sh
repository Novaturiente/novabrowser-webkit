# Shared helpers for tests/*.sh: throwaway profile dirs, IPC queries, start/stop.
# Source it after `set -u`; $1 of the test script is the binary (default: release build).
bin=$(realpath "${1:-target/release/novabrowser-webkit}")
tmp=$(mktemp -d /tmp/nova-wk-test.XXXXXX)
export XDG_CONFIG_HOME=$tmp/config XDG_DATA_HOME=$tmp/data XDG_STATE_HOME=$tmp/state XDG_CACHE_HOME=$tmp/cache
app=novabrowser-webkit
mkdir -p $XDG_DATA_HOME/$app && touch $XDG_DATA_HOME/$app/welcomed   # returning user: no welcome tab
fails=0
ok()   { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }
q()    { "$bin" --query state 2>/dev/null; }
page() { echo "data:text/html,<title>$1</title><p>$1 <a href='%23a'>link a</a> <a href='%23b'>link b</a>"; }
# wait up to $2 s for `q` output to contain $1 (grep -E)
wait_for() { for _ in $(seq $(( ${2:-10} * 5 ))); do q | grep -qE -- "$1" && return 0; sleep 0.2; done; return 1; }
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
# local HTTP server on a random port serving $1; sets $port and $http (its pid)
serve() {
    port=$((20000 + RANDOM % 20000))
    (cd "$1" && exec python3 -m http.server $port --bind 127.0.0.1 2>$tmp/http.log) &
    http=$!
    sleep 0.5
}
finish() {
    grep -q 'panicked at' "$tmp/stderr" && fail "panic in stderr"
    echo; [ $fails = 0 ] && echo "$(basename "$0" .sh): all passed" || { echo "$(basename "$0" .sh): $fails failed"; grep -vi enchant "$tmp/stderr" | tail -20; }
    exit $fails
}
trap '[ -n "$pid" ] && kill $pid 2>/dev/null; [ -n "${http:-}" ] && kill $http 2>/dev/null; rm -rf "$tmp"' EXIT
