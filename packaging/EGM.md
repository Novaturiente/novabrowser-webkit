# packaging — Arch Linux packages and releases

## Overview
Arch packaging for novabrowser-webkit. `PKGBUILD` builds the development
package `novabrowser-webkit-git` from git. `release.sh` cuts a release: it
builds the tagged source package, publishes a prebuilt `/usr` tarball on
GitHub, and updates the AUR recipes. Also holds the desktop entry and icons.

- `aur/novabrowser-webkit/` — AUR source package from the tag tarball.
- `aur/novabrowser-webkit-bin/` — AUR prebuilt package from the release tarball.
- `aur/webkitgtk-6.0-nova-bin/` — AUR prebuilt WebKitGTK 2.54.2 (pkgrel 4): PGO + x86-64-v3 + ThinLTO, no introspection/docs, dark-mode colour-filter patch, webkitglib/2.54 security fixes, a Skia self-deadlock fix, upstream f311092c0d, WebRTC on (optdepends `libnice`, `gst-plugins-bad`). Recipe in `spikes/wk-build/`.
- `icons/` — PNG icons at 16, 32, 48, 64, 128, 256, 512 px; `novabrowser-webkit.svg` is the scalable icon.

## Interface
- `release.sh <version>` — needs tag `v<version>` on origin and `git`, `curl`, `makepkg`, `bsdtar`, `zstd`, `gh`; `WORK` overrides the build dir (default `$HOME/.cache/nova-webkit-release`). It creates the GitHub release with `--title "novabrowser-webkit $v" --generate-notes`; it does not commit or push to the AUR (it prints "done: commit packaging/aur, then push each dir to its AUR repo").
- `PKGBUILD` — `NOVA_SRC` overrides the git source (e.g. `NOVA_SRC=file://$PWD/.. makepkg -si`); `pkgver()` = `r<count>.<hash>`; `prepare()` = `cargo fetch --locked`; `build()` = `cargo build --release --frozen --workspace`; `check()` = `cargo test --release --frozen --workspace`; `package()` installs binary, extension, desktop file, icons, docs, license.
- `novabrowser-webkit.desktop` — `Exec=novabrowser-webkit %U`, `Icon=novabrowser-webkit`, `StartupWMClass=novabrowser-webkit`, HTML and http/https MIME types, `new-window` action.
- Installed layout: `/usr/bin/novabrowser-webkit`, `/usr/lib/novabrowser-webkit/libnovabrowser_webkit_ext.so`. The binary searches `$NOVA_WEBKIT_EXT_DIR`, then its own dir, then `/usr/lib/novabrowser-webkit` (`src/web.rs:143-151`).

## Invariants
- `release.sh` exits unless tag `v<version>` is already on origin and `Cargo.toml` pins `nova-core` as a git dependency.
- The source sha256 comes from the tag tarball; `pkgver`, `pkgrel=1` and `sha256sums` are written to a copy in `$WORK/<v>/src`; `aur/novabrowser-webkit/PKGBUILD` is updated from that copy only after `makepkg` and `gh release create` succeed, so a failed build leaves `packaging/aur/` untouched.
- The `-bin` tarball is the `usr/` tree of the built package, named `novabrowser-webkit-<v>-x86_64.tar.zst`, matching the `source=` URL in `aur/novabrowser-webkit-bin/PKGBUILD`. Its sha256 is taken after upload; the tarball must not change afterwards.
- `.SRCINFO` is regenerated in both app AUR dirs; commit it with the PKGBUILDs.
- Builds use `--locked` / `--frozen`: `Cargo.lock` must be current at the tag.
- `_env()` sets `RUSTFLAGS` / `CFLAGS` path remaps before build and check, so `$srcdir` does not leak into panic messages or QuickJS `__FILE__`.
- The extension must install to `/usr/lib/novabrowser-webkit/`; the binary looks there.
- Binary name, desktop `Exec`, `Icon`, `StartupWMClass` and the app ID `APP` (`src/main.rs:28`) all stay `novabrowser-webkit`.
- The icon sizes in `icons/` must match the `package()` loop.
- `novabrowser-webkit` conflicts with `-bin` and `-git`; `-bin` conflicts with both and provides `novabrowser-webkit`; `-git` provides and conflicts only `novabrowser-webkit`, so it conflicts with `-bin` through that `provides`.
- `webkitgtk-6.0-nova-bin` provides `webkitgtk-6.0=$pkgver`, `webkitgtk-6.0-nova=$pkgver`, `libjavascriptcoregtk-6.0.so`, `libwebkitgtk-6.0.so`; conflicts with `webkitgtk-6.0` and `webkitgtk-6.0-nova`; `prepare()` refuses CPUs below x86-64-v3 (AVX2) via `ld-linux-x86-64.so.2 --help`.

## Side Effects
- `release.sh`: network (`git ls-remote`, `curl` tag tarball, `cargo fetch`, public GitHub release upload); after the upload, rewrites both AUR PKGBUILDs and their `.SRCINFO`; build work under `$WORK/<v>`.
- Packages install to `/usr/bin`, `/usr/lib/novabrowser-webkit`, `/usr/share/applications`, `/usr/share/icons/hicolor`, `/usr/share/doc/novabrowser-webkit`, `/usr/share/licenses/$pkgname` (so the license dir differs: `novabrowser-webkit-git` vs `novabrowser-webkit`).
- The desktop entry claims `text/html`, XHTML and http/https, so desktops may offer it as default browser.

## Verification
- Run: `cd packaging && NOVA_SRC=file://$PWD/.. makepkg -f` (runs `cargo test --release --frozen --workspace` in `check()`). It clones through `git+file://`, so it builds committed HEAD only, not uncommitted work.
