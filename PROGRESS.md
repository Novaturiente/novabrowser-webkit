# novabrowser-webkit — progress

## Now

- **Phase:** 0 (planning done, no code)
- **Next action:** user decides D1–D6 (PLAN.md §4) and installs
  `webkitgtk-6.0`; then spikes W9 + W10 first (they decide whether to go on),
  then W1–W8, W11.
- **Blockers:** `webkitgtk-6.0` not installed (needs sudo).

## Checklist

### Phase 0 — spikes (PLAN.md §5)
- [ ] W1 Wayland basics + media
- [ ] W2 layout
- [ ] W3 key interception
- [ ] W4 script world bridge
- [ ] W5 input without CDP
- [ ] W6 frame-targeted reply
- [ ] W7 adblock
- [ ] W8 session state
- [~] W9 baseline — Speedometer 3.1 only (2026-10-28, `spikes/w9-baseline`, `results/speedometer.json`): bare WebKitGTK 6.0 2.54.2 = 19.4 / 19.0 / 19.2, median **19.2**, all valid; rerun back-to-back 18.4/19.2/19.5. Same session, installed CEF novabrowser 0.1.8 bare profile: 23.4/21.3/22.9, median **22.9** → WebKit 16% lower. Left: Chrome in the same session, startup, idle RSS, YouTube scroll.
- [ ] W10 sign-in and compat
- [ ] W11 permissions, TLS, dialogs

### Phases 1–10 (PLAN.md §6)
- [ ] 1 Skeleton
- [ ] 2 Modal core
- [ ] 3 Palette, completion, history
- [ ] 4 Hints, find, yank
- [ ] 5 config.ts and theme
- [ ] 6 External tools
- [ ] 7 Privacy, blocking, prompts
- [ ] 8 Sessions and daily driving
- [ ] 9 Packaging
- [ ] 10 Passwords

## Decisions

Newest first.

- 2026-10-28 — Project created as a separate repo next to `../novabrowser`;
  plan written from novabrowser 0.1.8 (`9574fca`). D1–D6 open.

## Session log

- 2026-10-28 (IST, 18:15) — Built spike `spikes/w9-baseline` (gtk4 0.11 + `v4_18`, webkit6 0.6 + `v2_52`; gtk4 needs a `v4_10`+ feature or webkit6 fails on `gtk::Accessible`). `--speedometer` loads Speedometer 3.1, clicks Start after 3 s, polls via `evaluate_javascript`, prints result JSON. Median 19.2 (3 valid runs). Next: rerun CEF/Chrome on the same power profile for a fair gap.

- 2026-10-28 (IST) — Read the whole CEF edition (all `src/` modules, UI pages,
  PLAN/PROGRESS decisions); checked WebKitGTK 6.0 API against webkitgtk.org
  docs (2.54.2), webkit2gtk-4.1 headers and feature list on novarch, and the
  Rust crates on crates.io. Wrote README, PLAN, docs/feature-map, docs/porting.
  Next: decisions D1–D6, install webkitgtk-6.0, spikes W9/W10.
