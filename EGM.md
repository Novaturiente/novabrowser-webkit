# novabrowser-webkit Blueprint

## System Overview
Keyboard-driven, qutebrowser-style browser in Rust on WebKitGTK 6.0 + GTK 4:
the WebKit edition of novabrowser (CEF). Two processes matter:

- **UI process** (`src/`, binary `novabrowser-webkit`): windows, tabs, modes,
  commands, chrome overlays (`ui/*.html`, compiled in), IPC control socket,
  adblock list management, passwords, downloads.
- **Web processes** (WebKit's), each loading the extension (`ext/`): network
  blocking, HTTPS upgrade and headers per request.

Engine-neutral logic (config.ts, paths, history, sessions, vault CLIs, URL
matching) comes from the external `nova-core` crate at a pinned git rev. Design:
`PLAN.md`; features: `docs/feature-map.md`; reuse: `docs/porting.md`; state:
`PROGRESS.md`; user docs: `docs/user/`. Human overview: `README.md`.

## Subdirectories & System Boundaries
- src: [`./src/EGM.md`](./src/EGM.md) — UI process crate: CLI, GTK app, window/tab state, commands, modes, IPC socket, adblock engine builds, passwords, downloads, permissions.
- ext: [`./ext/EGM.md`](./ext/EGM.md) — web-process extension crate: per-request adblock, HTTPS-only, headers, memory stats.
- tests: [`./tests/EGM.md`](./tests/EGM.md) — end-to-end GUI scripts over the IPC socket and real input.
- packaging: [`./packaging/EGM.md`](./packaging/EGM.md) — Arch PKGBUILDs, AUR recipes, release script, desktop entry, icons.
- `ui/` — HTML/JS overlays (tabbar, palette, keyhint, popup, prompt, toast), compiled into `src/web.rs`; no contract.
- `docs/` — design and user documentation; no code.
- `spikes/` — throwaway experiments, excluded from the workspace and from these contracts.

## Global Invariants
- UI process and extension share three channels: init user data (`ext_config(true)`), WebKit user messages (`cfg` extension-wide; `nav` and `mem` per page), and the adblock dir that the UI process adds to the web-process sandbox (`src/web.rs:111-119`). `nav` must reach the extension before the navigation's first request.
- Both crates pin the same `nova-core` rev; bump them together.
- The installed extension path `/usr/lib/novabrowser-webkit/libnovabrowser_webkit_ext.so` is what the binary searches; packaging must keep it.
- Every profile (`--profile`) has its own data dir and IPC socket, except the adblock lists and `engine.dat`, which all profiles share (`src/adblock.rs:37-40`). Tests and spikes use throwaway XDG dirs and never the user's real profile.
- Two memory limits: WebKit's per-process cap of 3072 MB RSS (`src/web.rs:89-92`), and `memory_watch` in `src/window.rs` (hibernate hidden tabs over 1024 MB GPU after 120 s; kill tabs over 8192 MB RSS + GPU, or the biggest tab when swap is over 75% used or MemAvailable is under 1 GB).
- No CDP: automated checks use `--cmd` / `--query` / `--key` and real keys.
- Anything injected into every frame gets a Speedometer A/B before it ships.
- Logic shared with novabrowser lives in `nova-core`, never copied; ported modules are WebKit rewrites without a source header. Only a file copied verbatim keeps `// from novabrowser <commit> src/<file>` on line 1 (AGENTS.md).

## Global Side Effects
- Profile data, state and cache dirs under XDG (`novabrowser-webkit`), including `cookies.sqlite`, history, sessions, adblock lists and `engine.dat`.
- A Unix control socket per profile (mode `0600`).
- Network: page loads, hourly adblock list refresh via `curl`.
- External programs: `curl`, password vault CLIs, user commands and userscripts. aria2 is started by `downloads::init` and stopped on exit, both through `nova_core::aria2`.

## System Verification
- Run: `cargo test --workspace`
- GUI: `cargo build --release && tests/smoke.sh`
