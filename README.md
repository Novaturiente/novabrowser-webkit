# novabrowser-webkit

A WebKitGTK edition of [novabrowser](../novabrowser): the same keyboard-driven,
qutebrowser-style browser (modes, hints, palette, config.ts, password-manager
integration, adblock, userscripts), on **WebKitGTK 6.0 + GTK 4** instead of
CEF/Chromium.

Status: **planned, spikes running.** Plan from novabrowser 0.1.8 (`9574fca`).
Speedometer 3.1 with our PGO WebKitGTK build (`webkitgtk-6.0-nova`): 27.25 vs
26.3 for the CEF edition (2026-10-28). Full feature checklist: `PROGRESS.md`.

## Why a WebKit edition

- No own engine build: H.264/AAC come from GStreamer (`gst-libav`), VA-API from
  `gst-plugin-va`, security updates from the distro's `webkitgtk-6.0` package.
  The CEF edition needs a self-built CEF on every Chromium bump.
- Small package: one Rust binary plus one web-process extension `.so`.
- Things CEF could not do become native: restoring back/forward history,
  private (ephemeral) sessions, moving a live tab to another window, our own
  TLS-error page, form-submit detection without page-world listeners.

## What it costs

Details in [docs/feature-map.md](docs/feature-map.md).

- **No trusted synthetic input.** CEF's DevTools `Input.dispatch*` has no
  WebKitGTK / GTK 4 equivalent. Hint clicks, `fake-key` and scrolling become
  JavaScript events (`isTrusted=false`).
- **No forced dark mode.** WebKit has no `WebContentsForceDark`. Dark mode
  becomes `prefers-color-scheme` plus an injected stylesheet.
- **No Widevine.** Netflix, Spotify web and similar will not play.
- **Per-site cookie rules** shrink to one global policy plus WebKit ITP.
- **Site compatibility and speed** are WebKitGTK's, not Chrome's (measured in spike W9).

## Documents

| File | Contents |
|---|---|
| [PLAN.md](PLAN.md) | Stack, architecture, open decisions, spikes, phases, risks |
| [docs/feature-map.md](docs/feature-map.md) | Every CEF-edition feature: CEF way, WebKitGTK way, verdict |
| [docs/porting.md](docs/porting.md) | Module reuse plan, settings mapping, data migration |
| [AGENTS.md](AGENTS.md) | Rules for agent sessions in this repo |
| [PROGRESS.md](PROGRESS.md) | Where the work stands |

## License

GPL-3.0-or-later, like novabrowser (it reuses novabrowser and qutebrowser
code). WebKitGTK is LGPL-2.1 / BSD.
