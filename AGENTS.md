# novabrowser-webkit — agent instructions

Keyboard-driven browser: Rust core on WebKitGTK 6.0 + GTK 4. The WebKit
edition of `../novabrowser/novabrowser` (CEF). Design lives in `PLAN.md`, feature detail
in `docs/feature-map.md`, module reuse in `docs/porting.md`, state in
`PROGRESS.md`.

## Every session

1. **Start:** read `PROGRESS.md` (Now, Checklist, Decisions), then the
   `PLAN.md` sections for the next action. Don't re-derive anything listed
   under Decisions.
2. **During:** work on the item under "Now". New scope found → add a
   checklist item; don't absorb it silently.
3. **End:** update `PROGRESS.md` (tick items with evidence, dated decisions,
   session-log entry in IST, next "Now"), then commit
   `git add -A && git commit -m "<phase/spike>: <what>"`.
4. An item is `[x]` only when its pass condition (PLAN.md §5/§6) was checked
   this session. Partial → `[~]` with what's left.

## Rules

- `../novabrowser/novabrowser` is the reference implementation: read the matching module
  before porting a feature; keep its behaviour, names and messages unless
  `docs/feature-map.md` says otherwise. Never edit it from this repo.
- Copied files keep `// from novabrowser <commit> src/<file>` on line 1.
- `../qutebrowser` (if present) is read-only reference (GPL-3.0).
- Anything injected into every frame gets a Speedometer A/B before it ships.
- No CDP here: automated checks use the IPC socket (`--cmd`, `--query`) and
  real keys (`wtype` / compositor `send_shortcut`).
- Run GUI tests with the `process` tool (never `&`); stop them when done.
- Spikes: own dir `spikes/wN-<name>/`, own `--profile`/data dir, never touch
  the user's real profile.

## Build / run (once Phase 1 exists)

```sh
# needs: sudo pacman -S webkitgtk-6.0   (user runs it)
cargo build --release
target/release/novabrowser-webkit https://example.com
```
