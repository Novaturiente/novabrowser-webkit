# Building and packaging

## Arch Linux package

The AUR packages are not published yet. Build the package from a checkout
of this repository:

```sh
cd packaging
NOVA_SRC=file://$PWD/.. makepkg -si
```

`NOVA_SRC` points the PKGBUILD (`packaging/PKGBUILD`, package
`novabrowser-webkit-git`) at your local checkout instead of GitHub. Building
runs the test suite (`cargo test`).

The package depends on `libwebkitgtk-6.0.so`, so either WebKitGTK package
works:

| Package | What you get |
|---|---|
| `webkitgtk-6.0` (Arch) | works on any x86-64 CPU |
| `webkitgtk-6.0-nova` | faster (PGO) build with the dark-mode patch; AVX2 (x86-64-v3) CPUs only |

Optional packages: `gst-libav` (H.264/AAC, most web video),
`gst-plugins-bad` (more codecs), `gst-plugin-va` (hardware video decoding),
`aria2` (faster downloads, torrents), `rbw`, `pass` or `bitwarden-cli`
(passwords), `xdg-utils` (`:download-open`, default browser setup).

Make it the default browser:

```sh
xdg-settings set default-web-browser novabrowser-webkit.desktop
```

## From source (any distribution)

Requirements:

- Rust (stable, via rustup), `pkg-config`, a C compiler.
- Development files for GTK 4 (4.18 or newer) and WebKitGTK 6.0 (2.52 or
  newer), and SQLite.
- At run time: `curl` (filter list downloads) and the GStreamer base and
  good plugins; the optional packages above.

```sh
cargo build --release --workspace
target/release/novabrowser-webkit
```

`--workspace` builds two things: the browser, and the web process extension
`libnovabrowser_webkit_ext.so` (ad blocking, HTTPS upgrades of a page's
requests, custom headers). The browser looks for the extension in this
order:

1. the directory in `$NOVA_WEBKIT_EXT_DIR`,
2. the directory of the `novabrowser-webkit` binary (so `target/release/`
   works as is),
3. `/usr/lib/novabrowser-webkit/`.

Without it the browser still runs, but prints `web process extension not
found; no network adblock or subresource HTTPS-only` and blocks nothing.

Tests:

```sh
cargo test --release --workspace
tests/smoke.sh target/release/novabrowser-webkit      # needs a running desktop session
tests/features.sh target/release/novabrowser-webkit
tests/passwords.sh target/release/novabrowser-webkit  # needs pass, gpg, openssl, python3
```

The scripts start the browser with throwaway profile directories and drive
it over the control socket. `smoke.sh` needs no network.

## Runtime layout

```
/usr/bin/novabrowser-webkit                                   the binary
/usr/lib/novabrowser-webkit/libnovabrowser_webkit_ext.so      web process extension
/usr/share/applications/novabrowser-webkit.desktop
/usr/share/icons/hicolor/scalable/apps/novabrowser-webkit.svg   (+ PNGs, 16–512 px)
/usr/share/doc/novabrowser-webkit/                            README and docs
```
