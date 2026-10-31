#!/bin/sh
# usage: headless.sh <name> <seconds> <command...>
# Runs <command> alone in a headless labwc (always visible, unaffected by the user's
# workspaces), samples its web processes' RSS every 15 s, screenshots at the end.
# Output: <name>.log, <name>.png next to this script.
d=$(dirname "$(readlink -f "$0")")
name=$1; secs=$2; shift 2
inner=$(mktemp)
cat >"$inner" <<EOF
#!/bin/sh
"\$@" >"$d/$name.stderr" 2>&1 &
bp=\$!
sleep 8
sh "$d/sample.sh" \$bp $secs >"$d/$name.log" 2>&1
echo "browser alive: \$(kill -0 \$bp 2>/dev/null && echo yes || echo no)" >>"$d/$name.log"
grim "$d/$name.png"
kill \$bp
sleep 1
labwc --exit
EOF
WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 labwc -s "sh $inner $*"
rm -f "$inner"
