#!/bin/bash
# Custom WebKitGTK 6.0 build on novahome (PGO + x86-64-v3 + ThinLTO).
# usage: build.sh gen   -> instrumented build, installed to ~/.local/opt/wk-pgo-gen
#        build.sh use   -> PGO build from ~/wk-build/wk.profdata, installed to ~/.local/opt/wk-pgo
#        build.sh pkg   -> same as use but prefix /usr (Arch layout), staged in ~/wk-build/stage-pkg
#                          for pkg/PKGBUILD. Same source dir as gen: PGO names of static functions
#                          include the source path, so moving the tree would lose part of the profile.
# Run it through run.sh: novahome runs other services, the build must stay under 20 GB RAM.
# Flags follow Arch's webkitgtk-6.0 PKGBUILD (clang, lld, FORTIFY 2, no CET) plus our changes.
set -euo pipefail
stage=$1
V=2.54.2
SRC=$HOME/wk-build/webkitgtk-$V
BUILD=$HOME/wk-build/build-$stage
base="-march=x86-64-v3 -O2 -pipe -fno-plt -fexceptions -Wp,-D_FORTIFY_SOURCE=2 -fcf-protection=none"
case $stage in
  gen) prefix=$HOME/.local/opt/wk-pgo-gen; pgo="-fprofile-generate"; lto=() ;;
  use) prefix=$HOME/.local/opt/wk-pgo; pgo="-fprofile-use=$HOME/wk-build/wk.profdata -Wno-profile-instr-unprofiled -Wno-profile-instr-out-of-date -Wno-backend-plugin"; lto=(-D LTO_MODE=thin); ldx="-Wl,--thinlto-jobs=4" ;;
  pkg) prefix=/usr; pgo="-fprofile-use=$HOME/wk-build/wk.profdata -Wno-profile-instr-unprofiled -Wno-profile-instr-out-of-date -Wno-backend-plugin"; lto=(-D LTO_MODE=thin -D CMAKE_INSTALL_LIBEXECDIR=lib -D CMAKE_SKIP_RPATH=ON); ldx="-Wl,--thinlto-jobs=4" ;;
  *) echo "stage: gen|use|pkg" >&2; exit 2 ;;
esac
export CC=clang CXX=clang++
export CFLAGS="$base $pgo" CXXFLAGS="$base $pgo" LDFLAGS="-fuse-ld=lld $pgo ${ldx:-}"
cmake -S "$SRC" -B "$BUILD" -G Ninja \
  -D CMAKE_BUILD_TYPE=Release -D CMAKE_INSTALL_PREFIX="$prefix" -D CMAKE_INSTALL_LIBDIR=lib \
  -D PORT=GTK -D USE_GTK4=ON -D USE_SOUP2=OFF \
  -D ENABLE_DOCUMENTATION=OFF -D ENABLE_INTROSPECTION=OFF -D ENABLE_MINIBROWSER=OFF \
  -D ENABLE_SPEECH_SYNTHESIS=OFF -D USE_FLITE=OFF -D USE_LIBBACKTRACE=OFF "${lto[@]}"
# -j8: WebKit unified sources take up to ~2 GB per clang job; run.sh's 20 GB cap is the hard limit
ninja -C "$BUILD" -j8
if [ "$stage" = pkg ]; then
  rm -rf "$HOME/wk-build/stage-pkg"
  DESTDIR=$HOME/wk-build/stage-pkg ninja -C "$BUILD" install
  echo "staged $HOME/wk-build/stage-pkg"
else
  rm -rf "$prefix"
  ninja -C "$BUILD" install
  echo "installed $prefix"
fi
