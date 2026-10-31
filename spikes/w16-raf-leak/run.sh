#!/bin/sh
# usage: run.sh <name> <url>   isolated XDG dirs per run, never the real profile
d=$(dirname "$(readlink -f "$0")")/run-$1
mkdir -p "$d"
XDG_DATA_HOME=$d/data XDG_CONFIG_HOME=$d/config XDG_STATE_HOME=$d/state XDG_CACHE_HOME=$d/cache \
  exec /usr/sbin/novabrowser-webkit "$2"
