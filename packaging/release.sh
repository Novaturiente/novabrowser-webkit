#!/bin/bash
# Release vX.Y.Z: build the source package from the pushed tag, upload the -bin tarball,
# fill in sha256sums and .SRCINFO for packaging/aur/*. Run from anywhere; needs gh + makepkg.
# Usage: packaging/release.sh 0.1.0   (tag v0.1.0 must already be pushed)
set -euo pipefail
v=${1:?version}
repo=$(cd "$(dirname "$0")/.." && pwd)
aur=$repo/packaging/aur
work=${WORK:-$HOME/.cache/nova-webkit-release}/$v
url=https://github.com/Novaturiente/novabrowser-webkit
mkdir -p "$work"

git -C "$repo" ls-remote --exit-code --tags origin "v$v" >/dev/null || { echo "push tag v$v first"; exit 1; }
grep -q '^nova-core = { git' "$repo/Cargo.toml" || { echo "nova-core must be a pinned git rev"; exit 1; }

# 1. source package: pin the tag tarball's sha256, build + test it
src_sha=$(curl -fsSL "$url/archive/refs/tags/v$v.tar.gz" | sha256sum | cut -d' ' -f1)
sed -i -e "s/^pkgver=.*/pkgver=$v/" -e "s/^pkgrel=.*/pkgrel=1/" \
  -e "s/^sha256sums=.*/sha256sums=('$src_sha')/" "$aur/novabrowser-webkit/PKGBUILD"
mkdir -p "$work/src" && cp "$aur/novabrowser-webkit/PKGBUILD" "$work/src/"
(cd "$work/src" && makepkg -f --cleanbuild)
pkg=$(ls "$work"/src/novabrowser-webkit-"$v"-1-x86_64.pkg.tar.zst)

# 2. -bin tarball = the package's /usr tree
rm -rf "$work/stage" && mkdir -p "$work/stage"
bsdtar -xf "$pkg" -C "$work/stage" usr
tarball=$work/novabrowser-webkit-$v-x86_64.tar.zst
tar -C "$work/stage" -cf - usr | zstd -19 -T0 -f -o "$tarball"
gh release create "v$v" "$tarball" -R Novaturiente/novabrowser-webkit --title "novabrowser-webkit $v" --generate-notes

bin_sha=$(sha256sum "$tarball" | cut -d' ' -f1)
sed -i -e "s/^pkgver=.*/pkgver=$v/" -e "s/^pkgrel=.*/pkgrel=1/" \
  -e "s/^sha256sums=.*/sha256sums=('$bin_sha')/" "$aur/novabrowser-webkit-bin/PKGBUILD"

# 3. .SRCINFO for both
for p in novabrowser-webkit novabrowser-webkit-bin; do (cd "$aur/$p" && makepkg --printsrcinfo > .SRCINFO); done
echo "done: commit packaging/aur, then push each dir to its AUR repo"
