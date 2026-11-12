# src — novabrowser-webkit UI process

## Overview
The main binary crate `novabrowser-webkit`: a keyboard-driven, qutebrowser-style
browser on WebKitGTK 6.0 + GTK 4. `main.rs` parses arguments, sets up the
profile and XDG directories, and starts the GTK app. The other modules own
windows and tabs, key modes, commands, the IPC control socket, adblock,
passwords, downloads, permissions, dark mode and zoom. Shared, engine-neutral
logic (config, paths, history, sessions, password vault CLIs, URL matching,
HTTPS upgrade) lives in the external crate `nova-core`; this crate plugs into it
through the `WebKitHost` seam. The `.js` files are page scripts compiled in with
`include_str!`; the chrome overlays are compiled in from `../ui/*.html`.

| File | Role |
|------|------|
| `main.rs` | Entry point, CLI switches, profile re-exec, `WebKitHost` |
| `ipc.rs` | Per-profile Unix control socket (`--cmd`, `--query`, `--key`) |
| `web.rs` | WebContexts, network sessions, `nova://` scheme, views |
| `window.rs` | Window/tab state, overlays, sessions, hibernation (~2100 lines) |
| `tabview.rs` | Per-WebView signal wiring (load, policy, menus, messages) |
| `commands.rs` | Command-line parser and dispatcher |
| `modes.rs` | Key mode state machine and key conversion |
| `hints.rs` / `hints.js` | Link hint mode |
| `adblock.rs` / `adblock-scan.js` | Brave adblock engine, lists, cosmetic filters |
| `adblock-resources.json` | Bundled uBlock scriptlet resources |
| `passwords.rs` / `passwords.js` | Login autofill and save prompts |
| `permissions.rs` | Permission, dialog, TLS-error and hung-page prompts |
| `downloads.rs` | Download tracking, aria2 handoff, downloads page |
| `darkmode.rs` | Per-view dark mode |
| `zoom.rs` | Per-host zoom levels |
| `clipboard.rs` | Clipboard and primary selection |
| `spawn.rs` | External commands, userscripts, synthetic input |
| `anim.rs` | GTK tick-callback fades |

## Interface
- `main::main()` — process entry: CLI, profile re-exec, IPC client mode, then the GTK app.
- CLI switches: `--profile <name>` / `-P <name>`, `--list-profiles`, `--version`, `--import`, `--check-config`, `--cmd <command>`, `--query [state]`, `--key <keys>`. A plain start while an instance runs hands off over IPC: `url <u>` with a URL, `cmd open -w` without (`main.rs:176-198`).
- `main::post(f)` — runs a closure on the GTK main thread from a worker thread (`main.rs:73`).
- `main::start_url()` — first non-switch argument; bare hosts get `https://`.
- `ipc::socket_path()` / `ipc::request(line)` / `ipc::start()` — socket location, client call to a running instance, server bind (`ipc.rs:14-28`).
- IPC requests (one line each, `ipc::handle`, `ipc.rs:60-84`): `url <text>` (open + raise), `cmd <command>`, `query state`, `key <keys>`. Replies `ok`, a JSON string, or `error: ...`; `error: quitting` once quit started.
- `web::init(data, cache)` — builds the page and chrome WebContexts, registers `nova://`, sets memory caps.
- `web::page_view(related, private)` / `web::chrome_view(ui, on_message)` — build page views and UI overlay views.
- `web::internal_url(html)` — `nova://` URL for inline HTML.
- `web::eval_world` / `eval_page` / `eval_quiet` — run JS in the novabrowser isolated world (`web::WORLD`) or the page world.
- `web::apply_config()` / `web::user_agent_for(url)` — apply cookies, ITP, languages, spellcheck; per-URL user agent.
- `window::create(start_url)` — opens the first window.
- `window::open` / `open_related` / `adopt` / `show` / `close` / `remove` / `go` — tab lifecycle and navigation.
- `window::in_slot` / `in_owner` — run a closure against the owning window's state.
- `window::session_json()` / `restore(v, clear)` / `state_json()` — session save/restore; JSON for `--query state`.
- `window::message(level, text)` / `startup_message` / `set_prompt(...)` — toasts and prompt text.
- `window::start_hibernation()` / `hibernate(n)` / `keep_awake(n)` / `quitting()` — tab sleep and shutdown flag. `start_hibernation` runs `hibernate_idle` and `memory_watch` every 60 s (`window.rs:1794-1800`).
- `commands::parse_line(line)` / `run(line, count)` / `run_from_bar` / `search(text, backward)` / `url_or_search(u)` — command parsing and dispatch.
- `modes::init()` / `on_key(key)` / `enter(mode)` / `leave()` / `mode()` / `key_from_event(keyval, state)` — key handling.
- `hints::start(cmd)` / `on_key(name)` / `on_leave()` — hint mode.
- `adblock::start()` / `update_now()` / `push_config()` / `add_scripts(ucm)` / `on_navigate` / `on_message` / `forget` — adblock engine and its page scripts.
- `passwords::add_scripts(ucm)` / `on_message` / `fill_command(otp)` / `forget` — autofill; vault CLIs (`rbw`, `bw`, `pass`) live in `nova_core::passwords`. `add_scripts` wraps `passwords.js` as `globalThis.__novaPw` and adds focusin/focusout listeners that send `pw focus` (`passwords.rs:18-23`).
- `downloads::init()` / `start(view: Option<&WebView>, url: &str)` / `magnet(url: String) -> bool` / `open` / `cancel` / `clear`.
- `permissions::init_notifications(ctx)` / `request(view, req)` / `dialog` / `tls_error` / `hung` / `responsive`.
- `clipboard::write(text, primary)` / `expand_then_run`.
- `spawn::spawn(argv, userscript, output, count)` / `fake_key(keystring)` / `insert_text(text)`.
- `zoom::set(view, pct)` / `apply(view)`; `darkmode::apply(view)` / `apply_for(view, url)` / `forget(view)`.
- `anim::reveal` / `warm` / `conceal`; `tabview::connect(view)` / `load(view, url)` / `config_changed()`.
- Web-process extension counterpart: [`../ext/EGM.md`](../ext/EGM.md).

## Invariants
- `nova_core::paths::set_app(APP)` runs first in `main` (`main.rs:115`), before any profile or XDG path lookup.
- `--profile <name>` re-execs the binary as `--profile=<name>` before GTK starts, so later argv reads see it.
- `app.run_with_args(&raw[..1])` hides the custom switches from GTK.
- `nova_core::host::set(WebKitHost, post)` (`main.rs:218`) must run before any `host::on_ui` / `host::later` call; all setup (`web::init`, `modes::init`, `history::start`, `adblock::start`, `downloads::init`, `ipc::start`, `window::create`) runs inside `connect_activate`.
- All window and tab state is `thread_local` on the GTK main thread (`window.rs`). Worker threads reach it only through `main::post` or `nova_core::host::on_ui`.
- `window::in_slot` / `in_owner` restore the previous current-window index only if that window still exists.
- `web` keeps a thread-local `OnceCell<Ctx>` set once in `web::init`: two WebContexts (pages, chrome) and three NetworkSessions (normal, private, chrome). Use before init panics with `web::init not called` (`web.rs:56-72`).
- Each web process is capped at 3072 MB RSS with kill threshold 1.0 (`web.rs:89-92`), on purpose: a leaking page once let systemd-oomd kill the whole session. WebKit's default never kills.
- `memory_watch` adds a second, GPU-aware limit because WebKit's cap counts only RSS: a hidden tab over `HEAVY_GPU_MB = 1024` of GPU buffers for `HEAVY_HIDDEN = 120 s` is hibernated early; any tab over `KILL_MB = 4096` RSS + GPU has its web process killed, and the crash page says "out of memory" (`window.rs:1835-1845`).
- The adblock dir is shared by all profiles (`xdg_root`, not the profile dir, `adblock.rs:37-40`); it is created and added to the WebKit sandbox before web processes start. The engine is rebuilt when older than `MAX_AGE` (24 h) or when the list set changes (`adblock.rs:29`, `adblock.rs:141`).
- The IPC socket lives in the profile data dir, so test profiles never reach the real instance. A stale socket is removed before bind; the socket is chmod `0600` because any client can run commands. Each request runs on the UI thread with a 5 s reply timeout (`ipc.rs:28-56`).
- `adblock::update` uses `try_lock`: a second concurrent update is skipped, not queued. If every list download fails, the old `engine.dat` and its age are kept so the next hourly check retries.
- Adblock list fetches use `curl --proto =https,http --proto-redir =https --max-filesize 64M`; `file://` URLs bypass curl.
- The adblock engine is an `RwLock<Option<Arc<Engine>>>`: readers clone the `Arc`, writers swap the whole value.
- `adblock-resources.json` is compiled in; invalid JSON panics at load (`adblock.rs:49`). It is generated by `scripts/gen-adblock-resources.mjs` in the CEF repo.
- `hints.js`, `adblock-scan.js` and `passwords.js` run in the isolated world `web::WORLD`, so page scripts cannot read them. `adblock-scan.js` and the `passwords.rs` wrapper post through `webkit.messageHandlers.nova` with a prefix (`adblock `, `pw `); `hints.js` posts nothing — Rust reads its results through `web::eval_world`.
- Scripts injected into every frame (`AllFrames`): `SAFARI_JS` (main world, start), `FOCUS_JS` (`web::WORLD`, end), `MEDIA_JS` (`web::WORLD`, start) (`web.rs:244-266`); dark mode's CSS and `NATIVE_DARK` (`web::WORLD`, end) when enabled (`darkmode.rs:97-100`); `adblock-scan.js`.
- `passwords.js` is injected into the top frame only, at document start, for `http(s)` URLs (`passwords.rs:24-30`).
- `:dark-toggle` writes `darkmode_excludes` atomically (plain text, one host per line), then calls `set_runtime_for` and `window::config_changed`.
- Anything injected into every frame needs a Speedometer A/B before it ships (project rule).

## Side Effects
- Re-execs itself (`Command::exec`) when `--profile` is given.
- Creates the profile data, state and cache dirs; chmods the data and state roots to `0700`.
- `NOVA_PW_SECRET` set means the process is the password editor helper: it runs `passwords::editor_main` and exits.
- `--import` writes `history.sqlite` and exits; `--check-config` reads the config and prints. A bad profile name exits with code 2.
- Cookies persist in `<data dir>/cookies.sqlite`; private windows use an ephemeral NetworkSession.
- Loads the extension `libnovabrowser_webkit_ext.so` from `NOVA_WEBKIT_EXT_DIR`, next to the binary, or `/usr/lib/novabrowser-webkit`.
- Binds the IPC Unix socket and spawns one thread per connection.
- Spawns the `adblock` thread (freshness check every 3600 s) and a `curl` subprocess per list; writes `list-<fnv>.txt`, `engine.dat` and `lists.txt` under the adblock data dir; sends `cfg` to every web-process extension.
- Reads/writes JSON state files in the profile data dir: `downloads.json`, `zoom.json`, `permissions.json`. Reads/writes plain-text host lists (one per line): `darkmode_excludes` and the https excludes file (`commands.rs:195-243`).
- `downloads::init` starts aria2 through `nova_core::aria2::start` (`downloads.rs:148`); `main` stops it on exit (`main.rs:243`).
- `history::start` opens `<data dir>/history.sqlite` (`main.rs:222`).
- Writes the GDK clipboard / primary selection.
- Runs user commands and userscripts (`spawn`); userscript output can open a new tab.
- `NOVA_KEY_TIMING` enables slow-key timing logs; `HOME` sets the default download dir.

## Verification
- Run: `cargo test --workspace` (unit tests are inline, e.g. `main.rs`, `adblock.rs`, `commands.rs`).
- GUI checks: see [`../tests/EGM.md`](../tests/EGM.md).
