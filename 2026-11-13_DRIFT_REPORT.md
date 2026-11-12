# EGM Drift Report — 2026-11-13

Scope: the six EGM contracts written on 2026-11-13. Each contract except the
root was checked by a fresh read-only reviewer against current source; the
root was checked directly. Tags: **wrong** = the contract states something the
code contradicts; **undocumented** = significant code behaviour the contract
omits.

## EGM.md

- [wrong] "Every profile has its own data dir" implies all state is per profile, but the adblock lists and `engine.dat` are shared by every profile: `adblock::dir()` uses `xdg_root`, not the profile dir — `src/adblock.rs:37-40`.
- [wrong] "UI process and extension talk only through WebKit user messages" — the first config also arrives as init user data (`ext_config(true)` via `connect_initialize_web_process_extensions`), and the UI process adds the adblock dir to the web-process sandbox — `src/web.rs:111-119`.
- [wrong] The rule "code copied from novabrowser keeps `// from novabrowser <commit> src/<file>` on line 1" is stated as a global invariant, but no file in `src/`, `ext/src/` or `ui/` carries that header (grep, no hits). Either nothing was copied verbatim or the rule is not followed.
- [undocumented] Global side effects omit the aria2 lifecycle: `downloads::init` starts aria2 through `nova_core::aria2`, and `main` stops it on exit — `src/downloads.rs:148`, `src/main.rs:241`.
- [undocumented] Global side effects omit the in-browser memory watchdog: background tabs are hibernated above `HEAVY_GPU_MB = 1024` after `HEAVY_HIDDEN = 120 s`, and any tab's web process is killed above `KILL_MB = 4096` (RSS + GPU) — `src/window.rs:1835-1841`.

## src/EGM.md

- [wrong] `hints.js` does not post messages back through `webkit.messageHandlers.nova`; Rust reads its results through `web::eval_world`. Only `adblock-scan.js` (`adblock `) and the `passwords.rs` wrapper (`pw `) post — `src/hints.js:1-3`, `src/adblock-scan.js:6`, `src/passwords.rs:20-22`.
- [wrong] `darkmode_excludes` and https excludes are plain-text files (one host per line), not JSON. Only `downloads.json`, `zoom.json` and `permissions.json` are JSON — `src/commands.rs:195-213`, `src/commands.rs:232-243`.
- [wrong] The `OnceCell` in `web` holds a thread-local `Ctx` with two WebContexts (pages, chrome) and three NetworkSessions (normal, private, chrome), not one WebContext; early use panics with "web::init not called" — `src/web.rs:56-72`.
- [wrong] Signatures imprecise: `downloads::start(view: Option<&WebView>, url: &str)` and `downloads::magnet(String) -> bool` — `src/downloads.rs:290`, `src/downloads.rs:297`.
- [undocumented] `window::start_hibernation` also runs `memory_watch` every 60 s (GPU-heavy hidden tabs hibernate early; tabs over `KILL_MB = 4096` are killed; `take_oom` makes the crash page say "out of memory") — `src/window.rs:1794-1890`.
- [undocumented] Scripts injected into every frame (relevant to the Speedometer rule): `SAFARI_JS` (main world, start), `FOCUS_JS` (end), `MEDIA_JS` (start), and darkmode `NATIVE_DARK` in `web::WORLD` at end — `src/web.rs:244-266`, `src/darkmode.rs:99`.
- [undocumented] Adblock dir is shared across profiles; the engine refreshes when older than `MAX_AGE` (24 h) or when the list set changes — `src/adblock.rs:29`, `src/adblock.rs:37-40`, `src/adblock.rs:133-142`.
- [undocumented] Setup list omits `history::start` during activate; side effects omit aria2 start/stop — `src/main.rs:223`, `src/downloads.rs:148`, `src/main.rs:241`.
- [undocumented] CLI switches `--list-profiles`, `--version`, `-P <name>`; a plain start with a running instance hands off over IPC (`url <u>` or `cmd open -w`) — `src/main.rs:90-129`, `src/main.rs:171`, `src/main.rs:191-196`.
- [undocumented] The `ipc` interface omits the `key <keys>` and `url <text>` requests handled in `handle` — `src/ipc.rs:59-84`.
- [undocumented] The `passwords.rs` wrapper defines `globalThis.__novaPw` and focusin/focusout listeners that send `pw focus` — `src/passwords.rs:19-23`.

## ext/EGM.md

- [undocumented] The crate also depends on `serde_json = "1.0.152"` — `ext/Cargo.toml:16`.
- [undocumented] If the extension `.so` is not found, the UI process only prints `novabrowser-webkit: web process extension not found; no network adblock or subresource HTTPS-only` and keeps running — `src/web.rs:107-110`.
- [undocumented] The UI process adds `adblock::dir()` to the web-process sandbox so the extension can read the engine; the initial cfg is `ext_config(true)` sent at extension init — `src/web.rs:111-119`.

## ext/src/EGM.md

- [undocumented] Each `cfg` replaces the whole config: a missing `whitelist`/`excludes` clears it; a missing `engine` sets the path to `""`, which counts as a change and sets `ENGINE` to `None` (blocking off) — `ext/src/lib.rs:82-88`.
- [undocumented] A `cfg` string that is not valid JSON is ignored silently and the old config stays — `ext/src/lib.rs:74`.
- [undocumented] `should_block` always passes method `"get"` to adblock, and the whitelist is matched against the request URL, not the page URL — `ext/src/lib.rs:253`, `ext/src/lib.rs:258`.

## tests/EGM.md

- [wrong] `memwatch.sh` row: 1.5 GB and 4.4 GB are the sizes the test allocates, not the limits. Real limits: hibernate above 1024 MB GPU after 120 s hidden; kill above 4096 MB RSS + GPU — `src/window.rs:1837-1841`, `tests/memwatch.sh:2-3`.
- [wrong] `idle-net.py` never deletes its `/tmp/nova-idle-*` dir; an `IDLE_PROFILE` dir is reused and kept — `tests/idle-net.py:12`.
- [wrong] `passwords.sh` and `memwatch.sh` also need a live Wayland/X11 session, not only `smoke.sh`, `features.sh`, `caret.sh` — `tests/passwords.sh:4`, `tests/memwatch.sh:8`.
- [wrong] `smoke.sh` does not use `finish` or `serve`: it does its own panic grep, summary and `exit $fails`, and starts its HTTP servers inline — `tests/smoke.sh:119-121`, `tests/smoke.sh:143-145`, `tests/smoke.sh:158`, `tests/smoke.sh:171-172`.
- [undocumented] `features.sh` needs `sqlite3` and `python3` — `tests/features.sh:9-10`, `tests/features.sh:166`.
- [undocumented] `features.sh` runs `profile-open work` (a second browser instance) and writes fake CEF-edition data into `$XDG_CONFIG_HOME/novabrowser` and `$XDG_DATA_HOME/novabrowser` — `tests/features.sh:153-158`, `tests/features.sh:174-183`.
- [undocumented] `pages/mixed-dark.html` is used by no script; it is a manual check page for the WebKit dark-mode patch — `tests/pages/mixed-dark.html:2-4`.

## packaging/EGM.md

- [wrong] "`novabrowser-webkit`, `-bin` and `-git` conflict with each other" — `-git` lists only `conflicts=(novabrowser-webkit)`; it conflicts with `-bin` only through `-bin`'s `provides=(novabrowser-webkit)` — `packaging/PKGBUILD:22-23`, `packaging/aur/novabrowser-webkit-bin/PKGBUILD:20-21`.
- [undocumented] `release.sh` also needs `git`, `curl`, `bsdtar` and `zstd` — `packaging/release.sh:13`, `:17`, `:26`, `:28`.
- [undocumented] `release.sh` edits `aur/novabrowser-webkit/PKGBUILD` in place before `makepkg -f --cleanbuild`; a failed build leaves the new `pkgver`/`sha256sums` behind — `packaging/release.sh:18-21`.
- [undocumented] `release.sh` creates the release with `--title "novabrowser-webkit $v" --generate-notes` and does not commit or push to the AUR; it prints "done: commit packaging/aur, then push each dir to its AUR repo" — `packaging/release.sh:28-29`, `:37`.
- [undocumented] `webkitgtk-6.0-nova-bin` (2.54.2, pkgrel 4) also carries x86-64-v3, ThinLTO, no introspection/docs, webkitglib/2.54 security fixes, a Skia self-deadlock fix, upstream f311092c0d and WebRTC (optdepends `libnice`, `gst-plugins-bad`); it also provides `webkitgtk-6.0=$pkgver`, `webkitgtk-6.0-nova=$pkgver`, `libjavascriptcoregtk-6.0.so`, and conflicts with `webkitgtk-6.0-nova` — `packaging/aur/webkitgtk-6.0-nova-bin/PKGBUILD:2-6`, `:10-11`, `:22-28`, `:34-39`.
- [undocumented] Extension lookup order is `$NOVA_WEBKIT_EXT_DIR`, then the binary's dir, then `/usr/lib/novabrowser-webkit` — `src/web.rs:143-151`.
- [undocumented] The desktop entry also sets `StartupWMClass=novabrowser-webkit` and `Icon=novabrowser-webkit`, tied to the app ID `APP` and icon file names — `packaging/novabrowser-webkit.desktop:7`, `:11`, `src/main.rs:28`.
- [undocumented] License path uses `$pkgname`, so it differs per package (`novabrowser-webkit-git` vs `novabrowser-webkit`) — `packaging/PKGBUILD:70`, `packaging/aur/novabrowser-webkit/PKGBUILD:64`.
- [undocumented] The verification command clones through `git+file://`, so it builds only committed HEAD, not uncommitted work — `packaging/PKGBUILD:25`.
