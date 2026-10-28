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
- [~] W9 baseline — Speedometer 3.1 only (2026-10-28, `spikes/w9-baseline`, `results/speedometer.json`): bare WebKitGTK 6.0 2.54.2 = 19.4 / 19.0 / 19.2, median **19.2**, all valid; rerun back-to-back 18.4/19.2/19.5. Same session, installed CEF novabrowser 0.1.8 bare profile: 23.4/21.3/22.9, median **22.9** → WebKit 16% lower. Own PGO+v3+ThinLTO WebKitGTK build: **23.95** (+26% over Arch's 18.95, sandbox off for both). Left: Chrome in the same session, startup, idle RSS, YouTube scroll; PGO build with sandbox (needs a system install) and a non-Speedometer training set.
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

- 2026-10-28 (IST, 23:10) — User installed `webkitgtk-6.0-nova` 2.54.2-1 (PGO + v3 + ThinLTO, prefix /usr, built by `spikes/wk-build/build.sh pkg` + `pkg/PKGBUILD`; peak 13.2 GB under the 20 GB cap). It runs with the bubblewrap sandbox on, but not inside novacode's own tool sandbox (nested bwrap: web process dies, "Error writing credentials to socket"), so benchmarks now run with `unsandboxed`. Interleaved A/B (`ab-cef.sh`, `results/ab-cef.jsonl`, 4 rounds, platform profile performance, EPP default): WebKit 27.1/27.2/27.3/27.3 median **27.25**, CEF novabrowser-bin 0.1.8 bare profile 26.4/26.2/26.4/26.1 median **26.3** → WebKit +3.6%. Earlier numbers (19.x / 22.9 / 23.95) were taken inside the tool sandbox and are ~15% low in absolute terms; ratios measured within one setup still hold. Viewports differ: WebKit tiled 1420×1146, CEF floated 1400×920. CEF runs with adblock + passwords.js, WebKit spike with neither.

- 2026-10-28 (IST, 21:00) — Env knob A/B (`ab.sh`, `results/ab-env.jsonl`, 3 rounds): base 19.2, `WEBKIT_SKIA_GPU_PAINTING_THREADS=4` 19.2, `WEBKIT_FORCE_VBLANK_TIMER=1` 19.5 (noise), CPU Skia 8 threads 18.4, `WEBKIT_DISABLE_COMPOSITING_MODE` 18.3, `WEBKIT_DISABLE_DMABUF_RENDERER` 13.4 → keep defaults. Own build on novahome (`spikes/wk-build/`: clang 23, `-march=x86-64-v3 -O2`, IR PGO trained on 3 Speedometer runs, ThinLTO; capped at 20 GB via `systemd-run --user --scope MemoryMax=20G`, `-j8`; peak 12.5 GB, ~42 + ~45 min) installed to `~/.local/opt/wk-pgo` on both hosts. A/B (`ab-build.sh`, `results/ab-build.jsonl`, 4 interleaved rounds): Arch build 18.8 (sandbox) / 18.95 (no sandbox), PGO build **23.95** (no sandbox) = +26%, vs CEF 0.1.8 22.9 (earlier session) +5%. Caveats: profile trained on Speedometer itself (real-site gain likely smaller); PGO build only run without the bubblewrap sandbox; no functional check (video, sites) yet.

- 2026-10-28 (IST, 18:15) — Built spike `spikes/w9-baseline` (gtk4 0.11 + `v4_18`, webkit6 0.6 + `v2_52`; gtk4 needs a `v4_10`+ feature or webkit6 fails on `gtk::Accessible`). `--speedometer` loads Speedometer 3.1, clicks Start after 3 s, polls via `evaluate_javascript`, prints result JSON. Median 19.2 (3 valid runs). Next: rerun CEF/Chrome on the same power profile for a fair gap.

- 2026-10-28 (IST) — Read the whole CEF edition (all `src/` modules, UI pages,
  PLAN/PROGRESS decisions); checked WebKitGTK 6.0 API against webkitgtk.org
  docs (2.54.2), webkit2gtk-4.1 headers and feature list on novarch, and the
  Rust crates on crates.io. Wrote README, PLAN, docs/feature-map, docs/porting.
  Next: decisions D1–D6, install webkitgtk-6.0, spikes W9/W10.
