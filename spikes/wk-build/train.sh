#!/bin/bash
# PGO training on novarch: copy the instrumented build (`build.sh gen`) from novahome, run
# Speedometer 3.1 three times on it (spikes/w9-baseline), merge the profiles and put
# wk.profdata on novahome for `build.sh use|pkg`. Needs a desktop session (WebKit renders)
# and an idle machine; runs without the bubblewrap sandbox (it doesn't bind the prefix).
# usage: spikes/wk-build/train.sh [rounds=3]
set -euo pipefail
cd "$(dirname "$0")/.."
GEN=$HOME/.local/opt/wk-pgo-gen
P=/tmp/wkprof
rsync -a --delete novahome:.local/opt/wk-pgo-gen/ "$GEN/"
rm -rf "$P" && mkdir -p "$P"
for r in $(seq "${1:-3}"); do
  # %m: one file per binary (UI lib, web process, network process), merged across processes
  env WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 LD_LIBRARY_PATH="$GEN/lib" \
    LLVM_PROFILE_FILE="$P/%m-%p.profraw" w9-baseline/target/release/w9-baseline --speedometer | grep '^{'
done
llvm-profdata merge -o "$P/wk.profdata" "$P"/*.profraw
ls -la "$P/wk.profdata"
scp "$P/wk.profdata" novahome:wk-build/wk.profdata
echo "trained: novahome ~/wk-build/wk.profdata"
