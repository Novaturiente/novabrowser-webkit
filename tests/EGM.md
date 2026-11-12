# tests — end-to-end GUI checks

## Overview
Bash and Python scripts that start a built browser with throwaway XDG dirs and
drive it through its IPC socket (`--cmd`, `--query`, `--key`) and real input
tools. No CDP. `lib.sh` is the shared harness for the bash scripts. `pages/`
holds local HTML fixtures, served on `127.0.0.1` or used as `data:` URLs.

| Script | Checks |
|--------|--------|
| `smoke.sh` | Broad IPC smoke test (tabs, hints, sessions, adblock, …) |
| `features.sh` | Phase 8 features (restore, hibernation, profiles, import, …) |
| `passwords.sh` | Phase 10 passwords with the `pass` backend |
| `caret.sh` | `gi` and mouse-click caret position |
| `memwatch.sh` | `memory_watch`: 1.5 GB hidden tab hibernates, 4.4 GB tab killed |
| `idle-net.py` | Remote network peers while idle on `about:blank` |

## Interface
- Run a bash script as `tests/<name>.sh [binary]`; binary defaults to `target/release/novabrowser-webkit`.
- `tests/idle-net.py [binary] [seconds=90]`; `IDLE_PROFILE` reuses a profile dir.
- `lib.sh` provides: `bin`, `tmp`, exported `XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_STATE_HOME` / `XDG_CACHE_HOME`, `ok`, `fail`, `q` (`--query state`), `page(title)`, `wait_for(regex, [secs=10])`, `ntabs`, `start(args...)`, `stop([cmd=quit])`, `serve(dir)`, `finish`, and an EXIT trap.
- Fixtures: `pages/caret.html` (caret.sh), `pages/gpu-hog.html?mb=N` (memwatch.sh), `pages/late-dark.html` (smoke.sh dark mode), `pages/login.html` and `pages/signup.html` (passwords.sh). `pages/mixed-dark.html` is used by no script: a by-eye check for the WebKit dark-mode patch.

## Invariants
- `XDG_*` are redirected into `tmp` before `start`, so tests never touch the real profile; a `welcomed` marker suppresses the welcome tab.
- End the browser with `stop`: it asserts exit code 0 within 10 s.
- A `panicked at` line in `$tmp/stderr` makes `finish` fail; the exit code is the fail count. `smoke.sh` uses neither `finish` nor `serve`: it does its own panic grep, summary and `exit $fails`, and starts its HTTP servers inline.
- `serve` picks a random port in 20000-39999 without a collision check.
- `memwatch.sh` sizes (1.5 GB, 4.4 GB) are chosen to cross the real limits in `src/window.rs` (`HEAVY_GPU_MB = 1024` after 120 s hidden; `KILL_MB = 4096` RSS + GPU).
- The crash test in `smoke.sh` kills only `WebKitWebProcess` children of the test instance (matched by walking `/proc` ppid).
- `passwords.sh` exits 0 with `skip` when `pass`, `gpg` or `openssl` is missing.
- `caret.sh` needs `ydotool` with `ydotoold`, Hyprland (`hyprctl`, `jq`) and scale 1.
- Every script starts the GUI browser and needs a live Wayland/X11 session; run them with the `process` tool, not `&`.
- `features.sh` needs `sqlite3` and `python3`; `passwords.sh` needs `python3` too.

## Side Effects
- Creates and deletes `/tmp/nova-wk-test.XXXXXX`. `idle-net.py` creates and deletes `/tmp/nova-idle-*`; an `IDLE_PROFILE` dir is reused and kept.
- Starts the browser and a `python3 -m http.server` on `127.0.0.1`.
- `smoke.sh` sends `kill -9` to the test instance's web processes and writes userscripts, sessions, quickmarks, bookmarks and exclude files in the temp XDG dirs.
- `features.sh` creates SQLite history fixtures, may start `aria2c`, runs `profile-open work` (a second browser instance with its own socket), and writes fake CEF-edition data into `$XDG_CONFIG_HOME/novabrowser` and `$XDG_DATA_HOME/novabrowser` for the import check.
- `passwords.sh` creates a temp GnuPG home and pass store, generates a key with an empty passphrase, starts a self-signed HTTPS server, and kills `gpg-agent` on exit.
- `caret.sh` moves and clicks the real pointer.
- `memwatch.sh` allocates about 1.5 GB and 4.4 GB of WebGL memory and runs about 4 minutes.

## Verification
- Run: `cargo build --release && tests/smoke.sh` (other scripts the same way).
