#!/bin/bash
# Dry-run upstream patches against a scratch copy of the WebKitGTK source on novahome.
# usage (on novahome): patch-check.sh <sha>...
#   First run builds ~/wk-build/check-tree (copy of webkitgtk-2.54.2 + our patches + the
#   webkitglib/2.54 branch fixes since 2.54.2, Source/ only). Each <sha> from WebKit main is
#   then dry-run against that tree: OK, or FAIL with the rejected files.
set -uo pipefail
V=2.54.2
SRC=$HOME/wk-build/webkitgtk-$V
T=$HOME/wk-build/check-tree
P=$HOME/wk-build/check-patches
mkdir -p "$P"
gh_patch() { curl -sfL "https://github.com/WebKit/WebKit/$1.diff" -o "$2"; }
# Source/ hunks only: the release tarball has no LayoutTests/Tools test files
source_only() { awk '/^diff --git /{keep = ($3 ~ /^a\/Source\//)} keep' "$1"; }
if [ ! -d "$T" ]; then
  cp -a "$SRC" "$T"
  gh_patch "compare/webkitgtk-$V...webkitglib/2.54" "$P/branch.diff" || { echo "branch diff download failed"; exit 1; }
  source_only "$P/branch.diff" > "$P/branch-src.diff"
  if patch -d "$T" -p1 -s -f < "$P/branch-src.diff" > "$P/branch.log" 2>&1; then
    echo "branch 2.54 fixes: applied cleanly"
  else
    echo "branch 2.54 fixes: rejects:"; grep -E "FAILED|can't find" "$P/branch.log" | head -20
  fi
fi
for sha in "$@"; do
  gh_patch "commit/$sha" "$P/$sha.diff" || { echo "$sha download failed"; continue; }
  source_only "$P/$sha.diff" > "$P/$sha-src.diff"
  if out=$(patch -d "$T" -p1 --dry-run -f < "$P/$sha-src.diff" 2>&1); then
    echo "$sha OK$(grep -q fuzz <<<"$out" && echo ' (fuzz)')"
  else
    echo "$sha FAIL: $(grep -oE 'patching file [^ ]+|FAILED.*' <<<"$out" | grep -B1 FAILED | grep -oE 'file .*' | tr '\n' ' ')"
  fi
done
