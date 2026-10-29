# novabrowser-webkit

A WebKitGTK edition of [novabrowser](../novabrowser/novabrowser): the same keyboard-driven,
qutebrowser-style browser (modes, hints, palette, config.ts, password-manager
integration, adblock, userscripts), on **WebKitGTK 6.0 + GTK 4** instead of
CEF/Chromium.

Status: **feature-complete against novabrowser 0.1.8, in manual testing**
(`docs/manual-test.md`). Not released yet.
Speedometer 3.1 with our PGO WebKitGTK build (`webkitgtk-6.0-nova`): 27.25 vs
26.3 for the CEF edition (2026-10-28). Full feature checklist: `PROGRESS.md`.

## Install

Arch Linux, from this checkout (the AUR packages are not published yet):

```sh
cd packaging && NOVA_SRC=file://$PWD/.. makepkg -si
```

It runs on Arch's `webkitgtk-6.0`. Our own build, `webkitgtk-6.0-nova`
(`spikes/wk-build/`), is faster (PGO) and has the dark-mode patch that keeps dark
parts of light pages dark. That build needs an AVX2 (x86-64-v3) CPU.

For most web video install `gst-libav` (H.264/AAC), and `gst-plugins-bad` for
more codecs. Pages show a warning when media fails to decode.

Make it the default browser:

```sh
xdg-settings set default-web-browser novabrowser-webkit.desktop
```

It can run next to the CEF edition: binary, config and data dirs are all
named `novabrowser-webkit`. Keys, commands and `config.ts` are the same
([docs/user/](docs/user/README.md)); settings that only apply to the other
edition (`chromium.*` here) are refused.

## Why a WebKit edition

- No own engine build: H.264/AAC come from GStreamer (`gst-libav`), VA-API from
  `gst-plugin-va`, security updates from the distro's `webkitgtk-6.0` package.
  The CEF edition needs a self-built CEF on every Chromium bump.
- Small package: one Rust binary plus one web-process extension `.so`.
- Things CEF could not do become native: restoring back/forward history,
  private (ephemeral) sessions, moving a live tab to another window, our own
  TLS-error prompt, form-submit detection without page-world listeners.

## What it costs

Details in [docs/feature-map.md](docs/feature-map.md).

- **No trusted synthetic input.** CEF's DevTools `Input.dispatch*` has no
  WebKitGTK / GTK 4 equivalent. Hint clicks, `fake-key` and scrolling become
  JavaScript events (`isTrusted=false`).
- **Dark mode is WebKit's colour filter.** WebKit has no `WebContentsForceDark`;
  light pages get `-apple-color-filter: apple-invert-lightness()` (the hidden
  `ColorFilter` feature); pages that are already dark are left alone. Images keep
  their colours. `webkitgtk-6.0-nova` treats text and backgrounds separately, so
  dark headers and code blocks stay dark.
- **Safari identity.** The default user agent is macOS Safari
  (`content.user_agent`), so some sites offer Mac downloads and Cmd shortcuts.
- **Known site issue:** HDFC NetBanking shows an "upgrade your browser" message.
- **Memory:** about 1.6x the CEF edition with 5 real sites open (spike W14).
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
| [docs/user/](docs/user/README.md) | User guide: keys, commands, settings, passwords |
| [docs/manual-test.md](docs/manual-test.md) | Checks that need a person |

## License

GPL-3.0-or-later, like novabrowser (it reuses novabrowser and qutebrowser
code). WebKitGTK is LGPL-2.1 / BSD.
