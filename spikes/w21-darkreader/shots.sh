#!/usr/bin/env bash
# Dark mode colours before/after patch 0001 (Dark Reader maps): screenshot each site with dark
# mode on, once per WebKit (installed webkitgtk-6.0-nova, then the test build in $WK), and put
# them side by side in $OUT/<n>.png (left: installed, right: test build).
# usage: spikes/w21-darkreader/shots.sh <browser binary> [url ...]
#   env: WK (default ~/.local/opt/wk-pgo-gen: the instrumented PGO build, copied from novahome),
#        OUT (default /tmp/w21)
set -u
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=$(realpath "$1"); shift
WK=${WK:-$HOME/.local/opt/wk-pgo-gen}
OUT=${OUT:-/tmp/w21}
mkdir -p "$OUT"
rm -f "$OUT"/*.png
# dark mode is for http(s) only: serve sample.html
port=$((20000 + RANDOM % 20000))
(cd "$root/spikes/w21-darkreader" && exec python3 -m http.server $port --bind 127.0.0.1 2>/dev/null) &
# not $http: lib.sh's exit trap in each run kills that
srv=$!
trap 'kill $srv' EXIT
sleep 0.5
urls=("$@")
[ ${#urls[@]} = 0 ] && urls=(
    "http://127.0.0.1:$port/sample.html"
    https://en.wikipedia.org/wiki/WebKit
    https://news.ycombinator.com/
    https://stackoverflow.com/questions/11227809
    https://docs.python.org/3/library/os.html
    https://www.bbc.com/news
)
shot() { # <label> <env...>: one browser run over all urls
    local label=$1; shift
    (
        . "$root/tests/lib.sh" "$bin"
        mkdir -p $XDG_CONFIG_HOME/$app
        printf 'import { c } from "nova";\nc.colors.webpage.preferred_color_scheme = "light";\nc.colors.webpage.darkmode.enabled = true;\n' > $XDG_CONFIG_HOME/$app/config.ts
        env "$@" "$bin" "${urls[0]}" 2>>"$tmp/stderr" & pid=$!
        wait_for '"loaded":true' 30
        for i in "${!urls[@]}"; do
            [ "$i" -gt 0 ] && "$bin" --cmd "open ${urls[$i]}" >/dev/null
            sleep 6
            # "nothing drawn" (window not drawn at that moment) or a timeout on a busy page: retry
            for _ in $(seq 15); do
                r=$("$bin" --query "snapshot $OUT/$i-$label.png" 2>&1)
                [ -s "$OUT/$i-$label.png" ] && break
                echo "retry $i-$label: $r" >&2; sleep 2
            done
        done
        "$bin" --cmd quit >/dev/null; wait $pid
    )
}
shot before
# no sandbox: it doesn't bind $WK; profile counters of the instrumented build go nowhere
shot after LD_LIBRARY_PATH="$WK/lib" WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 LLVM_PROFILE_FILE=/dev/null
for i in "${!urls[@]}"; do
    python3 -c "
from PIL import Image
a, b = Image.open('$OUT/$i-before.png'), Image.open('$OUT/$i-after.png')
o = Image.new('RGB', (a.width + b.width + 8, max(a.height, b.height)), 'red')
o.paste(a, (0, 0)); o.paste(b, (a.width + 8, 0)); o.thumbnail((1800, 1000)); o.save('$OUT/$i.png')"
    echo "$OUT/$i.png  ${urls[$i]}"
done
